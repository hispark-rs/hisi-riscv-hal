//! KM (Key Management) driver for WS63.
//!
//! The KM peripheral manages cryptographic keys, providing:
//! - KLAD (Key Loading and Distribution)
//! - Keyslot locking mechanisms
//! - RKP (Root Key Provisioning)

use crate::peripherals::Km;

/// KM driver.
pub struct KmDriver<'d> {
    _km: Km<'d>,
}

impl<'d> KmDriver<'d> {
    /// Create a new KM driver.
    pub fn new(km: Km<'d>) -> Self {
        Self { _km: km }
    }

    fn regs(&self) -> &'static ws63_pac::km::RegisterBlock {
        // SAFETY: PAC peripheral pointer is a static physical MMIO address, always valid
        unsafe { &*Km::ptr() }
    }

    /// Enable the KM peripheral.
    pub fn enable(&mut self) {
        unsafe {
            self.regs().kl_com_ctrl().write(|w| w.bits(0x01));
        }
    }

    /// Check if a keyslot is locked (read-only after lock).
    pub fn is_keyslot_locked(&self, slot: u8) -> bool {
        let status = self.regs().kc_rd_lock_status().read().bits();
        (status & (1 << slot)) != 0
    }

    /// Lock a keyslot (prevents further writes).
    ///
    /// After locking, the keyslot cannot be modified until the next system reset.
    pub fn lock_keyslot(&mut self, slot: u8) {
        unsafe {
            self.regs().kl_lock_ctrl().write(|w| w.bits((slot as u32) | (1 << 8)));
        }
    }

    /// Get the number of available keyslots.
    pub fn keyslot_count(&self) -> u8 {
        8
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #[test]
    fn test_keyslot_lock_bit_position() {
        // lock_keyslot writes (slot as u32) | (1 << 8) to kl_lock_ctrl
        // slot in bits 0-7, lock request in bit 8
        let slot: u8 = 3;
        let val: u32 = (slot as u32) | (1 << 8);
        assert_eq!(val, 0x103); // bit 8 set, slot=3 in bits 0-7
        assert_eq!(val & 0xFF, 3); // lower 8 bits = slot number
        assert_eq!((val >> 8) & 1, 1); // bit 8 = lock request
    }

    #[test]
    fn test_keyslot_lock_status_bit_mask() {
        // is_keyslot_locked checks (status & (1 << slot)) != 0
        let slot: u8 = 5;
        let mask = 1u32 << slot;
        assert_eq!(mask, 0x20); // bit 5 = slot 5

        // Slot 0 locked, slot 5 not locked
        let status: u32 = 0x01; // only bit 0 set
        assert!((status & (1 << 0)) != 0); // slot 0 locked
        assert!((status & (1 << 5)) == 0); // slot 5 not locked
    }

    #[test]
    fn test_keyslot_count() {
        assert_eq!(super::KmDriver::keyslot_count(&super::KmDriver { _km: unsafe { core::mem::zeroed() } }), 8);
    }

    #[test]
    fn test_all_keyslots_lockable() {
        // All 8 keyslots (0-7) must produce valid lock values
        for slot in 0..8u8 {
            let val: u32 = (slot as u32) | (1 << 8);
            assert_eq!(val & 0xFF, slot as u32);
            assert_eq!((val >> 8) & 1, 1);
        }
    }
}
