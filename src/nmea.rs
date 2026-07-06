use crate::bitcursor;
use crate::error::{HarborError, Result};
use crate::model::{FeedEvent, GpsFix};

#[derive(Debug, Clone)]
pub struct NmeaSentence {
    pub start: char,
    pub talker: String,
    pub formatter: String,
    pub fields: Vec<String>,
    pub checksum: u8,
    pub checksum_valid: bool,
    pub body: String,
}

pub fn parse_sentence(line: &str) -> Result<NmeaSentence> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(HarborError::EmptyInput);
    }
    let start = trimmed.chars().next().ok_or(HarborError::EmptyInput)?;
    if start != '$' && start != '!' {
        return Err(HarborError::InvalidNmeaStart);
    }
    let star = trimmed.rfind('*').ok_or(HarborError::MissingNmeaChecksum)?;
    if star + 3 > trimmed.len() {
        return Err(HarborError::MissingNmeaChecksum);
    }
    let body = &trimmed[1..star];
    let checksum_text = &trimmed[star + 1..star + 3];
    let checksum = u8::from_str_radix(checksum_text, 16).map_err(|_| HarborError::MissingNmeaChecksum)?;
    let computed = bitcursor::nmea_checksum(body);
    let mut parts = body.split(',');
    let header = parts.next().ok_or(HarborError::InvalidNmeaField("header"))?;
    if header.len() < 3 {
        return Err(HarborError::InvalidNmeaField("header"));
    }
    let talker = header[0..2].to_string();
    let formatter = header[2..].to_string();
    let fields = parts.map(|s| s.to_string()).collect();
    Ok(NmeaSentence {
        start,
        talker,
        formatter,
        fields,
        checksum,
        checksum_valid: computed == checksum,
        body: body.to_string(),
    })
}

pub fn parse_gps_event(sentence: &NmeaSentence) -> Result<FeedEvent> {
    match sentence.formatter.as_str() {
        "GGA" => Ok(FeedEvent::GpsFix(parse_gga(sentence)?)),
        "RMC" => Ok(FeedEvent::GpsFix(parse_rmc(sentence)?)),
        "GLL" => Ok(FeedEvent::GpsFix(parse_gll(sentence)?)),
        "VTG" => Ok(FeedEvent::GpsFix(parse_vtg(sentence)?)),
        "HDT" => Ok(FeedEvent::GpsFix(parse_hdt(sentence)?)),
        "ZDA" => Ok(FeedEvent::GpsFix(parse_zda(sentence)?)),
        other => Err(HarborError::UnsupportedSentence(other.to_string())),
    }
}

fn field<'a>(sentence: &'a NmeaSentence, index: usize, name: &'static str) -> Result<&'a str> {
    sentence
        .fields
        .get(index)
        .map(|s| s.as_str())
        .ok_or(HarborError::InvalidNmeaField(name))
}

fn parse_u8_opt(text: &str) -> Option<u8> {
    if text.is_empty() {
        None
    } else {
        text.parse::<u8>().ok()
    }
}

fn parse_f32_opt(text: &str) -> Option<f32> {
    if text.is_empty() {
        None
    } else {
        text.parse::<f32>().ok()
    }
}

fn parse_lat_lon(value: &str, hemi: &str, deg_digits: usize) -> Option<f64> {
    if value.len() < deg_digits + 2 || hemi.is_empty() {
        return None;
    }
    let deg = value.get(0..deg_digits)?.parse::<f64>().ok()?;
    let min = value.get(deg_digits..)?.parse::<f64>().ok()?;
    let mut out = deg + min / 60.0;
    match hemi.as_bytes()[0] {
        b'S' | b'W' => out = -out,
        b'N' | b'E' => {}
        _ => return None,
    }
    Some(out)
}

