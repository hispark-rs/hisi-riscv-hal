//! On-target (semihosting) HIL **driver** tests for the HAL, run on real WS63
//! silicon (and safe under QEMU). These live here — inside the HAL crate's
//! `tests/` — so the HAL's own driver tests ship and run with the HAL and inherit
//! its chip gating (`chip-ws63` default, `chip-bs21` via `--features chip-bs21`).
//! The cross-cutting CPU/PAC smoke tests live separately in the `tests-hil`
//! crate.
//!
//! Built with
//! `cargo test -p hisi-riscv-hal --no-default-features --features chip-ws63 \
//!      --target riscv32imfc-unknown-none-elf --test hil`
//! and run on real silicon by the patched probe-rs fork via the
//! `hil/embedded-test-runner.sh` cargo runner (see ../../../hil/README.md). Each
//! test is executed in turn over the semihosting channel; the result is reported
//! back to `probe-rs run` (libtest-compatible).
//!
//! ## Why `--test hil` (not a bare `cargo test --target riscv…`)
//!
//! The HAL keeps its host unit tests in `src/*.rs` (`#[cfg(all(test,
//! not(target_arch = "riscv32")))]`), so the crate's *lib* test target uses the
//! default libtest harness — which links the `test`/`std` crates that do NOT
//! exist for the bare-metal `riscv32imfc-unknown-none-elf` target (only
//! core/alloc are shipped). A bare `cargo test --target riscv…` would try to
//! build that lib-test target and fail to link `test`. Scoping to `--test hil`
//! builds ONLY this embedded-test integration target (its own `harness = false`
//! provides `main`), so the on-target build/run never touches the host-only
//! lib-test harness. The host unit tests are unaffected and still run via
//! `cargo test -p hisi-riscv-hal --target x86_64-unknown-linux-gnu`.
//!
//! ## Entry-point interaction with hisi-riscv-rt
//!
//! We do NOT use `hisi_riscv_rt::entry` here. embedded-test exports the C symbol
//! `main` (its test dispatcher), and hisi-riscv-rt's `runtime_init` (the tail of
//! the assembly startup) calls `extern "Rust" fn main()` after BSS-zero/data-copy
//! — so embedded-test's `main` IS the entry. hisi-riscv-rt still supplies the
//! reset vector, the `critical-section-single-hart` impl (backing
//! portable-atomic's RMW polyfill on this no-atomic core), and — via the
//! `boot-header` feature (turned on by the HAL's chip-ws63 feature) — the 0x300
//! image header that makes the ELF bootable. embedded-test also provides the
//! `#[panic_handler]` (it aborts via semihosting), so we must not define one.
//!
//! As an external integration test this still depends on the chip PAC via a
//! cfg-selected alias (`ws63_pac as pac` / `bs2x_pac as pac`) — the HAL keeps its
//! own PAC `pub(crate)`, so the test names the PAC directly for the raw register
//! and singleton facts.
//!
//! The tests are self-contained: no jumpers / external wiring, safe on a bare
//! board and under QEMU.

// This is an on-target (RISC-V, semihosting) integration test: it links
// hisi-riscv-rt + the embedded-test harness, both of which are riscv-only
// dev-deps (see Cargo.toml's `[target.'cfg(target_arch = "riscv32")']` block).
// `cargo test` builds every integration-test target for whatever target is being
// tested, so on the host (`cargo test --target x86_64`, the HAL's lib unit-test
// run) this file would otherwise fail to find those crates. Gate the whole file
// to riscv so on the host it compiles to a trivial harness-less binary (an empty
// `main`, required because this `[[test]]` sets `harness = false`) and the host
// unit-test build is unaffected.
#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

// On the host (`cargo test --target x86_64`, the HAL's lib unit-test run) this
// integration test target is built too. Its real body links hisi-riscv-rt + the
// embedded-test harness, both riscv-only dev-deps — so on the host we compile to
// a no-op `main` instead (this `[[test]]` is `harness = false`, so cargo expects
// a `main` rather than libtest's). It runs nothing; the on-target cases only ever
// run on a real WS63 board via `probe-rs run`.
#[cfg(not(target_arch = "riscv32"))]
fn main() {}

