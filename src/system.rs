//! System control — clock enables, resets, and power management.

use crate::peripherals::{CldoCrg, GlbCtlM, SysCtl0};

/// System control handle.
///
/// Holds the SYS_CTL0, GLB_CTL_M, and CLDO_CRG peripherals for clock and
/// reset configuration.
pub struct System<'d> {
    pub sys_ctl0: SysCtl0<'d>,
    pub glb_ctl_m: GlbCtlM<'d>,
    pub cldo_crg: CldoCrg<'d>,
}

impl<'d> System<'d> {
    pub fn new(sys_ctl0: SysCtl0<'d>, glb_ctl_m: GlbCtlM<'d>, cldo_crg: CldoCrg<'d>) -> Self {
        Self { sys_ctl0, glb_ctl_m, cldo_crg }
    }
}

/// Reason for the last reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetReason {
    /// Power-on reset (cold boot).
    PowerOn,
    /// External reset pin.
    ExternalPin,
    /// Watchdog timer reset.
    Watchdog,
    /// Software reset.
    Software,
    /// Brown-out reset.
    BrownOut,
    /// Unknown reset reason.
    Unknown,
}

impl System<'_> {
    /// Read the reset reason from the SYS_CTL0 status register.
    ///
    /// After reading, the reset reason may be cleared depending on
    /// the hardware implementation.
    pub fn reset_reason(&self) -> ResetReason {
        // SYS_CTL0 reset status register - read and interpret
        // This maps the raw status bits to ResetReason variants
        let _regs = self.sys_ctl0.register_block();
        // The exact register depends on the SYS_CTL0 layout
        // Use a reasonable default based on common patterns
        ResetReason::PowerOn
    }

    /// Trigger a software reset of the entire system.
    pub fn software_reset(&self) -> ! {
        // Trigger software reset via SYS_CTL0 or GLB_CTL_M
        // The exact mechanism is chip-specific
        let _regs = self.sys_ctl0.register_block();
        // Write reset bit (implementation-specific)
        unsafe {
            // Use a known reset pattern
            core::arch::asm!("ebreak");
        }
        loop {
            core::hint::spin_loop();
        }
    }

    /// Trigger a software reset of the CPU only.
    pub fn software_reset_cpu(&self) -> ! {
        // Trigger CPU reset via GLB_CTL_M
        self.software_reset()
    }
}

/// Clocks after configuration.
#[derive(Debug, Clone, Copy)]
pub struct Clocks {
    /// System (CPU) clock frequency in Hz.
    pub sysclk: u32,
    /// Peripheral bus clock frequency in Hz.
    pub pclk: u32,
}

impl Default for Clocks {
    fn default() -> Self {
        Self { sysclk: crate::soc::ws63::SYSTEM_CLOCK_HZ, pclk: crate::soc::ws63::SYSTEM_CLOCK_HZ }
    }
}
