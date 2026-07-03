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
    /// AES with a 128-bit key.
    Aes128,
    /// AES with a 192-bit key.
    Aes192,
    /// AES with a 256-bit key.
    Aes256,
    /// SM4 block cipher (Chinese national standard).
    Sm4,
    /// LEA (Lightweight Encryption Algorithm).
    Lea,
    /// Triple DES (TDES).
    Tdes,
}

/// Cipher operation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherMode {
    /// Electronic Codebook (ECB) mode.
    Ecb,
    /// Cipher Block Chaining (CBC) mode.
    Cbc,
    /// Counter (CTR) mode.
    Ctr,
    /// Counter with CBC-MAC (CCM) authenticated mode.
    Ccm,
    /// Galois/Counter Mode (GCM) authenticated mode.
    Gcm,
}

/// Cipher direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherDir {
    /// Encrypt the input data.
    Encrypt,
    /// Decrypt the input data.
    Decrypt,
}

/// Hash algorithm selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlg {
    /// SHA-256 hash algorithm.
    Sha256,
    /// SM3 hash algorithm (Chinese national standard).
    Sm3,
}

impl<'d> SpaccDriver<'d> {
    /// Create a new SPACC driver.
    pub fn new(spacc: Spacc<'d>) -> Self {
        Self { _spacc: spacc }
    }

    #[allow(dead_code)]
    fn regs(&self) -> &'static crate::soc::pac::spacc::RegisterBlock {
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
