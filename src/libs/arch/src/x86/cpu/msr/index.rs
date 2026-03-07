// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

// MSR indices used for filtering and ordering.
// References: Linux arch/x86/include/asm/msr-index.h and Firecracker's msr.rs.

/// Well-known MSR register indices.
/// Sorted by register address.
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MsrIndex {
    /// IA32_P5_MC_ADDR.
    Ia32P5McAddr = 0x0000_0000,
    /// IA32_P5_MC_TYPE.
    Ia32P5McType = 0x0000_0001,
    /// IA32_TSC (Time Stamp Counter).
    Ia32Tsc = 0x0000_0010,
    /// IA32_PLATFORM_ID.
    Ia32PlatformId = 0x0000_0017,
    /// IA32_APICBASE.
    Ia32Apicbase = 0x0000_001b,
    /// IA32_EBL_CR_POWERON.
    Ia32EblCrPoweron = 0x0000_002a,
    /// EBC_FREQUENCY_ID.
    EbcFrequencyId = 0x0000_002c,
    /// SMI_COUNT.
    SmiCount = 0x0000_0034,
    /// IA32_FEAT_CTL.
    Ia32FeatCtl = 0x0000_003a,
    /// IA32_TSC_ADJUST.
    Ia32TscAdjust = 0x0000_003b,
    /// IA32_SPEC_CTRL.
    Ia32SpecCtrl = 0x0000_0048,
    /// IA32_PRED_CMD.
    Ia32PredCmd = 0x0000_0049,
    /// IA32_UCODE_WRITE.
    Ia32UcodeWrite = 0x0000_0079,
    /// IA32_UCODE_REV.
    Ia32UcodeRev = 0x0000_008b,
    /// IA32_SMBASE.
    Ia32Smbase = 0x0000_009e,
    /// FSB_FREQ.
    FsbFreq = 0x0000_00cd,
    /// PLATFORM_INFO.
    PlatformInfo = 0x0000_00ce,
    /// PKG_CST_CONFIG_CONTROL.
    PkgCstConfigControl = 0x0000_00e2,
    /// IA32_MPERF.
    Ia32Mperf = 0x0000_00e7,
    /// IA32_APERF.
    Ia32Aperf = 0x0000_00e8,
    /// MTRR_CAP.
    MtrrCap = 0x0000_00fe,
    /// IA32_ARCH_CAPABILITIES.
    Ia32ArchCapabilities = 0x0000_010a,
    /// IA32_BBL_CR_CTL3.
    Ia32BblCrCtl3 = 0x0000_011e,
    /// IA32_TSX_CTRL.
    Ia32TsxCtrl = 0x0000_0122,
    /// MISC_FEATURES_ENABLES.
    MiscFeaturesEnables = 0x0000_0140,
    /// IA32_SYSENTER_CS.
    Ia32SysenterCs = 0x0000_0174,
    /// IA32_SYSENTER_ESP.
    Ia32SysenterEsp = 0x0000_0175,
    /// IA32_SYSENTER_EIP.
    Ia32SysenterEip = 0x0000_0176,
    /// IA32_MCG_CAP.
    Ia32McgCap = 0x0000_0179,
    /// IA32_MCG_STATUS.
    Ia32McgStatus = 0x0000_017a,
    /// IA32_PERF_STATUS.
    Ia32PerfStatus = 0x0000_0198,
    /// IA32_MISC_ENABLE.
    Ia32MiscEnable = 0x0000_01a0,
    /// MISC_FEATURE_CONTROL.
    MiscFeatureControl = 0x0000_01a4,
    /// MISC_PWR_MGMT.
    MiscPwrMgmt = 0x0000_01aa,
    /// TURBO_RATIO_LIMIT.
    TurboRatioLimit = 0x0000_01ad,
    /// TURBO_RATIO_LIMIT1.
    TurboRatioLimit1 = 0x0000_01ae,
    /// IA32_DEBUGCTLMSR.
    Ia32Debugctlmsr = 0x0000_01d9,
    /// IA32_LASTBRANCHFROMIP.
    Ia32Lastbranchfromip = 0x0000_01db,
    /// IA32_LASTBRANCHTOIP.
    Ia32Lastbranchtoip = 0x0000_01dc,
    /// IA32_LASTINTFROMIP.
    Ia32Lastintfromip = 0x0000_01dd,
    /// IA32_LASTINTTOIP.
    Ia32Lastinttoip = 0x0000_01de,
    /// IA32_POWER_CTL.
    Ia32PowerCtl = 0x0000_01fc,
    /// IA32_MTRR_PHYSBASE0.
    Ia32MtrrPhysbase0 = 0x0000_0200,
    /// CORE_C3_RESIDENCY.
    CoreC3Residency = 0x0000_03fc,
    /// IA32_MC0_CTL.
    Ia32Mc0Ctl = 0x0000_0400,
    /// RAPL_POWER_UNIT.
    RaplPowerUnit = 0x0000_0606,
    /// PKGC3_IRTL.
    Pkgc3Irtl = 0x0000_060a,
    /// PKG_POWER_LIMIT.
    PkgPowerLimit = 0x0000_0610,
    /// PKG_ENERGY_STATUS.
    PkgEnergyStatus = 0x0000_0611,
    /// PKG_PERF_STATUS.
    PkgPerfStatus = 0x0000_0613,
    /// PKG_POWER_INFO.
    PkgPowerInfo = 0x0000_0614,
    /// DRAM_POWER_LIMIT.
    DramPowerLimit = 0x0000_0618,
    /// DRAM_ENERGY_STATUS.
    DramEnergyStatus = 0x0000_0619,
    /// DRAM_PERF_STATUS.
    DramPerfStatus = 0x0000_061b,
    /// DRAM_POWER_INFO.
    DramPowerInfo = 0x0000_061c,
    /// CONFIG_TDP_NOMINAL.
    ConfigTdpNominal = 0x0000_0648,
    /// CONFIG_TDP_LEVEL_1.
    ConfigTdpLevel1 = 0x0000_0649,
    /// CONFIG_TDP_LEVEL_2.
    ConfigTdpLevel2 = 0x0000_064a,
    /// CONFIG_TDP_CONTROL.
    ConfigTdpControl = 0x0000_064b,
    /// TURBO_ACTIVATION_RATIO.
    TurboActivationRatio = 0x0000_064c,
    /// IA32_TSC_DEADLINE.
    Ia32TscDeadline = 0x0000_06e0,
    /// APIC base (x2APIC registers).
    ApicBase = 0x0000_0800,
    /// KVM_WALL_CLOCK_NEW.
    KvmWallClockNew = 0x4b56_4d00,
    /// KVM_SYSTEM_TIME_NEW.
    KvmSystemTimeNew = 0x4b56_4d01,
    /// KVM_ASYNC_PF_EN.
    KvmAsyncPfEn = 0x4b56_4d02,
    /// KVM_STEAL_TIME.
    KvmStealTime = 0x4b56_4d03,
    /// KVM_PV_EOI_EN.
    KvmPvEoiEn = 0x4b56_4d04,
    /// KVM_POLL_CONTROL.
    KvmPollControl = 0x4b56_4d05,
    /// KVM_ASYNC_PF_INT.
    KvmAsyncPfInt = 0x4b56_4d06,
    /// EFER (Extended Feature Enable Register).
    Efer = 0xc000_0080,
    /// STAR.
    Star = 0xc000_0081,
    /// LSTAR.
    Lstar = 0xc000_0082,
    /// CSTAR.
    Cstar = 0xc000_0083,
    /// SYSCALL_MASK.
    SyscallMask = 0xc000_0084,
    /// FS_BASE.
    FsBase = 0xc000_0100,
    /// GS_BASE.
    GsBase = 0xc000_0101,
    /// KERNEL_GS_BASE.
    KernelGsBase = 0xc000_0102,
    /// TSC_AUX.
    TscAux = 0xc000_0103,
    /// K7_HWCR.
    K7Hwcr = 0xc001_0015,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl MsrIndex {
    /// Number of x2APIC MSR registers (covers the `0x800..=0x8FF` window).
    pub const APIC_MSR_COUNT: u32 = 0x100;

    /// Returns the underlying `u32` register index.
    pub const fn as_u32(self) -> u32 {
        self as u32
    }

    /// Returns the number of consecutive MSR indices covered by this variant.
    ///
    /// Most MSR variants represent a single register (`1`). Variants that denote
    /// the start of an architecturally defined range return the span of that range.
    pub const fn range_count(self) -> u32 {
        match self {
            // IA32_MTRR_PHYSBASE0 .. +0xFF (MTRR physical base/mask pairs).
            Self::Ia32MtrrPhysbase0 => 0x100,
            // CORE_C3_RESIDENCY, CORE_C6_RESIDENCY, CORE_C7_RESIDENCY.
            Self::CoreC3Residency => 3,
            // IA32_MCi_CTL/STATUS/ADDR/MISC banks.
            Self::Ia32Mc0Ctl => 0x80,
            // PKGC3_IRTL, PKGC6_IRTL, PKGC7_IRTL.
            Self::Pkgc3Irtl => 3,
            // x2APIC registers.
            Self::ApicBase => Self::APIC_MSR_COUNT,
            // All other variants represent a single register.
            _ => 1,
        }
    }
}
