// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::core::sync::atomic::{
    AtomicU32,
    Ordering,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

pub type InterruptHandler = unsafe fn(InterruptNumber);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u32)]
pub enum InterruptNumber {
    Timer = 0,
    Keyboard = 1,
    Com2 = 3,
    Com1 = 4,
    Lpt2 = 5,
    Floppy = 6,
    Lpt1 = 7,
    Cmos = 8,
    #[cfg(feature = "microvm")]
    Ikc = 9,
    #[cfg(not(feature = "microvm"))]
    Free1 = 9,
    Free2 = 10,
    Free3 = 11,
    Mouse = 12,
    Coprocessor = 13,
    PrimaryAta = 14,
    SecondaryAta = 15,
}

impl InterruptNumber {
    #[cfg(feature = "microvm")]
    const IRQ9: Self = Self::Ikc;
    #[cfg(not(feature = "microvm"))]
    const IRQ9: Self = Self::Free1;

    pub const VALUES: [Self; 15] = [
        Self::Timer,
        Self::Keyboard,
        Self::Com2,
        Self::Com1,
        Self::Lpt2,
        Self::Floppy,
        Self::Lpt1,
        Self::Cmos,
        Self::IRQ9,
        Self::Free2,
        Self::Free3,
        Self::Mouse,
        Self::Coprocessor,
        Self::PrimaryAta,
        Self::SecondaryAta,
    ];
}

const INTERRUPT_VECTOR_LENGTH: usize = 16;
static mut INTERRUPT_VECTOR: [Option<InterruptHandler>; INTERRUPT_VECTOR_LENGTH] =
    [None; INTERRUPT_VECTOR_LENGTH];
static ACTIVE_IAR: AtomicU32 = AtomicU32::new(u32::MAX);

pub struct InterruptController;

impl InterruptController {
    pub const fn new() -> Self {
        Self
    }

    pub fn ack(&mut self, intnum: InterruptNumber) -> Result<(), Error> {
        if intnum == InterruptNumber::Timer {
            rearm_timer();
        }
        let iar: u64 = u64::from(ACTIVE_IAR.swap(u32::MAX, Ordering::Relaxed));
        if iar == u64::from(u32::MAX) {
            return Err(Error::new(ErrorCode::InvalidArgument, "no active GIC interrupt"));
        }
        unsafe {
            core::arch::asm!("msr ICC_EOIR1_EL1, {iar}", iar = in(reg) iar, options(nostack));
        }
        Ok(())
    }

    pub fn unmask(&mut self, intnum: InterruptNumber) -> Result<(), Error> {
        let intid: u32 = interrupt_id(intnum)?;
        let register: *mut u32 = if intid < 32 {
            (::config::microvm::DEFAULT_GICR_BASE + 0x1_0100) as *mut u32
        } else {
            (::config::microvm::DEFAULT_GICD_BASE + 0x100 + ((intid as usize / 32) * 4)) as *mut u32
        };
        unsafe { core::ptr::write_volatile(register, 1u32 << (intid % 32)) };
        Ok(())
    }

    pub fn set_handler(
        &mut self,
        intnum: InterruptNumber,
        handler: Option<InterruptHandler>,
    ) -> Result<(), Error> {
        unsafe { INTERRUPT_VECTOR[intnum as usize] = handler };
        Ok(())
    }

    pub fn get_handler(&self, intnum: InterruptNumber) -> Result<Option<InterruptHandler>, Error> {
        unsafe { Ok(INTERRUPT_VECTOR[intnum as usize]) }
    }

    #[cfg(feature = "smp")]
    pub fn start_core(
        &mut self,
        _coreid: u8,
        _entry: ::sys::mm::VirtualAddress,
        _kstack: *const u8,
    ) -> Result<(), Error> {
        Err(Error::new(ErrorCode::OperationNotSupported, "AArch64 SMP startup is not implemented"))
    }
}

fn interrupt_id(intnum: InterruptNumber) -> Result<u32, Error> {
    match intnum {
        InterruptNumber::Timer => Ok(::config::microvm::DEFAULT_ARM_TIMER_INTERRUPT),
        #[cfg(feature = "microvm")]
        InterruptNumber::Ikc => Ok(::config::microvm::DEFAULT_ARM_IKC_INTERRUPT),
        _ => Err(Error::new(
            ErrorCode::OperationNotSupported,
            "interrupt source is not wired on AArch64 MicroVM",
        )),
    }
}

