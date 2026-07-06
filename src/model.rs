#[derive(Debug, Clone, Default)]
pub struct Feed {
    pub events: Vec<FeedEvent>,
    pub invalid_nmea_checksums: usize,
    pub parse_warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum FeedEvent {
    VesselPosition(VesselPosition),
    VesselStatic(VesselStatic),
    GpsFix(GpsFix),
    CargoEvent(CargoEvent),
    PortCall(PortCall),
    TideReading(TideReading),
    EdifactMessage(EdifactSummary),
    UnknownSentence(String),
}

#[derive(Debug, Clone, Default)]
pub struct VesselPosition {
    pub mmsi: u32,
    pub latitude: f64,
    pub longitude: f64,
    pub speed_knots: Option<f32>,
    pub course_degrees: Option<f32>,
    pub heading_degrees: Option<u16>,
    pub nav_status: Option<u8>,
    pub timestamp_second: Option<u8>,
    pub source: &'static str,
}

#[derive(Debug, Clone, Default)]
pub struct VesselStatic {
    pub mmsi: u32,
    pub imo: Option<u32>,
    pub callsign: Option<String>,
    pub name: Option<String>,
    pub ship_type: Option<u8>,
    pub draught_dm: Option<u8>,
    pub destination: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct GpsFix {
    pub talker: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub speed_knots: Option<f32>,
    pub course_degrees: Option<f32>,
    pub altitude_m: Option<f32>,
    pub fix_quality: Option<u8>,
    pub satellites: Option<u8>,
    pub utc_time: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CargoEvent {
    pub message_ref: Option<String>,
    pub equipment_id: Option<String>,
    pub equipment_type: Option<String>,
    pub booking_ref: Option<String>,
    pub gross_mass_kg: Option<f32>,
    pub location: Option<String>,
    pub event_code: Option<String>,
    pub dangerous_goods: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PortCall {
    pub vessel_id: Option<String>,
    pub voyage: Option<String>,
    pub port_locode: Option<String>,
    pub berth: Option<String>,
    pub eta: Option<String>,
    pub etd: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TideReading {
    pub station: String,
    pub timestamp: String,
    pub observed_m: Option<f32>,
    pub predicted_m: Option<f32>,
    pub residual_m: Option<f32>,
    pub quality: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EdifactSummary {
    pub message_type: String,
    pub reference: String,
    pub segment_count: usize,
    pub container_count: usize,
    pub dangerous_goods_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ValidationFinding {
    pub rule_id: u16,
    pub severity: u8,
    pub subject: String,
    pub message: &'static str,
    pub evidence: u64,
}

impl Feed {
    pub fn push(&mut self, event: FeedEvent) {
        self.events.push(event);
    }

    pub fn digest_material(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for event in &self.events {
            match event {
                FeedEvent::VesselPosition(p) => {
                    out.extend_from_slice(&p.mmsi.to_le_bytes());
                    out.extend_from_slice(&p.latitude.to_bits().to_le_bytes());
                    out.extend_from_slice(&p.longitude.to_bits().to_le_bytes());
                }
                FeedEvent::VesselStatic(s) => {
                    out.extend_from_slice(&s.mmsi.to_le_bytes());
                    if let Some(name) = &s.name {
                        out.extend_from_slice(name.as_bytes());
                    }
                }
                FeedEvent::GpsFix(g) => {
                    out.extend_from_slice(g.talker.as_bytes());
                    if let Some(lat) = g.latitude {
                        out.extend_from_slice(&lat.to_bits().to_le_bytes());
                    }
                    if let Some(lon) = g.longitude {
                        out.extend_from_slice(&lon.to_bits().to_le_bytes());
                    }
                }
                FeedEvent::CargoEvent(c) => {
                    if let Some(id) = &c.equipment_id {
                        out.extend_from_slice(id.as_bytes());
                    }
                    if let Some(loc) = &c.location {
                        out.extend_from_slice(loc.as_bytes());
                    }
                }
                FeedEvent::PortCall(p) => {
                    if let Some(port) = &p.port_locode {
                        out.extend_from_slice(port.as_bytes());
                    }
                }
                FeedEvent::TideReading(t) => {
                    out.extend_from_slice(t.station.as_bytes());
                    out.extend_from_slice(t.timestamp.as_bytes());
                }
                FeedEvent::EdifactMessage(m) => {
                    out.extend_from_slice(m.message_type.as_bytes());
                    out.extend_from_slice(m.reference.as_bytes());
                    out.extend_from_slice(&(m.segment_count as u64).to_le_bytes());
                }
                FeedEvent::UnknownSentence(s) => out.extend_from_slice(s.as_bytes()),
            }
        }
        out
    }
}
