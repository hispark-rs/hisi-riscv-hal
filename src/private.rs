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

// The `DriverMode` / `Blocking` / `Async` marker traits were removed: every
// associated type was the identity (`type Async<D> = D`), nothing referenced
// them, and they advertised an async capability that does not exist. Re-add a
// real driver-mode distinction only when an async executor actually backs it
// (ROADMAP phase 6).
