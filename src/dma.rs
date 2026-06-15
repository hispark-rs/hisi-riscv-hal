//! DMA (Direct Memory Access) driver for WS63.
//!
//! The WS63 has two DMA controllers:
//! - **DMA** (MDMA) at 0x4A00_0000 — 4 channels, used for general-purpose transfers
//! - **SDMA** (Secure DMA) at 0x520A_0000 — 4 additional channels, same register layout
//!
//! Each channel supports:
//! - Memory-to-memory, memory-to-peripheral, peripheral-to-memory transfers
//! - Linked list (scatter-gather) mode
//! - Configurable source/destination burst sizes and widths
//! - Address increment control
//! - Transfer complete and error interrupts
//!
//! # Channel addressing
//!
//! The WS63 numbers DMA channels in a single **logical** space:
//! - **MDMA** (`Dma0`) owns logical channels **0-3**
//! - **SDMA** (`Sdma0`) owns logical channels **8-11**
//!
//! Both are backed by physical channels 0-3 on their own register block, so
//! `DmaDriver<Sdma0>` accepts channel numbers 8-11 and translates them to the
//! controller-local 0-3 internally (matching the C SDK `hal_dma_ch_get` /
//! `hal_dma_type_get` convention in fbb_ws63 `hal_dmac_v151.c`). Passing a
//! channel outside a controller's logical range panics.

use crate::peripherals::{Dma, Sdma};
use core::marker::PhantomData;

// ── Type-level DMA instance markers ───────────────────────────────

/// DMA instance trait.
pub trait DmaInstance {
    /// Returns the PAC pointer for this DMA controller.
    fn ptr() -> *const crate::soc::pac::dma::RegisterBlock;

    /// Logical channel number of this controller's first physical channel.
    ///
    /// MDMA exposes logical channels `CHANNEL_BASE..CHANNEL_BASE+4` (0-3); SDMA
    /// exposes 8-11. The driver subtracts this base to index the controller's
    /// physical channels 0-3 — see the module-level "Channel addressing" docs.
    const CHANNEL_BASE: u8;

    /// Auto clock-gate **bypass** `(register, mask)` for this controller, if any.
    ///
    /// The WS63 M_DMA clock is auto-gated when the controller is idle; the
    /// primary controller must set this bit so its clock keeps running across a
    /// transfer, otherwise a started transfer never advances (its done bit never
    /// sets). Mirrors the vendor `dma_port_register_irq()` "BYPASS dma自动门控"
    /// (`DMA_CLK_AUTO_CTRL_REG |= DMA_CLK_ON_MASK`). `None` = no ungate
    /// needed/available — the secure SDMA is never provisioned on WS63.
    const CLK_AUTO_CTRL: Option<(usize, u32)> = None;
}

/// Marker type for the primary DMA controller (logical channels 0-3).
pub struct Dma0;
impl DmaInstance for Dma0 {
    fn ptr() -> *const crate::soc::pac::dma::RegisterBlock {
        Dma::ptr()
    }
    const CHANNEL_BASE: u8 = 0;
    // Bypass the M_DMA auto clock-gate (DMA_CLK_AUTO_CTRL_REG bit 19) so the
    // primary controller's clock stays on across a transfer (vendor dma_porting).
    const CLK_AUTO_CTRL: Option<(usize, u32)> = Some((0x4400_0244, 0x0008_0000));
}

/// Marker type for the secure DMA controller (logical channels 8-11).
pub struct Sdma0;
impl DmaInstance for Sdma0 {
    fn ptr() -> *const crate::soc::pac::dma::RegisterBlock {
        Sdma::ptr()
    }
    const CHANNEL_BASE: u8 = 8;
}

/// Translate a logical channel number to this controller's physical channel
/// index (0-3), validating it falls in the controller's logical range.
#[inline]
fn physical_channel_index(base: u8, channel: u8) -> usize {
    assert!(channel >= base && channel < base + 4, "DMA channel out of range for this controller");
    (channel - base) as usize
}

// ── Configuration types ───────────────────────────────────────────

/// Transfer width (data size per beat).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferWidth {
    /// 8 bits (1 byte).
    Width8 = 0,
    /// 16 bits (2 bytes).
    Width16 = 1,
    /// 32 bits (4 bytes).
    Width32 = 2,
}

/// Burst size (number of beats per burst).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BurstSize {
    /// 1 beat.
    Beats1 = 0,
    /// 4 beats.
    Beats4 = 1,
    /// 8 beats.
    Beats8 = 2,
    /// 16 beats.
    Beats16 = 3,
    /// 32 beats.
    Beats32 = 4,
    /// 64 beats.
    Beats64 = 5,
    /// 128 beats.
    Beats128 = 6,
    /// 256 beats.
    Beats256 = 7,
}

/// DMA transfer direction / flow control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowControl {
    /// Memory-to-memory transfer.
    MemToMem = 0,
    /// Memory-to-peripheral transfer.
    MemToPeripheral = 1,
    /// Peripheral-to-memory transfer.
    PeripheralToMem = 2,
    /// Peripheral-to-peripheral transfer.
    PeripheralToPeripheral = 3,
}