pub unsafe fn init() {
    let gicr_waker: *mut u32 = (::config::microvm::DEFAULT_GICR_BASE + 0x14) as *mut u32;
    let mut waker: u32 = core::ptr::read_volatile(gicr_waker);
    waker &= !(1 << 1);
    core::ptr::write_volatile(gicr_waker, waker);
    while core::ptr::read_volatile(gicr_waker) & (1 << 2) != 0 {
        core::hint::spin_loop();
    }

    // Route the virtual timer PPI and the IKC SPI through Group 1.
    let gicr_igroupr0: *mut u32 = (::config::microvm::DEFAULT_GICR_BASE + 0x1_0080) as *mut u32;
    core::ptr::write_volatile(
        gicr_igroupr0,
        core::ptr::read_volatile(gicr_igroupr0)
            | (1 << ::config::microvm::DEFAULT_ARM_TIMER_INTERRUPT),
    );
    let gicd_igroupr1: *mut u32 = (::config::microvm::DEFAULT_GICD_BASE + 0x84) as *mut u32;
    core::ptr::write_volatile(
        gicd_igroupr1,
        core::ptr::read_volatile(gicd_igroupr1)
            | (1 << (::config::microvm::DEFAULT_ARM_IKC_INTERRUPT - 32)),
    );

    // IKC notifications are delivered as assert/deassert pulses, so configure their SPI as
    // edge-triggered before enabling the distributor.
    let ikc_intid: u32 = ::config::microvm::DEFAULT_ARM_IKC_INTERRUPT;
    let gicd_icfgr: *mut u32 = (::config::microvm::DEFAULT_GICD_BASE
        + 0xc00
        + ((ikc_intid as usize / 16) * 4)) as *mut u32;
    let shift: u32 = (ikc_intid % 16) * 2;
    let mask: u32 = 0b11 << shift;
    let configuration: u32 = core::ptr::read_volatile(gicd_icfgr);
    core::ptr::write_volatile(gicd_icfgr, (configuration & !mask) | (0b10 << shift));
    core::arch::asm!("dsb sy", options(nostack, preserves_flags));

    let gicd_ctlr: *mut u32 = ::config::microvm::DEFAULT_GICD_BASE as *mut u32;
    core::ptr::write_volatile(gicd_ctlr, 0x2);
    init_cpu_interface();
    rearm_timer();
}

pub unsafe fn init_cpu_interface() {
    let mut sre: u64;
    core::arch::asm!("mrs {sre}, ICC_SRE_EL1", sre = out(reg) sre, options(nostack));
    sre |= 1;
    core::arch::asm!(
        "msr ICC_SRE_EL1, {sre}",
        "msr ICC_PMR_EL1, {pmr}",
        "msr ICC_BPR1_EL1, xzr",
        "msr ICC_IGRPEN1_EL1, {enable}",
        "isb",
        sre = in(reg) sre,
        pmr = in(reg) 0xffu64,
        enable = in(reg) 1u64,
        options(nostack, preserves_flags),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn aarch64_interrupt_dispatch() {
    let iar: u64;
    core::arch::asm!("mrs {iar}, ICC_IAR1_EL1", iar = out(reg) iar, options(nostack));
    let intid: u32 = iar as u32 & 0x00ff_ffff;
    if intid >= 1020 {
        return;
    }
    ACTIVE_IAR.store(iar as u32, Ordering::Relaxed);

    let intnum: Option<InterruptNumber> = if intid == ::config::microvm::DEFAULT_ARM_TIMER_INTERRUPT
    {
        Some(InterruptNumber::Timer)
    } else if intid == ::config::microvm::DEFAULT_ARM_IKC_INTERRUPT {
        #[cfg(feature = "microvm")]
        {
            Some(InterruptNumber::Ikc)
        }
        #[cfg(not(feature = "microvm"))]
        {
            None
        }
    } else {
        None
    };

    if let Some(intnum) = intnum {
        crate::hal::cpu::InterruptManager::do_interrupt(intnum);
    } else {
        core::arch::asm!("msr ICC_EOIR1_EL1, {iar}", iar = in(reg) iar, options(nostack));
        ACTIVE_IAR.store(u32::MAX, Ordering::Relaxed);
    }
}

fn rearm_timer() {
    let frequency: u64;
    unsafe {
        core::arch::asm!("mrs {frequency}, cntfrq_el0", frequency = out(reg) frequency);
    }
    let interval: u64 = frequency / u64::from(::config::kernel::TIMER_FREQ);
    unsafe {
        core::arch::asm!(
            "msr cntv_tval_el0, {interval}",
            "msr cntv_ctl_el0, {enable}",
            "isb",
            interval = in(reg) interval,
            enable = in(reg) 1u64,
            options(nostack, preserves_flags),
        );
    }
}
