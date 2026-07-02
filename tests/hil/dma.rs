/// DMA memory-to-memory end-to-end on primary DMA channel 0.
#[cfg(all(feature = "chip-ws63", feature = "unstable"))]
pub(crate) fn dma_mem_to_mem() {
    use crate::hal;
    use hal::dma::{Dma0, DmaDriver};

    const N: usize = 8;
    #[repr(C, align(32))]
    struct Aligned([u32; N]);
    static SRC: Aligned = Aligned([
        0xaaaa_0001,
        0xaaaa_0002,
        0xaaaa_0003,
        0xaaaa_0004,
        0xaaaa_0005,
        0xaaaa_0006,
        0xaaaa_0007,
        0xaaaa_0008,
    ]);
    static mut DST: Aligned = Aligned([0u32; N]);

    // SAFETY: sequential single-hart run; DST is touched only in this test.
    let dst: &'static mut [u32] = unsafe { &mut (*core::ptr::addr_of_mut!(DST)).0 };

    // SAFETY: sequential single-hart run; DMA singleton not otherwise held.
    let dma = DmaDriver::<Dma0>::new_dma(unsafe { hal::peripherals::Dma::steal() });
    let chs = dma.split_channels().expect("DMA channels already claimed");
    let transfer = dma.start_mem_to_mem(chs.ch0, &SRC.0[..], dst).expect("DMA mem-to-mem start failed");
    let (_dma, _ch0, _src, dst) = transfer.wait().expect("DMA mem-to-mem wait failed");

    for (i, &want) in SRC.0.iter().enumerate() {
        let got = unsafe { core::ptr::read_volatile(dst.as_ptr().add(i)) };
        assert_eq!(got, want, "DMA mem→mem mismatch @{}: got=0x{:08x} want=0x{:08x}", i, got, want);
    }
}

/// Owned-buffer `Transfer` guard over the same mem-to-mem path on silicon.
#[cfg(all(feature = "chip-ws63", feature = "unstable"))]
pub(crate) fn dma_transfer_guard() {
    use crate::hal;
    use hal::dma::{Dma0, DmaDriver};

    #[repr(C, align(32))]
    struct Aligned([u32; 8]);
    static mut SRC: Aligned = Aligned([
        0xbbbb_0001,
        0xbbbb_0002,
        0xbbbb_0003,
        0xbbbb_0004,
        0xbbbb_0005,
        0xbbbb_0006,
        0xbbbb_0007,
        0xbbbb_0008,
    ]);
    static mut DST: Aligned = Aligned([0u32; 8]);

    // SAFETY: sequential single-hart run; these statics are touched only here.
    let src: &'static mut [u32] = unsafe { &mut (*core::ptr::addr_of_mut!(SRC)).0 };
    let dst: &'static mut [u32] = unsafe { &mut (*core::ptr::addr_of_mut!(DST)).0 };
    let want =
        [0xbbbb_0001u32, 0xbbbb_0002, 0xbbbb_0003, 0xbbbb_0004, 0xbbbb_0005, 0xbbbb_0006, 0xbbbb_0007, 0xbbbb_0008];

    // SAFETY: sequential single-hart run; DMA singleton not otherwise held.
    let dma = DmaDriver::<Dma0>::new_dma(unsafe { hal::peripherals::Dma::steal() });
    let chs = dma.split_channels().expect("DMA channels already claimed");

    let transfer = dma.start_mem_to_mem(chs.ch0, &*src, dst).expect("DMA guard start failed");
    let (_dma, _ch0, _src, dst) = transfer.wait().expect("DMA guard wait failed");

    for (i, &w) in want.iter().enumerate() {
        let got = unsafe { core::ptr::read_volatile(dst.as_ptr().add(i)) };
        assert_eq!(got, w, "DMA guard mem→mem mismatch @{}: got=0x{:08x} want=0x{:08x}", i, got, w);
    }
}
