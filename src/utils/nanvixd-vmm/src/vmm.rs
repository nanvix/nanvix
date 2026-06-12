// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Builds and runs a KVM-backed partition that boots the Nanvix guest.
//!
//! The flow mirrors OpenVMM's host VMMs: create a proto-partition from the KVM
//! hypervisor, allocate and map guest RAM, load the guest, set the boot register
//! state, and run the single vCPU, dispatching its port-I/O exits to the
//! [`NanvixDevice`].

// UNSAFETY: needed to map guest memory into the partition.
#![expect(unsafe_code)]

use crate::{
    device::{
        ExitReason,
        NanvixDevice,
    },
    ikc::IkcBridge,
    load,
    load::GuestImage,
};
use anyhow::Context as _;
use chipset_device_resources::BSP_LINT_LINE_SET;
use chipset_resources::{
    pic::PicDeviceHandle,
    pit::PitDeviceHandle,
};
use guestmem::GuestMemory;
use hvdef::Vtl;
use state_unit::StateUnits;
use std::{
    future::{
        poll_fn,
        Future,
    },
    path::PathBuf,
    pin::pin,
    sync::{
        Arc,
        Weak,
    },
    task::{
        Context,
        Waker,
    },
};
use virt::{
    vp::AccessVpState as _,
    BindProcessor,
    Hypervisor,
    Partition,
    PartitionConfig,
    PartitionMemoryMapper,
    Processor,
    ProtoPartition,
    ProtoPartitionConfig,
    StopVpSource,
    VpIndex,
};
use vm_resource::{
    IntoResource as _,
    ResourceId as _,
    ResourceResolver,
};
use vm_topology::{
    memory::MemoryLayout,
    processor::{
        x86::X2ApicState,
        TopologyBuilder,
    },
};
use vmcore::{
    vm_task::{
        SingleDriverBackend,
        VmTaskDriverSource,
    },
    vmtime::{
        VmTime,
        VmTimeKeeper,
    },
};
use vmm_core::emuplat::apic::ApicLintLineTarget;
use vmotherboard::{
    options::{
        BaseChipsetDevices,
        BaseChipsetFoundation,
    },
    BaseChipsetBuilder,
    BaseChipsetBuilderOutput,
    Chipset,
    ChipsetDeviceHandle,
};

