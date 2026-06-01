//! WS63 interrupt controller — riscv31 *custom local interrupts* (NOT a PLIC).
//!
//! WS63's application core is a HiSilicon "riscv31" hart with **no PLIC**. There
//! are two interrupt tiers, mirroring fbb_ws63
//! `drivers/chips/ws63/arch/riscv/riscv31/interrupt.c`:
//!
//! * **IRQ 26..=31** — standard machine *local* interrupts, enabled by the
//!   matching bit of the standard `mie` CSR (bit index == IRQ number).
//! * **IRQ >= 32** — HiSilicon custom local interrupts, gated by the custom
//!   CSRs `LOCIEN0..2` (`0xBE0..2`, enable, 32 IRQs/reg from base 32),
//!   `LOCIPD0..` (`0xBE8..`, pending) and cleared by writing the IRQ number to
//!   `LOCIPCLR` (`0xBF0`). Each local IRQ (from base **26**) also has a 4-bit
//!   priority in `LOCIPRI0..15` (`0xBC0..`, 8 IRQs/reg); a global threshold
//!   lives in `PRITHD` (`0xBFE`). An IRQ is delivered only when it is enabled
//!   **and** its priority is strictly greater than the threshold (ties broken
//!   by the lowest IRQ number). Reset default priority is 1, threshold 0.
//!
//! This module owns only the interrupt *controller* (mask / priority /
//! threshold / pending / clear). The trap vector table is the runtime's concern
//! (`ws63-rt`); the `timer_irq` (mie path) and `gpio_irq` (LOCIEN + LOCIPCLR
//! path) examples drive this API end-to-end and are exercised on ws63-qemu.

// On non-riscv (host) builds the CSR access compiles to no-op stubs, leaving the
// surrounding `unsafe` blocks empty — silence the resulting lints there.
#![cfg_attr(not(target_arch = "riscv32"), allow(unused_unsafe, unused_variables, unused_mut))]

pub use crate::soc::ws63::Interrupt;

// --- model constants (fbb_ws63 vectors.h / riscv_interrupt.h) ---------------
/// First IRQ with a configurable `LOCIPRI` priority (IRQs 0..26 are system vectors).
const SYS_VECTOR_CNT: u16 = 26;
/// IRQs below this use `mie` bits; IRQs at/above it use the `LOCIEN*` custom CSRs.
const LOCAL_IRQ_VECTOR_CNT: u16 = 32;
/// IRQs per `LOCIEN`/`LOCIPD` register.
const LOCIEN_IRQ_NUM: u16 = 32;
/// IRQs per `LOCIPRI` register (each a [`LOCIPRI_IRQ_BITS`]-bit field).
const LOCIPRI_IRQ_NUM: u16 = 8;
const LOCIPRI_IRQ_BITS: u16 = 4;
const LOCIPRI_FIELD_MASK: u32 = 0xF;
/// Number of `LOCIPRI` registers (0..15) → highest priority-configurable IRQ.
const PRIORITY_IRQ_END: u16 = SYS_VECTOR_CNT + 16 * LOCIPRI_IRQ_NUM;
/// Default priority word written to every `LOCIPRI` register (priority 1 per IRQ).
const LOCIPRI_DEFAULT_VAL: u32 = 0x1111_1111;

// --- raw CSR helpers --------------------------------------------------------
// Custom CSR addresses must be assembler immediates, so each register needs its
// own instruction; the `match` arms below mirror the C SDK's per-register switch.
// Pseudo-ops (`csrs`/`csrc`/`csrw`/`csrr`) with a numeric CSR are accepted by the
// assembler (same forms the examples already use).