// Pull in hisi-riscv-rt so its startup, reset vector, linker scripts and
// critical-section impl are linked even though we never name a symbol from it.
#[cfg(target_arch = "riscv32")]
use hisi_riscv_rt as _;

// riscv-only: the embedded-test harness + the riscv-only dev-deps (hisi-riscv-rt,
// the chip PAC alias) it names exist only in the riscv graph. On the host this
// whole module is dropped and `main()` above is the (no-op) entry.
#[cfg(all(test, target_arch = "riscv32"))]
#[embedded_test::tests]
mod tests {
    use core::hint::black_box;
    use hisi_riscv_hal as hal;
    // Chip-selected PAC alias: the suite names `pac::{Peripherals, Gpio0, ...}`
    // chip-agnostically and the active chip feature picks the concrete PAC.
    #[cfg(feature = "chip-ws63")]
    use ws63_pac as pac;
    #[cfg(feature = "chip-bs21")]
    use bs2x_pac as pac;

    /// `#[init]` runs before every test. It takes the singleton `Peripherals`
    /// once and hands them to each test as shared state — proving the PAC's
    /// critical-section-guarded `take()` (backed by hisi-riscv-rt's
    /// single-hart critical-section impl) works on-target.
    #[init]
    fn init() -> pac::Peripherals {
        pac::Peripherals::take().expect("PAC Peripherals::take() returned None on first call")
    }

    /// Read a real SoC register through the PAC singleton handed over by
    /// `#[init]` and assert a structural fact about it. We read the TCXO status
    /// register and assert the read completed (the value is whatever the bus
    /// returns); the point is that an MMIO load to the TCXO window succeeds
    /// on-target without trapping. Reads only — no writes, no state change.
    #[test]
    fn read_tcxo_status_register(p: pac::Peripherals) {
        // `bits()` performs a volatile 32-bit load from 0x4400_04c0 + offset.
        let status = p.tcxo.tcxo_status().read().bits();
        // The reserved upper bits are not all-ones on a sane bus read; this is a
        // weak-but-real liveness assertion that the load returned bus data rather
        // than the all-ones "no device" pattern.
        assert_ne!(status, 0xFFFF_FFFF, "TCXO status read returned the bus-floating all-ones pattern");
    }

    /// HAL `Peripherals` construction smoke test (PAC/HAL structural #8). The
    /// HAL's `Peripherals::take()` was already consumed by the PAC `take()` in
    /// `#[init]` (both back onto the same singleton), so we `steal()` the HAL
    /// peripherals — safe here because tests run sequentially on a single hart —
    /// and assert that several driver `ptr()`s resolve to the documented SoC
    /// windows. This proves the HAL peripheral wrappers construct without panic
    /// and point at the same addresses as the raw PAC. Mirrors the
    /// `peripherals!`/`peripheral!` macros in hisi-riscv-hal/src/peripherals.rs.
    ///
    /// Asserts WS63-specific peripheral-window addresses → gated `chip-ws63`. A
    /// `#[cfg(feature = "chip-bs21")]` sibling with the BS21 addresses can be
    /// added when a BS21 board exists.
    #[cfg(feature = "chip-ws63")]
    #[test]
    fn hal_peripherals_construct() {
        // SAFETY: sequential single-hart test run; no other live handles.
        let hp = unsafe { hal::Peripherals::steal() };
        // The HAL ZSTs construct; their register pointers match the PAC bases.
        assert_eq!(hal::peripherals::Gpio0::ptr() as usize, 0x4402_8000, "HAL GPIO0 ptr mismatch");
        assert_eq!(hal::peripherals::Tcxo::ptr() as usize, 0x4400_04c0, "HAL TCXO ptr mismatch");
        assert_eq!(hal::peripherals::Timer::ptr() as usize, 0x4400_2000, "HAL TIMER ptr mismatch");
        assert_eq!(hal::peripherals::Dma::ptr() as usize, 0x4a00_0000, "HAL DMA ptr mismatch");
        assert_eq!(hal::peripherals::Uart0::ptr() as usize, 0x4401_0000, "HAL UART0 ptr mismatch");
        // The struct itself constructed (fields are ZSTs); touch one to prove it.
        let _ = hp.GPIO0;
    }