/// DMA channel configuration.
#[derive(Debug, Clone, Copy)]
pub struct DmaChannelConfig {
    /// Source peripheral select (0-15).
    pub src_peripheral: u8,
    /// Destination peripheral select (0-15).
    pub dst_peripheral: u8,
    /// Flow control mode.
    pub flow_control: FlowControl,
    /// Source transfer width.
    pub src_width: TransferWidth,
    /// Destination transfer width.
    pub dst_width: TransferWidth,
    /// Source burst size.
    pub src_burst: BurstSize,
    /// Destination burst size.
    pub dst_burst: BurstSize,
    /// Increment source address after each beat.
    pub src_inc: bool,
    /// Increment destination address after each beat.
    pub dst_inc: bool,
    /// Enable transfer complete interrupt.
    pub transfer_int: bool,
    /// Enable error interrupt.
    pub error_int: bool,
    /// Bus lock during transfer.
    pub bus_lock: bool,
}

impl Default for DmaChannelConfig {
    fn default() -> Self {
        Self {
            src_peripheral: 0,
            dst_peripheral: 0,
            flow_control: FlowControl::MemToMem,
            src_width: TransferWidth::Width32,
            dst_width: TransferWidth::Width32,
            src_burst: BurstSize::Beats1,
            dst_burst: BurstSize::Beats1,
            src_inc: true,
            dst_inc: true,
            transfer_int: false,
            error_int: false,
            bus_lock: false,
        }
    }
}

// ── DMA driver ────────────────────────────────────────────────────

/// DMA controller driver.
pub struct DmaDriver<'d, T: DmaInstance> {
    _instance: PhantomData<&'d T>,
}

impl<'d, T: DmaInstance> DmaDriver<'d, T> {
    /// Create a new DMA driver from a DMA peripheral.
    pub fn new(_dma: impl Into<PhantomData<&'d T>>) -> Self {
        Self { _instance: PhantomData }
    }

