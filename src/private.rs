//! Private module for sealed traits.
//!
//! These traits cannot be implemented outside this crate, enabling
//! the sealed trait pattern used throughout the HAL.

/// A trait that is sealed (cannot be implemented outside this crate).
pub trait Sealed {}

// ── DMA-related sealed traits ─────────────────────────────────────

/// Marker trait for types that can be used as DMA transfer words.
pub trait DmaWord: Sealed + Copy + 'static {}

impl Sealed for u8 {}
impl DmaWord for u8 {}
impl Sealed for u16 {}
impl DmaWord for u16 {}
impl Sealed for u32 {}
impl DmaWord for u32 {}

// ── GPIO-related sealed traits ───────────────────────────────────

/// Types that can serve as peripheral inputs (signals from GPIO matrix towards peripherals).
pub trait PeripheralOutput: Sealed {}

/// Types that can serve as peripheral outputs (signals from peripherals towards GPIO matrix).
pub trait PeripheralInput: Sealed {}

// ── Driver mode sealed traits ─────────────────────────────────────

/// Trait for driver operation mode (blocking or async).
pub trait DriverMode: Sealed + Sized {
    /// Convert a blocking-mode driver instance to async mode.
    type Async<D>;
}

/// Marker type for blocking (synchronous) driver operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Blocking;
impl Sealed for Blocking {}
impl DriverMode for Blocking {
    type Async<D> = D;
}

/// Marker type for async driver operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Async;
impl Sealed for Async {}
impl DriverMode for Async {
    type Async<D> = D;
}