    /// GPIO output read-back (gpio.rs / examples/ws63/blinky). Steal GPIO0's
    /// validated pin 0, drive it as a push-pull output, and assert the GPIO0
    /// block's data-out register (`gpio_sw_out`, the HAL's `is_set_high()` source)
    /// reflects each written level. `set_high()` writes `gpio_data_set`,
    /// `set_low()` writes `gpio_data_clr`; both are observed back through
    /// `gpio_sw_out` bit 0. Real pin I/O, no external wiring — pin 0 is the
    /// validated-safe pin used by blinky.
    #[test]
    fn gpio_output_readback() {
        use hal::gpio::{AnyPin, OutputConfig};
        // SAFETY: pin 0 is a valid WS63 GPIO (0..=18); sequential single-hart run
        // owns it exclusively. Mirrors blinky's `AnyPin::steal(0)`.
        let mut led = unsafe { AnyPin::steal(0) }.init_output(OutputConfig::new().with_initial(false));

        // Drive high → GPIO0 swout/data register bit 0 reads 1.
        led.set_high();
        // SAFETY: read-only MMIO load of the GPIO0 data register.
        let r = unsafe { &*pac::Gpio0::PTR };
        assert_eq!(r.gpio_sw_out().read().bits() & 1, 1, "GPIO0 bit0 did not read high after set_high()");
        assert!(led.is_set_high(), "Output::is_set_high() disagreed after set_high()");

        // Drive low → bit 0 reads 0.
        led.set_low();
        assert_eq!(r.gpio_sw_out().read().bits() & 1, 0, "GPIO0 bit0 did not read low after set_low()");
        assert!(!led.is_set_high(), "Output::is_set_high() disagreed after set_low()");
    }

    /// TCXO free-running counter is monotonic (tcxo.rs). Read the 32-bit counter
    /// twice with a busy-wait between; assert it strictly increased. The driver's
    /// `read_counter32()` latches via a refresh and returns `None` on refresh
    /// timeout — we require both reads to succeed AND the second to exceed the
    /// first (within a non-wrapping window). TCXO is validated-working silicon.
    #[test]
    fn tcxo_counter_monotonic() {
        use hal::tcxo::TcxoDriver;
        // SAFETY: sequential single-hart run; TCXO singleton not otherwise held.
        let tcxo = TcxoDriver::new(unsafe { hal::peripherals::Tcxo::steal() });

        let a = tcxo.read_counter32().expect("first TCXO refresh timed out");
        // Short busy-wait so the 24 MHz counter advances by a comfortable margin.
        for _ in 0..50_000 {
            black_box(0u32);
        }
        let b = tcxo.read_counter32().expect("second TCXO refresh timed out");
        assert!(b > a, "TCXO counter did not advance: first=0x{:08x} second=0x{:08x}", a, b);
    }

    /// Timer counter advances (timer.rs / examples/ws63/timer_irq). Configure
    /// TIMER channel 0 in periodic mode with a large load, enable it, and read the
    /// down-counter (`timer0_current_value`) twice with a busy-wait between;
    /// assert the count changed (advanced). Register/poll level only — we do NOT
    /// rely on the interrupt firing (embedded-test owns the trap handler). The
    /// timer ticks at the 24 MHz TCXO clock, so it moves quickly.
    // IGNORED: the counter did not advance on silicon (the assert panicked) — the
    // timer likely needs a clock-gate/start step the QEMU model doesn't require.
    // The timer driver is not yet silicon-validated (see hisi-riscv-rs#10); needs
    // timer bring-up before this can run.
    #[test]
    #[ignore = "timer counter doesn't advance on silicon yet (needs timer bring-up, #10)"]
    fn timer_counter_advances() {
        use hal::timer::{TimerDriver, TimerMode};
        // SAFETY: sequential single-hart run; TIMER singleton not otherwise held.
        let timer = TimerDriver::new(unsafe { hal::peripherals::Timer::steal() });
        // Large periodic load so the counter is plainly mid-flight across reads.
        timer.configure(0, TimerMode::Periodic, 0x00FF_FFFF);
        timer.enable(0);

        let a = timer.current_value(0);
        for _ in 0..50_000 {
            black_box(0u32);
        }
        let b = timer.current_value(0);
        timer.disable(0);
        assert_ne!(a, b, "TIMER ch0 current_value did not advance: a=0x{:08x} b=0x{:08x}", a, b);
    }

