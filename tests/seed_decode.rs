#[test]
fn nmea_seed_decodes() {
    let data = include_str!("../fuzz/corpus/nmea_fuzzer/gga_fix.nmea");
    let event = harborfeed::parse_nmea_line(data.trim()).expect("NMEA seed should parse");
    assert!(matches!(event, harborfeed::model::FeedEvent::GpsFix(_)));
}

#[test]
fn edifact_seed_decodes() {
    let data = include_bytes!("../fuzz/corpus/edifact_fuzzer/coprar_message.edi");
    let interchange = harborfeed::parse_edifact_bytes(data).expect("EDIFACT seed should parse");
    assert_eq!(interchange.messages.len(), 1);
}

#[test]
fn mixed_feed_audits() {
    let data = include_bytes!("../fuzz/corpus/feed_fuzzer/mixed_feed.txt");
    let feed = harborfeed::parse_feed_bytes(data).expect("mixed feed should parse");
    assert!(feed.events.len() >= 4);
    let _findings = harborfeed::audit_feed(&feed);
}

#[test]
fn tide_seed_decodes() {
    let data = include_bytes!("../fuzz/corpus/tide_csv_fuzzer/noaa_water_level.csv");
    let readings = harborfeed::parse_tide_csv_bytes(data).expect("tide seed should parse");
    assert_eq!(readings.len(), 2);
}
