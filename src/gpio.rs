//! GPIO driver for WS63 (19 pins: GPIO0 bits 7-0, GPIO1 bits 15-8, GPIO2 bits 18-16).
//!
//! Three GPIO blocks at 0x4402_8000, 0x4402_9000, 0x4402_A000.
//!
//! # Pin drivers
//!
//! - [`Input`] — digital input with configurable pull
//! - [`Output`] — push-pull output
//! - [`Flex`] — combined input + output driver
//! - [`AnyPin`] — type-erased pin

use crate::peripherals::{Gpio0, Gpio1, Gpio2, IoConfig};
use core::marker::PhantomData;

// ── Configuration types ───────────────────────────────────────────

/// Pull resistor configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pull {
    None,
    Up,
    Down,
}

/// GPIO interrupt trigger condition (sets `GPIO_INT_TYPE` + `GPIO_INT_POLARITY`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptTrigger {
    /// Edge-sensitive, low→high transition.
    RisingEdge,
    /// Edge-sensitive, high→low transition.
    FallingEdge,
    /// Level-sensitive, asserted while high.
    HighLevel,
    /// Level-sensitive, asserted while low.
    LowLevel,
}

/// Digital input configuration.
#[derive(Debug, Clone, Copy)]
pub struct InputConfig {
    pub pull: Pull,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self { pull: Pull::None }
    }
}

impl InputConfig {
    pub const fn new() -> Self {
        Self { pull: Pull::None }
    }
    pub const fn with_pull(mut self, pull: Pull) -> Self {
        self.pull = pull;
        self
    }
}

/// Digital output configuration.
#[derive(Debug, Clone, Copy, Default)]
pub struct OutputConfig {
    pub open_drain: bool,
    pub initial_high: bool,
}

impl OutputConfig {
    pub const fn new() -> Self {
        Self { open_drain: false, initial_high: false }
    }
    pub const fn with_open_drain(mut self, od: bool) -> Self {
        self.open_drain = od;
        self
    }
    pub const fn with_initial(mut self, high: bool) -> Self {
        self.initial_high = high;
        self
    }
}

/// Mode marker types.
pub struct InputMode;
pub struct OutputMode;

// ── Type-erased pin ────────────────────────────────────────────────

/// A type-erased pin that can represent any GPIO pin.
pub struct AnyPin<'d> {
    block: u8,
    bit: u8,
    _lifetime: PhantomData<&'d mut ()>,
}

impl<'d> AnyPin<'d> {
    /// Create an `AnyPin` from a raw pin number without checking.
    ///
    /// # Safety
    /// The pin must be valid (0-18). This bypasses the type system.
    pub unsafe fn steal(pin: u8) -> Self {
        Self { block: pin / 8, bit: pin % 8, _lifetime: PhantomData }
    }

    /// Get the pin number (0-18).
    pub fn number(&self) -> u8 {
        self.block * 8 + self.bit
    }

    /// Set output enable (true = input, false = output).
    fn set_oen(&self, input: bool) {
        let r = regs(self.block);
        let mask = 1 << self.bit;
        if input {
            r.gpio_sw_oen().modify(|r, w| unsafe { w.bits(r.bits() | mask) });
        } else {
            r.gpio_sw_oen().modify(|r, w| unsafe { w.bits(r.bits() & !mask) });
        }
    }

    /// Initialize this pin as a GPIO input.
    ///
    /// Applies `config.pull` to the pad via the IO_CONFIG pad register
    /// (`apply_pull`; pins 0..=14 have a pad-control register).
    pub fn init_input(self, config: InputConfig) -> Input<'d> {
        self.set_oen(true);
        apply_pull(self.number(), config.pull);
        Input { pin: self, config }
    }

    /// Initialize this pin as a GPIO output.
    pub fn init_output(self, config: OutputConfig) -> Output<'d> {
        let mut out = Output { pin: self, config };
        if config.initial_high {
            out.set_high();
        } else {
            out.set_low();
        }
        out.pin.set_oen(false);
        out
    }

    /// Initialize this pin as a Flex (combined input/output) driver.
    pub fn init_flex(self, config: OutputConfig) -> Flex<'d> {
        if config.initial_high {
            unsafe { regs(self.block).gpio_data_set().write(|w| w.bits(1 << self.bit)) };
        } else {
            unsafe { regs(self.block).gpio_data_clr().write(|w| w.bits(1 << self.bit)) };
        }
        self.set_oen(false);
        Flex { pin: self, config }
    }
}

