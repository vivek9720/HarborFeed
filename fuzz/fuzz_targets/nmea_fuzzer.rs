#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    if let Ok(line) = core::str::from_utf8(data) {
        let _ = harborfeed::parse_nmea_line(line);
    }
});