/// Runs the Nanvix guest to completion and returns its exit code.
///
/// `stdin` is the buffered host input forwarded to the guest, and `console`
/// receives the guest's kernel console output. When `mount_directory` is set the
/// guest's HostFS requests are served by the reused `hostfsd` daemon rooted at
/// that path; when `networking` is enabled, networking IKC traffic is served by
/// the reused `networkd` daemon.
pub async fn run(
    driver: pal_async::DefaultDriver,
    image: GuestImage,
    io: Box<dyn crate::io::GuestIo>,
    console: crate::ConsoleSink,
    mount_directory: Option<PathBuf>,
    networking: bool,
) -> anyhow::Result<u16> {
    // Single-vCPU x86 topology, advertising x2apic support like a modern PC.
    let processor_topology = TopologyBuilder::new_x86()
        .x2apic(X2ApicState::Supported)
        .build(1)
        .context("failed to build processor topology")?;

    let memory_layout = MemoryLayout::new(image.mem_size, &[], &[], &[], None)
        .context("failed to build memory layout")?;

    // Task/driver plumbing shared by the vmtime keeper, the chipset's device
    // tasks (the PIT poll loop), and the state-unit machinery.
    let driver_source = VmTaskDriverSource::new(SingleDriverBackend::new(driver.clone()));
    let mut state_units = StateUnits::new();

    // The vmtime keeper drives the virtual clock that the PIT samples. It is
    // owned by a state unit so that the device model can start/stop it in
    // lockstep with the rest of the chipset.
    let vmtime_keeper = VmTimeKeeper::new(&driver_source.simple(), VmTime::from_100ns(0));
    let vmtime_source = vmtime_keeper
        .builder()
        .build(&driver_source.simple())
        .await
        .context("failed to build vmtime source")?;
    let vmtime_unit = state_units
        .add("vmtime")
        .spawn(driver_source.simple(), |recv| {
            let mut vmtime_keeper = vmtime_keeper;
            async move {
                vmm_core::vmtime_unit::run_vmtime(&mut vmtime_keeper, recv).await;
                vmtime_keeper
            }
        })
        .context("failed to spawn vmtime unit")?;

    let mut hv = virt_kvm::Kvm::new().context("failed to create KVM hypervisor")?;
    let proto = hv
        .new_partition(ProtoPartitionConfig {
            processor_topology: &processor_topology,
            hv_config: None,
            vmtime: &vmtime_source,
            isolation: virt::IsolationType::None,
        })
        .context("failed to create proto partition")?;

    // Back guest RAM with a lazily-committed anonymous mapping: pages are
    // demand-zero and fault in only on first touch, so a large guest RAM size
    // costs almost nothing to "allocate" (matching the host `mmap` behavior the
    // Nanvix uservm relies on, rather than eagerly zeroing the whole region).
    let ram = sparse_mmap::SparseMapping::new(image.mem_size as usize)
        .context("failed to reserve guest memory")?;
    ram.alloc(0, ram.len())
        .context("failed to allocate guest memory")?;
    let ram_ptr = ram.as_ptr().cast::<u8>();
    let guest_memory = GuestMemory::new("nanvix-guest-ram", ram);

    let (partition, vps) = proto
        .build(PartitionConfig {
            mem_layout: &memory_layout,
            guest_memory: &guest_memory,
            cpuid: &[],
            vtl0_alias_map: None,
        })
        .context("failed to build partition")?;

    let partition = Arc::new(partition);

    // Map guest RAM into the partition.
    for r in memory_layout.ram() {
        let range = r.range;
        // SAFETY: `ram_ptr` is the base of the guest RAM mapping, kept alive by
        // `guest_memory` until the vCPU thread joins below.
        unsafe {
            partition
                .memory_mapper(Vtl::Vtl0)
                .map_range(
                    ram_ptr.add(range.start() as usize).cast(),
                    range.len() as usize,
                    range.start(),
                    true,
                    true,
                )
                .context("failed to map guest memory")?;
        }
    }

    // Load the kernel, initrd, RAMFS, and control/pvclock pages.
    let loaded = load::load(&guest_memory, &image)?;

    // Build the real-mode boot register state.
    let regs = boot_registers(&loaded);

    // Build the legacy chipset: an 8259 PIC and an 8254 PIT, reusing the
    // in-tree device implementations. The PIT raises its timer line into the
    // PIC, whose `ready` output is wired (below) into the BSP's APIC LINT0 so
    // the guest sees periodic IRQ 0 interrupts in virtual-wire mode.
    let resolver = ResourceResolver::new();
    let noop_handler = Arc::new(NoopChipsetHandler);
    let BaseChipsetBuilderOutput {
        chipset_builder,
        device_interfaces: _,
    } = BaseChipsetBuilder::new(
        BaseChipsetFoundation {
            is_restoring: false,
            untrusted_dma_memory: guest_memory.clone(),
            trusted_vtl0_dma_memory: guest_memory.clone(),
            power_event_handler: noop_handler.clone(),
            debug_event_handler: noop_handler.clone(),
            vmtime: &vmtime_source,
            vmtime_unit: vmtime_unit.handle(),
            doorbell_registration: None,
        },
        BaseChipsetDevices::empty(),
    )
    .with_device_handles(vec![
        ChipsetDeviceHandle {
            name: PicDeviceHandle::ID.to_owned(),
            resource: PicDeviceHandle.into_resource(),
        },
        ChipsetDeviceHandle {
            name: PitDeviceHandle::ID.to_owned(),
            resource: PitDeviceHandle.into_resource(),
        },
    ])
    .build(&driver_source, &state_units, &resolver)
    .await
    .context("failed to build chipset")?;

    // Route the BSP's local APIC LINT lines so the PIC's output reaches the
    // guest as an interrupt (LINT0 in ExtINT mode, configured per-vCPU below).
    chipset_builder.add_external_line_target(
        BSP_LINT_LINE_SET,
        0..=1,
        0,
        "bsp",
        Arc::new(ApicLintLineTarget::new(partition.clone(), Vtl::Vtl0)),
    );

    let (chipset, _chipset_devices) = chipset_builder.build().context("failed to build chipset")?;

    // Start the device model (PIC/PIT) and the vmtime clock.
    state_units.start().await;

    // Run the single vCPU on its own thread; KVM requires the vCPU ioctls to be
    // issued from the thread that owns the vCPU file descriptor.
    let [vp] = vps
        .try_into()
        .ok()
        .context("expected exactly one vCPU binder")?;

    // The vCPU thread reports its result over a oneshot channel rather than via
    // a blocking `join`, so this task can keep yielding to the single-threaded
    // executor (which must keep servicing the PIT poll and vmtime tasks).
    let (result_tx, result_rx) = mesh::oneshot();
    let vp_thread = run_vp(
        partition.clone(),
        vp,
        regs,
        guest_memory.clone(),
        chipset.clone(),
        io,
        console,
        mount_directory,
        networking,
        result_tx,
    );

    let exit = result_rx
        .await
        .map_err(|_| anyhow::anyhow!("vcpu thread panicked"))?;
    let _ = vp_thread.join();

    // Stop the device model and release the partition.
    state_units.stop().await;
    drop(chipset);
    drop(partition);

    match exit? {
        ExitReason::Shutdown(code) => Ok(code),
        ExitReason::Snapshot => Ok(0),
    }
}

