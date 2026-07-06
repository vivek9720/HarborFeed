use crate::bitcursor::{checksum64, fold_pair};
use crate::lifecycle::{self, LifecycleSignal};
use crate::model::{Feed, FeedEvent, ValidationFinding};
use crate::ports;
use crate::rules::{self, RuleInput};

pub fn audit_feed(feed: &Feed) -> Vec<ValidationFinding> {
    let mut findings = Vec::new();
    let mut positions = 0usize;
    let mut statics = 0usize;
    let mut cargo = 0usize;
    let mut tides = 0usize;
    let mut messages = 0usize;
    let mut digest = checksum64(&feed.digest_material());
    let mut max_draught_dm = 0u16;
    let mut port_depth_dm = 0u16;
    for event in &feed.events {
        match event {
            FeedEvent::VesselPosition(p) => {
                positions += 1;
                if !(p.latitude >= -90.0 && p.latitude <= 90.0 && p.longitude >= -180.0 && p.longitude <= 180.0) {
                    findings.push(finding(10, 9, p.mmsi.to_string(), "vessel position is outside valid WGS84 bounds", digest));
                }
                if let Some(speed) = p.speed_knots {
                    if speed > 80.0 {
                        findings.push(finding(11, 6, p.mmsi.to_string(), "AIS speed over ground is implausibly high", speed.to_bits() as u64));
                    }
                }
                if let Some(port) = ports::nearest_port(p.latitude, p.longitude) {
                    port_depth_dm = port.max_draft_dm;
                    digest = fold_pair(digest, checksum64(port.locode.as_bytes()));
                }
            }
            FeedEvent::VesselStatic(s) => {
                statics += 1;
                if let Some(draught) = s.draught_dm {
                    max_draught_dm = max_draught_dm.max(draught as u16);
                }
            }
            FeedEvent::CargoEvent(c) => {
                cargo += 1;
                if c.equipment_id.as_deref().unwrap_or("").len() < 4 {
                    findings.push(finding(20, 4, "cargo".to_string(), "container or equipment identifier is missing", digest));
                }
                if c.dangerous_goods && c.location.is_none() {
                    findings.push(finding(21, 7, "dangerous-goods".to_string(), "dangerous goods event has no location qualifier", digest));
                }
            }
            FeedEvent::PortCall(call) => {
                if let Some(code) = &call.port_locode {
                    if ports::by_locode(code).is_none() {
                        findings.push(finding(30, 3, code.clone(), "port LOCODE is not in the built-in operations profile table", digest));
                    }
                }
            }
            FeedEvent::TideReading(t) => {
                tides += 1;
                if let Some(residual) = t.residual_m {
                    if residual.abs() > 2.5 {
                        findings.push(finding(40, 5, t.station.clone(), "observed tide differs sharply from prediction", residual.to_bits() as u64));
                    }
                }
            }
            FeedEvent::EdifactMessage(m) => {
                messages += 1;
                if m.segment_count == 0 {
                    findings.push(finding(50, 2, m.reference.clone(), "EDIFACT message has no parsed segments", digest));
                }
            }
            FeedEvent::GpsFix(_) | FeedEvent::UnknownSentence(_) => {}
        }
    }
    if max_draught_dm > 0 && port_depth_dm > 0 && max_draught_dm > port_depth_dm {
        findings.push(finding(60, 8, "draught".to_string(), "vessel draught exceeds nearby port profile depth", max_draught_dm as u64));
    }
    let input = RuleInput {
        digest,
        event_count: feed.events.len() as u16,
        position_count: positions as u16,
        static_count: statics as u16,
        cargo_count: cargo as u16,
        tide_count: tides as u16,
        message_count: messages as u16,
        invalid_checksums: feed.invalid_nmea_checksums as u16,
        max_draught_dm,
        port_depth_dm,
    };
    for rule in rules::RULES {
        if findings.len() >= 256 {
            break;
        }
        if let Some(hit) = rules::evaluate(*rule, input) {
            findings.push(finding(hit.rule_id, hit.severity, hit.subject, hit.message, hit.evidence));
        }
    }
    let risk_score = findings.iter().fold(digest, |acc, f| fold_pair(acc, f.evidence ^ f.rule_id as u64));
    lifecycle::reconcile(LifecycleSignal {
        digest,
        event_count: feed.events.len(),
        vessel_count: positions + statics,
        cargo_count: cargo,
        tide_count: tides,
        invalid_checksums: feed.invalid_nmea_checksums,
        risk_score,
        port_depth_dm,
    });
    findings
}

fn finding(rule_id: u16, severity: u8, subject: String, message: &'static str, evidence: u64) -> ValidationFinding {
    ValidationFinding {
        rule_id,
        severity,
        subject,
        message,
        evidence,
    }
}