// Two macro sets: real CSR asm on riscv32, and no-op/zero stubs elsewhere so the
// crate (and its host unit tests) build on x86. The interrupt-controller functions
// are not exercised by host tests; the stubs only need to compile.
#[cfg(target_arch = "riscv32")]
macro_rules! csr_set {
    ($csr:literal, $v:expr) => {
        core::arch::asm!(concat!("csrs ", $csr, ", {0}"), in(reg) $v, options(nomem, nostack))
    };
}
#[cfg(target_arch = "riscv32")]
macro_rules! csr_clear {
    ($csr:literal, $v:expr) => {
        core::arch::asm!(concat!("csrc ", $csr, ", {0}"), in(reg) $v, options(nomem, nostack))
    };
}
#[cfg(target_arch = "riscv32")]
macro_rules! csr_write {
    ($csr:literal, $v:expr) => {
        core::arch::asm!(concat!("csrw ", $csr, ", {0}"), in(reg) $v, options(nomem, nostack))
    };
}
#[cfg(target_arch = "riscv32")]
macro_rules! csr_read {
    ($csr:literal) => {{
        let v: u32;
        core::arch::asm!(concat!("csrr {0}, ", $csr), out(reg) v, options(nomem, nostack));
        v
    }};
}

#[cfg(not(target_arch = "riscv32"))]
macro_rules! csr_set {
    ($csr:literal, $v:expr) => {{
        let _ = $v;
    }};
}
#[cfg(not(target_arch = "riscv32"))]
macro_rules! csr_clear {
    ($csr:literal, $v:expr) => {{
        let _ = $v;
    }};
}
#[cfg(not(target_arch = "riscv32"))]
macro_rules! csr_write {
    ($csr:literal, $v:expr) => {{
        let _ = $v;
    }};
}
#[cfg(not(target_arch = "riscv32"))]
macro_rules! csr_read {
    ($csr:literal) => {{ 0u32 }};
}

/// Set/clear a bit-mask in `LOCIEN{idx}` (idx 0..=2 cover IRQ 32..=127).
#[inline]
unsafe fn locien_write(idx: u16, mask: u32, set: bool) {
    unsafe {
        match (idx, set) {
            (0, true) => csr_set!("0xbe0", mask),
            (0, false) => csr_clear!("0xbe0", mask),
            (1, true) => csr_set!("0xbe1", mask),
            (1, false) => csr_clear!("0xbe1", mask),
            (2, true) => csr_set!("0xbe2", mask),
            (2, false) => csr_clear!("0xbe2", mask),
            _ => {}
        }
    }
}

#[inline]
fn locien_read(idx: u16) -> u32 {
    unsafe {
        match idx {
            0 => csr_read!("0xbe0"),
            1 => csr_read!("0xbe1"),
            2 => csr_read!("0xbe2"),
            _ => 0,
        }
    }
}

#[inline]
fn locipd_read(idx: u16) -> u32 {
    unsafe {
        match idx {
            0 => csr_read!("0xbe8"),
            1 => csr_read!("0xbe9"),
            2 => csr_read!("0xbea"),
            _ => 0,
        }
    }
}

/// Read-modify-write a 4-bit priority field in `LOCIPRI{idx}` (idx 0..=15).
/// A bare `csrs` (as the C SDK uses) cannot lower a field below its current
/// value, so we clear the nibble first — strictly more correct than the SDK.
#[inline]
fn locipri_set_field(idx: u16, shift: u16, value: u32) {
    let clear = LOCIPRI_FIELD_MASK << shift;
    let set = (value & LOCIPRI_FIELD_MASK) << shift;
    unsafe {
        macro_rules! rmw {
            ($csr:literal) => {{
                csr_clear!($csr, clear);
                csr_set!($csr, set);
            }};
        }
        match idx {
            0 => rmw!("0xbc0"),
            1 => rmw!("0xbc1"),
            2 => rmw!("0xbc2"),
            3 => rmw!("0xbc3"),
            4 => rmw!("0xbc4"),
            5 => rmw!("0xbc5"),
            6 => rmw!("0xbc6"),
            7 => rmw!("0xbc7"),
            8 => rmw!("0xbc8"),
            9 => rmw!("0xbc9"),
            10 => rmw!("0xbca"),
            11 => rmw!("0xbcb"),
            12 => rmw!("0xbcc"),
            13 => rmw!("0xbcd"),
            14 => rmw!("0xbce"),
            15 => rmw!("0xbcf"),
            _ => {}
        }
    }
}