// ── Digital input driver ──────────────────────────────────────────

/// Digital input pin driver.
pub struct Input<'d> {
    pin: AnyPin<'d>,
    #[allow(dead_code)]
    config: InputConfig,
}

impl<'d> Input<'d> {
    pub fn is_high(&self) -> bool {
        (regs(self.pin.block).gpio_sw_out().read().bits() >> self.pin.bit) & 1 != 0
    }

    pub fn is_low(&self) -> bool {
        !self.is_high()
    }

    pub fn number(&self) -> u8 {
        self.pin.number()
    }

    pub fn enable_interrupt(&self) {
        let r = regs(self.pin.block);
        r.gpio_int_en().modify(|r, w| unsafe { w.bits(r.bits() | (1 << self.pin.bit)) });
    }

    pub fn disable_interrupt(&self) {
        let r = regs(self.pin.block);
        r.gpio_int_en().modify(|r, w| unsafe { w.bits(r.bits() & !(1 << self.pin.bit)) });
    }

    pub fn clear_interrupt(&self) {
        unsafe { regs(self.pin.block).gpio_int_eoi().write(|w| w.bits(1 << self.pin.bit)) };
    }

    /// Set the interrupt trigger condition for this pin (edge/level + polarity).
    ///
    /// Configures `GPIO_INT_TYPE` (edge vs level) and `GPIO_INT_POLARITY`
    /// (rising/high vs falling/low). Call before [`enable_interrupt`](Self::enable_interrupt).
    pub fn set_interrupt_trigger(&self, trigger: InterruptTrigger) {
        let r = regs(self.pin.block);
        let mask = 1u32 << self.pin.bit;
        let (edge, high) = match trigger {
            InterruptTrigger::RisingEdge => (true, true),
            InterruptTrigger::FallingEdge => (true, false),
            InterruptTrigger::HighLevel => (false, true),
            InterruptTrigger::LowLevel => (false, false),
        };
        r.gpio_int_type().modify(|r, w| unsafe { w.bits(if edge { r.bits() | mask } else { r.bits() & !mask }) });
        r.gpio_int_polarity().modify(|r, w| unsafe { w.bits(if high { r.bits() | mask } else { r.bits() & !mask }) });
    }

    pub fn interrupt_pending(&self) -> bool {
        (regs(self.pin.block).gpio_int_raw().read().bits() >> self.pin.bit) & 1 != 0
    }
}

impl embedded_hal::digital::ErrorType for Input<'_> {
    type Error = core::convert::Infallible;
}

impl embedded_hal::digital::InputPin for Input<'_> {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(Input::is_high(self))
    }
    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(Input::is_low(self))
    }
}

// ── Digital output driver ──────────────────────────────────────────

/// Digital output pin driver.
pub struct Output<'d> {
    pin: AnyPin<'d>,
    config: OutputConfig,
}

impl<'d> Output<'d> {
    pub fn set_high(&mut self) {
        unsafe { regs(self.pin.block).gpio_data_set().write(|w| w.bits(1 << self.pin.bit)) };
    }

    pub fn set_low(&mut self) {
        unsafe { regs(self.pin.block).gpio_data_clr().write(|w| w.bits(1 << self.pin.bit)) };
    }

    pub fn toggle(&mut self) {
        let r = regs(self.pin.block);
        let val = r.gpio_sw_out().read().bits();
        if val & (1 << self.pin.bit) != 0 {
            unsafe { r.gpio_data_clr().write(|w| w.bits(1 << self.pin.bit)) };
        } else {
            unsafe { r.gpio_data_set().write(|w| w.bits(1 << self.pin.bit)) };
        }
    }

