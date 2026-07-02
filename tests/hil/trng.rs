use crate::hal;

/// TRNG produces entropy (trng.rs).
pub(crate) fn trng_produces_entropy() {
    use hal::trng::TrngDriver;

    // SAFETY: sequential single-hart run; TRNG singleton not otherwise held.
    let trng = TrngDriver::new(unsafe { hal::peripherals::Trng::steal() });
    let mut samples = [0u32; 4];
    let mut got = 0usize;
    for _ in 0..16 {
        if got >= samples.len() {
            break;
        }
        if let Ok(w) = trng.read_blocking() {
            samples[got] = w;
            got += 1;
        }
    }
    assert!(got >= 2, "TRNG produced fewer than 2 words (got {got})");
    let all_same = samples[..got].iter().all(|&w| w == samples[0]);
    assert!(!all_same, "TRNG returned {got} identical words 0x{:08x} — no entropy", samples[0]);
}

/// TRNG fill path (trng.rs).
pub(crate) fn trng_fill_bytes_produces_data() {
    use hal::trng::TrngDriver;

    // SAFETY: sequential single-hart run; TRNG singleton not otherwise held.
    let trng = TrngDriver::new(unsafe { hal::peripherals::Trng::steal() });
    let mut buf = [0u8; 16];

    trng.fill_bytes(&mut buf).expect("TRNG fill_bytes timed out");
    let all_same = buf.iter().all(|&b| b == buf[0]);
    assert!(!all_same, "TRNG fill_bytes returned 16 identical bytes 0x{:02x}", buf[0]);
}
