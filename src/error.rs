use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarborError {
    EmptyInput,
    InvalidUtf8,
    InvalidNmeaStart,
    MissingNmeaChecksum,
    InvalidNmeaChecksum,
    InvalidNmeaField(&'static str),
    UnsupportedSentence(String),
    InvalidAisPayload,
    UnsupportedAisType(u8),
    EdifactSyntax(&'static str),
    CsvSyntax(&'static str),
    LimitExceeded(&'static str),
}

pub type Result<T> = core::result::Result<T, HarborError>;

impl fmt::Display for HarborError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HarborError::EmptyInput => f.write_str("empty input"),
            HarborError::InvalidUtf8 => f.write_str("input is not valid UTF-8"),
            HarborError::InvalidNmeaStart => f.write_str("NMEA sentence must start with $ or !"),
            HarborError::MissingNmeaChecksum => f.write_str("NMEA sentence is missing a checksum"),
            HarborError::InvalidNmeaChecksum => f.write_str("NMEA checksum mismatch"),
            HarborError::InvalidNmeaField(name) => write!(f, "invalid NMEA field {name}"),
            HarborError::UnsupportedSentence(name) => write!(f, "unsupported sentence {name}"),
            HarborError::InvalidAisPayload => f.write_str("invalid AIS six-bit payload"),
            HarborError::UnsupportedAisType(kind) => write!(f, "unsupported AIS message type {kind}"),
            HarborError::EdifactSyntax(name) => write!(f, "EDIFACT syntax error in {name}"),
            HarborError::CsvSyntax(name) => write!(f, "CSV syntax error in {name}"),
            HarborError::LimitExceeded(name) => write!(f, "limit exceeded while parsing {name}"),
        }
    }
}

impl std::error::Error for HarborError {}
