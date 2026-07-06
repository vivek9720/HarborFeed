//! HarborFeed parses and normalizes maritime operations feeds.

pub mod ais;
pub mod bitcursor;
pub mod edifact;
pub mod error;
pub mod lifecycle;
pub mod model;
pub mod nmea;
pub mod normalize;
pub mod ports;
pub mod rules;
pub mod tide_csv;
pub mod validate;

pub use error::{HarborError, Result};
pub use model::{Feed, FeedEvent, ValidationFinding};

pub fn parse_nmea_line(line: &str) -> Result<model::FeedEvent> {
    normalize::parse_nmea_event(line)
}

pub fn parse_ais_line(line: &str) -> Result<ais::AisMessage> {
    let fragment = ais::parse_aivdm(line)?;
    ais::decode_single_fragment(&fragment)
}

pub fn parse_edifact_bytes(data: &[u8]) -> Result<edifact::EdifactInterchange> {
    let text = std::str::from_utf8(data).map_err(|_| HarborError::InvalidUtf8)?;
    edifact::parse_interchange(text)
}

pub fn parse_tide_csv_bytes(data: &[u8]) -> Result<Vec<model::TideReading>> {
    let text = std::str::from_utf8(data).map_err(|_| HarborError::InvalidUtf8)?;
    tide_csv::parse_tide_csv(text)
}

pub fn parse_feed_bytes(data: &[u8]) -> Result<Feed> {
    normalize::parse_feed_bytes(data)
}

pub fn audit_feed(feed: &Feed) -> Vec<ValidationFinding> {
    validate::audit_feed(feed)
}

pub fn parse_and_audit(data: &[u8]) -> Result<Vec<ValidationFinding>> {
    let feed = parse_feed_bytes(data)?;
    Ok(audit_feed(&feed))
}
