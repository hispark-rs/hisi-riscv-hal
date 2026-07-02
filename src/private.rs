//! Private module for sealed traits.
//!
//! These traits cannot be implemented outside this crate, enabling
//! the sealed trait pattern used throughout the HAL.

/// A trait that is sealed (cannot be implemented outside this crate).
pub trait Sealed {}

// (The vestigial empty `DmaWord`/`PeripheralInput`/`PeripheralOutput` markers were
// removed — the real DMA buffer and GPIO signal traits live in their driver modules.)

// The `DriverMode` / `Blocking` / `Async` marker traits were removed: every
// associated type was the identity (`type Async<D> = D`), nothing referenced
// them, and they advertised an async capability that does not exist. Re-add a
// real driver-mode distinction only when an async executor actually backs it
// (ROADMAP phase 6).