    fn regs() -> &'static crate::soc::pac::dma::RegisterBlock {
        // SAFETY: PAC peripheral pointer is a static physical MMIO address, always valid
        unsafe { &*T::ptr() }
    }

    /// Translate a logical channel number (0-3 for MDMA, 8-11 for SDMA) to this
    /// controller's physical channel index (0-3). Panics if out of range.
    #[inline]
    fn physical_channel(channel: u8) -> usize {
        physical_channel_index(T::CHANNEL_BASE, channel)
    }

    /// Enable the DMA controller.
    ///
    /// First bypasses the controller's auto clock-gate (if it has one — the WS63
    /// M_DMA does), so the DMA clock stays running across a transfer. Without
    /// this the idle-gated clock leaves a started transfer stuck (done bit never
    /// sets); this was why `dma_mem_to_mem` failed on silicon while passing under
    /// QEMU, which models no clock gating. Mirrors the vendor `dma_porting`.
    pub fn enable_controller(&mut self) {
        if let Some((reg, mask)) = T::CLK_AUTO_CTRL {
            // SAFETY: fixed glue MMIO register; read-modify-write sets only the
            // clock-on bit, preserving the rest. Not exposed by the PAC.
            unsafe {
                let p = reg as *mut u32;
                p.write_volatile(p.read_volatile() | mask);
            }
        }
        let r = Self::regs();
        unsafe {
            r.dmac_config().write(|w| w.bits(0x01));
        }
    }

    /// Disable the DMA controller.
    pub fn disable_controller(&mut self) {
        unsafe {
            Self::regs().dmac_config().write(|w| w.bits(0));
        }
    }

    /// Configure a DMA channel.
    ///
    /// * `channel` — Logical channel number (0-3 for MDMA, 8-11 for SDMA).
    /// * `src_addr` — Source address.
    /// * `dst_addr` — Destination address.
    /// * `transfer_size` — Number of source-width beats to transfer.
    /// * `config` — Channel configuration.
    pub fn configure_channel(
        &mut self,
        channel: u8,
        src_addr: u32,
        dst_addr: u32,
        transfer_size: u16,
        config: &DmaChannelConfig,
    ) {
        let ch = Self::physical_channel(channel);
        let r = Self::regs();

        // Disable channel before configuration
        unsafe {
            r.dmac_chn_config_0(ch).write(|w| w.bits(0));
        }

        // Set source address
        unsafe {
            r.dmac_s_addr_0(ch).write(|w| w.bits(src_addr));
        }

        // Set destination address
        unsafe {
            r.dmac_d_addr_0(ch).write(|w| w.bits(dst_addr));
        }

        // Clear linked list pointer
        unsafe {
            r.dmac_lli_0(ch).write(|w| w.bits(0));
        }

        // Build control register
        let mut control: u32 = 0;
        control |= (transfer_size as u32) & 0xFFF; // trans_size [0:11]
        control |= ((config.src_burst as u32) & 0x07) << 12; // s_bsize [12:14]
        control |= ((config.dst_burst as u32) & 0x07) << 15; // d_bsize [15:17]
        control |= ((config.src_width as u32) & 0x07) << 18; // s_width [18:20]
        control |= ((config.dst_width as u32) & 0x07) << 21; // d_width [21:23]
        control |= 0 << 24; // s_master = M1
        control |= 0 << 25; // d_master = M1
        if config.src_inc {
            control |= 1 << 26;
        }
        if config.dst_inc {
            control |= 1 << 27;
        }
        control |= 0 << 28; // prot = 0
        if config.transfer_int {
            control |= 1 << 31;
        }

        unsafe {
            r.dmac_chn_control_0(ch).write(|w| w.bits(control));
        }

        // Build channel config register
        let mut ch_cfg: u32 = 0;
        ch_cfg |= 0x01; // chn_en
        ch_cfg |= ((config.src_peripheral as u32) & 0x0F) << 1; // s_peripheral [1:4]
        ch_cfg |= ((config.dst_peripheral as u32) & 0x0F) << 5; // d_peripheral [5:8]
        ch_cfg |= ((config.flow_control as u32) & 0x07) << 9; // flow_ctl [9:11]
        if config.error_int {
            ch_cfg |= 1 << 12; // int_en
        }
        if config.transfer_int {
            ch_cfg |= 1 << 13; // int_tc
        }
        if config.bus_lock {
            ch_cfg |= 1 << 14; // lock
        }

        unsafe {
            r.dmac_chn_config_0(ch).write(|w| w.bits(ch_cfg));
        }

        // Actually START the transfer: set this channel's bit in the global
        // channel-enable register (`dmac_en_chns`, the DesignWare ChEnReg). On
        // real silicon the per-channel CFG `ch_enable` bit alone does NOT kick the
        // engine — the vendor `hal_dma_v151_enable` writes `en_chns`, and the
        // hardware auto-clears the bit when the (single-block) transfer completes.
        // (QEMU runs its synchronous memcpy on the CFG write and may not model
        // `en_chns`, so this is a harmless extra write there.) Completion is then
        // observed via `channel_enabled()` going false — see the HIL test.
        let en = r.dmac_en_chns().read().bits();
        unsafe {
            r.dmac_en_chns().write(|w| w.bits(en | (1 << ch)));
        }
    }

    /// Enable a specific DMA channel.
    pub fn enable_channel(&mut self, channel: u8) {
        let ch = Self::physical_channel(channel);
        let r = Self::regs();
        let cfg = r.dmac_chn_config_0(ch).read().bits();
        unsafe {
            r.dmac_chn_config_0(ch).write(|w| w.bits(cfg | 0x01));
        }
    }

    /// Disable a specific DMA channel.
    pub fn disable_channel(&mut self, channel: u8) {
        let ch = Self::physical_channel(channel);
        let r = Self::regs();
        let cfg = r.dmac_chn_config_0(ch).read().bits();
        unsafe {
            r.dmac_chn_config_0(ch).write(|w| w.bits(cfg & !0x01));
        }
    }

    /// Check if a DMA channel is enabled.
    pub fn channel_enabled(&self, channel: u8) -> bool {
        let ch = Self::physical_channel(channel);
        let mask = 1u32 << ch;
        Self::regs().dmac_en_chns().read().bits() & mask != 0
    }

    /// Check if a channel has data in its FIFO (active transfer).
    pub fn channel_active(&self, channel: u8) -> bool {
        let ch = Self::physical_channel(channel);
        Self::regs().dmac_chn_config_0(ch).read().bits() & (1 << 15) != 0
    }

    /// Halt a DMA channel (ignore further DMA requests).
    pub fn halt_channel(&mut self, channel: u8) {
        let ch = Self::physical_channel(channel);
        let r = Self::regs();
        let cfg = r.dmac_chn_config_0(ch).read().bits();
        unsafe {
            r.dmac_chn_config_0(ch).write(|w| w.bits(cfg | (1 << 16)));
        }
    }

    /// Resume a halted DMA channel.
    pub fn resume_channel(&mut self, channel: u8) {
        let ch = Self::physical_channel(channel);
        let r = Self::regs();
        let cfg = r.dmac_chn_config_0(ch).read().bits();
        unsafe {
            r.dmac_chn_config_0(ch).write(|w| w.bits(cfg & !(1 << 16)));
        }
    }

    /// Issue a software burst request for a channel.
    pub fn burst_request(&mut self, channel: u8) {
        let ch = Self::physical_channel(channel);
        unsafe {
            Self::regs().dmac_burst_req().write(|w| w.bits(1 << ch));
        }
    }

    /// Issue a software single request for a channel.
    pub fn single_request(&mut self, channel: u8) {
        let ch = Self::physical_channel(channel);
        unsafe {
            Self::regs().dmac_single_req().write(|w| w.bits(1 << ch));
        }
    }

    /// Get the raw interrupt status.
    ///
    /// Returns `(transfer_done_mask, error_mask)`. Bit `n` of each mask is the
    /// **physical** channel `n` of this controller — i.e. logical channel
    /// `CHANNEL_BASE + n`. For SDMA, logical channel 8 is bit 0, channel 9 is
    /// bit 1, etc. (the controller's per-channel status registers are local).
    pub fn raw_interrupt_status(&self) -> (u8, u8) {
        let sts = Self::regs().dmac_ori_int_st().read().bits();
        ((sts & 0xFF) as u8, ((sts >> 8) & 0xFF) as u8)
    }

    /// Get the masked interrupt status.
    ///
    /// Returns `(transfer_done_mask, error_mask)`, with the same **physical**
    /// channel bit indexing as [`raw_interrupt_status`](Self::raw_interrupt_status)
    /// (bit `n` = physical channel `n` = logical `CHANNEL_BASE + n`).
    pub fn interrupt_status(&self) -> (u8, u8) {
        let sts = Self::regs().dmac_int_st().read().bits();
        ((sts & 0xFF) as u8, ((sts >> 16) & 0xFF) as u8)
    }

    /// Clear transfer complete interrupt for a channel.
    pub fn clear_transfer_interrupt(&mut self, channel: u8) {
        let ch = Self::physical_channel(channel);
        unsafe {
            Self::regs().dmac_int_clr().write(|w| w.bits(1 << ch));
        }
    }

    /// Clear error interrupt for a channel.
    pub fn clear_error_interrupt(&mut self, channel: u8) {
        let ch = Self::physical_channel(channel);
        unsafe {
            Self::regs().dmac_int_clr().write(|w| w.bits(1 << (ch + 8)));
        }
    }

    /// Set DMA sync configuration.
    ///
    /// Each bit controls sync for the corresponding channel
    /// (0 = enable sync logic, 1 = disable sync logic).
    pub fn set_sync(&mut self, sync_mask: u16) {
        unsafe {
            Self::regs().dmac_sync().write(|w| w.bits(sync_mask as u32));
        }
    }
}

