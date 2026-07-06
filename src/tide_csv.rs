use crate::error::{HarborError, Result};
use crate::model::TideReading;

pub fn parse_tide_csv(text: &str) -> Result<Vec<TideReading>> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines.next().ok_or(HarborError::CsvSyntax("header"))?;
    let columns: Vec<String> = split_csv_line(header).into_iter().map(|s| s.to_ascii_lowercase()).collect();
    let station_idx = find_column(&columns, &["station", "station_id", "station id"]).unwrap_or(0);
    let time_idx = find_column(&columns, &["time", "timestamp", "date time", "datetime"]).unwrap_or(1);
    let observed_idx = find_column(&columns, &["water_level", "observed", "verified", "prediction"]);
    let predicted_idx = find_column(&columns, &["predicted", "predicted_level", "prediction"]);
    let quality_idx = find_column(&columns, &["quality", "qc", "sigma"]);
    let mut readings = Vec::new();
    for line in lines.take(4096) {
        let fields = split_csv_line(line);
        if fields.len() <= station_idx || fields.len() <= time_idx {
            continue;
        }
        let observed = observed_idx.and_then(|idx| fields.get(idx)).and_then(|v| parse_float(v));
        let predicted = predicted_idx.and_then(|idx| fields.get(idx)).and_then(|v| parse_float(v));
        let residual = match (observed, predicted) {
            (Some(o), Some(p)) => Some(o - p),
            _ => None,
        };
        let quality = quality_idx.and_then(|idx| fields.get(idx)).filter(|v| !v.is_empty()).cloned();
        readings.push(TideReading {
            station: fields[station_idx].clone(),
            timestamp: fields[time_idx].clone(),
            observed_m: observed,
            predicted_m: predicted,
            residual_m: residual,
            quality,
        });
    }
    Ok(readings)
}

fn split_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if quoted && chars.peek() == Some(&'"') {
                    current.push('"');
                    let _ = chars.next();
                } else {
                    quoted = !quoted;
                }
            }
            ',' if !quoted => {
                fields.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    fields.push(current.trim().to_string());
    fields
}

fn find_column(columns: &[String], names: &[&str]) -> Option<usize> {
    columns.iter().position(|column| names.iter().any(|name| column == name))
}

fn parse_float(text: &str) -> Option<f32> {
    let clean = text.trim();
    if clean.is_empty() {
        None
    } else {
        clean.parse::<f32>().ok()
    }
}
