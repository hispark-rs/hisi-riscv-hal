use crate::{hal, pac};

/// I2C0 SCL divider configuration (i2c.rs / examples/ws63/i2c_scan).
#[cfg(feature = "chip-ws63")]
pub(crate) fn i2c0_scl_config() {
    use hal::i2c::{I2c, Speed};

    // SAFETY: sequential single-hart run; I2C0 singleton not otherwise held.
    let _i2c = I2c::new_i2c0(unsafe { hal::peripherals::I2c0::steal() }, Speed::Standard);

    let pclk = hal::soc::chip::I2C_CLOCK_HZ;
    let expected_half = (pclk / (2 * Speed::Standard.hz())) / 2;
    // SAFETY: read-only MMIO loads of the I2C0 config registers.
    let r = unsafe { &*pac::I2c0::PTR };
    assert_eq!(r.i2c_scl_h().read().bits(), expected_half, "I2C0 scl_h mismatch");
    assert_eq!(r.i2c_scl_l().read().bits(), expected_half, "I2C0 scl_l mismatch");
    assert!(r.i2c_ctrl().read().i2c_en().bit_is_set(), "I2C0 i2c_en not set after new_i2c0");
}

/// I2C 7-bit address validation (i2c.rs). Invalid addresses fail before START.
#[cfg(feature = "chip-ws63")]
pub(crate) fn i2c0_rejects_invalid_7bit_address() {
    use embedded_hal::i2c::I2c as _;
    use hal::i2c::{I2c, I2cError, Speed};

    // SAFETY: sequential single-hart run; I2C0 singleton not otherwise held.
    let mut i2c = I2c::new_i2c0(unsafe { hal::peripherals::I2c0::steal() }, Speed::Standard);

    assert!(matches!(i2c.write(0x80, &[]), Err(I2cError::InvalidAddress)));
    let mut one = [0u8; 1];
    assert!(matches!(i2c.read(0x80, &mut one), Err(I2cError::InvalidAddress)));
    assert!(matches!(i2c.write_read(0x80, &[0], &mut one), Err(I2cError::InvalidAddress)));

    let mut ops = [embedded_hal::i2c::Operation::Write(&[0x00])];
    assert!(matches!(i2c.transaction(0x80, &mut ops), Err(I2cError::InvalidAddress)));
}

/// I2C v150 completes an unacknowledged address command with DONE + ACK_ERR.
///
/// Address 0x7f is reserved and no slave is present on the HIL rig. Returning
/// `Ack` proves the driver waited for DONE and then inspected ACK_ERR; polling
/// TX/RX instead would time out on this silicon path.
#[cfg(feature = "chip-ws63")]
pub(crate) fn i2c0_nack_is_reported_after_done() {
    use hal::i2c::{I2c, I2cError, Speed};
    use hal::io_config::{IoConfigDriver, MuxFunction, UartPad};

    // The vendor WS63 board route uses pads 15/16 in function 2 for I2C0.
    // SAFETY: sequential single-hart run; IO_CONFIG singleton not otherwise held.
    let mut io = IoConfigDriver::new(unsafe { hal::peripherals::IoConfig::steal() });
    io.set_uart_mux(UartPad::Uart1Txd, MuxFunction::F2);
    io.set_uart_mux(UartPad::Uart1Rxd, MuxFunction::F2);

    // SAFETY: sequential single-hart run; I2C0 singleton not otherwise held.
    let mut i2c = I2c::new_i2c0(unsafe { hal::peripherals::I2c0::steal() }, Speed::Standard);

    let result = i2c.write(0x7f, &[]);
    semihosting::println!("[i2c-done] reserved-address result={result:?}");
    assert!(matches!(result, Err(I2cError::Ack)));
}
