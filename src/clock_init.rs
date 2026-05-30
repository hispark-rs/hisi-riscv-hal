//! Clock initialization for WS63.
//!
//! Provides system clock tree configuration based on the fbb_ws63 C SDK
//! reference (soc_porting.c, pm_porting.c). Handles TCXO detection,
//! PLL switching, and clock divider setup.
//!
//! # Register map (from fbb_ws63)
//!
//! | Register | Address | Description |
//! |----------|---------|-------------|
//! | HW_CTL | 0x4000_0014 | TCXO frequency detect (bit[0]: 0=24MHz, 1=40MHz) |
//! | REG_EXCEP_RO_RG | 0x4000_319C | PLL lock status (bit 12) |
//! | REG_CMU_FNPLL_SIG | 0x4000_342C | CMU PLL signal (bit 15 = PD) |
//! | CMU_NEW_CFG1 | 0x4000_34A4 | Flash clock control |
//! | CLDO_CRG_CLK_SEL | 0x4400_1134 | Clock source select (bit 18 = PLL) |
//!
//! # Clock tree (from ws63-guide ch2_system.md)
//!
//! | Domain | Frequency | Clock Source |
//! |--------|-----------|-------------|
//! | CPU | 240 MHz | PLL |
//! | CPU Bus | 240 MHz | PLL |
//! | GPIO | 120 MHz | PLL / 2 |
//! | UART | 160 MHz | PLL-derived |
//! | SPI | 160 MHz | PLL-derived |
//! | I2C | 80 MHz | PLL-derived |
//! | QSPI | 64 MHz | PLL-derived |
//! | Timer | 32 kHz | Crystal |
//! | WDT | 32 kHz | Crystal |
//! | RTC | 32 kHz | Crystal |
//! | Crystal | 40/24 MHz | TCXO |

use crate::peripherals::{CldoCrg, SysCtl0};
use crate::soc::ws63::SYSTEM_CLOCK_HZ;

// ── Register addresses (from fbb_ws63 soc_porting.c / pm_porting.c) ──

/// Hardware control register — TCXO frequency detect.
const HW_CTL: *mut u32 = 0x4000_0014 as *mut u32;
/// Exception RO register — PLL lock status (bit 12).
const REG_EXCEP_RO_RG: *mut u32 = 0x4000_319C as *mut u32;
/// CMU PLL signal register — PLL power-down control (bit 15).
#[allow(dead_code)]
const REG_CMU_FNPLL_SIG: *mut u32 = 0x4000_342C as *mut u32;
/// Flash clock control register.
const CMU_NEW_CFG1: *mut u32 = 0x4000_34A4 as *mut u32;

// ── TCXO frequency ────────────────────────────────────────────────

/// TCXO crystal frequency in Hz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcxoFreq {
    /// 24 MHz crystal.
    MHz24 = 24_000_000,
    /// 40 MHz crystal.
    MHz40 = 40_000_000,
}

impl TcxoFreq {
    /// Detect the TCXO frequency by reading the HW_CTL register.
    pub fn detect() -> Self {
        let hw_ctl = unsafe { HW_CTL.read_volatile() };
        if hw_ctl & 0x01 == 0 {
            TcxoFreq::MHz24
        } else {
            TcxoFreq::MHz40
        }
    }

    /// Return the frequency in Hz.
    pub const fn hz(&self) -> u32 {
        *self as u32
    }
}

// ── PLL status ────────────────────────────────────────────────────

/// Result of PLL lock check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PllStatus {
    /// PLL is locked and stable.
    Locked,
    /// PLL is unlocked — system runs from TCXO.
    Unlocked,
}

/// Check if the PLL is locked by reading REG_EXCEP_RO_RG bit 12.
fn pll_is_locked() -> bool {
    // fbb_ws63: cmu_is_fnpll_locked() reads REG_EXCEP_RO_RG bit 12
    (unsafe { REG_EXCEP_RO_RG.read_volatile() } >> 12) & 1 == 1
}

/// Poll the PLL lock status with retries.
///
/// * `retry_count` — Number of polling attempts (default 30).
/// * `retry_delay_us` — Delay between attempts in µs (default 1000 = 1ms).
fn wait_pll_lock(retry_count: u32, retry_delay_us: u32) -> PllStatus {
    for _ in 0..retry_count {
        if pll_is_locked() {
            return PllStatus::Locked;
        }
        // Busy-wait delay (approximate at TCXO speed before PLL is on)
        let cycles = TcxoFreq::detect().hz() as u64 * retry_delay_us as u64 / 1_000_000;
        for _ in 0..cycles / 3 {
            core::hint::spin_loop();
        }
    }
    PllStatus::Unlocked
}

