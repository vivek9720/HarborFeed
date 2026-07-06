#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = harborfeed::parse_and_audit(data);
});
