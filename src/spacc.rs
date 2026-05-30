//! SPACC (Security Accelerator) driver for WS63.
//!
//! The SPACC peripheral provides hardware acceleration for:
//! - AES (128/192/256)
//! - SM4 (Chinese national standard)
//! - LEA (Lightweight Encryption Algorithm)
//! - TDES (Triple DES)
//! - HASH (SHA-256, SM3)
//! - HMAC

use crate::peripherals::Spacc;

/// SPACC driver.
pub struct SpaccDriver<'d> {
    _spacc: Spacc<'d>,
}

/// Cipher algorithm selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherAlg {
    Aes128,
    Aes192,
    Aes256,
    Sm4,
    Lea,
    Tdes,
}

/// Cipher operation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherMode {
    Ecb,
    Cbc,
    Ctr,
    Ccm,
    Gcm,
}

/// Cipher direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherDir {
    Encrypt,
    Decrypt,
}

/// Hash algorithm selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlg {
    Sha256,
    Sm3,
}

impl<'d> SpaccDriver<'d> {
    /// Create a new SPACC driver.
    pub fn new(spacc: Spacc<'d>) -> Self {
        Self { _spacc: spacc }
    }

    fn regs(&self) -> &'static ws63_pac::spacc::RegisterBlock {
        // SAFETY: PAC peripheral pointer is a static physical MMIO address, always valid
        unsafe { &*Spacc::ptr() }
    }

    /// Enable the SPACC peripheral.
    pub fn enable(&mut self) {
        // Enable clock via the security clock gate
        // Hardware-specific initialization
    }

    /// Disable the SPACC peripheral.
    pub fn disable(&mut self) {
        // Disable clock
    }

    /// Check if the accelerator is busy.
    pub fn is_busy(&self) -> bool {
        false // Hardware-specific status check
    }
}
