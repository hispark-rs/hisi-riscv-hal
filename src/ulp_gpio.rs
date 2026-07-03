//! Ultra-Low-Power GPIO (ULP_GPIO) driver for WS63.
//!
//! The WS63 ULP_GPIO provides 8 additional GPIO pins (GPIO107-114) that
//! can operate in ultra-low-power modes. The register layout is identical
//! to the standard GPIO blocks.
//!
//! These pins are accessible at physical address 0x5703_0000 and use
//! the same register layout as GPIO0.

use crate::peripherals::UlpGpio;
use core::marker::PhantomData;

/// ULP GPIO pin mode marker types.
pub struct Input;
/// Marker type for a ULP GPIO pin configured in output mode.
pub struct Output;

/// ULP GPIO pin driver.
pub struct UlpGpioPin<'d, MODE> {
    bit: u8,
    _mode: PhantomData<&'d MODE>,
}

fn regs() -> &'static crate::soc::pac::gpio0::RegisterBlock {
    // SAFETY: PAC peripheral pointer is a static physical MMIO address, always valid
    unsafe { &*UlpGpio::ptr() }
}

impl<MODE> UlpGpioPin<'_, MODE> {
    /// Get the pin number (107-114).
    pub fn number(&self) -> u8 {
        107 + self.bit
    }
}

impl UlpGpioPin<'_, Output> {
    /// Set the pin high.
    pub fn set_high(&mut self) {
        unsafe { regs().gpio_data_set().write(|w| w.bits(1 << self.bit)) };
    }

    /// Set the pin low.
    pub fn set_low(&mut self) {
        unsafe { regs().gpio_data_clr().write(|w| w.bits(1 << self.bit)) };
    }

    /// Toggle the pin output level.
    pub fn toggle(&mut self) {
        let val = regs().gpio_sw_out().read().bits();
        if val & (1 << self.bit) != 0 {
            unsafe { regs().gpio_data_clr().write(|w| w.bits(1 << self.bit)) };
        } else {
            unsafe { regs().gpio_data_set().write(|w| w.bits(1 << self.bit)) };
        }
    }

    /// Check if the output is set high.
    pub fn is_set_high(&self) -> bool {
        (regs().gpio_sw_out().read().bits() >> self.bit) & 1 != 0
    }

    /// Convert the pin to input mode.
    pub fn into_input(self) -> UlpGpioPin<'static, Input> {
        regs().gpio_sw_oen().modify(|r, w| unsafe { w.bits(r.bits() | (1 << self.bit)) });
        UlpGpioPin { bit: self.bit, _mode: PhantomData }
    }
}

impl UlpGpioPin<'_, Input> {
    /// Check if the input is high.
    pub fn is_high(&self) -> bool {
        (regs().gpio_sw_out().read().bits() >> self.bit) & 1 != 0
    }

    /// Check if the input is low.
    pub fn is_low(&self) -> bool {
        !self.is_high()
    }

    /// Enable the interrupt for this pin.
    pub fn enable_interrupt(&self) {
        regs().gpio_int_en().modify(|r, w| unsafe { w.bits(r.bits() | (1 << self.bit)) });
    }

    /// Disable the interrupt for this pin.
    pub fn disable_interrupt(&self) {
        regs().gpio_int_en().modify(|r, w| unsafe { w.bits(r.bits() & !(1 << self.bit)) });
    }

    /// Clear the interrupt for this pin.
    pub fn clear_interrupt(&self) {
        unsafe { regs().gpio_int_eoi().write(|w| w.bits(1 << self.bit)) };
    }

    /// Check if an interrupt is pending for this pin.
    pub fn interrupt_pending(&self) -> bool {
        (regs().gpio_int_raw().read().bits() >> self.bit) & 1 != 0
    }

    /// Convert the pin to output mode.
    pub fn into_output(self) -> UlpGpioPin<'static, Output> {
        regs().gpio_sw_oen().modify(|r, w| unsafe { w.bits(r.bits() & !(1 << self.bit)) });
        UlpGpioPin { bit: self.bit, _mode: PhantomData }
    }
}

// ── embedded-hal traits ────────────────────────────────────────────

impl embedded_hal::digital::ErrorType for UlpGpioPin<'_, Output> {
    type Error = core::convert::Infallible;
}
impl embedded_hal::digital::OutputPin for UlpGpioPin<'_, Output> {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        UlpGpioPin::set_low(self);
        Ok(())
    }
    fn set_high(&mut self) -> Result<(), Self::Error> {
        UlpGpioPin::set_high(self);
        Ok(())
    }
}
impl embedded_hal::digital::StatefulOutputPin for UlpGpioPin<'_, Output> {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok(UlpGpioPin::is_set_high(self))
    }
    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!UlpGpioPin::is_set_high(self))
    }
}
impl embedded_hal::digital::ErrorType for UlpGpioPin<'_, Input> {
    type Error = core::convert::Infallible;
}
impl embedded_hal::digital::InputPin for UlpGpioPin<'_, Input> {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(UlpGpioPin::is_high(self))
    }
    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!UlpGpioPin::is_high(self))
    }
}