    pub fn is_set_high(&self) -> bool {
        (regs(self.pin.block).gpio_sw_out().read().bits() >> self.pin.bit) & 1 != 0
    }

    pub fn number(&self) -> u8 {
        self.pin.number()
    }

    /// Convert this output into a Flex pin.
    pub fn into_flex(self) -> Flex<'d> {
        Flex { pin: self.pin, config: self.config }
    }
}

impl embedded_hal::digital::ErrorType for Output<'_> {
    type Error = core::convert::Infallible;
}

impl embedded_hal::digital::OutputPin for Output<'_> {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Output::set_low(self);
        Ok(())
    }
    fn set_high(&mut self) -> Result<(), Self::Error> {
        Output::set_high(self);
        Ok(())
    }
}

impl embedded_hal::digital::StatefulOutputPin for Output<'_> {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok(Output::is_set_high(self))
    }
    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!Output::is_set_high(self))
    }
}

// ── Flex pin driver (combined input + output) ─────────────────────

/// Combined input + output pin driver.
pub struct Flex<'d> {
    pin: AnyPin<'d>,
    #[allow(dead_code)]
    config: OutputConfig,
}

impl<'d> Flex<'d> {
    pub fn set_high(&mut self) {
        self.pin.set_oen(false);
        unsafe { regs(self.pin.block).gpio_data_set().write(|w| w.bits(1 << self.pin.bit)) };
    }

    pub fn set_low(&mut self) {
        self.pin.set_oen(false);
        unsafe { regs(self.pin.block).gpio_data_clr().write(|w| w.bits(1 << self.pin.bit)) };
    }

    pub fn toggle(&mut self) {
        let r = regs(self.pin.block);
        self.pin.set_oen(false);
        let val = r.gpio_sw_out().read().bits();
        if val & (1 << self.pin.bit) != 0 {
            unsafe { r.gpio_data_clr().write(|w| w.bits(1 << self.pin.bit)) };
        } else {
            unsafe { r.gpio_data_set().write(|w| w.bits(1 << self.pin.bit)) };
        }
    }

    pub fn is_set_high(&self) -> bool {
        (regs(self.pin.block).gpio_sw_out().read().bits() >> self.pin.bit) & 1 != 0
    }

    pub fn is_high(&self) -> bool {
        // Save output enable state, switch to input, read, restore
        let r = regs(self.pin.block);
        let oen = r.gpio_sw_oen().read().bits();
        self.pin.set_oen(true);
        let val = (r.gpio_sw_out().read().bits() >> self.pin.bit) & 1 != 0;
        // Restore original OEN state
        let mask = 1 << self.pin.bit;
        if oen & mask != 0 {
            r.gpio_sw_oen().modify(|reg, w| unsafe { w.bits(reg.bits() | mask) });
        } else {
            r.gpio_sw_oen().modify(|reg, w| unsafe { w.bits(reg.bits() & !mask) });
        }
        val
    }

    pub fn is_low(&self) -> bool {
        !self.is_high()
    }

    pub fn number(&self) -> u8 {
        self.pin.number()
    }
}

impl embedded_hal::digital::ErrorType for Flex<'_> {
    type Error = core::convert::Infallible;
}

impl embedded_hal::digital::OutputPin for Flex<'_> {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Flex::set_low(self);
        Ok(())
    }
    fn set_high(&mut self) -> Result<(), Self::Error> {
        Flex::set_high(self);
        Ok(())
    }
}

impl embedded_hal::digital::StatefulOutputPin for Flex<'_> {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok(Flex::is_set_high(self))
    }
    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!Flex::is_set_high(self))
    }
}

impl embedded_hal::digital::InputPin for Flex<'_> {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(Flex::is_high(self))
    }
    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(Flex::is_low(self))
    }
}

// ── Internal register access ──────────────────────────────────────

fn regs(block: u8) -> &'static ws63_pac::gpio0::RegisterBlock {
    unsafe {
        match block {
            0 => &*Gpio0::ptr(),
            1 => &*Gpio1::ptr(),
            2 => &*Gpio2::ptr(),
            _ => unreachable!(),
        }
    }
}

