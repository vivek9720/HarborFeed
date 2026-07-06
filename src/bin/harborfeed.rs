use std::env;
use std::fs;
use std::io::{self, Read};

fn main() {
    let mut data = Vec::new();
    if let Some(path) = env::args().nth(1) {
        match fs::read(path) {
            Ok(bytes) => data = bytes,
            Err(err) => {
                eprintln!("failed to read input: {err}");
                std::process::exit(2);
            }
        }
    } else if let Err(err) = io::stdin().read_to_end(&mut data) {
        eprintln!("failed to read stdin: {err}");
        std::process::exit(2);
    }
    match harborfeed::parse_feed_bytes(&data) {
        Ok(feed) => {
            let findings = harborfeed::audit_feed(&feed);
            println!("events={}", feed.events.len());
            println!("invalid_nmea_checksums={}", feed.invalid_nmea_checksums);
            println!("warnings={}", feed.parse_warnings.len());
            println!("findings={}", findings.len());
            for finding in findings.iter().take(20) {
                println!(
                    "rule={} severity={} subject={} message={}",
                    finding.rule_id, finding.severity, finding.subject, finding.message
                );
            }
        }
        Err(err) => {
            eprintln!("parse error: {err}");
            std::process::exit(1);
        }
    }
}