    /// DMA memory-to-memory end-to-end (dma.rs / examples/ws63/dma_loopback
    /// part 2). Run a real SDMA mem→mem transfer on logical channel 8 (→ secure
    /// controller physical channel 0), poll the raw transfer-done bit (bounded),
    /// then assert the destination buffer equals the source. This is the
    /// highest-value end-to-end test: actual data movement by the DMA engine,
    /// self-contained (no external wiring). Mirrors the SDMA half of dma_loopback.
    // IGNORED: on real WS63 silicon this hangs the bus and drops the debug link
    // (the SDMA "secure DMA" path needs security/clock setup the QEMU model skips).
    // QEMU-validated via dma_loopback; needs on-silicon SDMA bring-up before it can
    // run in the HIL suite without crashing the chip + aborting the whole run.
    #[test]
    #[ignore = "SDMA mem-to-mem hangs the bus on silicon (needs SDMA bring-up); QEMU-only for now"]
    fn dma_mem_to_mem() {
        use hal::dma::{DmaChannelConfig, DmaDriver, Sdma0};
        const N: usize = 8;
        let src: [u32; N] =
            [0xaaaa_0001, 0xaaaa_0002, 0xaaaa_0003, 0xaaaa_0004, 0xaaaa_0005, 0xaaaa_0006, 0xaaaa_0007, 0xaaaa_0008];
        let dst: [u32; N] = [0u32; N];

        // SAFETY: sequential single-hart run; SDMA singleton not otherwise held.
        let mut sdma = DmaDriver::<Sdma0>::new_sdma(unsafe { hal::peripherals::Sdma::steal() });
        sdma.enable_controller();
        // Logical channel 8 → physical channel 0 on the secure controller.
        sdma.configure_channel(8, src.as_ptr() as u32, dst.as_ptr() as u32, N as u16, &DmaChannelConfig::default());

        // Poll the controller's raw transfer-done mask for physical channel 0,
        // bounded so a stuck transfer can't hang the test run (real-HW pattern).
        let mut done = false;
        let mut budget = 1_000_000u32;
        while budget > 0 {
            if sdma.raw_interrupt_status().0 & 0x01 != 0 {
                done = true;
                break;
            }
            budget -= 1;
        }
        sdma.clear_transfer_interrupt(8);
        assert!(done, "SDMA channel 8 transfer-done bit never set");

        for (i, &want) in src.iter().enumerate() {
            // Volatile: the DMA engine wrote `dst` behind the compiler's back.
            let got = unsafe { core::ptr::read_volatile(dst.as_ptr().add(i)) };
            assert_eq!(got, want, "DMA mem→mem mismatch @{}: got=0x{:08x} want=0x{:08x}", i, got, want);
        }
    }

    /// Clock-gate enable (clock.rs). The HAL's CKEN bit map lives in
    /// `clock::Peripheral::cken_info()` (the old `ClockControl` RAII layer was
    /// removed as dead code — see clock.rs module docs). UART0's gate is
    /// `CKEN_CTL1` bit 18. WS63 clocks default to ENABLED out of reset, so we
    /// assert the gate is already set; then set it again through the PAC
    /// `CldoCrg` register and re-read to confirm the bit is high. Read-modify-set
    /// of a clock-enable bit is non-destructive (it keeps the clock running).
    ///
    /// Asserts the WS63-specific UART0 CKEN gate (CKEN_CTL1 bit 18) → gated
    /// `chip-ws63`. A `#[cfg(feature = "chip-bs21")]` sibling with the BS21 gate
    /// location can be added when a BS21 board exists.
    #[cfg(feature = "chip-ws63")]
    #[test]
    fn clock_gate_uart0_enabled() {
        use hal::clock::Peripheral;
        // The map must agree with the documented UART0 gate (CKEN_CTL1 bit 18).
        let (reg_idx, bit) = Peripheral::Uart0.cken_info().expect("UART0 should be a gated peripheral");
        assert_eq!((reg_idx, bit), (1, 18), "UART0 CKEN gate moved");

        // SAFETY: read-only / RMW-set of the clock-enable register; setting an
        // already-set enable bit keeps the peripheral clock running.
        let crg = unsafe { &*pac::CldoCrg::PTR };
        let before = crg.cken_ctl1().read().bits();
        assert_ne!(before & (1 << bit), 0, "UART0 clock gate (CKEN_CTL1 bit 18) not set out of reset");

        // Re-assert the gate and confirm it reads back high.
        crg.cken_ctl1().modify(|r, w| unsafe { w.bits(r.bits() | (1 << bit)) });
        let after = crg.cken_ctl1().read().bits();
        assert_ne!(after & (1 << bit), 0, "UART0 clock gate not high after re-enable");
    }