// ── Pad pull-resistor control (IO_CONFIG) ─────────────────────────
// The pull resistor lives in the per-pad IO_CONFIG control register, not the
// GPIO block. `pad_gpio_NN_ctrl` is at IO_CONFIG + 0x800 + N*4 (PAC layout) with
// PE = bit 9 (pull enable), PS = bit 10 (pull select); per the SVD the {PE,PS}
// pair encodes 00 = none, 11 = pull-up, 10 = pull-down (matches
// `io_config::build_pad_ctrl`). Only GPIO pads 0..=14 have a control register.
const IO_CONFIG_BASE: usize = 0x4400_D000;
const PAD_GPIO_CTRL_OFF: usize = 0x800;
const PAD_PE_BIT: u32 = 1 << 9;
const PAD_PS_BIT: u32 = 1 << 10;

/// Apply a pull-resistor setting to a GPIO pad via IO_CONFIG.
///
/// Read-modify-write so drive strength / Schmitt / input-enable bits are kept.
/// A no-op for pins 15..=18, which have no `pad_gpio_NN_ctrl` register in this
/// layout (their pull is configured through other pads / the ROM pin map).
fn apply_pull(pin: u8, pull: Pull) {
    if pin > 14 {
        return;
    }
    let reg = (IO_CONFIG_BASE + PAD_GPIO_CTRL_OFF + (pin as usize) * 4) as *mut u32;
    let (pe, ps) = match pull {
        Pull::None => (false, false),
        Pull::Up => (true, true),
        Pull::Down => (true, false),
    };
    unsafe {
        let mut v = core::ptr::read_volatile(reg);
        v &= !(PAD_PE_BIT | PAD_PS_BIT);
        if pe {
            v |= PAD_PE_BIT;
        }
        if ps {
            v |= PAD_PS_BIT;
        }
        core::ptr::write_volatile(reg, v);
    }
}

// ── Legacy GpioPin (backward-compatible type-state GPIO) ──────────

/// Legacy GPIO pin with type-state (Input/Output mode).
pub struct GpioPin<'d, MODE> {
    block: u8,
    bit: u8,
    _mode: PhantomData<&'d MODE>,
}

impl<MODE> GpioPin<'_, MODE> {
    pub fn number(&self) -> u8 {
        self.block * 8 + self.bit
    }
}

impl GpioPin<'_, OutputMode> {
    pub fn set_high(&mut self) {
        unsafe { regs(self.block).gpio_data_set().write(|w| w.bits(1 << self.bit)) };
    }
    pub fn set_low(&mut self) {
        unsafe { regs(self.block).gpio_data_clr().write(|w| w.bits(1 << self.bit)) };
    }
    pub fn toggle(&mut self) {
        let r = regs(self.block);
        let val = r.gpio_sw_out().read().bits();
        if val & (1 << self.bit) != 0 {
            unsafe { r.gpio_data_clr().write(|w| w.bits(1 << self.bit)) };
        } else {
            unsafe { r.gpio_data_set().write(|w| w.bits(1 << self.bit)) };
        }
    }
    pub fn is_set_high(&self) -> bool {
        (regs(self.block).gpio_sw_out().read().bits() >> self.bit) & 1 != 0
    }
    pub fn into_input(self) -> GpioPin<'static, InputMode> {
        regs(self.block).gpio_sw_oen().modify(|r, w| unsafe { w.bits(r.bits() | (1 << self.bit)) });
        GpioPin { block: self.block, bit: self.bit, _mode: PhantomData }
    }
}