#[inline]
fn locipri_read(idx: u16) -> u32 {
    unsafe {
        match idx {
            0 => csr_read!("0xbc0"),
            1 => csr_read!("0xbc1"),
            2 => csr_read!("0xbc2"),
            3 => csr_read!("0xbc3"),
            4 => csr_read!("0xbc4"),
            5 => csr_read!("0xbc5"),
            6 => csr_read!("0xbc6"),
            7 => csr_read!("0xbc7"),
            8 => csr_read!("0xbc8"),
            9 => csr_read!("0xbc9"),
            10 => csr_read!("0xbca"),
            11 => csr_read!("0xbcb"),
            12 => csr_read!("0xbcc"),
            13 => csr_read!("0xbcd"),
            14 => csr_read!("0xbce"),
            15 => csr_read!("0xbcf"),
            _ => 0,
        }
    }
}

// --- public types -----------------------------------------------------------

/// Local-interrupt priority. Valid range **1 (lowest) ..= 7 (highest)**; the
/// reset default is 1. An interrupt is delivered only when its priority is
/// strictly greater than the global threshold (see [`set_threshold`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(u8);

impl Priority {
    pub const P1: Self = Priority(1);
    pub const P2: Self = Priority(2);
    pub const P3: Self = Priority(3);
    pub const P4: Self = Priority(4);
    pub const P5: Self = Priority(5);
    pub const P6: Self = Priority(6);
    pub const P7: Self = Priority(7);

    /// Lowest deliverable priority (1).
    pub const LOWEST: Self = Priority(1);
    /// Highest priority (7).
    pub const HIGHEST: Self = Priority(7);

    /// Create a priority, clamping `level` into the valid `1..=7` range.
    pub const fn new(level: u8) -> Self {
        let l = if level < 1 {
            1
        } else if level > 7 {
            7
        } else {
            level
        };
        Priority(l)
    }

    /// The numeric priority level (1..=7).
    pub const fn level(self) -> u8 {
        self.0
    }
}

#[inline]
fn irq_num(irq: Interrupt) -> u16 {
    irq as u16
}

// --- enable / disable -------------------------------------------------------

/// Enable an interrupt source at the controller.
///
/// IRQ 26..=31 set the matching `mie` bit; IRQ >= 32 set the matching `LOCIEN`
/// bit. The global interrupt enable ([`enable_global`]) must also be set.
///
/// # Safety
/// Enabling an interrupt can cause its handler to run, which may break
/// assumptions held outside a critical section (reentrancy, data races with
/// data the handler touches). The caller must ensure the source's handler and
/// shared state are ready.
pub unsafe fn enable(irq: Interrupt) {
    let n = irq_num(irq);
    if (SYS_VECTOR_CNT..LOCAL_IRQ_VECTOR_CNT).contains(&n) {
        unsafe { csr_set!("mie", 1u32 << n) };
    } else if n >= LOCAL_IRQ_VECTOR_CNT {
        let bit = (n - LOCAL_IRQ_VECTOR_CNT) % LOCIEN_IRQ_NUM;
        let reg = (n - LOCAL_IRQ_VECTOR_CNT) / LOCIEN_IRQ_NUM;
        unsafe { locien_write(reg, 1u32 << bit, true) };
    }
}