pub fn parse_gga(sentence: &NmeaSentence) -> Result<GpsFix> {
    let utc_time = field(sentence, 0, "gga time")?.to_string();
    let latitude = parse_lat_lon(field(sentence, 1, "gga lat")?, field(sentence, 2, "gga ns")?, 2);
    let longitude = parse_lat_lon(field(sentence, 3, "gga lon")?, field(sentence, 4, "gga ew")?, 3);
    let fix_quality = parse_u8_opt(field(sentence, 5, "gga quality")?);
    let satellites = parse_u8_opt(field(sentence, 6, "gga satellites")?);
    let altitude_m = parse_f32_opt(field(sentence, 8, "gga altitude")?);
    Ok(GpsFix {
        talker: sentence.talker.clone(),
        latitude,
        longitude,
        altitude_m,
        fix_quality,
        satellites,
        utc_time: Some(utc_time),
        ..GpsFix::default()
    })
}

pub fn parse_rmc(sentence: &NmeaSentence) -> Result<GpsFix> {
    let utc_time = field(sentence, 0, "rmc time")?.to_string();
    let status = field(sentence, 1, "rmc status")?;
    let latitude = if status == "A" {
        parse_lat_lon(field(sentence, 2, "rmc lat")?, field(sentence, 3, "rmc ns")?, 2)
    } else {
        None
    };
    let longitude = if status == "A" {
        parse_lat_lon(field(sentence, 4, "rmc lon")?, field(sentence, 5, "rmc ew")?, 3)
    } else {
        None
    };
    let speed_knots = parse_f32_opt(field(sentence, 6, "rmc speed")?);
    let course_degrees = parse_f32_opt(field(sentence, 7, "rmc course")?);
    let date = Some(field(sentence, 8, "rmc date")?.to_string());
    Ok(GpsFix {
        talker: sentence.talker.clone(),
        latitude,
        longitude,
        speed_knots,
        course_degrees,
        utc_time: Some(utc_time),
        date,
        ..GpsFix::default()
    })
}

pub fn parse_gll(sentence: &NmeaSentence) -> Result<GpsFix> {
    let latitude = parse_lat_lon(field(sentence, 0, "gll lat")?, field(sentence, 1, "gll ns")?, 2);
    let longitude = parse_lat_lon(field(sentence, 2, "gll lon")?, field(sentence, 3, "gll ew")?, 3);
    let utc_time = Some(field(sentence, 4, "gll time")?.to_string());
    Ok(GpsFix {
        talker: sentence.talker.clone(),
        latitude,
        longitude,
        utc_time,
        ..GpsFix::default()
    })
}

pub fn parse_vtg(sentence: &NmeaSentence) -> Result<GpsFix> {
    let course_degrees = parse_f32_opt(field(sentence, 0, "vtg course")?);
    let speed_knots = parse_f32_opt(field(sentence, 4, "vtg speed")?);
    Ok(GpsFix {
        talker: sentence.talker.clone(),
        course_degrees,
        speed_knots,
        ..GpsFix::default()
    })
}

pub fn parse_hdt(sentence: &NmeaSentence) -> Result<GpsFix> {
    let course_degrees = parse_f32_opt(field(sentence, 0, "hdt heading")?);
    Ok(GpsFix {
        talker: sentence.talker.clone(),
        course_degrees,
        ..GpsFix::default()
    })
}

pub fn parse_zda(sentence: &NmeaSentence) -> Result<GpsFix> {
    let utc_time = Some(field(sentence, 0, "zda time")?.to_string());
    let day = field(sentence, 1, "zda day")?;
    let month = field(sentence, 2, "zda month")?;
    let year = field(sentence, 3, "zda year")?;
    let date = if day.is_empty() || month.is_empty() || year.is_empty() {
        None
    } else {
        Some(format!("{year}-{month}-{day}"))
    };
    Ok(GpsFix {
        talker: sentence.talker.clone(),
        utc_time,
        date,
        ..GpsFix::default()
    })
}

pub fn sentence_name(sentence: &NmeaSentence) -> String {
    format!("{}{}", sentence.talker, sentence.formatter)
}
