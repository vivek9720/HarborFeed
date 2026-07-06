use crate::ais::{self, AisMessage};
use crate::edifact;
use crate::error::{HarborError, Result};
use crate::model::{Feed, FeedEvent, VesselPosition};
use crate::nmea;
use crate::tide_csv;

pub fn parse_feed_bytes(data: &[u8]) -> Result<Feed> {
    let text = std::str::from_utf8(data).map_err(|_| HarborError::InvalidUtf8)?;
    parse_feed_text(text)
}

pub fn parse_feed_text(text: &str) -> Result<Feed> {
    let mut feed = Feed::default();
    if looks_like_edifact(text) {
        ingest_edifact(text, &mut feed)?;
        return Ok(feed);
    }
    if looks_like_tide_csv(text) {
        for reading in tide_csv::parse_tide_csv(text)? {
            feed.push(FeedEvent::TideReading(reading));
        }
        return Ok(feed);
    }
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("UNB+") || trimmed.starts_with("UNA") {
            ingest_edifact(trimmed, &mut feed)?;
        } else if trimmed.starts_with('$') || trimmed.starts_with('!') {
            match parse_nmea_event(trimmed) {
                Ok(event) => feed.push(event),
                Err(HarborError::InvalidNmeaChecksum) => feed.invalid_nmea_checksums += 1,
                Err(err) => feed.parse_warnings.push(err.to_string()),
            }
        } else if trimmed.contains(',') && trimmed.to_ascii_lowercase().contains("station") {
            for reading in tide_csv::parse_tide_csv(trimmed)? {
                feed.push(FeedEvent::TideReading(reading));
            }
        } else {
            feed.push(FeedEvent::UnknownSentence(trimmed.to_string()));
        }
    }
    Ok(feed)
}

pub fn parse_nmea_event(line: &str) -> Result<FeedEvent> {
    let sentence = nmea::parse_sentence(line)?;
    if !sentence.checksum_valid {
        return Err(HarborError::InvalidNmeaChecksum);
    }
    if sentence.formatter == "VDM" || sentence.formatter == "VDO" {
        let fragment = ais::parse_aivdm(line)?;
        match ais::decode_single_fragment(&fragment)? {
            AisMessage::Position(position) => Ok(FeedEvent::VesselPosition(position)),
            AisMessage::Static(static_data) => Ok(FeedEvent::VesselStatic(static_data)),
            AisMessage::BaseStation { mmsi, .. } => Ok(FeedEvent::VesselPosition(VesselPosition {
                mmsi,
                source: "AIS-BASE",
                ..VesselPosition::default()
            })),
            AisMessage::Unknown { message_type, .. } => Ok(FeedEvent::UnknownSentence(format!("AIS type {message_type}"))),
        }
    } else {
        nmea::parse_gps_event(&sentence)
    }
}

fn ingest_edifact(text: &str, feed: &mut Feed) -> Result<()> {
    let interchange = edifact::parse_interchange(text)?;
    for message in &interchange.messages {
        let summary = edifact::summarize_message(message);
        feed.push(FeedEvent::EdifactMessage(summary));
        if let Some(port_call) = edifact::extract_port_call(message) {
            feed.push(FeedEvent::PortCall(port_call));
        }
        for cargo in edifact::extract_cargo_events(message) {
            feed.push(FeedEvent::CargoEvent(cargo));
        }
    }
    Ok(())
}

fn looks_like_edifact(text: &str) -> bool {
    text.contains("UNB+") && text.contains("UNH+") && (text.contains("UNT+") || text.contains("UNZ+"))
}

fn looks_like_tide_csv(text: &str) -> bool {
    let first = text.lines().next().unwrap_or("").to_ascii_lowercase();
    first.contains("station") && first.contains("time") && first.contains(',')
}
