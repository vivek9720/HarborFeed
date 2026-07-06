use crate::error::{HarborError, Result};
use crate::model::{CargoEvent, EdifactSummary, PortCall};

#[derive(Debug, Clone)]
pub struct Separators {
    pub component: char,
    pub element: char,
    pub decimal: char,
    pub release: char,
    pub repetition: char,
    pub segment: char,
}

impl Default for Separators {
    fn default() -> Self {
        Self { component: ':', element: '+', decimal: '.', release: '?', repetition: '*', segment: '\'' }
    }
}

#[derive(Debug, Clone)]
pub struct Segment {
    pub tag: String,
    pub elements: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct EdifactMessage {
    pub reference: String,
    pub message_type: String,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone)]
pub struct EdifactInterchange {
    pub syntax_identifier: Option<String>,
    pub sender: Option<String>,
    pub recipient: Option<String>,
    pub control_ref: Option<String>,
    pub messages: Vec<EdifactMessage>,
    pub separators: Separators,
}

pub fn parse_interchange(text: &str) -> Result<EdifactInterchange> {
    let (separators, body) = parse_una(text);
    let raw_segments = split_segments(body, &separators);
    if raw_segments.len() > 4096 {
        return Err(HarborError::LimitExceeded("edifact segments"));
    }
    let segments: Vec<Segment> = raw_segments
        .iter()
        .filter_map(|s| parse_segment(s, &separators).ok())
        .collect();
    let mut syntax_identifier = None;
    let mut sender = None;
    let mut recipient = None;
    let mut control_ref = None;
    let mut messages = Vec::new();
    let mut current: Option<EdifactMessage> = None;
    for segment in segments {
        match segment.tag.as_str() {
            "UNB" => {
                syntax_identifier = segment.elements.get(0).and_then(|c| c.get(0)).cloned();
                sender = segment.elements.get(1).and_then(|c| c.get(0)).cloned();
                recipient = segment.elements.get(2).and_then(|c| c.get(0)).cloned();
                control_ref = segment.elements.get(4).and_then(|c| c.get(0)).cloned();
            }
            "UNH" => {
                if let Some(message) = current.take() {
                    messages.push(message);
                }
                let reference = segment.elements.get(0).and_then(|c| c.get(0)).cloned().unwrap_or_default();
                let message_type = segment
                    .elements
                    .get(1)
                    .and_then(|c| c.get(0))
                    .cloned()
                    .unwrap_or_else(|| "UNKNOWN".to_string());
                current = Some(EdifactMessage { reference, message_type, segments: vec![segment] });
            }
            "UNT" => {
                if let Some(mut message) = current.take() {
                    message.segments.push(segment);
                    messages.push(message);
                }
            }
            "UNZ" => {}
            _ => {
                if let Some(message) = current.as_mut() {
                    message.segments.push(segment);
                }
            }
        }
    }
    if let Some(message) = current.take() {
        messages.push(message);
    }
    Ok(EdifactInterchange {
        syntax_identifier,
        sender,
        recipient,
        control_ref,
        messages,
        separators,
    })
}

fn parse_una(text: &str) -> (Separators, &str) {
    if text.starts_with("UNA") && text.len() >= 9 {
        let mut chars = text[3..9].chars();
        let separators = Separators {
            component: chars.next().unwrap_or(':'),
            element: chars.next().unwrap_or('+'),
            decimal: chars.next().unwrap_or('.'),
            release: chars.next().unwrap_or('?'),
            repetition: chars.next().unwrap_or('*'),
            segment: chars.next().unwrap_or('\''),
        };
        (separators, &text[9..])
    } else {
        (Separators::default(), text)
    }
}

fn split_segments(text: &str, separators: &Separators) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut released = false;
    for ch in text.chars() {
        if released {
            current.push(ch);
            released = false;
            continue;
        }
        if ch == separators.release {
            released = true;
            continue;
        }
        if ch == separators.segment {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
            current.clear();
        } else {
            current.push(ch);
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        out.push(trimmed.to_string());
    }
    out
}

