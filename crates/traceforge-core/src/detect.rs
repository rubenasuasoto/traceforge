use crate::{EventRecord, Outcome, Severity};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DetectionKind {
    BruteForce,
    PasswordSpray,
    FailureThenSuccess,
    LateralMovement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    pub id: String,
    pub kind: DetectionKind,
    pub severity: Severity,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub ended_at: chrono::DateTime<chrono::Utc>,
    pub entities: Vec<String>,
    pub event_ids: Vec<String>,
    pub explanation: String,
    pub evidence: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorConfig {
    pub brute_force_failures: usize,
    pub spray_accounts: usize,
    pub window_seconds: i64,
    pub lateral_hosts: usize,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            brute_force_failures: 5,
            spray_accounts: 5,
            window_seconds: 300,
            lateral_hosts: 3,
        }
    }
}

pub fn detect_all(events: &[EventRecord], config: &DetectorConfig) -> Vec<Detection> {
    let mut ordered: Vec<_> = events.iter().collect();
    ordered.sort_by_key(|event| (event.timestamp, &event.id));
    let mut detections = Vec::new();
    detections.extend(detect_brute_force(&ordered, config));
    detections.extend(detect_spray(&ordered, config));
    detections.extend(detect_failure_success(&ordered, config));
    detections.extend(detect_lateral(&ordered, config));
    detections.sort_by_key(|item| (item.started_at, item.id.clone()));
    detections
}

fn detect_brute_force(events: &[&EventRecord], config: &DetectorConfig) -> Vec<Detection> {
    let mut windows: HashMap<String, VecDeque<&EventRecord>> = HashMap::new();
    let mut detected = HashSet::new();
    let mut results = Vec::new();
    for event in events
        .iter()
        .copied()
        .filter(|event| event.outcome == Outcome::Failure)
    {
        let Some(user) = event.user.as_ref() else {
            continue;
        };
        let window = windows.entry(user.clone()).or_default();
        evict(window, event, config.window_seconds);
        window.push_back(event);
        if window.len() >= config.brute_force_failures && detected.insert(user.clone()) {
            results.push(Detection {
                id: format!("brute-force-{}-{}", user, event.timestamp.timestamp()),
                kind: DetectionKind::BruteForce,
                severity: Severity::High,
                started_at: window.front().unwrap().timestamp,
                ended_at: event.timestamp,
                entities: vec![format!("user:{user}")],
                event_ids: window.iter().map(|item| item.id.clone()).collect(),
                explanation: format!(
                    "{} failed authentications for one account inside a {} second sliding window.",
                    window.len(),
                    config.window_seconds
                ),
                evidence: BTreeMap::from([
                    ("failures".into(), window.len().to_string()),
                    ("window_seconds".into(), config.window_seconds.to_string()),
                ]),
            });
        }
    }
    results
}

fn detect_spray(events: &[&EventRecord], config: &DetectorConfig) -> Vec<Detection> {
    let mut windows: HashMap<String, VecDeque<&EventRecord>> = HashMap::new();
    let mut detected = HashSet::new();
    let mut results = Vec::new();
    for event in events
        .iter()
        .copied()
        .filter(|event| event.outcome == Outcome::Failure)
    {
        let Some(ip) = event.source_ip.as_ref() else {
            continue;
        };
        let window = windows.entry(ip.clone()).or_default();
        evict(window, event, config.window_seconds);
        window.push_back(event);
        let users: HashSet<_> = window
            .iter()
            .filter_map(|item| item.user.as_ref())
            .collect();
        if users.len() >= config.spray_accounts && detected.insert(ip.clone()) {
            results.push(Detection {
                id: format!("password-spray-{}-{}", ip.replace('.', "-"), event.timestamp.timestamp()),
                kind: DetectionKind::PasswordSpray,
                severity: Severity::High,
                started_at: window.front().unwrap().timestamp,
                ended_at: event.timestamp,
                entities: std::iter::once(format!("ip:{ip}"))
                    .chain(users.iter().map(|user| format!("user:{user}")))
                    .collect(),
                event_ids: window.iter().map(|item| item.id.clone()).collect(),
                explanation: format!("One source IP failed against {} distinct accounts inside a {} second sliding window.", users.len(), config.window_seconds),
                evidence: BTreeMap::from([
                    ("distinct_accounts".into(), users.len().to_string()),
                    ("window_seconds".into(), config.window_seconds.to_string()),
                ]),
            });
        }
    }
    results
}

