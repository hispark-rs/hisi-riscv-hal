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

    fn regs(&self) -> &'static crate::soc::pac::io_config::RegisterBlock {
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

// ── Tests ──────────────────────────────────────────────────────

#[cfg(all(test, not(target_arch = "riscv32")))]
mod tests {
    use super::*;

    // Bit positions in the pad-control register (mirrors build_pad_ctrl).
    const ST_BIT: u32 = 1 << 3; // Schmitt trigger
    const DS_MASK: u32 = 0x7 << 4; // drive strength ds[2:0] -> bits [6:4]
    const PE_BIT: u32 = 1 << 9; // pull enable
    const PS_BIT: u32 = 1 << 10; // pull select (1 = up, 0 = down)
    const IE_BIT: u32 = 1 << 11; // input enable

    #[test]
    fn drive_strength_discriminants_are_dense() {
        // The enum encodes DS[2:0] directly; values must be 0..=7 contiguous.
        assert_eq!(DriveStrength::Strongest as u32, 0);
        assert_eq!(DriveStrength::Strong as u32, 1);
        assert_eq!(DriveStrength::Medium as u32, 2);
        assert_eq!(DriveStrength::Weak as u32, 3);
        assert_eq!(DriveStrength::Weaker as u32, 4);
        assert_eq!(DriveStrength::VeryWeak as u32, 5);
        assert_eq!(DriveStrength::Weakest as u32, 6);
        assert_eq!(DriveStrength::Minimum as u32, 7);
    }

    #[test]
    fn drive_strength_lands_in_bits_4_to_6() {
        // ds[2:0] must be shifted intact into register bits [6:4] (val == ds << 4).
        for (ds, expect) in [
            (DriveStrength::Strongest, 0u32),
            (DriveStrength::Strong, 1 << 4),
            (DriveStrength::Medium, 2 << 4),
            (DriveStrength::Minimum, 7 << 4),
        ] {
            let val = build_pad_ctrl(ds, PullResistor::None, false, false);
            assert_eq!(val & DS_MASK, expect, "ds {ds:?} mis-placed");
            // Drive strength must not bleed into any other field.
            assert_eq!(val & !DS_MASK, 0);
        }
    }

    #[test]
    fn drive_strength_covers_full_mask() {
        // The strongest..minimum span must exactly fill the 3-bit DS field.
        let strongest = build_pad_ctrl(DriveStrength::Strongest, PullResistor::None, false, false);
        let minimum = build_pad_ctrl(DriveStrength::Minimum, PullResistor::None, false, false);
        assert_eq!(strongest & DS_MASK, 0);
        assert_eq!(minimum & DS_MASK, DS_MASK);
    }

    #[test]
    fn pull_none_sets_neither_pe_nor_ps() {
        // No pull => both PE and PS clear.
        let val = build_pad_ctrl(DriveStrength::Strongest, PullResistor::None, false, false);
        assert_eq!(val & (PE_BIT | PS_BIT), 0);
    }

    #[test]
    fn pull_up_sets_pe_and_ps() {
        // Pull-up => PE=1, PS=1.
        let val = build_pad_ctrl(DriveStrength::Strongest, PullResistor::Up, false, false);
        assert_eq!(val & PE_BIT, PE_BIT);
        assert_eq!(val & PS_BIT, PS_BIT);
    }

    #[test]
    fn pull_down_sets_pe_only() {
        // Pull-down => PE=1, PS=0 (selecting the down resistor).
        let val = build_pad_ctrl(DriveStrength::Strongest, PullResistor::Down, false, false);
        assert_eq!(val & PE_BIT, PE_BIT);
        assert_eq!(val & PS_BIT, 0);
    }

    #[test]
    fn schmitt_and_input_enable_are_independent_bits() {
        // ST (bit3) and IE (bit11) toggle only their own bits.
        let none = build_pad_ctrl(DriveStrength::Strongest, PullResistor::None, false, false);
        assert_eq!(none & (ST_BIT | IE_BIT), 0);

        let st = build_pad_ctrl(DriveStrength::Strongest, PullResistor::None, true, false);
        assert_eq!(st, ST_BIT);

        let ie = build_pad_ctrl(DriveStrength::Strongest, PullResistor::None, false, true);
        assert_eq!(ie, IE_BIT);

        let both = build_pad_ctrl(DriveStrength::Strongest, PullResistor::None, true, true);
        assert_eq!(both, ST_BIT | IE_BIT);
    }