fn parse_segment(text: &str, separators: &Separators) -> Result<Segment> {
    let mut parts = split_unreleased(text, separators.element, separators.release);
    let tag = parts.next().ok_or(HarborError::EdifactSyntax("segment tag"))?.to_string();
    if tag.is_empty() || tag.len() > 3 && !tag.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(HarborError::EdifactSyntax("segment tag"));
    }
    let mut elements = Vec::new();
    for part in parts {
        let composite = split_unreleased(&part, separators.component, separators.release).collect();
        elements.push(composite);
    }
    Ok(Segment { tag, elements })
}

fn split_unreleased(text: &str, delim: char, release: char) -> impl Iterator<Item = String> + '_ {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut released = false;
    for ch in text.chars() {
        if released {
            current.push(ch);
            released = false;
        } else if ch == release {
            released = true;
        } else if ch == delim {
            parts.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    parts.push(current);
    parts.into_iter()
}

pub fn summarize_message(message: &EdifactMessage) -> EdifactSummary {
    let container_count = message.segments.iter().filter(|s| s.tag == "EQD").count();
    let dangerous_goods_count = message.segments.iter().filter(|s| s.tag == "DGS").count();
    EdifactSummary {
        message_type: message.message_type.clone(),
        reference: message.reference.clone(),
        segment_count: message.segments.len(),
        container_count,
        dangerous_goods_count,
    }
}

pub fn extract_cargo_events(message: &EdifactMessage) -> Vec<CargoEvent> {
    let mut events = Vec::new();
    let mut current: Option<CargoEvent> = None;
    for segment in &message.segments {
        match segment.tag.as_str() {
            "EQD" => {
                if let Some(event) = current.take() {
                    events.push(event);
                }
                let equipment_type = segment.elements.get(0).and_then(|c| c.get(0)).cloned();
                let equipment_id = segment.elements.get(1).and_then(|c| c.get(0)).cloned();
                current = Some(CargoEvent {
                    message_ref: Some(message.reference.clone()),
                    equipment_id,
                    equipment_type,
                    ..CargoEvent::default()
                });
            }
            "RFF" => {
                if let Some(event) = current.as_mut() {
                    event.booking_ref = segment.elements.get(0).and_then(|c| c.get(1)).cloned();
                }
            }
            "MEA" => {
                if let Some(event) = current.as_mut() {
                    event.gross_mass_kg = segment
                        .elements
                        .get(2)
                        .and_then(|c| c.get(1))
                        .and_then(|v| v.parse::<f32>().ok());
                }
            }
            "LOC" => {
                if let Some(event) = current.as_mut() {
                    event.location = segment.elements.get(1).and_then(|c| c.get(0)).cloned();
                    event.event_code = segment.elements.get(0).and_then(|c| c.get(0)).cloned();
                }
            }
            "DGS" => {
                if let Some(event) = current.as_mut() {
                    event.dangerous_goods = true;
                }
            }
            _ => {}
        }
    }
    if let Some(event) = current.take() {
        events.push(event);
    }
    events
}

pub fn extract_port_call(message: &EdifactMessage) -> Option<PortCall> {
    let mut call = PortCall::default();
    for segment in &message.segments {
        match segment.tag.as_str() {
            "TDT" => {
                call.voyage = segment.elements.get(1).and_then(|c| c.get(0)).cloned();
                call.vessel_id = segment.elements.get(7).and_then(|c| c.get(0)).cloned();
            }
            "LOC" => {
                let qualifier = segment.elements.get(0).and_then(|c| c.get(0)).map(String::as_str);
                match qualifier {
                    Some("9") | Some("11") | Some("153") => {
                        call.port_locode = segment.elements.get(1).and_then(|c| c.get(0)).cloned();
                    }
                    Some("60") | Some("61") => {
                        call.berth = segment.elements.get(1).and_then(|c| c.get(0)).cloned();
                    }
                    _ => {}
                }
            }
            "DTM" => {
                let qualifier = segment.elements.get(0).and_then(|c| c.get(0)).map(String::as_str);
                match qualifier {
                    Some("132") | Some("178") => call.eta = segment.elements.get(0).and_then(|c| c.get(1)).cloned(),
                    Some("133") | Some("186") => call.etd = segment.elements.get(0).and_then(|c| c.get(1)).cloned(),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    if call.port_locode.is_some() || call.vessel_id.is_some() || call.voyage.is_some() {
        Some(call)
    } else {
        None
    }
}