/// Disable an interrupt source at the controller and clear any latched pending
/// state (mirrors the C SDK's `int_disable_irq`).
///
/// # Safety
/// See [`enable`]; toggling interrupt masks outside a critical section can race
/// with a handler that is mid-flight.
pub unsafe fn disable(irq: Interrupt) {
    let n = irq_num(irq);
    if (SYS_VECTOR_CNT..LOCAL_IRQ_VECTOR_CNT).contains(&n) {
        unsafe { csr_clear!("mie", 1u32 << n) };
    } else if n >= LOCAL_IRQ_VECTOR_CNT {
        let bit = (n - LOCAL_IRQ_VECTOR_CNT) % LOCIEN_IRQ_NUM;
        let reg = (n - LOCAL_IRQ_VECTOR_CNT) / LOCIEN_IRQ_NUM;
        unsafe { locien_write(reg, 1u32 << bit, false) };
    }
    clear_pending(irq);
}

/// Whether the given interrupt source is currently enabled at the controller.
pub fn is_enabled(irq: Interrupt) -> bool {
    let n = irq_num(irq);
    if (SYS_VECTOR_CNT..LOCAL_IRQ_VECTOR_CNT).contains(&n) {
        let mie: u32 = unsafe { csr_read!("mie") };
        (mie & (1u32 << n)) != 0
    } else if n >= LOCAL_IRQ_VECTOR_CNT {
        let bit = (n - LOCAL_IRQ_VECTOR_CNT) % LOCIEN_IRQ_NUM;
        let reg = (n - LOCAL_IRQ_VECTOR_CNT) / LOCIEN_IRQ_NUM;
        (locien_read(reg) & (1u32 << bit)) != 0
    } else {
        false
    }
}

// --- priority / threshold ---------------------------------------------------

/// Set a local interrupt's 4-bit priority (`LOCIPRI`). Applies to IRQ >= 26;
/// a no-op for the system vectors below that.
pub fn set_priority(irq: Interrupt, priority: Priority) {
    let n = irq_num(irq);
    if (SYS_VECTOR_CNT..PRIORITY_IRQ_END).contains(&n) {
        let order = (n - SYS_VECTOR_CNT) % LOCIPRI_IRQ_NUM;
        let reg = (n - SYS_VECTOR_CNT) / LOCIPRI_IRQ_NUM;
        locipri_set_field(reg, order * LOCIPRI_IRQ_BITS, priority.0 as u32);
    }
}

/// Read back a local interrupt's configured priority (`LOCIPRI`).
pub fn priority(irq: Interrupt) -> Priority {
    let n = irq_num(irq);
    if (SYS_VECTOR_CNT..PRIORITY_IRQ_END).contains(&n) {
        let order = (n - SYS_VECTOR_CNT) % LOCIPRI_IRQ_NUM;
        let reg = (n - SYS_VECTOR_CNT) / LOCIPRI_IRQ_NUM;
        let shift = order * LOCIPRI_IRQ_BITS;
        let field = (locipri_read(reg) >> shift) & LOCIPRI_FIELD_MASK;
        Priority::new(field as u8)
    } else {
        Priority::LOWEST
    }
}

/// Set the global priority threshold (`PRITHD`, 0..=7). Only interrupts with a
/// priority **strictly greater** than this are delivered; 0 admits all
/// priorities >= 1.
pub fn set_threshold(level: u8) {
    unsafe { csr_write!("0xbfe", (level & 0x7) as u32) };
}

/// Read the current global priority threshold (`PRITHD`).
pub fn threshold() -> u8 {
    (unsafe { csr_read!("0xbfe") } & 0x7) as u8
}

// --- pending ----------------------------------------------------------------

/// Clear a pending local interrupt by writing its number to `LOCIPCLR`. Safe to
/// call from within the interrupt's own handler (the usual place).
pub fn clear_pending(irq: Interrupt) {
    let n = irq_num(irq) as u32;
    unsafe {
        #[cfg(target_arch = "riscv32")]
        core::arch::asm!("fence", options(nostack));
        csr_write!("0xbf0", n);
        #[cfg(target_arch = "riscv32")]
        core::arch::asm!("fence", options(nostack));
    }
}

