use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn score(self) -> u32 {
        match self {
            Self::Info => 1,
            Self::Low => 3,
            Self::Medium => 5,
            Self::High => 8,
            Self::Critical => 10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Success,
    Failure,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventRecord {
    #[serde(default)]
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub event_type: String,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub source_ip: Option<String>,
    pub outcome: Outcome,
    pub severity: Severity,
    pub message: String,
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

impl EventRecord {
    pub fn ensure_id(&mut self) {
        if self.id.trim().is_empty() {
            self.id = self.fingerprint();
        }
    }

    pub fn fingerprint(&self) -> String {
        let mut hasher = Sha256::new();
        for value in [
            self.timestamp.to_rfc3339(),
            self.source.clone(),
            self.event_type.clone(),
            self.user.clone().unwrap_or_default(),
            self.host.clone().unwrap_or_default(),
            self.source_ip.clone().unwrap_or_default(),
            self.message.clone(),
        ] {
            hasher.update(value.as_bytes());
            hasher.update([0]);
        }
        format!("tf-{}", &format!("{:x}", hasher.finalize())[..20])
    }

    pub fn field(&self, field: &str) -> Option<String> {
        match field.to_ascii_lowercase().as_str() {
            "id" => Some(self.id.clone()),
            "source" => Some(self.source.clone()),
            "type" | "event_type" => Some(self.event_type.clone()),
            "user" => self.user.clone(),
            "host" => self.host.clone(),
            "ip" | "source_ip" => self.source_ip.clone(),
            "outcome" | "result" => Some(format!("{:?}", self.outcome).to_ascii_lowercase()),
            "severity" => Some(format!("{:?}", self.severity).to_ascii_lowercase()),
            "message" => Some(self.message.clone()),
            other => self.attributes.get(other).cloned(),
        }
    }
}