/// No-op power/debug event handler for the chipset.
///
/// The Nanvix guest drives shutdown through its own control port, and there is
/// no debugger attached, so neither chipset event is acted upon.
struct NoopChipsetHandler;

impl vmotherboard::PowerEventHandler for NoopChipsetHandler {
    fn on_power_event(&self, _evt: vmotherboard::PowerEvent) {}
}

impl vmotherboard::DebugEventHandler for NoopChipsetHandler {
    fn on_debug_break(&self, _vp: Option<u32>) {}
}

/// Builds the initial register state for booting the Nanvix kernel.
///
/// The kernel's entry point is 16-bit real-mode trampoline code at physical
/// `0x8000`. The guest expects the VMM identification magic in `RAX` and an
/// encoded description of the initrd in `RBX`.
fn boot_registers(loaded: &load::LoadedGuest) -> virt::vp::Registers {
    use virt::x86::{
        SegmentRegister,
        TableRegister,
    };

    // Real-mode code segment with a zero base, so the linear fetch address
    // equals the instruction pointer.
    let cs = SegmentRegister {
        base: 0,
        limit: 0xffff,
        selector: 0,
        attributes: 0x9b,
    };
    let data = SegmentRegister {
        base: 0,
        limit: 0xffff,
        selector: 0,
        attributes: 0x93,
    };
    let tr = SegmentRegister {
        base: 0,
        limit: 0xffff,
        selector: 0,
        attributes: 0x8b,
    };
    let ldtr = SegmentRegister {
        base: 0,
        limit: 0xffff,
        selector: 0,
        attributes: 0x82,
    };
    let table = TableRegister {
        base: 0,
        limit: 0xffff,
    };

    virt::vp::Registers {
        rip: loaded.entry,
        rax: u64::from(config::microvm::DEFAULT_BOOT_MAGIC),
        rbx: encode_initrd(loaded.initrd_base, loaded.initrd_size),
        // Bit 1 is reserved-as-one; interrupts remain disabled until the kernel
        // installs its IDT and enables them itself.
        rflags: 0x2,
        cs,
        ds: data,
        es: data,
        fs: data,
        gs: data,
        ss: data,
        tr,
        ldtr,
        gdtr: table,
        idtr: table,
        // Standard x86 reset control-register state: protected mode disabled.
        cr0: x86defs::X64_CR0_ET | x86defs::X64_CR0_CD | x86defs::X64_CR0_NW,
        ..Default::default()
    }
}

/// Encodes the initrd base address and size into the value placed in `RBX`.
///
/// The low bits (below the alignment of the initrd base) hold the size in 4 KiB
/// pages; the high bits hold the base address.
fn encode_initrd(base: u64, size: u64) -> u64 {
    if size == 0 {
        return 0;
    }
    let nzeros = (config::microvm::DEFAULT_INITRD_BASE as u64).trailing_zeros();
    let mask = (1u64 << nzeros) - 1;
    (base & !mask) | ((size >> 12) & mask)
}

/// Spawns the vCPU thread, reporting the guest exit reason over `result_tx`.
///
/// The returned join handle is only used for cleanup; the result is delivered
/// through the channel so the caller can await it without blocking the executor
/// thread that services the chipset's timer tasks.
#[expect(clippy::too_many_arguments)]
fn run_vp(
    partition: Arc<dyn RequestYield>,
    mut binder: impl 'static + BindProcessor + Send,
    regs: virt::vp::Registers,
    guest_memory: GuestMemory,
    chipset: Arc<Chipset>,
    io: Box<dyn crate::io::GuestIo>,
    console: crate::ConsoleSink,
    mount_directory: Option<PathBuf>,
    networking: bool,
    result_tx: mesh::OneshotSender<anyhow::Result<ExitReason>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let result = run_vp_inner(
            &partition,
            &mut binder,
            regs,
            guest_memory,
            chipset,
            io,
            console,
            mount_directory,
            networking,
        );
        result_tx.send(result);
    })
}