/// Create a ULP GPIO pin in input mode.
pub fn create_input_pin(pin: u8) -> UlpGpioPin<'static, Input> {
    let bit = pin - 107;
    assert!(bit < 8, "ULP GPIO pin must be 107-114");
    regs().gpio_sw_oen().modify(|r, w| unsafe { w.bits(r.bits() | (1 << bit)) });
    UlpGpioPin { bit, _mode: PhantomData }
}

/// Create a ULP GPIO pin in output mode.
pub fn create_output_pin(pin: u8) -> UlpGpioPin<'static, Output> {
    let bit = pin - 107;
    assert!(bit < 8, "ULP GPIO pin must be 107-114");
    regs().gpio_sw_oen().modify(|r, w| unsafe { w.bits(r.bits() & !(1 << bit)) });
    UlpGpioPin { bit, _mode: PhantomData }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(all(test, not(target_arch = "riscv32")))]
mod tests {
    // The ULP block exposes 8 pins (GPIO107..=GPIO114) mapped to bit indices
    // 0..=7. These tests cover the pure pin<->bit arithmetic only; no register
    // access (`regs()` dereferences MMIO 0x5703_0000 and would segfault on host).

    /// The first pin (107) maps to bit 0.
    const BASE_PIN: u8 = 107;
    /// 8 usable ULP pins → bit indices 0..=7.
    const PIN_COUNT: u8 = 8;

    /// Mirror of `UlpGpioPin::number()`: bit index → pin number.
    fn pin_number(bit: u8) -> u8 {
        BASE_PIN + bit
    }

    /// Mirror of the `create_*_pin` mapping: pin number → bit index.
    fn pin_to_bit(pin: u8) -> u8 {
        pin - BASE_PIN
    }

    #[test]
    fn number_of_first_pin() {
        // bit 0 is GPIO107 (the block's base pin).
        assert_eq!(pin_number(0), 107);
    }

    #[test]
    fn number_of_last_pin() {
        // bit 7 is GPIO114, the last ULP pin.
        assert_eq!(pin_number(PIN_COUNT - 1), 114);
    }

    #[test]
    fn pin_to_bit_round_trips() {
        // pin_to_bit and pin_number are exact inverses across the whole block.
        for bit in 0..PIN_COUNT {
            let pin = pin_number(bit);
            assert_eq!(pin_to_bit(pin), bit);
        }
    }

    #[test]
    fn valid_pins_pass_bit_bound() {
        // Every pin 107..=114 derives a bit < 8 (the `assert!(bit < 8)` guard).
        for pin in 107..=114u8 {
            assert!(pin_to_bit(pin) < PIN_COUNT, "pin {pin} should be valid");
        }
    }

    #[test]
    fn first_out_of_range_pin_fails_bit_bound() {
        // GPIO115 derives bit 8, which the constructor's `assert!(bit < 8)` rejects.
        assert_eq!(pin_to_bit(115), PIN_COUNT);
        assert!(!(pin_to_bit(115) < PIN_COUNT));
    }

    #[test]
    fn bit_mask_is_single_bit() {
        // The drivers select a pin with `1 << bit`; each mask has exactly one set bit
        // and they are mutually exclusive across the 8 pins.
        let mut seen: u32 = 0;
        for bit in 0..PIN_COUNT {
            let mask = 1u32 << bit;
            assert_eq!(mask.count_ones(), 1);
            assert_eq!(seen & mask, 0, "bit {bit} mask overlaps a previous pin");
            seen |= mask;
        }
        // All 8 pins together occupy the low byte.
        assert_eq!(seen, 0xFF);
    }
}

// ── Property-based fuzz tests ──────────────────────────────────

#[cfg(all(test, not(target_arch = "riscv32")))]
mod proptests {
    use proptest::prelude::*;

    const BASE_PIN: u8 = 107;
    const PIN_COUNT: u8 = 8;

    proptest! {
        /// Fuzz: across the valid range, pin → bit → pin is a lossless round-trip.
        #[test]
        fn pin_bit_round_trip(pin in 107u8..=114) {
            let bit = pin - BASE_PIN;
            prop_assert!(bit < PIN_COUNT);
            prop_assert_eq!(BASE_PIN + bit, pin);
        }

        /// Fuzz: the `1 << bit` selection mask always has exactly one bit set,
        /// for every in-range pin index.
        #[test]
        fn single_bit_mask(bit in 0u8..PIN_COUNT) {
            let mask = 1u32 << bit;
            prop_assert_eq!(mask.count_ones(), 1);
        }
    }
}