impl GpioPin<'_, InputMode> {
    pub fn is_high(&self) -> bool {
        (regs(self.block).gpio_sw_out().read().bits() >> self.bit) & 1 != 0
    }
    pub fn is_low(&self) -> bool {
        !self.is_high()
    }
    pub fn enable_interrupt(&self) {
        regs(self.block).gpio_int_en().modify(|r, w| unsafe { w.bits(r.bits() | (1 << self.bit)) });
    }
    pub fn disable_interrupt(&self) {
        regs(self.block).gpio_int_en().modify(|r, w| unsafe { w.bits(r.bits() & !(1 << self.bit)) });
    }
    pub fn clear_interrupt(&self) {
        unsafe { regs(self.block).gpio_int_eoi().write(|w| w.bits(1 << self.bit)) };
    }
    pub fn interrupt_pending(&self) -> bool {
        (regs(self.block).gpio_int_raw().read().bits() >> self.bit) & 1 != 0
    }
    pub fn into_output(self) -> GpioPin<'static, OutputMode> {
        regs(self.block).gpio_sw_oen().modify(|r, w| unsafe { w.bits(r.bits() & !(1 << self.bit)) });
        GpioPin { block: self.block, bit: self.bit, _mode: PhantomData }
    }
}

// Legacy embedded-hal impls for GpioPin
impl embedded_hal::digital::ErrorType for GpioPin<'_, OutputMode> {
    type Error = core::convert::Infallible;
}
impl embedded_hal::digital::OutputPin for GpioPin<'_, OutputMode> {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        GpioPin::set_low(self);
        Ok(())
    }
    fn set_high(&mut self) -> Result<(), Self::Error> {
        GpioPin::set_high(self);
        Ok(())
    }
}
impl embedded_hal::digital::StatefulOutputPin for GpioPin<'_, OutputMode> {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok(GpioPin::is_set_high(self))
    }
    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!GpioPin::is_set_high(self))
    }
}
impl embedded_hal::digital::ErrorType for GpioPin<'_, InputMode> {
    type Error = core::convert::Infallible;
}
impl embedded_hal::digital::InputPin for GpioPin<'_, InputMode> {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(GpioPin::is_high(self))
    }
    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(GpioPin::is_low(self))
    }
}

// ── Pin creation functions ────────────────────────────────────────

/// Create an input pin from a pin number (0-18).
pub fn create_input_pin(pin: u8) -> GpioPin<'static, InputMode> {
    let block = pin / 8;
    let bit = pin % 8;
    regs(block).gpio_sw_oen().modify(|r, w| unsafe { w.bits(r.bits() | (1 << bit)) });
    GpioPin { block, bit, _mode: PhantomData }
}

/// Create an output pin from a pin number (0-18).
pub fn create_output_pin(pin: u8) -> GpioPin<'static, OutputMode> {
    let block = pin / 8;
    let bit = pin % 8;
    regs(block).gpio_sw_oen().modify(|r, w| unsafe { w.bits(r.bits() & !(1 << bit)) });
    GpioPin { block, bit, _mode: PhantomData }
}

// ── InputSignal / OutputSignal (peripheral interconnect) ──────────

/// An output signal from a peripheral that can be routed to a GPIO pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputSignal(pub(crate) u8);

/// An input signal to a peripheral that can be routed from a GPIO pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputSignal(pub(crate) u8);

/// Types that can serve as peripheral outputs (signals towards GPIO matrix).
pub trait PeripheralOutput: crate::private::Sealed {
    fn output_signal(&self) -> OutputSignal;
}

/// Types that can serve as peripheral inputs (signals from GPIO matrix towards peripherals).
pub trait PeripheralInput: crate::private::Sealed {
    fn input_signal(&self) -> InputSignal;
}

// Seal GPIO types for peripheral traits
impl crate::private::Sealed for Output<'_> {}
impl crate::private::Sealed for Input<'_> {}
impl crate::private::Sealed for Flex<'_> {}
impl crate::private::Sealed for GpioPin<'_, OutputMode> {}
impl crate::private::Sealed for GpioPin<'_, InputMode> {}

// ── IO MUX configuration ──────────────────────────────────────────

/// IO MUX configuration.
pub struct Io<'d> {
    pub io_config: IoConfig<'d>,
}

impl<'d> Io<'d> {
    pub fn new(io_config: IoConfig<'d>) -> Self {
        Self { io_config }
    }
    pub fn register_block(&self) -> &ws63_pac::io_config::RegisterBlock {
        self.io_config.register_block()
    }
}