// ── Clock initialization ──────────────────────────────────────────

/// System clock configuration after initialization.
#[derive(Debug, Clone, Copy)]
pub struct SystemClocks {
    /// CPU clock (PLL output, 240 MHz).
    pub cpu_clk: u32,
    /// Peripheral bus clock (PCLK, 240 MHz default).
    pub pclk: u32,
    /// TCXO crystal frequency.
    pub tcxo_freq: TcxoFreq,
    /// Whether the PLL is locked.
    pub pll_locked: bool,
}

impl SystemClocks {
    /// Default clocks (assumes boot ROM has configured PLL).
    pub const fn assumed() -> Self {
        Self {
            cpu_clk: SYSTEM_CLOCK_HZ,
            pclk: SYSTEM_CLOCK_HZ,
            tcxo_freq: TcxoFreq::MHz40,
            pll_locked: true,
        }
    }
}

/// Initialize the system clock tree.
///
/// Performs the clock initialization sequence from fbb_ws63:
/// 1. Detect TCXO frequency (24 or 40 MHz)
/// 2. Switch flash clock to PLL
/// 3. Verify PLL lock
/// 4. Configure peripheral clock gates if needed
///
/// # Arguments
///
/// * `sys_ctl0` — SYS_CTL0 peripheral (for PLL status registers).
/// * `cldo_crg` — CLDO_CRG peripheral (for clock select).
///
/// # Returns
///
/// The resolved [`SystemClocks`] configuration.
pub fn init_clocks(_sys_ctl0: &SysCtl0<'_>, _cldo_crg: &CldoCrg<'_>) -> SystemClocks {
    let tcxo_freq = TcxoFreq::detect();

    // Switch flash clock to PLL (fbb_ws63: switch_flash_clock_to_pll)
    // Step 1: CMU_NEW_CFG1 = CPU_DIV_FLASH_RSTN_SYNC (0x1)
    unsafe { CMU_NEW_CFG1.write_volatile(0x1) };
    // Step 2: Delay 1 µs
    for _ in 0..tcxo_freq.hz() / 1_000_000 / 3 {
        core::hint::spin_loop();
    }
    // Step 3: CMU_NEW_CFG1 = CPU_DIV_FLASH_RSTN (0x3)
    unsafe { CMU_NEW_CFG1.write_volatile(0x3) };
    // Step 4: Set CLDO_CRG_CLK_SEL bit 18 (select PLL as clock source)
    unsafe {
        // CLDO_CRG_CLK_SEL is at absolute address 0x4400_1134
        let clk_sel_ptr = 0x4400_1134 as *mut u32;
        let val = clk_sel_ptr.read_volatile();
        clk_sel_ptr.write_volatile(val | (1 << 18));
    }

    // Verify PLL lock (fbb_ws63: check_cmu_lock_status)
    let pll_locked = match wait_pll_lock(30, 1000) {
        PllStatus::Locked => true,
        PllStatus::Unlocked => false,
    };

    SystemClocks {
        cpu_clk: if pll_locked { SYSTEM_CLOCK_HZ } else { tcxo_freq.hz() },
        pclk: if pll_locked { SYSTEM_CLOCK_HZ } else { tcxo_freq.hz() },
        tcxo_freq,
        pll_locked,
    }
}

/// Simple version: detect clocks without modifying them.
///
/// Reads TCXO frequency and checks PLL status. Does NOT reconfigure
/// the clock tree. Safe to call when boot ROM has already configured
/// the PLL (which is the default on WS63).
pub fn probe_clocks() -> SystemClocks {
    let tcxo_freq = TcxoFreq::detect();
    let pll_locked = pll_is_locked();

    SystemClocks {
        cpu_clk: if pll_locked { SYSTEM_CLOCK_HZ } else { tcxo_freq.hz() },
        pclk: if pll_locked { SYSTEM_CLOCK_HZ } else { tcxo_freq.hz() },
        tcxo_freq,
        pll_locked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcxo_freq_values() {
        assert_eq!(TcxoFreq::MHz24.hz(), 24_000_000);
        assert_eq!(TcxoFreq::MHz40.hz(), 40_000_000);
    }

    #[test]
    fn test_default_clocks() {
        let c = SystemClocks::assumed();
        assert_eq!(c.cpu_clk, 240_000_000);
        assert_eq!(c.pclk, 240_000_000);
        assert!(c.pll_locked);
    }
}
