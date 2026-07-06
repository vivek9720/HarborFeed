use crate::bitcursor::BitCursor;
use crate::error::{HarborError, Result};
use crate::model::{VesselPosition, VesselStatic};
use crate::nmea;

#[derive(Debug, Clone)]
pub struct AisFragment {
    pub fragment_count: u8,
    pub fragment_number: u8,
    pub sequential_id: Option<String>,
    pub radio_channel: Option<char>,
    pub payload: String,
    pub fill_bits: u8,
}

#[derive(Debug, Clone)]
pub enum AisMessage {
    Position(VesselPosition),
    Static(VesselStatic),
    BaseStation { mmsi: u32, year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8 },
    Unknown { message_type: u8, mmsi: Option<u32> },
}

pub fn parse_aivdm(line: &str) -> Result<AisFragment> {
    let sentence = nmea::parse_sentence(line)?;
    if sentence.formatter != "VDM" && sentence.formatter != "VDO" {
        return Err(HarborError::UnsupportedSentence(nmea::sentence_name(&sentence)));
    }
    if sentence.fields.len() < 6 {
        return Err(HarborError::InvalidNmeaField("aivdm fields"));
    }
    let fragment_count = sentence.fields[0].parse::<u8>().map_err(|_| HarborError::InvalidNmeaField("fragment count"))?;
    let fragment_number = sentence.fields[1].parse::<u8>().map_err(|_| HarborError::InvalidNmeaField("fragment number"))?;
    let sequential_id = if sentence.fields[2].is_empty() { None } else { Some(sentence.fields[2].clone()) };
    let radio_channel = sentence.fields[3].chars().next();
    let payload = sentence.fields[4].clone();
    let fill_bits = sentence.fields[5].parse::<u8>().map_err(|_| HarborError::InvalidNmeaField("fill bits"))?;
    Ok(AisFragment {
        fragment_count,
        fragment_number,
        sequential_id,
        radio_channel,
        payload,
        fill_bits,
    })
}

pub fn decode_single_fragment(fragment: &AisFragment) -> Result<AisMessage> {
    if fragment.fragment_count != 1 || fragment.fragment_number != 1 {
        return Err(HarborError::InvalidAisPayload);
    }
    decode_payload(&fragment.payload, fragment.fill_bits)
}

pub fn decode_payload(payload: &str, fill_bits: u8) -> Result<AisMessage> {
    let mut bits = BitCursor::from_sixbit_payload(payload, fill_bits)?;
    let message_type = bits.read_u8(6)?;
    let _repeat = bits.read_u8(2)?;
    let mmsi = bits.read_u32(30)?;
    match message_type {
        1 | 2 | 3 => decode_position_report(message_type, mmsi, bits),
        4 => decode_base_station(mmsi, bits),
        5 => decode_static_voyage(mmsi, bits),
        18 => decode_class_b_position(mmsi, bits),
        19 => decode_extended_class_b(mmsi, bits),
        24 => decode_static_data_report(mmsi, bits),
        other => Ok(AisMessage::Unknown { message_type: other, mmsi: Some(mmsi) }),
    }
}

fn decode_position_report(_message_type: u8, mmsi: u32, mut bits: BitCursor) -> Result<AisMessage> {
    let nav_status = bits.read_u8(4)?;
    let _rot = bits.read_u8(8)?;
    let sog_raw = bits.read_u32(10)?;
    let position_accuracy = bits.read_bool()?;
    let lon_raw = bits.read_signed(28)?;
    let lat_raw = bits.read_signed(27)?;
    let cog_raw = bits.read_u32(12)?;
    let heading_raw = bits.read_u32(9)?;
    let timestamp = bits.read_u8(6)?;
    let _maneuver = bits.read_u8(2)?;
    bits.skip(3)?;
    let _raim = bits.read_bool()?;
    let mut position = VesselPosition {
        mmsi,
        latitude: ais_coord(lat_raw),
        longitude: ais_coord(lon_raw),
        speed_knots: if sog_raw == 1023 { None } else { Some(sog_raw as f32 / 10.0) },
        course_degrees: if cog_raw == 3600 { None } else { Some(cog_raw as f32 / 10.0) },
        heading_degrees: if heading_raw == 511 { None } else { Some(heading_raw as u16) },
        nav_status: Some(nav_status),
        timestamp_second: Some(timestamp),
        source: "AIS",
    };
    if !position_accuracy {
        position.course_degrees = position.course_degrees.map(|v| (v * 10.0).round() / 10.0);
    }
    Ok(AisMessage::Position(position))
}