/// Body of the vCPU thread, factored out so the result can be captured and sent
/// over the oneshot channel by [`run_vp`].
#[expect(clippy::too_many_arguments)]
fn run_vp_inner(
    partition: &Arc<dyn RequestYield>,
    binder: &mut (impl 'static + BindProcessor + Send),
    regs: virt::vp::Registers,
    guest_memory: GuestMemory,
    chipset: Arc<Chipset>,
    io: Box<dyn crate::io::GuestIo>,
    console: crate::ConsoleSink,
    mount_directory: Option<PathBuf>,
    networking: bool,
) -> anyhow::Result<ExitReason> {
    let vp_index = VpIndex::BSP;
    let mut vp = binder.bind().context("failed to bind vcpu")?;

    // Install the boot register state and configure the local APIC for
    // legacy "virtual wire" mode.
    {
        let mut state = vp.access_state(Vtl::Vtl0);
        state
            .set_registers(&regs)
            .context("failed to set registers")?;

        // The Nanvix guest boots without an ACPI MADT and never initializes
        // the local APIC, expecting legacy 8259 PIC interrupts on the INTR
        // pin. With KVM's split IRQ chip those interrupts are delivered
        // through the in-kernel APIC's LINT0 in ExtINT mode, so configure
        // that here (as platform firmware normally would): software-enable
        // the APIC and route LINT0 as an unmasked ExtINT source.
        let mut apic = state.apic().context("failed to read apic state")?;
        let mut apic_regs = virt::vp::ApicRegisters::from_array(apic.registers);
        apic_regs.svr |= 0x100;
        apic_regs.lvt_lint0 = 0x700;
        apic.registers = *apic_regs.as_array();
        state.set_apic(&apic).context("failed to set apic state")?;

        state.commit().context("failed to commit vp state")?;
    }

    let stop = StopVpSource::new();
    let bridge = IkcBridge::new(io, mount_directory, networking);
    let device = NanvixDevice::new(guest_memory, bridge, &stop, chipset, console);
    let result = block_on(async {
        let mut run = pin!(vp.run_vp(stop.checker(), &device));
        poll_fn(|cx| {
            let waker = Waker::from(Arc::new(VpWaker::new(
                Arc::downgrade(partition),
                vp_index,
                cx.waker().clone(),
            )));
            run.as_mut().poll(&mut Context::from_waker(&waker))
        })
        .await
    });

    match result {
        Err(reason) => match device.take_exit() {
            Some(exit) => Ok(exit),
            None => Err(anyhow::anyhow!("guest faulted: {reason:?}")),
        },
    }
}

/// Trait object wrapper so the vCPU waker can request a yield without naming the
/// concrete partition type.
trait RequestYield: Send + Sync {
    /// Forces the `run_vp` call to yield to the scheduler.
    fn request_yield(&self, vp_index: VpIndex);
}

impl<T: Partition> RequestYield for T {
    fn request_yield(&self, vp_index: VpIndex) {
        self.request_yield(vp_index)
    }
}

/// Waker that nudges a blocked vCPU out of its run call when woken.
struct VpWaker {
    partition: Weak<dyn RequestYield>,
    vp: VpIndex,
    inner: Waker,
}

impl VpWaker {
    fn new(partition: Weak<dyn RequestYield>, vp: VpIndex, waker: Waker) -> Self {
        Self {
            partition,
            vp,
            inner: waker,
        }
    }
}

impl std::task::Wake for VpWaker {
    fn wake_by_ref(self: &Arc<Self>) {
        if let Some(partition) = self.partition.upgrade() {
            partition.request_yield(self.vp);
        }
        self.inner.wake_by_ref();
    }

    fn wake(self: Arc<Self>) {
        self.wake_by_ref()
    }
}

/// Minimal single-thread executor: polls `fut` to completion, parking the
/// current thread between wakeups. Equivalent to `futures::executor::block_on`
/// but without pulling in the `futures` crate.
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    use std::task::Poll;

    struct ThreadWaker(std::thread::Thread);
    impl std::task::Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    let mut fut = pin!(fut);
    let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park(),
        }
    }
}
