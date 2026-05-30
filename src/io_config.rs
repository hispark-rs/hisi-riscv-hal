//! IO Configuration (Pin Mux and Pad Control) driver for WS63.
//!
//! The WS63 IO_CONFIG peripheral controls pin function selection (muxing)
//! and pad characteristics (drive strength, pull resistors, Schmitt trigger,
//! input enable) for all GPIO, UART, and SFC pads.
//!
//! # Pin mux
//!
//! Each GPIO pin can be assigned one of several functions via a 3-bit
//! mux select register. The exact function mapping depends on the chip
//! configuration.
//!
//! # Pad control
//!
//! Each pad has independent control over:
//! - Drive strength (3-bit, 000 = strongest, 111 = weakest)
//! - Pull resistor (none, pull-up, pull-down)
//! - Schmitt trigger (enable/disable)
//! - Input enable (enable/disable)

use crate::peripherals::IoConfig;

/// Pad drive strength.
///
/// DS[2:0] values: smaller = stronger drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveStrength {
    /// Strongest drive.
    Strongest = 0,
    /// Medium-strong drive.
    Strong = 1,
    /// Medium drive.
    Medium = 2,
    /// Medium-weak drive.
    Weak = 3,
    /// Weaker drive.
    Weaker = 4,
    /// Very weak drive.
    VeryWeak = 5,
    /// Weakest drive.
    Weakest = 6,
    /// Minimum drive.
    Minimum = 7,
}

/// Pull resistor configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullResistor {
    /// No pull.
    None,
    /// Pull-up resistor.
    Up,
    /// Pull-down resistor.
    Down,
}

/// GPIO pin mux selection (15 GPIO pins, 4 UART pads).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinMux {
    Gpio00,
    Gpio01,
    Gpio02,
    Gpio03,
    Gpio04,
    Gpio05,
    Gpio06,
    Gpio07,
    Gpio08,
    Gpio09,
    Gpio10,
    Gpio11,
    Gpio12,
    Gpio13,
    Gpio14,
    Uart0Txd,
    Uart0Rxd,
    Uart1Txd,
    Uart1Rxd,
}

/// SFC pad selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SfcPad {
    Clk,
    Csn,
    Io0,
    Io1,
    Io2,
    Io3,
}

/// IO Configuration driver.
pub struct IoConfigDriver<'d> {
    _io_config: IoConfig<'d>,
}

impl<'d> IoConfigDriver<'d> {
    /// Create a new IO configuration driver.
    pub fn new(io_config: IoConfig<'d>) -> Self {
        Self { _io_config: io_config }
    }