// ── Convenience constructors ──────────────────────────────────────

impl<'d> DmaDriver<'d, Dma0> {
    /// Create a new primary DMA driver.
    pub fn new_dma(_dma: Dma<'d>) -> Self {
        Self { _instance: PhantomData }
    }
}

// ── DMA peripheral handshaking request IDs ────────────────────────

/// DMA peripheral hardware-handshaking request ID.
///
/// Values are the `HAL_DMA_HANDSHAKING_*` indices from fbb_ws63
/// `drivers/chips/ws63/porting/dma/dma_porting.h` — the hardware request line a
/// channel uses for peripheral-paced flow control. They go into the channel
/// config's `src_peripheral` / `dst_peripheral` field (a 4-bit field; all the
/// IDs below fit). UART bus mapping per `platform_core.h`: UART0 = UART_L,
/// UART1 = UART_H0, UART2 = UART_H1.
///
/// (These superseded the earlier fabricated sequential 0..11 values; only the
/// main-DMA (MDMA) sources hisi-riscv-hal models are listed — the SDMA-group I2C IDs
/// (≥29) don't fit the 4-bit field and aren't modelled here.)
// Peripheral-paced DMA request IDs are WS63-specific (the fbb_ws63 vs fbb_bs2x
// dma_porting.h handshake enums are entirely different). chip-ws63 only; BS2X DMA
// is enabled for memory-to-memory transfers, whose flow control needs no request
// ID. A BS2X request-ID table can be added later from fbb_bs2x dma_porting.h.
#[cfg(feature = "chip-ws63")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DmaPeripheral {
    /// No handshaking (tie-off) — used for memory-to-memory transfers.
    Tie0 = 0,
    /// UART0 (UART_L) transmit.
    Uart0Tx = 1,
    /// UART0 (UART_L) receive.
    Uart0Rx = 2,
    /// UART1 (UART_H0) transmit.
    Uart1Tx = 3,
    /// UART1 (UART_H0) receive.
    Uart1Rx = 4,
    /// UART2 (UART_H1) transmit.
    Uart2Tx = 5,
    /// UART2 (UART_H1) receive.
    Uart2Rx = 6,
    /// SPI0 (SPI_MS0) transmit.
    Spi0Tx = 7,
    /// SPI0 (SPI_MS0) receive.
    Spi0Rx = 8,
    /// I2S transmit.
    I2sTx = 11,
    /// I2S receive.
    I2sRx = 12,
    /// SPI1 (SPI_MS1) transmit.
    Spi1Tx = 13,
    /// SPI1 (SPI_MS1) receive.
    Spi1Rx = 14,
}

#[cfg(feature = "chip-ws63")]
impl DmaPeripheral {
    /// The hardware handshaking request ID (the `dma_porting.h` index), as
    /// programmed into the channel config's peripheral-select field.
    pub const fn request_id(self) -> u8 {
        self as u8
    }
}

/// DMA transfer direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaDirection {
    /// Transmit (memory to peripheral).
    Tx,
    /// Receive (peripheral to memory).
    Rx,
}

impl DmaChannelConfig {
    /// Configure this channel for a **memory → peripheral** transfer to `peri`:
    /// sets `MemToPeripheral` flow control, the destination handshaking ID, and
    /// holds the destination address fixed (a peripheral data register).
    #[cfg(feature = "chip-ws63")]
    pub fn mem_to_peripheral(mut self, peri: DmaPeripheral) -> Self {
        self.flow_control = FlowControl::MemToPeripheral;
        self.dst_peripheral = peri.request_id();
        self.dst_inc = false;
        self
    }

    /// Configure this channel for a **peripheral → memory** transfer from `peri`:
    /// sets `PeripheralToMem` flow control, the source handshaking ID, and holds
    /// the source address fixed (a peripheral data register).
    #[cfg(feature = "chip-ws63")]
    pub fn peripheral_to_mem(mut self, peri: DmaPeripheral) -> Self {
        self.flow_control = FlowControl::PeripheralToMem;
        self.src_peripheral = peri.request_id();
        self.src_inc = false;
        self
    }
}

// The `DmaEligible` / `DmaChannelFor` binding traits were removed (dead — impl'd
// for Spi0/Spi1 but never called; no DmaChannelFor impls). Peripheral-paced DMA
// is now wired through [`DmaPeripheral`] + [`DmaChannelConfig::mem_to_peripheral`]
// / [`peripheral_to_mem`](DmaChannelConfig::peripheral_to_mem), which feed the
// correct handshaking ID + flow control into `configure_channel`.

impl<'d> DmaDriver<'d, Sdma0> {
    /// Create a new secure DMA driver.
    pub fn new_sdma(_sdma: Sdma<'d>) -> Self {
        Self { _instance: PhantomData }
    }
}

// ── Owned-buffer transfer guard (embedded-dma) ──────────────────────────────

use core::mem::ManuallyDrop;
use embedded_dma::{ReadBuffer, WriteBuffer};

/// The DMA cache-maintenance granularity (the 32-byte D-cache line). Source and
/// destination buffers should be [`DMA_ALIGN`]-aligned so a partial-line invalidate
/// on `wait()` cannot clobber a neighbouring allocation's dirty data (see
/// [`crate::cache::invalidate_range`]).
pub const DMA_ALIGN: usize = 32;

/// Bounded completion poll for [`Transfer::wait`] — a wedged channel returns control
/// (the transfer is then treated as done) instead of spinning the CPU forever.
const DMA_WAIT_LOOPS: u32 = 5_000_000;