fn detect_failure_success(events: &[&EventRecord], config: &DetectorConfig) -> Vec<Detection> {
    let mut failures: HashMap<String, VecDeque<&EventRecord>> = HashMap::new();
    let mut results = Vec::new();
    for event in events.iter().copied() {
        let (Some(user), Some(ip)) = (event.user.as_ref(), event.source_ip.as_ref()) else {
            continue;
        };
        let key = format!("{user}|{ip}");
        let window = failures.entry(key).or_default();
        evict(window, event, config.window_seconds * 2);
        match event.outcome {
            Outcome::Failure => window.push_back(event),
            Outcome::Success if window.len() >= config.brute_force_failures => {
                let started_at = window.front().unwrap().timestamp;
                let mut event_ids: Vec<_> = window.iter().map(|item| item.id.clone()).collect();
                event_ids.push(event.id.clone());
                results.push(Detection {
                    id: format!("failure-success-{}-{}", user, event.timestamp.timestamp()),
                    kind: DetectionKind::FailureThenSuccess,
                    severity: Severity::Critical,
                    started_at,
                    ended_at: event.timestamp,
                    entities: vec![format!("user:{user}"), format!("ip:{ip}")],
                    event_ids,
                    explanation: format!("A successful authentication followed {} recent failures for the same identity.", window.len()),
                    evidence: BTreeMap::from([("preceding_failures".into(), window.len().to_string())]),
                });
                window.clear();
            }
            _ => {}
        }
    }
    results
}

fn detect_lateral(events: &[&EventRecord], config: &DetectorConfig) -> Vec<Detection> {
    let mut windows: HashMap<String, VecDeque<&EventRecord>> = HashMap::new();
    let mut detected = HashSet::new();
    let mut results = Vec::new();
    for event in events
        .iter()
        .copied()
        .filter(|event| event.outcome == Outcome::Success && event.severity >= Severity::High)
    {
        let (Some(user), Some(_host)) = (event.user.as_ref(), event.host.as_ref()) else {
            continue;
        };
        let window = windows.entry(user.clone()).or_default();
        evict(window, event, config.window_seconds * 3);
        window.push_back(event);
        let hosts: HashSet<_> = window
            .iter()
            .filter_map(|item| item.host.as_ref())
            .collect();
        if hosts.len() >= config.lateral_hosts && detected.insert(user.clone()) {
            results.push(Detection {
                id: format!("lateral-{}-{}", user, event.timestamp.timestamp()),
                kind: DetectionKind::LateralMovement,
                severity: Severity::High,
                started_at: window.front().unwrap().timestamp,
                ended_at: event.timestamp,
                entities: std::iter::once(format!("user:{user}"))
                    .chain(hosts.iter().map(|host| format!("host:{host}")))
                    .collect(),
                event_ids: window.iter().map(|item| item.id.clone()).collect(),
                explanation: format!("One identity authenticated successfully on {} distinct hosts inside the correlation window.", hosts.len()),
                evidence: BTreeMap::from([("distinct_hosts".into(), hosts.len().to_string())]),
            });
        }
    }
    results
}

fn evict(window: &mut VecDeque<&EventRecord>, current: &EventRecord, seconds: i64) {
    let cutoff = current.timestamp - Duration::seconds(seconds);
    while window.front().is_some_and(|event| event.timestamp < cutoff) {
        window.pop_front();
    }
}

/// Returns the most frequent entity values using a fixed-size binary heap.
pub fn top_entities(events: &[EventRecord], field: &str, limit: usize) -> Vec<(String, usize)> {
    let mut counts = HashMap::<String, usize>::new();
    for event in events {
        if let Some(value) = event.field(field) {
            *counts.entry(value).or_default() += 1;
        }
    }
    let mut heap = BinaryHeap::<Reverse<(usize, String)>>::new();
    for (value, count) in counts {
        heap.push(Reverse((count, value)));
        if heap.len() > limit {
            heap.pop();
        }
    }
    let mut result: Vec<_> = heap
        .into_iter()
        .map(|Reverse((count, value))| (value, count))
        .collect();
    result.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::{Scenario, generate_events};

    #[test]
    fn mixed_scenario_triggers_all_explainable_rules() {
        let events = generate_events(800, 42, Scenario::Mixed);
        let detections = detect_all(&events, &DetectorConfig::default());
        for kind in [
            DetectionKind::BruteForce,
            DetectionKind::PasswordSpray,
            DetectionKind::FailureThenSuccess,
            DetectionKind::LateralMovement,
        ] {
            assert!(
                detections.iter().any(|item| item.kind == kind),
                "missing {kind:?}"
            );
        }
    }
}
