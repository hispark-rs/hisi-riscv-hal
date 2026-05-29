//! SFC (SPI Flash Controller) driver for WS63.
//!
//! The WS63 SFC provides a high-speed interface to external SPI NOR Flash
//! memory. It supports standard, dual, and quad SPI modes, configurable
//! timing, command-based operations, and bus DMA for efficient data transfer.
//!
//! # Features
//!
//! - Standard/Dual/Quad SPI modes
//! - 3-byte and 4-byte addressing
//! - Configurable SPI mode (Mode0/Mode3)
//! - Command-based flash operations (read, write, erase)
//! - 16-word (64-byte) data buffer for indirect operations
//! - Bus DMA for direct memory-mapped flash access
//! - Hardware write protect
//! - AES low-power mode for XIP encryption

use crate::peripherals::SfcCfg;

/// SPI interface type for flash operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiIfType {
    /// Standard SPI (1 data line).
    Standard = 0,
    /// Dual I/O (2 data lines).
    DualIO = 1,
    /// Dual I/O continuous.
    DualIOCont = 2,
    /// Quad I/O (4 data lines).
    QuadIO = 3,
    /// Quad I/O continuous.
    QuadIOCont = 4,
}

/// Flash address mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressMode {
    /// 3-byte addressing (up to 16 MB).
    ThreeByte = 0,
    /// 4-byte addressing (up to 4 GB).
    FourByte = 1,
}

/// SPI mode for the flash bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashSpiMode {
    /// CPOL=0, CPHA=0.
    Mode0 = 0,
    /// CPOL=1, CPHA=1.
    Mode3 = 1,
}

/// SFC read data delay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadDelay {
    /// No delay.
    Delay0,
    /// Half cycle delay.
    DelayHalf,
    /// 1 cycle delay.
    Delay1,
    /// 1.5 cycle delay.
    Delay1_5,
}

/// SFC bus configuration for memory-mapped reads.
#[derive(Debug, Clone, Copy)]
pub struct BusConfig {
    /// SPI interface type for read operations.
    pub read_if_type: SpiIfType,
    /// Number of dummy bytes for read operations.
    pub read_dummy_bytes: u8,
    /// Read instruction code (e.g., 0x03 for standard read, 0xEB for quad read).
    pub read_instruction: u8,
    /// Prefetch count for read operations.
    pub read_prefetch_cnt: u8,
    /// SPI interface type for write operations.
    pub write_if_type: SpiIfType,
    /// Number of dummy bytes for write operations.
    pub write_dummy_bytes: u8,
    /// Write instruction code.
    pub write_instruction: u8,
}

impl Default for BusConfig {
    fn default() -> Self {
        Self {
            read_if_type: SpiIfType::Standard,
            read_dummy_bytes: 0,
            read_instruction: 0x03, // Standard SPI read
            read_prefetch_cnt: 0,
            write_if_type: SpiIfType::Standard,
            write_dummy_bytes: 0,
            write_instruction: 0x02, // Standard page program
        }
    }
}

/// SFC driver.
pub struct SfcDriver<'d> {
    _sfc: SfcCfg<'d>,
}

/// Minimum inter-operation delay in clock cycles.
const MIN_TSHSL: u32 = 5;

impl<'d> SfcDriver<'d> {
    /// Create a new SFC driver.
    pub fn new(sfc: SfcCfg<'d>) -> Self {
        Self { _sfc: sfc }
    }