fn decode_class_b_position(mmsi: u32, mut bits: BitCursor) -> Result<AisMessage> {
    bits.skip(8)?;
    let sog_raw = bits.read_u32(10)?;
    let _accuracy = bits.read_bool()?;
    let lon_raw = bits.read_signed(28)?;
    let lat_raw = bits.read_signed(27)?;
    let cog_raw = bits.read_u32(12)?;
    let heading_raw = bits.read_u32(9)?;
    let timestamp = bits.read_u8(6)?;
    Ok(AisMessage::Position(VesselPosition {
        mmsi,
        latitude: ais_coord(lat_raw),
        longitude: ais_coord(lon_raw),
        speed_knots: if sog_raw == 1023 { None } else { Some(sog_raw as f32 / 10.0) },
        course_degrees: if cog_raw == 3600 { None } else { Some(cog_raw as f32 / 10.0) },
        heading_degrees: if heading_raw == 511 { None } else { Some(heading_raw as u16) },
        nav_status: None,
        timestamp_second: Some(timestamp),
        source: "AIS-B",
    }))
}

fn decode_extended_class_b(mmsi: u32, mut bits: BitCursor) -> Result<AisMessage> {
    bits.skip(8)?;
    let sog_raw = bits.read_u32(10)?;
    let _accuracy = bits.read_bool()?;
    let lon_raw = bits.read_signed(28)?;
    let lat_raw = bits.read_signed(27)?;
    let cog_raw = bits.read_u32(12)?;
    let heading_raw = bits.read_u32(9)?;
    let timestamp = bits.read_u8(6)?;
    bits.skip(4)?;
    let name = bits.read_text(20).ok();
    let ship_type = bits.read_u8(8).ok();
    Ok(AisMessage::Position(VesselPosition {
        mmsi,
        latitude: ais_coord(lat_raw),
        longitude: ais_coord(lon_raw),
        speed_knots: if sog_raw == 1023 { None } else { Some(sog_raw as f32 / 10.0) },
        course_degrees: if cog_raw == 3600 { None } else { Some(cog_raw as f32 / 10.0) },
        heading_degrees: if heading_raw == 511 { None } else { Some(heading_raw as u16) },
        nav_status: ship_type,
        timestamp_second: Some(timestamp),
        source: if name.as_deref().unwrap_or("").is_empty() { "AIS-B" } else { "AIS-B-EXT" },
    }))
}

fn decode_static_voyage(mmsi: u32, mut bits: BitCursor) -> Result<AisMessage> {
    bits.skip(2)?;
    let imo = bits.read_u32(30)?;
    let callsign = bits.read_text(7)?;
    let name = bits.read_text(20)?;
    let ship_type = bits.read_u8(8)?;
    bits.skip(30)?;
    bits.skip(4)?;
    bits.skip(4)?;
    bits.skip(5)?;
    bits.skip(5)?;
    let draught = bits.read_u8(8)?;
    let destination = bits.read_text(20).ok();
    Ok(AisMessage::Static(VesselStatic {
        mmsi,
        imo: if imo == 0 { None } else { Some(imo) },
        callsign: empty_to_none(callsign),
        name: empty_to_none(name),
        ship_type: Some(ship_type),
        draught_dm: if draught == 0 { None } else { Some(draught) },
        destination,
    }))
}

fn decode_static_data_report(mmsi: u32, mut bits: BitCursor) -> Result<AisMessage> {
    let part_no = bits.read_u8(2)?;
    if part_no == 0 {
        let name = bits.read_text(20)?;
        Ok(AisMessage::Static(VesselStatic {
            mmsi,
            name: empty_to_none(name),
            ..VesselStatic::default()
        }))
    } else {
        let ship_type = bits.read_u8(8)?;
        let vendor = bits.read_text(7).ok();
        let callsign = bits.read_text(7).ok();
        Ok(AisMessage::Static(VesselStatic {
            mmsi,
            callsign: callsign.and_then(empty_to_none),
            name: vendor.and_then(empty_to_none),
            ship_type: Some(ship_type),
            ..VesselStatic::default()
        }))
    }
}

fn decode_base_station(mmsi: u32, mut bits: BitCursor) -> Result<AisMessage> {
    let year = bits.read_u32(14)? as u16;
    let month = bits.read_u8(4)?;
    let day = bits.read_u8(5)?;
    let hour = bits.read_u8(5)?;
    let minute = bits.read_u8(6)?;
    let second = bits.read_u8(6)?;
    Ok(AisMessage::BaseStation { mmsi, year, month, day, hour, minute, second })
}

fn ais_coord(raw: i32) -> f64 {
    raw as f64 / 600000.0
}

fn empty_to_none(s: String) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
