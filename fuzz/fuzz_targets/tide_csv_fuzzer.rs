#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = harborfeed::parse_tide_csv_bytes(data);
});