impl<'d, T: DmaInstance> DmaDriver<'d, T> {
    /// Launch a **memory-to-memory** transfer on `channel` that OWNS both buffers
    /// for its whole lifetime, returning a [`Transfer`] guard you [`wait`] to reclaim
    /// the driver + buffers.
    ///
    /// Because the guard owns the buffers (and aborts the channel on `Drop`), a
    /// use-after-free of an in-flight DMA region is **unrepresentable in safe code**:
    /// the buffers cannot be touched, moved, or freed while the DMA reads/writes them.
    /// The source cache lines are cleaned before launch and the destination
    /// invalidated on `wait()` (the WS63 core is non-coherent), so the cache
    /// maintenance lives inside the type rather than being a caller obligation.
    ///
    /// The DMA beat is a 32-bit word (`Word = u32`); buffers should be
    /// [`DMA_ALIGN`]-aligned. The transfer length is `min(src.len(), dst.len())` words.
    ///
    /// [`wait`]: Transfer::wait
    pub fn start_mem_to_mem<SRC, DST>(mut self, channel: u8, src: SRC, mut dst: DST) -> Transfer<'d, T, SRC, DST>
    where
        SRC: ReadBuffer<Word = u32>,
        DST: WriteBuffer<Word = u32>,
    {
        // SAFETY: `src`/`dst` are moved into the returned Transfer and held until
        // wait() reclaims them, so these pointers stay valid for the whole transfer.
        let (src_ptr, src_len) = unsafe { src.read_buffer() };
        let (dst_ptr, dst_len) = unsafe { dst.write_buffer() };
        let words = src_len.min(dst_len);
        let bytes = words * core::mem::size_of::<u32>();

        // Clean the source so the DMA master reads CPU-written data (non-coherent core).
        #[cfg(feature = "chip-ws63")]
        unsafe {
            crate::cache::clean_range(src_ptr as usize, bytes);
        }

        // mem-to-mem, 32-bit, both addresses incrementing (the Default config).
        let cfg = DmaChannelConfig::default();
        self.configure_channel(channel, src_ptr as u32, dst_ptr as u32, words as u16, &cfg);

        Transfer { driver: self, channel, src, dst, dst_addr: dst_ptr as usize, bytes }
    }
}

/// An in-flight DMA transfer that **owns the driver and both buffers**. Reclaim them
/// with [`wait`](Self::wait); dropping the guard early aborts the channel (so the DMA
/// stops touching the buffers before they are freed). Owning the buffers is what makes
/// a use-after-free of an active DMA region unrepresentable in safe code.
pub struct Transfer<'d, T: DmaInstance, SRC, DST> {
    driver: DmaDriver<'d, T>,
    channel: u8,
    src: SRC,
    dst: DST,
    // Only read by the chip-ws63 cache maintenance in `wait()`; BS2X DMA is coherent
    // here, so these are unused there.
    #[cfg_attr(not(feature = "chip-ws63"), allow(dead_code))]
    dst_addr: usize,
    #[cfg_attr(not(feature = "chip-ws63"), allow(dead_code))]
    bytes: usize,
}

impl<'d, T: DmaInstance, SRC, DST> Transfer<'d, T, SRC, DST> {
    /// True once the channel has auto-cleared its enable bit (single-block done).
    pub fn is_done(&self) -> bool {
        !self.driver.channel_enabled(self.channel)
    }

    /// Block (bounded) until the transfer completes, invalidate the destination cache
    /// lines, and return the driver + both buffers. Consumes the guard, so no abort
    /// `Drop` runs.
    pub fn wait(self) -> (DmaDriver<'d, T>, SRC, DST) {
        // Skip the abort-on-drop: the transfer is completing normally.
        let this = ManuallyDrop::new(self);
        let mut n = DMA_WAIT_LOOPS;
        while this.driver.channel_enabled(this.channel) {
            n -= 1;
            if n == 0 {
                break;
            }
            core::hint::spin_loop();
        }
        // Drop the stale destination cache lines so the CPU re-reads what DMA wrote.
        #[cfg(feature = "chip-ws63")]
        unsafe {
            crate::cache::invalidate_range(this.dst_addr, this.bytes);
        }
        // SAFETY: `this` is ManuallyDrop (its Drop never runs) and each field is read
        // exactly once, so there is no double-read or double-drop.
        unsafe {
            let driver = core::ptr::read(&this.driver);
            let src = core::ptr::read(&this.src);
            let dst = core::ptr::read(&this.dst);
            (driver, src, dst)
        }
    }
}

impl<T: DmaInstance, SRC, DST> Drop for Transfer<'_, T, SRC, DST> {
    /// Abort the channel so the engine stops reading/writing the owned buffers before
    /// they are dropped — the use-after-free guard for an early-dropped transfer.
    fn drop(&mut self) {
        self.driver.halt_channel(self.channel);
        self.driver.disable_channel(self.channel);
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(all(test, not(target_arch = "riscv32")))]
mod tests {
    use super::*;

    #[test]
    fn test_dma_direction_tx_rx_distinct() {
        // TX and RX directions must map to different peripheral IDs
        assert_ne!(DmaDirection::Tx as u8, DmaDirection::Rx as u8);
    }

    // The request IDs below are the HAL_DMA_HANDSHAKING_* indices from fbb_ws63
    // dma_porting.h (the single source of truth) — NOT a fabricated 0..11 run.

