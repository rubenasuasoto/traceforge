use crate::{EventRecord, Outcome, Severity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputFormat {
    Jsonl,
    Csv,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestIssue {
    pub row: usize,
    pub code: String,
    pub message: String,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestReport {
    pub events: Vec<EventRecord>,
    pub invalid: Vec<IngestIssue>,
    pub duplicate_ids: Vec<String>,
    pub total_rows: usize,
}

pub fn ingest_bytes(bytes: &[u8], format: InputFormat) -> IngestReport {
    let mut events = Vec::new();
    let mut invalid = Vec::new();
    let mut total_rows = 0;

    match format {
        InputFormat::Jsonl => {
            for (index, line) in String::from_utf8_lossy(bytes).lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                total_rows += 1;
                match serde_json::from_str::<EventRecord>(line) {
                    Ok(mut event) => {
                        event.ensure_id();
                        events.push(event);
                    }
                    Err(error) => invalid.push(IngestIssue {
                        row: index + 1,
                        code: "invalid_json".into(),
                        message: error.to_string(),
                        raw: line.chars().take(240).collect(),
                    }),
                }
            }
        }
        InputFormat::Csv => {
            let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(bytes);
            for (index, row) in reader.deserialize::<CsvEventRow>().enumerate() {
                total_rows += 1;
                match row {
                    Ok(row) => {
                        let mut event = match row.into_event() {
                            Ok(event) => event,
                            Err(message) => {
                                invalid.push(IngestIssue {
                                    row: index + 2,
                                    code: "invalid_attributes".into(),
                                    message,
                                    raw: String::new(),
                                });
                                continue;
                            }
                        };
                        event.ensure_id();
                        events.push(event);
                    }
                    Err(error) => invalid.push(IngestIssue {
                        row: index + 2,
                        code: "invalid_csv".into(),
                        message: error.to_string(),
                        raw: String::new(),
                    }),
                }
            }
        }
    }

    events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp).then(a.id.cmp(&b.id)));
    let mut counts = HashMap::<String, usize>::new();
    for event in &events {
        *counts.entry(event.id.clone()).or_default() += 1;
    }
    let duplicates: HashSet<_> = counts
        .into_iter()
        .filter_map(|(id, count)| (count > 1).then_some(id))
        .collect();

    IngestReport {
        duplicate_ids: duplicates.into_iter().collect(),
        events,
        invalid,
        total_rows,
    }
}

#[derive(Debug, Deserialize)]
struct CsvEventRow {
    #[serde(default)]
    id: String,
    timestamp: DateTime<Utc>,
    source: String,
    event_type: String,
    #[serde(default)]
    user: Option<String>,
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    source_ip: Option<String>,
    outcome: Outcome,
    severity: Severity,
    message: String,
    #[serde(default)]
    attributes_json: String,
}

impl CsvEventRow {
    fn into_event(self) -> Result<EventRecord, String> {
        let attributes: BTreeMap<String, String> = if self.attributes_json.trim().is_empty() {
            BTreeMap::new()
        } else {
            serde_json::from_str(&self.attributes_json).map_err(|error| error.to_string())?
        };
        Ok(EventRecord {
            id: self.id,
            timestamp: self.timestamp,
            source: self.source,
            event_type: self.event_type,
            user: self.user.filter(|value| !value.is_empty()),
            host: self.host.filter(|value| !value.is_empty()),
            source_ip: self.source_ip.filter(|value| !value.is_empty()),
            outcome: self.outcome,
            severity: self.severity,
            message: self.message,
            attributes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_rows_are_reported_not_silently_dropped() {
        let report = ingest_bytes(b"{not-json}\n", InputFormat::Jsonl);
        assert_eq!(report.total_rows, 1);
        assert_eq!(report.events.len(), 0);
        assert_eq!(report.invalid.len(), 1);
    }

    #[test]
    fn csv_contract_reads_attributes_json() {
        let csv = "id,timestamp,source,event_type,user,host,source_ip,outcome,severity,message,attributes_json\n,2026-01-01T00:00:00Z,windows,authentication,ana,ws-01,10.0.0.1,failure,high,Invalid password,\"{\"\"synthetic\"\":\"\"true\"\"}\"\n";
        let report = ingest_bytes(csv.as_bytes(), InputFormat::Csv);
        assert_eq!(report.events.len(), 1);
        assert_eq!(
            report.events[0].attributes.get("synthetic").unwrap(),
            "true"
        );
        assert!(report.events[0].id.starts_with("tf-"));
    }
}
