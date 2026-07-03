use crate::{hal, pac};

/// GPIO output read-back (gpio.rs / examples/ws63/blinky).
pub(crate) fn gpio_output_readback() {
    use hal::gpio::{AnyPin, OutputConfig};

    // SAFETY: pin 0 is a valid WS63 GPIO (0..=18); sequential single-hart run owns it exclusively.
    let mut led = unsafe { AnyPin::steal(0) }.init_output(OutputConfig::new().with_initial(false));

    led.set_high();
    // SAFETY: read-only MMIO load of the GPIO0 data register.
    let r = unsafe { &*pac::Gpio0::PTR };
    assert_eq!(r.gpio_sw_out().read().bits() & 1, 1, "GPIO0 bit0 did not read high after set_high()");
    assert!(led.is_set_high(), "Output::is_set_high() disagreed after set_high()");

    led.set_low();
    assert_eq!(r.gpio_sw_out().read().bits() & 1, 0, "GPIO0 bit0 did not read low after set_low()");
    assert!(!led.is_set_high(), "Output::is_set_high() disagreed after set_low()");
}

/// GPIO output→input loopback (gpio.rs + io_config.rs). Requires GPIO0 → GPIO3 jumper.
#[cfg(all(feature = "chip-ws63", feature = "hil-loopback"))]
pub(crate) fn gpio_loopback_0_to_3() {
    use core::hint::black_box;
    use hal::gpio::{AnyPin, InputConfig, OutputConfig, Pull};
    use hal::io_config::{GpioPad, IoConfigDriver, MuxFunction};

    let mut io = IoConfigDriver::new(unsafe { hal::peripherals::IoConfig::steal() });
    io.set_gpio_mux(GpioPad::Gpio00, MuxFunction::F0);
    io.set_gpio_mux(GpioPad::Gpio03, MuxFunction::F0);

    let settle = || {
        for _ in 0..50_000u32 {
            black_box(0u32);
        }
    };

    // SAFETY: GPIO0/3 owned by this test; jumpered 0->3 on the board.
    let mut out = unsafe { AnyPin::steal(0) }.init_output(OutputConfig::new().with_initial(false));
    let inp = unsafe { AnyPin::steal(3) }.init_input(InputConfig::new().with_pull(Pull::Down));

    out.set_high();
    settle();
    let hi = inp.is_high();
    out.set_low();
    settle();
    let lo = inp.is_high();

    semihosting::println!("[gpio-lb] GPIO0->GPIO3: drive-high read={hi} drive-low read={lo}");
    assert!(hi, "GPIO3 did not read high when GPIO0 driven high — check GPIO0->GPIO3 jumper");
    assert!(!lo, "GPIO3 did not read low when GPIO0 driven low — check GPIO0->GPIO3 jumper");
}