    fn regs(&self) -> &'static ws63_pac::sfc_cfg::RegisterBlock {
        unsafe { &*SfcCfg::ptr() }
    }

    // ── Global configuration ───────────────────────────────────────

    /// Configure the global SFC settings.
    pub fn configure_global(
        &mut self,
        spi_mode: FlashSpiMode,
        addr_mode: AddressMode,
        read_delay: ReadDelay,
        write_protect: bool,
    ) {
        let mut val: u32 = 0;
        val |= spi_mode as u32; // mode [0]
        if write_protect {
            val |= 1 << 1; // wp_en [1]
        }
        if matches!(addr_mode, AddressMode::FourByte) {
            val |= 1 << 2; // flash_addr_mode [2]
        }
        val |= (read_delay as u32) << 3; // rd_delay [3:5]

        unsafe {
            self.regs().global_config().write(|w| w.bits(val));
        }
    }

    /// Configure the SFC timing parameters.
    ///
    /// * `tshsl` — Inter-operation delay: (tshsl + 2) clock cycles
    /// * `tcss` — CS setup time: (tcss + 1) clock cycles
    /// * `tcsh` — CS hold time: (tcsh + 1) clock cycles
    pub fn configure_timing(&mut self, tshsl: u8, tcss: u8, tcsh: u8) {
        // Clamp to register field widths
        let tshsl = (tshsl.max(MIN_TSHSL as u8 - 2) as u32) & 0x0F;
        let tcss = (tcss as u32) & 0x07;
        let tcsh = (tcsh as u32) & 0x07;

        let val = tshsl | (tcss << 8) | (tcsh << 12);

        unsafe {
            self.regs().timing().write(|w| w.bits(val));
        }
    }

    // ── Bus configuration ──────────────────────────────────────────

    /// Configure the bus read/write parameters for memory-mapped access.
    pub fn configure_bus(&mut self, config: &BusConfig) {
        let r = self.regs();

        // BUS_CONFIG1
        let mut cfg1: u32 = 0;
        cfg1 |= (config.read_if_type as u32) & 0x07; // rd_mem_if_type [0:2]
        cfg1 |= ((config.read_dummy_bytes as u32) & 0x07) << 3; // rd_dummy_bytes [3:5]
        cfg1 |= ((config.read_prefetch_cnt as u32) & 0x03) << 6; // rd_prefetch_cnt [6:7]
        cfg1 |= ((config.read_instruction as u32) & 0xFF) << 8; // rd_ins [8:15]
        cfg1 |= ((config.write_if_type as u32) & 0x07) << 16; // wr_mem_if_type [16:18]
        cfg1 |= ((config.write_dummy_bytes as u32) & 0x07) << 19; // wr_dummy_bytes [19:21]
        cfg1 |= ((config.write_instruction as u32) & 0xFF) << 22; // wr_ins [22:29]

        unsafe {
            r.bus_config1().write(|w| w.bits(cfg1));
        }
    }

    /// Release the SFC bus from soft reset.
    pub fn release_bus_reset(&mut self) {
        unsafe {
            self.regs().soft_rst_mask().write(|w| w.bits(0x01));
        }
    }

    /// Hold the SFC bus in soft reset.
    pub fn hold_bus_reset(&mut self) {
        unsafe {
            self.regs().soft_rst_mask().write(|w| w.bits(0x00));
        }
    }

    // ── Command operations ──────────────────────────────────────────

    /// Write a 32-bit word to a specific data buffer register.
    fn write_databuf(r: &ws63_pac::sfc_cfg::RegisterBlock, idx: usize, word: u32) {
        unsafe {
            match idx {
                0 => {
                    r.cmd_databuf_0().write(|w| w.bits(word));
                }
                1 => {
                    r.cmd_databuf_1().write(|w| w.bits(word));
                }
                2 => {
                    r.cmd_databuf_2().write(|w| w.bits(word));
                }
                3 => {
                    r.cmd_databuf_3().write(|w| w.bits(word));
                }
                4 => {
                    r.cmd_databuf_4().write(|w| w.bits(word));
                }
                5 => {
                    r.cmd_databuf_5().write(|w| w.bits(word));
                }
                6 => {
                    r.cmd_databuf_6().write(|w| w.bits(word));
                }
                7 => {
                    r.cmd_databuf_7().write(|w| w.bits(word));
                }
                8 => {
                    r.cmd_databuf_8().write(|w| w.bits(word));
                }
                9 => {
                    r.cmd_databuf_9().write(|w| w.bits(word));
                }
                10 => {
                    r.cmd_databuf_10().write(|w| w.bits(word));
                }
                11 => {
                    r.cmd_databuf_11().write(|w| w.bits(word));
                }
                12 => {
                    r.cmd_databuf_12().write(|w| w.bits(word));
                }
                13 => {
                    r.cmd_databuf_13().write(|w| w.bits(word));
                }
                14 => {
                    r.cmd_databuf_14().write(|w| w.bits(word));
                }
                15 => {
                    r.cmd_databuf_15().write(|w| w.bits(word));
                }
                _ => {}
            }
        }
    }

    /// Read a 32-bit word from a specific data buffer register.
    fn read_databuf(r: &ws63_pac::sfc_cfg::RegisterBlock, idx: usize) -> u32 {
        match idx {
            0 => r.cmd_databuf_0().read().bits(),
            1 => r.cmd_databuf_1().read().bits(),
            2 => r.cmd_databuf_2().read().bits(),
            3 => r.cmd_databuf_3().read().bits(),
            4 => r.cmd_databuf_4().read().bits(),
            5 => r.cmd_databuf_5().read().bits(),
            6 => r.cmd_databuf_6().read().bits(),
            7 => r.cmd_databuf_7().read().bits(),
            8 => r.cmd_databuf_8().read().bits(),
            9 => r.cmd_databuf_9().read().bits(),
            10 => r.cmd_databuf_10().read().bits(),
            11 => r.cmd_databuf_11().read().bits(),
            12 => r.cmd_databuf_12().read().bits(),
            13 => r.cmd_databuf_13().read().bits(),
            14 => r.cmd_databuf_14().read().bits(),
            15 => r.cmd_databuf_15().read().bits(),
            _ => 0,
        }
    }

    /// Execute a flash command (no data phase).
    ///
    /// * `instruction` — Flash operation code.
    /// * `address` — Operation address (for commands with address phase).
    /// * `address_enable` — Whether the command includes an address phase.
    pub fn send_command(&mut self, instruction: u8, address: u32, address_enable: bool) {
        let r = self.regs();

        // Write instruction
        unsafe {
            r.cmd_ins().write(|w| w.bits(instruction as u32));
        }

        // Write address
        if address_enable {
            unsafe {
                r.cmd_addr().write(|w| w.bits(address));
            }
        }

        // Build command config
        let mut cmd_cfg: u32 = 0;
        cmd_cfg |= 0x01; // start
        if address_enable {
            cmd_cfg |= 1 << 2; // addr_en
        }
        cmd_cfg |= 0 << 17; // mem_if_type = Standard

        unsafe {
            r.cmd_config().write(|w| w.bits(cmd_cfg));
        }

        // Wait for command completion
        while !self.command_done() {}
        self.clear_interrupts();
    }

    /// Execute a flash command with data phase.
    ///
    /// * `instruction` — Flash operation code.
    /// * `address` — Operation address.
    /// * `write_data` — Data to write (for write commands).
    /// * `read` — `true` for read commands, `false` for write commands.
    ///
    /// Returns the read data (up to 64 bytes) for read commands.
    pub fn command_with_data(
        &mut self,
        instruction: u8,
        address: u32,
        write_data: &[u8],
        read: bool,
    ) -> Result<[u8; 64], SfcError> {
        let r = self.regs();
        let data_len = write_data.len().min(64);

        if !read && !write_data.is_empty() {
            // Load write data into data buffer
            for (i, chunk) in write_data[..data_len].chunks(4).enumerate() {
                let mut word: u32 = 0;
                for (j, &b) in chunk.iter().enumerate() {
                    word |= (b as u32) << (j * 8);
                }
                Self::write_databuf(r, i, word);
            }
        }

        // Write instruction and address
        unsafe {
            r.cmd_ins().write(|w| w.bits(instruction as u32));
            r.cmd_addr().write(|w| w.bits(address));
        }

        // Build command config
        let mut cmd_cfg: u32 = 0;
        cmd_cfg |= 0x01; // start
        cmd_cfg |= 1 << 2; // addr_en
        cmd_cfg |= 1 << 7; // data_en
        if read {
            cmd_cfg |= 1 << 8; // rw = read
        }
        cmd_cfg |= (((data_len.saturating_sub(1)) as u32) & 0x3F) << 9; // data_cnt
        cmd_cfg |= 0 << 17; // mem_if_type = Standard

        unsafe {
            r.cmd_config().write(|w| w.bits(cmd_cfg));
        }

        // Wait for command completion
        while !self.command_done() {}
        self.clear_interrupts();

        // Read back data for read commands
        let mut result = [0u8; 64];
        if read {
            let mut idx = 0;
            for i in 0..(data_len + 3) / 4 {
                let word = Self::read_databuf(r, i);
                let bytes = word.to_le_bytes();
                for &b in &bytes {
                    if idx < data_len {
                        result[idx] = b;
                        idx += 1;
                    }
                }
            }
        }

        Ok(result)
    }

    // ── Bus DMA ────────────────────────────────────────────────────

    /// Start a bus DMA transfer between flash and memory.
    ///
    /// * `mem_addr` — Memory address (must be in valid range).
    /// * `flash_addr` — Flash address.
    /// * `length` — Number of bytes to transfer.
    /// * `read` — `true` for flash-to-memory read, `false` for memory-to-flash write.
    pub fn bus_dma_start(&mut self, mem_addr: u32, flash_addr: u32, length: u32, read: bool) {
        let r = self.regs();

        unsafe {
            r.bus_dma_mem_saddr().write(|w| w.bits(mem_addr));
            r.bus_dma_flash_saddr().write(|w| w.bits(flash_addr));
            r.bus_dma_len().write(|w| w.bits(length & 0x3FFF_FFFF));
        }

        let mut ctrl: u32 = 0;
        ctrl |= 0x01; // dma_start
        if read {
            ctrl |= 1 << 1; // dma_rw = read
        }

        unsafe {
            r.bus_dma_ctrl().write(|w| w.bits(ctrl));
        }
    }

    /// Wait for bus DMA to complete.
    pub fn bus_dma_wait(&self) {
        while !self.dma_done() {}
        self.clear_interrupts();
    }

    /// Check if the DMA transfer is complete.
    pub fn dma_done(&self) -> bool {
        self.regs().int_status().read().bits() & 0x02 != 0
    }

    /// Check if a flash command is complete.
    pub fn command_done(&self) -> bool {
        self.regs().int_status().read().bits() & 0x01 != 0
    }

    /// Clear all SFC interrupts.
    pub fn clear_interrupts(&self) {
        unsafe {
            self.regs().int_clear().write(|w| w.bits(0x03));
        }
    }

    /// Enable specific SFC interrupts.
    ///
    /// * `cmd_done` — Command operation complete interrupt.
    /// * `dma_done` — DMA transfer complete interrupt.
    pub fn enable_interrupts(&mut self, cmd_done: bool, dma_done: bool) {
        let mut mask: u32 = 0;
        if cmd_done {
            mask |= 0x01;
        }
        if dma_done {
            mask |= 0x02;
        }
        unsafe {
            self.regs().int_mask().write(|w| w.bits(mask));
        }
    }

    /// Check raw interrupt status.
    ///
    /// Returns `(cmd_done_raw, dma_done_raw)`.
    pub fn raw_interrupt_status(&self) -> (bool, bool) {
        let sts = self.regs().int_raw_status().read().bits();
        ((sts & 0x01) != 0, (sts & 0x02) != 0)
    }

    // ── AES (XIP encryption) control ───────────────────────────────

    /// Enable AES low-power mode (for XIP encrypted execution).
    pub fn enable_aes_low_power(&mut self) {
        unsafe {
            self.regs().lea_lp_en().write(|w| w.bits(0x01));
        }
    }

    /// Disable AES low-power mode.
    pub fn disable_aes_low_power(&mut self) {
        unsafe {
            self.regs().lea_lp_en().write(|w| w.bits(0x00));
        }
    }

    /// Set AES IV valid flag.
    pub fn set_iv_valid(&mut self) {
        unsafe {
            self.regs().lea_iv_vld().write(|w| w.bits(0x01));
        }
    }

    /// Read AES DFX information (for debugging).
    pub fn read_aes_dfx(&self) -> u32 {
        self.regs().lea_dfx_info().read().bits()
    }
}

/// SFC operation error.
#[derive(Debug)]
pub enum SfcError {
    /// Command timeout.
    Timeout,
    /// DMA transfer error.
    DmaError,
}
