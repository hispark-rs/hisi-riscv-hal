//! PKE (Public Key Engine) driver for WS63.
//!
//! The PKE peripheral accelerates modular arithmetic operations used in:
//! - RSA (modular exponentiation)
//! - ECC (elliptic curve point operations)
//! - SM2 (Chinese national standard ECC)

use crate::peripherals::Pke;

/// PKE driver.
pub struct PkeDriver<'d> {
    _pke: Pke<'d>,
}

/// PKE operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PkeOp {
    /// Modular exponentiation (RSA).
    ModExp,
    /// ECC point multiplication.
    EccPointMul,
    /// ECC point addition.
    EccPointAdd,
    /// ECC point verification.
    EccPointVerify,
}

impl<'d> PkeDriver<'d> {
    /// Create a new PKE driver.
    pub fn new(pke: Pke<'d>) -> Self {
        Self { _pke: pke }
    }

    fn regs(&self) -> &'static ws63_pac::pke::RegisterBlock {
        unsafe { &*Pke::ptr() }
    }

    /// Enable the PKE peripheral.
    pub fn enable(&mut self) {}

    /// Disable the PKE peripheral.
    pub fn disable(&mut self) {}

    /// Check if the engine is busy.
    pub fn is_busy(&self) -> bool {
        false
    }
}