/// Whether the given interrupt is currently pending (`LOCIPD` for IRQ >= 32,
/// `mip` for IRQ 26..=31).
pub fn is_pending(irq: Interrupt) -> bool {
    let n = irq_num(irq);
    if (SYS_VECTOR_CNT..LOCAL_IRQ_VECTOR_CNT).contains(&n) {
        let mip: u32 = unsafe { csr_read!("mip") };
        (mip & (1u32 << n)) != 0
    } else if n >= LOCAL_IRQ_VECTOR_CNT {
        let bit = (n - LOCAL_IRQ_VECTOR_CNT) % LOCIEN_IRQ_NUM;
        let reg = (n - LOCAL_IRQ_VECTOR_CNT) / LOCIEN_IRQ_NUM;
        (locipd_read(reg) & (1u32 << bit)) != 0
    } else {
        false
    }
}

// --- global enable / critical section / init --------------------------------

/// Set the default priority of every local interrupt to 1 (`LOCIPRI = 0x1111_1111`
/// for all registers), mirroring the C SDK's `int_setup`. Without this, an IRQ
/// whose priority resets to 0 would never clear the default threshold of 0.
pub fn init() {
    unsafe {
        csr_write!("0xbc0", LOCIPRI_DEFAULT_VAL);
        csr_write!("0xbc1", LOCIPRI_DEFAULT_VAL);
        csr_write!("0xbc2", LOCIPRI_DEFAULT_VAL);
        csr_write!("0xbc3", LOCIPRI_DEFAULT_VAL);
        csr_write!("0xbc4", LOCIPRI_DEFAULT_VAL);
        csr_write!("0xbc5", LOCIPRI_DEFAULT_VAL);
        csr_write!("0xbc6", LOCIPRI_DEFAULT_VAL);
        csr_write!("0xbc7", LOCIPRI_DEFAULT_VAL);
        csr_write!("0xbc8", LOCIPRI_DEFAULT_VAL);
        csr_write!("0xbc9", LOCIPRI_DEFAULT_VAL);
        csr_write!("0xbca", LOCIPRI_DEFAULT_VAL);
        csr_write!("0xbcb", LOCIPRI_DEFAULT_VAL);
        csr_write!("0xbcc", LOCIPRI_DEFAULT_VAL);
        csr_write!("0xbcd", LOCIPRI_DEFAULT_VAL);
        csr_write!("0xbce", LOCIPRI_DEFAULT_VAL);
        csr_write!("0xbcf", LOCIPRI_DEFAULT_VAL);
    }
}

/// Set the global machine-interrupt enable (`mstatus.MIE`).
///
/// # Safety
/// After this, any enabled-and-ready source can preempt. Configure sources and
/// their handlers first.
pub unsafe fn enable_global() {
    #[cfg(target_arch = "riscv32")]
    unsafe {
        core::arch::asm!("csrsi mstatus, 0x8", options(nomem, nostack))
    };
}

/// Clear the global machine-interrupt enable (`mstatus.MIE`).
pub fn disable_global() {
    #[cfg(target_arch = "riscv32")]
    unsafe {
        core::arch::asm!("csrci mstatus, 0x8", options(nomem, nostack))
    };
}

/// Run `f` with machine interrupts globally masked, restoring the previous
/// `mstatus.MIE` afterwards (nesting-safe — only re-enables if it was enabled).
pub fn free<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    #[cfg(target_arch = "riscv32")]
    let prev: usize;
    #[cfg(target_arch = "riscv32")]
    unsafe {
        core::arch::asm!("csrrci {0}, mstatus, 0x8", out(reg) prev, options(nomem, nostack))
    };
    let r = f();
    #[cfg(target_arch = "riscv32")]
    if prev & 0x8 != 0 {
        unsafe { core::arch::asm!("csrsi mstatus, 0x8", options(nomem, nostack)) };
    }
    r
}