    fn regs(&self) -> &'static ws63_pac::io_config::RegisterBlock {
        // SAFETY: PAC peripheral pointer is a static physical MMIO address, always valid
        unsafe { &*IoConfig::ptr() }
    }

    // ── Pin mux ───────────────────────────────────────────────────

    /// Set the function select value for a GPIO pin.
    ///
    /// * `pin` — GPIO pin index (0-14).
    /// * `function` — 3-bit function select value.
    pub fn set_gpio_mux(&mut self, pin: u8, function: u8) {
        assert!(pin < 15);
        let val = (function & 0x07) as u32;
        let r = self.regs();
        match pin {
            0 => unsafe {
                r.gpio_00_sel().write(|w| w.bits(val));
            },
            1 => unsafe {
                r.gpio_01_sel().write(|w| w.bits(val));
            },
            2 => unsafe {
                r.gpio_02_sel().write(|w| w.bits(val));
            },
            3 => unsafe {
                r.gpio_03_sel().write(|w| w.bits(val));
            },
            4 => unsafe {
                r.gpio_04_sel().write(|w| w.bits(val));
            },
            5 => unsafe {
                r.gpio_05_sel().write(|w| w.bits(val));
            },
            6 => unsafe {
                r.gpio_06_sel().write(|w| w.bits(val));
            },
            7 => unsafe {
                r.gpio_07_sel().write(|w| w.bits(val));
            },
            8 => unsafe {
                r.gpio_08_sel().write(|w| w.bits(val));
            },
            9 => unsafe {
                r.gpio_09_sel().write(|w| w.bits(val));
            },
            10 => unsafe {
                r.gpio_10_sel().write(|w| w.bits(val));
            },
            11 => unsafe {
                r.gpio_11_sel().write(|w| w.bits(val));
            },
            12 => unsafe {
                r.gpio_12_sel().write(|w| w.bits(val));
            },
            13 => unsafe {
                r.gpio_13_sel().write(|w| w.bits(val));
            },
            14 => unsafe {
                r.gpio_14_sel().write(|w| w.bits(val));
            },
            _ => {}
        }
    }

    /// Get the function select value for a GPIO pin.
    pub fn get_gpio_mux(&self, pin: u8) -> u8 {
        assert!(pin < 15);
        let r = self.regs();
        (match pin {
            0 => r.gpio_00_sel().read().bits(),
            1 => r.gpio_01_sel().read().bits(),
            2 => r.gpio_02_sel().read().bits(),
            3 => r.gpio_03_sel().read().bits(),
            4 => r.gpio_04_sel().read().bits(),
            5 => r.gpio_05_sel().read().bits(),
            6 => r.gpio_06_sel().read().bits(),
            7 => r.gpio_07_sel().read().bits(),
            8 => r.gpio_08_sel().read().bits(),
            9 => r.gpio_09_sel().read().bits(),
            10 => r.gpio_10_sel().read().bits(),
            11 => r.gpio_11_sel().read().bits(),
            12 => r.gpio_12_sel().read().bits(),
            13 => r.gpio_13_sel().read().bits(),
            14 => r.gpio_14_sel().read().bits(),
            _ => 0,
        } & 0x07) as u8
    }

    /// Set the function select for a UART pad.
    pub fn set_uart_mux(&mut self, pin: PinMux, function: u8) {
        let val = (function & 0x07) as u32;
        let r = self.regs();
        match pin {
            PinMux::Uart0Txd => unsafe {
                r.uart0_txd_sel().write(|w| w.bits(val));
            },
            PinMux::Uart0Rxd => unsafe {
                r.uart0_rxd_sel().write(|w| w.bits(val));
            },
            PinMux::Uart1Txd => unsafe {
                r.uart1_txd_sel().write(|w| w.bits(val));
            },
            PinMux::Uart1Rxd => unsafe {
                r.uart1_rxd_sel().write(|w| w.bits(val));
            },
            _ => {}
        }
    }

    // ── Pad control ────────────────────────────────────────────────

    /// Configure pad characteristics for a GPIO pad.
    ///
    /// * `pin` — GPIO pad index (0-14).
    pub fn configure_gpio_pad(
        &mut self,
        pin: u8,
        drive: DriveStrength,
        pull: PullResistor,
        schmitt_trigger: bool,
        input_enable: bool,
    ) {
        assert!(pin < 15);
        let val = build_pad_ctrl(drive, pull, schmitt_trigger, input_enable);
        let r = self.regs();
        match pin {
            0 => unsafe {
                r.pad_gpio_00_ctrl().write(|w| w.bits(val));
            },
            1 => unsafe {
                r.pad_gpio_01_ctrl().write(|w| w.bits(val));
            },
            2 => unsafe {
                r.pad_gpio_02_ctrl().write(|w| w.bits(val));
            },
            3 => unsafe {
                r.pad_gpio_03_ctrl().write(|w| w.bits(val));
            },
            4 => unsafe {
                r.pad_gpio_04_ctrl().write(|w| w.bits(val));
            },
            5 => unsafe {
                r.pad_gpio_05_ctrl().write(|w| w.bits(val));
            },
            6 => unsafe {
                r.pad_gpio_06_ctrl().write(|w| w.bits(val));
            },
            7 => unsafe {
                r.pad_gpio_07_ctrl().write(|w| w.bits(val));
            },
            8 => unsafe {
                r.pad_gpio_08_ctrl().write(|w| w.bits(val));
            },
            9 => unsafe {
                r.pad_gpio_09_ctrl().write(|w| w.bits(val));
            },
            10 => unsafe {
                r.pad_gpio_10_ctrl().write(|w| w.bits(val));
            },
            11 => unsafe {
                r.pad_gpio_11_ctrl().write(|w| w.bits(val));
            },
            12 => unsafe {
                r.pad_gpio_12_ctrl().write(|w| w.bits(val));
            },
            13 => unsafe {
                r.pad_gpio_13_ctrl().write(|w| w.bits(val));
            },
            14 => unsafe {
                r.pad_gpio_14_ctrl().write(|w| w.bits(val));
            },
            _ => {}
        }
    }

    /// Configure pad characteristics for a UART pad.
    pub fn configure_uart_pad(
        &mut self,
        uart_pad: PinMux,
        drive: DriveStrength,
        pull: PullResistor,
        schmitt_trigger: bool,
        input_enable: bool,
    ) {
        let val = build_pad_ctrl(drive, pull, schmitt_trigger, input_enable);
        let r = self.regs();
        match uart_pad {
            PinMux::Uart0Txd => unsafe {
                r.pad_uart0_txd_ctrl().write(|w| w.bits(val));
            },
            PinMux::Uart0Rxd => unsafe {
                r.pad_uart0_rxd_ctrl().write(|w| w.bits(val));
            },
            PinMux::Uart1Txd => unsafe {
                r.pad_uart1_txd_ctrl().write(|w| w.bits(val));
            },
            PinMux::Uart1Rxd => unsafe {
                r.pad_uart1_rxd_ctrl().write(|w| w.bits(val));
            },
            _ => {}
        }
    }

    /// Configure pad characteristics for an SFC pad.
    pub fn configure_sfc_pad(
        &mut self,
        sfc_pad: SfcPad,
        drive: DriveStrength,
        pull: PullResistor,
        schmitt_trigger: bool,
        input_enable: bool,
    ) {
        let val = build_pad_ctrl(drive, pull, schmitt_trigger, input_enable);
        let r = self.regs();
        match sfc_pad {
            SfcPad::Clk => unsafe {
                r.pad_sfc_clk_ctrl().write(|w| w.bits(val));
            },
            SfcPad::Csn => unsafe {
                r.pad_sfc_csn_ctrl().write(|w| w.bits(val));
            },
            SfcPad::Io0 => unsafe {
                r.pad_sfc_io0_ctrl().write(|w| w.bits(val));
            },
            SfcPad::Io1 => unsafe {
                r.pad_sfc_io1_ctrl().write(|w| w.bits(val));
            },
            SfcPad::Io2 => unsafe {
                r.pad_sfc_io2_ctrl().write(|w| w.bits(val));
            },
            SfcPad::Io3 => unsafe {
                r.pad_sfc_io3_ctrl().write(|w| w.bits(val));
            },
        }
    }

    /// Read the current pad control value for a GPIO pad.
    pub fn read_gpio_pad(&self, pin: u8) -> u32 {
        assert!(pin < 15);
        let r = self.regs();
        match pin {
            0 => r.pad_gpio_00_ctrl().read().bits(),
            1 => r.pad_gpio_01_ctrl().read().bits(),
            2 => r.pad_gpio_02_ctrl().read().bits(),
            3 => r.pad_gpio_03_ctrl().read().bits(),
            4 => r.pad_gpio_04_ctrl().read().bits(),
            5 => r.pad_gpio_05_ctrl().read().bits(),
            6 => r.pad_gpio_06_ctrl().read().bits(),
            7 => r.pad_gpio_07_ctrl().read().bits(),
            8 => r.pad_gpio_08_ctrl().read().bits(),
            9 => r.pad_gpio_09_ctrl().read().bits(),
            10 => r.pad_gpio_10_ctrl().read().bits(),
            11 => r.pad_gpio_11_ctrl().read().bits(),
            12 => r.pad_gpio_12_ctrl().read().bits(),
            13 => r.pad_gpio_13_ctrl().read().bits(),
            14 => r.pad_gpio_14_ctrl().read().bits(),
            _ => 0,
        }
    }
}

/// Build a pad control register value.
fn build_pad_ctrl(drive: DriveStrength, pull: PullResistor, schmitt_trigger: bool, input_enable: bool) -> u32 {
    let mut val: u32 = 0;

    // Drive strength: ds2=bit6, ds1=bit5, ds0=bit4
    let ds = drive as u32;
    val |= ((ds & 0x04) << 4) | ((ds & 0x02) << 4) | ((ds & 0x01) << 4); // ds2:6, ds1:5, ds0:4

    // Pull resistor: PE=bit9, PS=bit10
    match pull {
        PullResistor::None => {} // PE=0, PS=0
        PullResistor::Up => {
            val |= (1 << 9) | (1 << 10); // PE=1, PS=1
        }
        PullResistor::Down => {
            val |= 1 << 9; // PE=1, PS=0
        }
    }

    // Schmitt trigger: ST=bit3
    if schmitt_trigger {
        val |= 1 << 3;
    }

    // Input enable: IE=bit11
    if input_enable {
        val |= 1 << 11;
    }

    val
}
