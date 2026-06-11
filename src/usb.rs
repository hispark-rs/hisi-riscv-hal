//! BS2X USB 2.0 OTG — Synopsys DesignWare DWC2 OTG controller, device mode.
//!
//! BS2X-only (`chip-bs21`); no WS63 analogue. This brings the DWC2 OTG **device
//! controller** up to the enumerated state: core soft-reset → force device mode →
//! global config → device config → soft-connect → wait for the host USB-reset and
//! enumeration-done, then read the negotiated speed. Higher layers (descriptors,
//! endpoint transfers, the USB class stack) sit on top and are out of scope here.
//!
//! Sequence + bits from the fbb_bs2x DWC OTG PCD (`dwc_otgreg.h` / `dwc_otg_pcd.h`,
//! the canonical Synopsys DWC2 device init). All registers are in bs2x-pac's `usb`
//! block; `Usb` @ 0x5800_0000. The PHY/PMU power-on (a CRG/PHY concern) is assumed
//! done by the boot/clock layer.

use crate::peripherals::Usb as UsbPeriph;
use core::marker::PhantomData;

/// The Synopsys signature in the top half of GSNPSID ("OT").
pub const SNPS_SIGNATURE: u16 = 0x4F54;

const POLL_LIMIT: u32 = 1_000_000;

#[derive(Debug, PartialEq, Eq)]
pub enum UsbError {
    /// A status bit (reset-done / USB reset / enumeration done) never asserted.
    Timeout,
    /// The DWC OTG core-ID signature was wrong (controller not present).
    NotPresent,
}

/// The USB speed the host enumerated the device at (`DSTS.ENUMSPD`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speed {
    High,
    Full,
    Low,
    FullFs,
}

pub struct Usb<'d> {
    _u: PhantomData<UsbPeriph<'d>>,
}

impl<'d> Usb<'d> {
    fn regs(&self) -> &'static crate::soc::pac::usb::RegisterBlock {
        // SAFETY: static physical MMIO base (0x5800_0000) from bs2x-pac.
        unsafe { &*UsbPeriph::ptr() }
    }

    pub fn new(_u: UsbPeriph<'d>) -> Self {
        Self { _u: PhantomData }
    }

    /// Read the DWC OTG core-ID register (`gsnpsid`); the top 16 bits are the
    /// Synopsys "OT" signature.
    pub fn core_id(&self) -> u32 {
        self.regs().gsnpsid().read().bits()
    }

    /// True if the core-ID carries the Synopsys DWC OTG signature.
    pub fn is_present(&self) -> bool {
        (self.core_id() >> 16) as u16 == SNPS_SIGNATURE
    }

    /// Bring the DWC2 OTG device controller up and wait until the host has reset
    /// and enumerated it; returns the negotiated [`Speed`].
    ///
    /// Steps (DWC2 device init): core soft reset → force device mode → GAHBCFG +
    /// interrupt unmask → DCFG → enable global interrupts → soft-connect (clear
    /// `DCTL.SFTDISCON`) → wait `GINTSTS.USBRST` → wait `GINTSTS.ENUMDONE` → read
    /// `DSTS.ENUMSPD`.
    pub fn device_enumerate(&mut self) -> Result<Speed, UsbError> {
        let r = self.regs();
        if !self.is_present() {
            return Err(UsbError::NotPresent);
        }

        // A. Core soft reset: wait AHB idle, assert CSFTRST, wait it self-clear.
        self.poll(|| r.grstctl().read().ahbidle().bit_is_set())?;
        r.grstctl().write(|w| w.csftrst().set_bit());
        self.poll(|| r.grstctl().read().csftrst().bit_is_clear())?;
        self.poll(|| r.grstctl().read().ahbidle().bit_is_set())?;

        // B. Global config: force device mode, clear pending, unmask reset + enum.
        r.gusbcfg().modify(|_, w| w.forcedevmode().set_bit());
        unsafe {
            r.gintsts().write(|w| w.bits(0xFFFF_FFFF)); // clear all (W1C)
        }
        r.gintmsk().modify(|_, w| {
            w.usbrstmsk().set_bit();
            w.enumdonemsk().set_bit()
        });

        // C. Device config: high speed, address 0.
        unsafe {
            r.dcfg().modify(|_, w| {
                w.devspd().bits(0); // 0 = high speed
                w.devaddr().bits(0)
            });
        }

        // D. Enable global interrupts.
        r.gahbcfg().modify(|_, w| w.glblintrmsk().set_bit());

        // E. Soft-connect (attach D+ pull-up): clear SFTDISCON.
        r.dctl().modify(|_, w| w.sftdiscon().clear_bit());

        // F. Wait for the host reset + enumeration, clearing each (W1C).
        self.poll(|| r.gintsts().read().usbrst().bit_is_set())?;
        r.gintsts().write(|w| w.usbrst().set_bit());
        self.poll(|| r.gintsts().read().enumdone().bit_is_set())?;
        r.gintsts().write(|w| w.enumdone().set_bit());

        // G. Read the negotiated speed.
        Ok(match r.dsts().read().enumspd().bits() {
            0 => Speed::High,
            1 => Speed::FullFs,
            2 => Speed::Low,
            _ => Speed::Full,
        })
    }

    fn poll(&self, mut ready: impl FnMut() -> bool) -> Result<(), UsbError> {
        for _ in 0..POLL_LIMIT {
            if ready() {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(UsbError::Timeout)
    }
}