    #[test]
    fn test_dma_peripheral_spi_handshaking_ids() {
        // SPI_MS0 = 7/8, SPI_MS1 = 13/14.
        assert_eq!(DmaPeripheral::Spi0Tx.request_id(), 7);
        assert_eq!(DmaPeripheral::Spi0Rx.request_id(), 8);
        assert_eq!(DmaPeripheral::Spi1Tx.request_id(), 13);
        assert_eq!(DmaPeripheral::Spi1Rx.request_id(), 14);
    }

    #[test]
    fn test_dma_peripheral_uart_handshaking_ids() {
        // UART0=UART_L (1/2), UART1=UART_H0 (3/4), UART2=UART_H1 (5/6).
        assert_eq!(DmaPeripheral::Uart0Tx.request_id(), 1);
        assert_eq!(DmaPeripheral::Uart0Rx.request_id(), 2);
        assert_eq!(DmaPeripheral::Uart1Tx.request_id(), 3);
        assert_eq!(DmaPeripheral::Uart1Rx.request_id(), 4);
        assert_eq!(DmaPeripheral::Uart2Tx.request_id(), 5);
        assert_eq!(DmaPeripheral::Uart2Rx.request_id(), 6);
    }

    #[test]
    fn test_dma_peripheral_i2s_handshaking_ids() {
        assert_eq!(DmaPeripheral::I2sTx.request_id(), 11);
        assert_eq!(DmaPeripheral::I2sRx.request_id(), 12);
    }

    #[test]
    fn test_dma_peripheral_ids_fit_4bit_field() {
        // src/dst_peripheral is a 4-bit channel-config field.
        for p in [
            DmaPeripheral::Uart0Tx,
            DmaPeripheral::Uart2Rx,
            DmaPeripheral::Spi0Tx,
            DmaPeripheral::Spi1Rx,
            DmaPeripheral::I2sTx,
            DmaPeripheral::I2sRx,
        ] {
            assert!(p.request_id() <= 0x0F, "{:?} id {} > 4 bits", p, p.request_id());
        }
    }

    #[test]
    fn test_dma_channel_config_peripheral_wiring() {
        // mem_to_peripheral / peripheral_to_mem set flow control + the handshaking
        // ID on the correct side, and hold the peripheral-register address fixed.
        let tx = DmaChannelConfig::default().mem_to_peripheral(DmaPeripheral::Spi0Tx);
        assert_eq!(tx.flow_control, FlowControl::MemToPeripheral);
        assert_eq!(tx.dst_peripheral, 7);
        assert!(!tx.dst_inc);

        let rx = DmaChannelConfig::default().peripheral_to_mem(DmaPeripheral::Uart1Rx);
        assert_eq!(rx.flow_control, FlowControl::PeripheralToMem);
        assert_eq!(rx.src_peripheral, 4);
        assert!(!rx.src_inc);
    }

    #[test]
    fn test_channel_base_consts() {
        // MDMA owns logical channels 0-3; SDMA owns 8-11.
        assert_eq!(Dma0::CHANNEL_BASE, 0);
        assert_eq!(Sdma0::CHANNEL_BASE, 8);
    }

    #[test]
    fn test_mdma_logical_to_physical() {
        // MDMA: logical channel n == physical channel n.
        for ch in 0u8..4 {
            assert_eq!(physical_channel_index(Dma0::CHANNEL_BASE, ch), ch as usize);
        }
    }