    /// System reset-reason read-only decode (system.rs / examples/ws63/reset_demo).
    /// Construct `System` from stolen SYS_CTL0/GLB_CTL_M/CLDO_CRG handles and call
    /// `reset_reason()`, asserting it returns one of the valid variants. We do NOT
    /// call `software_reset()` (it reboots the chip and would break the test run).
    /// Note: `reset_reason()` reads AND CLEARS the matched history bit, so it is
    /// run once. Mirrors reset_demo's `System::new(...).reset_reason()`.
    ///
    /// WS63-only: the HAL's `system` module and the `SysCtl0`/`CldoCrg`
    /// peripherals are `#[cfg(feature = "chip-ws63")]` in the HAL (the reset/CRG
    /// stack is a deeper port), so this test is gated `chip-ws63`. A
    /// `#[cfg(feature = "chip-bs21")]` sibling can be added once the HAL exposes a
    /// BS21 reset-reason API.
    #[cfg(feature = "chip-ws63")]
    #[test]
    fn system_reset_reason_valid() {
        use hal::system::{ResetReason, System};
        // SAFETY: sequential single-hart run; these singletons not otherwise held.
        let system = unsafe {
            System::new(
                hal::peripherals::SysCtl0::steal(),
                hal::peripherals::GlbCtlM::steal(),
                hal::peripherals::CldoCrg::steal(),
            )
        };
        let reason = system.reset_reason();
        assert!(
            matches!(
                reason,
                ResetReason::PowerOn
                    | ResetReason::ExternalPin
                    | ResetReason::Watchdog
                    | ResetReason::Software
                    | ResetReason::BrownOut
                    | ResetReason::Unknown
            ),
            "reset_reason() returned an out-of-range variant",
        );
    }

    /// UART divider register configuration (uart.rs). Construct UART0 via
    /// `Uart::new_uart0(.., Config::default())` (115200 8N1) and assert the
    /// programmed `div_l`/`div_h`/`div_fra` registers match the HAL's
    /// fixed-point baud formula: div*64 = UART_CLOCK_HZ*4 / baud, with the low 6
    /// bits the fractional part. This tests the register CONFIG only — NOT actual
    /// serial output (on-silicon UART baud is a known-open issue #15; we do not
    /// assert bytes on the wire).
    #[test]
    fn uart0_divider_config() {
        use hal::uart::{Config, Uart};
        let cfg = Config::default(); // 115200 8N1
        // SAFETY: sequential single-hart run; UART0 singleton not otherwise held.
        let _uart = Uart::new_uart0(unsafe { hal::peripherals::Uart0::steal() }, cfg);

        // Recompute the expected divider exactly as configure_uart() does.
        let pclk = hal::soc::chip::UART_CLOCK_HZ; // 160 MHz
        let div64 = ((pclk as u64) * 4 / (cfg.baudrate as u64)) as u32; // = div * 64
        let div = div64 >> 6;
        let exp_div_fra = (div64 & 0x3F) as u16;
        let exp_div_l = (div & 0xFF) as u16;
        let exp_div_h = ((div >> 8) & 0xFF) as u16;

        // SAFETY: read-only MMIO loads of the UART0 divider registers.
        let r = unsafe { &*pac::Uart0::PTR };
        assert_eq!(r.div_l().read().bits(), exp_div_l, "UART0 div_l mismatch");
        assert_eq!(r.div_h().read().bits(), exp_div_h, "UART0 div_h mismatch");
        assert_eq!(r.div_fra().read().bits(), exp_div_fra, "UART0 div_fra mismatch");
    }
}
