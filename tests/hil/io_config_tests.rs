use crate::{hal, pac};

/// IO_CONFIG mux roundtrip: set GPIO mux function, readback, confirm it matches.
#[cfg(feature = "chip-ws63")]
pub(crate) fn io_config_mux_roundtrip() {
    use hal::io_config::{GpioPad, IoConfigDriver, MuxFunction};

    // SAFETY: sequential single-hart run; IO_CONFIG singleton not otherwise held.
    let mut io = IoConfigDriver::new(unsafe { hal::peripherals::IoConfig::steal() });

    let orig = io.gpio_mux(GpioPad::Gpio01);
    io.set_gpio_mux(GpioPad::Gpio01, MuxFunction::F0);
    assert_eq!(
        io.gpio_mux(GpioPad::Gpio01),
        MuxFunction::F0,
        "GPIO1 mux readback after F0 set: got {:?}",
        io.gpio_mux(GpioPad::Gpio01)
    );
    io.set_gpio_mux(GpioPad::Gpio01, MuxFunction::F1);
    assert_eq!(
        io.gpio_mux(GpioPad::Gpio01),
        MuxFunction::F1,
        "GPIO1 mux readback after F1 set: got {:?}",
        io.gpio_mux(GpioPad::Gpio01)
    );
    io.set_gpio_mux(GpioPad::Gpio01, orig);
}

/// IO_CONFIG pad readback: configure GPIO pad pull/drive, read register, verify fields.
#[cfg(feature = "chip-ws63")]
pub(crate) fn io_config_pad_readback() {
    use hal::io_config::{DriveStrength, GpioPad, IoConfigDriver, PullResistor};

    // SAFETY: sequential single-hart run; IO_CONFIG singleton not otherwise held.
    let mut io = IoConfigDriver::new(unsafe { hal::peripherals::IoConfig::steal() });
    let saved = io.read_gpio_pad(GpioPad::Gpio01);

    // configure_gpio_pad(pin, drive, pull, schmitt_trigger, input_enable)
    io.configure_gpio_pad(GpioPad::Gpio01, DriveStrength::Medium, PullResistor::Down, false, false);
    let val = io.read_gpio_pad(GpioPad::Gpio01);
    assert_eq!((val >> 9) & 1, 1, "pull-enable bit (PE, bit 9) not set after configure_gpio_pad");

    // Restore original pad configuration.
    let r = unsafe { &*pac::IoConfig::PTR };
    unsafe {
        r.pad_gpio_01_ctrl().write(|w| w.bits(saved));
    }
}
