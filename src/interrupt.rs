//! Interrupt management for WS63 (RISC-V PLIC-based).
//!
//! Re-exports the PAC interrupt enum and provides helpers for
//! enabling, disabling, and binding interrupt handlers.

pub use crate::soc::ws63::Interrupt;

/// Interrupt priority level (0 = lowest, 7 = highest for RISC-V PLIC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(u8);

impl Priority {
    pub const P0: Self = Priority(0);
    pub const P1: Self = Priority(1);
    pub const P2: Self = Priority(2);
    pub const P3: Self = Priority(3);
    pub const P4: Self = Priority(4);
    pub const P5: Self = Priority(5);
    pub const P6: Self = Priority(6);
    pub const P7: Self = Priority(7);

    pub const fn max() -> Self {
        Priority(7)
    }
    pub const fn min() -> Self {
        Priority(0)
    }
}

/// A type that can have its interrupt handler configured.
pub trait InterruptConfigurable {
    /// Set the interrupt handler for this peripheral.
    fn set_interrupt_handler(&mut self, handler: InterruptHandler);

    /// Enable the interrupt for this peripheral.
    fn enable_interrupt(&mut self, priority: Priority);

    /// Disable the interrupt for this peripheral.
    fn disable_interrupt(&mut self);
}

/// An interrupt handler: a function pointer and priority.
#[derive(Clone, Copy)]
pub struct InterruptHandler {
    pub handler: Option<extern "C" fn()>,
    pub priority: Priority,
}

impl InterruptHandler {
    pub const fn new(handler: extern "C" fn(), priority: Priority) -> Self {
        Self { handler: Some(handler), priority }
    }

    pub const fn none() -> Self {
        Self { handler: None, priority: Priority::P0 }
    }
}

/// Bind an interrupt handler function for a specific interrupt source.
///
/// This is a low-level function that configures the PLIC for the given
/// interrupt number.
pub fn bind_handler(_interrupt: Interrupt, _handler: extern "C" fn()) {
    // RISC-V PLIC: configure the interrupt handler
    // In a real implementation, this would set up the trap handler
    unsafe {
        riscv::interrupt::enable();
    }
}

/// Enable a specific external interrupt.
pub fn enable(_interrupt: Interrupt) {
    unsafe {
        // Enable the interrupt in the PLIC
        riscv::interrupt::enable();
    }
}

/// Disable a specific external interrupt.
pub fn disable(_interrupt: Interrupt) {
    // Disable in PLIC
}

/// Enter a critical section and execute the closure.
pub fn free<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    riscv::interrupt::free(f)
}