    #[test]
    fn all_fields_compose_without_collision() {
        // A fully-populated config must be the bitwise OR of every field — no
        // field may share or stomp another's bits.
        let val = build_pad_ctrl(DriveStrength::Minimum, PullResistor::Up, true, true);
        assert_eq!(val, DS_MASK | PE_BIT | PS_BIT | ST_BIT | IE_BIT);
    }

    #[test]
    fn no_bits_set_outside_known_fields() {
        // Every legal combination must stay within the defined field mask;
        // nothing should ever set a reserved bit.
        let defined = DS_MASK | PE_BIT | PS_BIT | ST_BIT | IE_BIT;
        for &drive in &[DriveStrength::Strongest, DriveStrength::Medium, DriveStrength::Minimum] {
            for &pull in &[PullResistor::None, PullResistor::Up, PullResistor::Down] {
                for st in [false, true] {
                    for ie in [false, true] {
                        let val = build_pad_ctrl(drive, pull, st, ie);
                        assert_eq!(val & !defined, 0, "reserved bit set for {drive:?}/{pull:?}/{st}/{ie}");
                    }
                }
            }
        }
    }
}

// ── Property-based fuzz tests ──────────────────────────────────

#[cfg(all(test, not(target_arch = "riscv32")))]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    const ST_BIT: u32 = 1 << 3;
    const DS_MASK: u32 = 0x7 << 4;
    const PE_BIT: u32 = 1 << 9;
    const PS_BIT: u32 = 1 << 10;
    const IE_BIT: u32 = 1 << 11;
    const DEFINED: u32 = DS_MASK | PE_BIT | PS_BIT | ST_BIT | IE_BIT;

    fn drive_from(i: u8) -> DriveStrength {
        match i % 8 {
            0 => DriveStrength::Strongest,
            1 => DriveStrength::Strong,
            2 => DriveStrength::Medium,
            3 => DriveStrength::Weak,
            4 => DriveStrength::Weaker,
            5 => DriveStrength::VeryWeak,
            6 => DriveStrength::Weakest,
            _ => DriveStrength::Minimum,
        }
    }

    fn pull_from(i: u8) -> PullResistor {
        match i % 3 {
            0 => PullResistor::None,
            1 => PullResistor::Up,
            _ => PullResistor::Down,
        }
    }

    proptest! {
        /// Fuzz: any field combination only ever sets defined bits, never reserved ones.
        #[test]
        fn never_sets_reserved_bits(d in any::<u8>(), p in any::<u8>(), st: bool, ie: bool) {
            let val = build_pad_ctrl(drive_from(d), pull_from(p), st, ie);
            prop_assert_eq!(val & !DEFINED, 0);
        }

        /// Fuzz: the DS field always equals the drive enum value shifted left by 4.
        #[test]
        fn ds_field_equals_discriminant_shifted(d in any::<u8>(), p in any::<u8>(), st: bool, ie: bool) {
            let drive = drive_from(d);
            let val = build_pad_ctrl(drive, pull_from(p), st, ie);
            prop_assert_eq!(val & DS_MASK, (drive as u32) << 4);
        }

        /// Fuzz: ST and IE bits track their boolean inputs exactly, regardless of other fields.
        #[test]
        fn st_ie_track_inputs(d in any::<u8>(), p in any::<u8>(), st: bool, ie: bool) {
            let val = build_pad_ctrl(drive_from(d), pull_from(p), st, ie);
            prop_assert_eq!(val & ST_BIT != 0, st);
            prop_assert_eq!(val & IE_BIT != 0, ie);
        }

        /// Fuzz: PE is set iff a pull is requested; PS is set iff that pull is up.
        #[test]
        fn pull_encoding_is_consistent(d in any::<u8>(), p in any::<u8>(), st: bool, ie: bool) {
            let pull = pull_from(p);
            let val = build_pad_ctrl(drive_from(d), pull, st, ie);
            let pe = val & PE_BIT != 0;
            let ps = val & PS_BIT != 0;
            match pull {
                PullResistor::None => { prop_assert!(!pe); prop_assert!(!ps); }
                PullResistor::Up => { prop_assert!(pe); prop_assert!(ps); }
                PullResistor::Down => { prop_assert!(pe); prop_assert!(!ps); }
            }
        }
    }
}