    #[test]
    fn test_sdma_logical_to_physical() {
        // SDMA: logical channels 8-11 map to physical 0-3 on the secure block
        // (matches fbb_ws63 hal_dma_ch_get: ch % 4 with the SDMA base).
        assert_eq!(physical_channel_index(Sdma0::CHANNEL_BASE, 8), 0);
        assert_eq!(physical_channel_index(Sdma0::CHANNEL_BASE, 9), 1);
        assert_eq!(physical_channel_index(Sdma0::CHANNEL_BASE, 10), 2);
        assert_eq!(physical_channel_index(Sdma0::CHANNEL_BASE, 11), 3);
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn test_mdma_channel_4_panics() {
        // 4 is out of MDMA's logical range (0-3).
        physical_channel_index(Dma0::CHANNEL_BASE, 4);
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn test_sdma_channel_7_panics() {
        // 7 is below SDMA's logical range (8-11) — passing an MDMA-style index
        // to the secure controller must not silently alias physical channel 0.
        physical_channel_index(Sdma0::CHANNEL_BASE, 7);
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn test_sdma_channel_12_panics() {
        // 12 is above SDMA's logical range (8-11).
        physical_channel_index(Sdma0::CHANNEL_BASE, 12);
    }
}

// ── Property-based fuzz tests ──────────────────────────────────

#[cfg(all(test, not(target_arch = "riscv32")))]
mod proptests {
    use super::{BurstSize, FlowControl, TransferWidth, physical_channel_index};
    use proptest::prelude::*;

    // Re-derive `configure_channel`'s control-word encoder (same masks/shifts as
    // the driver — pure arithmetic, no MMIO). Mirrors dma.rs lines 244-261.
    fn control_word(
        transfer_size: u16,
        src_burst: BurstSize,
        dst_burst: BurstSize,
        src_width: TransferWidth,
        dst_width: TransferWidth,
        src_inc: bool,
        dst_inc: bool,
        transfer_int: bool,
    ) -> u32 {
        let mut control: u32 = 0;
        control |= (transfer_size as u32) & 0xFFF; // trans_size [0:11]
        control |= ((src_burst as u32) & 0x07) << 12; // s_bsize [12:14]
        control |= ((dst_burst as u32) & 0x07) << 15; // d_bsize [15:17]
        control |= ((src_width as u32) & 0x07) << 18; // s_width [18:20]
        control |= ((dst_width as u32) & 0x07) << 21; // d_width [21:23]
        if src_inc {
            control |= 1 << 26;
        }
        if dst_inc {
            control |= 1 << 27;
        }
        if transfer_int {
            control |= 1 << 31;
        }
        control
    }

    // Re-derive `configure_channel`'s channel-config-word encoder. Mirrors
    // dma.rs lines 268-281.
    fn ch_cfg_word(
        src_peripheral: u8,
        dst_peripheral: u8,
        flow_control: FlowControl,
        error_int: bool,
        transfer_int: bool,
        bus_lock: bool,
    ) -> u32 {
        let mut ch_cfg: u32 = 0;
        ch_cfg |= 0x01; // chn_en
        ch_cfg |= ((src_peripheral as u32) & 0x0F) << 1; // s_peripheral [1:4]
        ch_cfg |= ((dst_peripheral as u32) & 0x0F) << 5; // d_peripheral [5:8]
        ch_cfg |= ((flow_control as u32) & 0x07) << 9; // flow_ctl [9:11]
        if error_int {
            ch_cfg |= 1 << 12; // int_en
        }
        if transfer_int {
            ch_cfg |= 1 << 13; // int_tc
        }
        if bus_lock {
            ch_cfg |= 1 << 14; // lock
        }
        ch_cfg
    }

    fn burst(v: u8) -> BurstSize {
        match v & 0x07 {
            0 => BurstSize::Beats1,
            1 => BurstSize::Beats4,
            2 => BurstSize::Beats8,
            3 => BurstSize::Beats16,
            4 => BurstSize::Beats32,
            5 => BurstSize::Beats64,
            6 => BurstSize::Beats128,
            _ => BurstSize::Beats256,
        }
    }

    fn width(v: u8) -> TransferWidth {
        match v % 3 {
            0 => TransferWidth::Width8,
            1 => TransferWidth::Width16,
            _ => TransferWidth::Width32,
        }
    }

    fn flow(v: u8) -> FlowControl {
        match v & 0x03 {
            0 => FlowControl::MemToMem,
            1 => FlowControl::MemToPeripheral,
            2 => FlowControl::PeripheralToMem,
            _ => FlowControl::PeripheralToPeripheral,
        }
    }

    proptest! {
        // ── physical_channel_index (logical→physical translation + range gate) ──

        /// Fuzz: any in-range logical channel maps to a physical index 0..=3.
        #[test]
        fn physical_index_in_range(base in 0u8..=200, off in 0u8..4) {
            let idx = physical_channel_index(base, base + off);
            prop_assert_eq!(idx, off as usize);
            prop_assert!(idx < 4);
        }

        /// Fuzz: translation is monotonic across the controller's 4 logical channels.
        #[test]
        fn physical_index_monotonic(base in 0u8..=200) {
            let mut prev = None;
            for off in 0u8..4 {
                let idx = physical_channel_index(base, base + off);
                if let Some(p) = prev {
                    prop_assert!(idx > p);
                }
                prev = Some(idx);
            }
        }

        /// Fuzz: a channel below the controller's base must panic, never silently
        /// alias physical channel 0 (the guard's lower bound).
        #[test]
        fn physical_index_below_base_panics(base in 1u8..=200, under in 1u8..=100) {
            let under = under.min(base);
            let ch = base - under;
            let r = std::panic::catch_unwind(|| physical_channel_index(base, ch));
            prop_assert!(r.is_err());
        }

        /// Fuzz: a channel at or above base+4 must panic (the guard's upper bound).
        #[test]
        fn physical_index_above_range_panics(base in 0u8..=200, over in 0u8..=50) {
            let ch = (base as u16 + 4 + over as u16).min(255) as u8;
            // Guard against wrap making ch land back in [base, base+4).
            prop_assume!(ch >= base.saturating_add(4) || ch < base);
            let r = std::panic::catch_unwind(|| physical_channel_index(base, ch));
            prop_assert!(r.is_err());
        }

        // ── control-word bit encoder ──────────────────────────────────────────

        /// Fuzz: the control word never sets a bit outside its documented fields.
        /// Reserved positions [24:25] (masters), [28:30] (prot/reserved) must be 0.
        #[test]
        fn control_word_no_stray_bits(
            ts in any::<u16>(),
            sb in any::<u8>(), db in any::<u8>(),
            sw in any::<u8>(), dw in any::<u8>(),
            si in any::<bool>(), di in any::<bool>(), ti in any::<bool>(),
        ) {
            let c = control_word(ts, burst(sb), burst(db), width(sw), width(dw), si, di, ti);
            // Defined bits: [0:11] trans, [12:14] sb, [15:17] db, [18:20] sw,
            // [21:23] dw, 26 s_inc, 27 d_inc, 31 t_int. Everything else is reserved 0.
            const DEFINED: u32 = 0x00FF_FFFF | (1 << 26) | (1 << 27) | (1 << 31);
            prop_assert_eq!(c & !DEFINED, 0, "stray bits in control word {:#010x}", c);
        }

        /// Fuzz: trans_size is a 12-bit field — the encoder truncates a u16 > 0xFFF
        /// (mask, not overflow). Verify the masked low 12 bits round-trip exactly.
        #[test]
        fn control_word_trans_size_field(ts in any::<u16>()) {
            let c = control_word(ts, BurstSize::Beats1, BurstSize::Beats1,
                TransferWidth::Width8, TransferWidth::Width8, false, false, false);
            prop_assert_eq!(c & 0xFFF, (ts as u32) & 0xFFF);
            prop_assert!((c & 0xFFF) <= 0xFFF);
        }

        /// Fuzz: every enum field round-trips out of the control word at its shift.
        #[test]
        fn control_word_fields_round_trip(
            sb in 0u8..8, db in 0u8..8, sw in 0u8..3, dw in 0u8..3,
        ) {
            let (sbe, dbe, swe, dwe) = (burst(sb), burst(db), width(sw), width(dw));
            let c = control_word(0, sbe, dbe, swe, dwe, false, false, false);
            prop_assert_eq!((c >> 12) & 0x07, sbe as u32);
            prop_assert_eq!((c >> 15) & 0x07, dbe as u32);
            prop_assert_eq!((c >> 18) & 0x07, swe as u32);
            prop_assert_eq!((c >> 21) & 0x07, dwe as u32);
        }

        /// Fuzz: each boolean control flag lands in exactly its documented bit.
        #[test]
        fn control_word_flag_bits(si in any::<bool>(), di in any::<bool>(), ti in any::<bool>()) {
            let c = control_word(0, BurstSize::Beats1, BurstSize::Beats1,
                TransferWidth::Width8, TransferWidth::Width8, si, di, ti);
            prop_assert_eq!((c >> 26) & 1 == 1, si);
            prop_assert_eq!((c >> 27) & 1 == 1, di);
            prop_assert_eq!((c >> 31) & 1 == 1, ti);
        }

        // ── channel-config-word bit encoder ───────────────────────────────────

        /// Fuzz: ch_cfg always has chn_en set and no bits above the lock bit (14).
        #[test]
        fn ch_cfg_word_no_stray_bits(
            sp in any::<u8>(), dp in any::<u8>(), fc in any::<u8>(),
            ei in any::<bool>(), ti in any::<bool>(), bl in any::<bool>(),
        ) {
            let w = ch_cfg_word(sp, dp, flow(fc), ei, ti, bl);
            prop_assert_eq!(w & 0x01, 1, "chn_en must always be set");
            // Defined: bit0 en, [1:4] s_peri, [5:8] d_peri, [9:11] flow, 12 int_en,
            // 13 int_tc, 14 lock → low 15 bits. Nothing above bit 14.
            prop_assert_eq!(w & !0x7FFF, 0, "stray bits in ch_cfg {:#010x}", w);
        }

        /// Fuzz: a 4-bit peripheral field truncates ids > 0x0F — verify both
        /// peripheral selects round-trip their low nibble and never collide.
        #[test]
        fn ch_cfg_word_peripheral_fields(sp in any::<u8>(), dp in any::<u8>()) {
            let w = ch_cfg_word(sp, dp, FlowControl::MemToMem, false, false, false);
            prop_assert_eq!((w >> 1) & 0x0F, (sp as u32) & 0x0F);
            prop_assert_eq!((w >> 5) & 0x0F, (dp as u32) & 0x0F);
        }

        /// Fuzz: flow control occupies bits [9:11] and round-trips its discriminant.
        #[test]
        fn ch_cfg_word_flow_field(fc in 0u8..4) {
            let f = flow(fc);
            let w = ch_cfg_word(0, 0, f, false, false, false);
            prop_assert_eq!((w >> 9) & 0x07, f as u32);
        }

        /// Fuzz: int_en / int_tc / lock flags map to bits 12 / 13 / 14 exactly.
        #[test]
        fn ch_cfg_word_flag_bits(ei in any::<bool>(), ti in any::<bool>(), bl in any::<bool>()) {
            let w = ch_cfg_word(0, 0, FlowControl::MemToMem, ei, ti, bl);
            prop_assert_eq!((w >> 12) & 1 == 1, ei);
            prop_assert_eq!((w >> 13) & 1 == 1, ti);
            prop_assert_eq!((w >> 14) & 1 == 1, bl);
        }
    }
}

// ── Async DMA completion (bespoke; DMA_INT = IRQ 59) ────────────────────────
#[cfg(feature = "async")]
mod asynch_impl {
    use super::{Dma0, DmaDriver};
    use crate::asynch::IrqSignal;
    use crate::interrupt::{self, Interrupt};
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll};

    static DMA_SIGNAL: IrqSignal = IrqSignal::new();

    /// DMA trap hook (IRQ 59): wake the awaiting transfer.
    pub fn on_interrupt() {
        DMA_SIGNAL.signal();
        interrupt::clear_pending(Interrupt::DMA_INT);
    }

    struct DmaDoneFuture;
    impl Future for DmaDoneFuture {
        type Output = ();
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if DMA_SIGNAL.take_fired() {
                Poll::Ready(())
            } else {
                DMA_SIGNAL.register(cx.waker());
                Poll::Pending
            }
        }
    }

    impl DmaDriver<'_, Dma0> {
        /// Await transfer-complete for `channel` (after configuring + enabling it),
        /// then clear its interrupt. Parks on the DMA IRQ on hardware; the WS63
        /// model completes the copy synchronously, so the fast path returns at once.
        pub async fn wait_transfer_done(&mut self, channel: u8) {
            let bit = 1u8 << channel; // Dma0: physical channel == logical channel
            if self.raw_interrupt_status().0 & bit == 0 {
                DMA_SIGNAL.reset();
                // SAFETY: enabling a known, fixed WS63 IRQ line.
                unsafe { interrupt::enable(Interrupt::DMA_INT) };
                if self.raw_interrupt_status().0 & bit == 0 {
                    DmaDoneFuture.await;
                }
            }
            self.clear_transfer_interrupt(channel);
        }
    }
}

#[cfg(feature = "async")]
pub use asynch_impl::on_interrupt;