// ── Async (embedded-hal-async) ──────────────────────────────────────────────
#[cfg(feature = "async")]
mod asynch_impl {
    use super::{regs, Input, InterruptTrigger};
    use crate::asynch::IrqSignal;
    use crate::interrupt::{self, Interrupt};
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll};
    use embedded_hal_async::digital::Wait;

    static GPIO_SIGNAL: [IrqSignal; 3] = [IrqSignal::new(), IrqSignal::new(), IrqSignal::new()];

    fn bank_irq(bank: usize) -> Interrupt {
        match bank {
            0 => Interrupt::GPIO_INT0,
            1 => Interrupt::GPIO_INT1,
            _ => Interrupt::GPIO_INT2,
        }
    }

    /// GPIO trap-handler hook for `bank` (0..2 → IRQ 33..35, custom local). Masks
    /// the fired pins (so they don't storm), clears their edge latch, wakes the
    /// awaiting [`Wait`] future, and clears the `LOCIPCLR` pending bit. Call this
    /// from the trap when `mcause` is GPIO_INT0..2.
    pub fn on_interrupt(bank: u8) {
        let r = regs(bank);
        let fired = r.gpio_int_raw().read().bits();
        // Mask the fired pins (a fresh wait re-enables) + clear the edge latch.
        r.gpio_int_en()
            .modify(|v, w| unsafe { w.bits(v.bits() & !fired) });
        unsafe { r.gpio_int_eoi().write(|w| w.bits(fired)) };
        GPIO_SIGNAL[bank as usize].signal();
        interrupt::clear_pending(bank_irq(bank as usize));
    }

    async fn arm_and_wait(input: &mut Input<'_>, trig: InterruptTrigger) {
        let bank = input.pin.block as usize;
        input.set_interrupt_trigger(trig);
        input.clear_interrupt();
        GPIO_SIGNAL[bank].reset();
        input.enable_interrupt();
        // SAFETY: enabling a known, fixed WS63 GPIO IRQ line.
        unsafe { interrupt::enable(bank_irq(bank)) };
        GpioWaitFuture { bank }.await;
    }

    struct GpioWaitFuture {
        bank: usize,
    }

    impl Future for GpioWaitFuture {
        type Output = ();
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if GPIO_SIGNAL[self.bank].take_fired() {
                Poll::Ready(())
            } else {
                GPIO_SIGNAL[self.bank].register(cx.waker());
                Poll::Pending
            }
        }
    }

    /// Interrupt-driven async edge/level waiting on a GPIO input. Requires the
    /// app to route the GPIO trap to [`on_interrupt`] and enable global
    /// interrupts. Level waits return immediately if the pin already matches.
    impl Wait for Input<'_> {
        async fn wait_for_high(&mut self) -> Result<(), Self::Error> {
            if self.is_high() {
                return Ok(());
            }
            arm_and_wait(self, InterruptTrigger::HighLevel).await;
            Ok(())
        }
        async fn wait_for_low(&mut self) -> Result<(), Self::Error> {
            if self.is_low() {
                return Ok(());
            }
            arm_and_wait(self, InterruptTrigger::LowLevel).await;
            Ok(())
        }
        async fn wait_for_rising_edge(&mut self) -> Result<(), Self::Error> {
            arm_and_wait(self, InterruptTrigger::RisingEdge).await;
            Ok(())
        }
        async fn wait_for_falling_edge(&mut self) -> Result<(), Self::Error> {
            arm_and_wait(self, InterruptTrigger::FallingEdge).await;
            Ok(())
        }
        async fn wait_for_any_edge(&mut self) -> Result<(), Self::Error> {
            let trig = if self.is_high() {
                InterruptTrigger::FallingEdge
            } else {
                InterruptTrigger::RisingEdge
            };
            arm_and_wait(self, trig).await;
            Ok(())
        }
    }
}

#[cfg(feature = "async")]
pub use asynch_impl::on_interrupt;
