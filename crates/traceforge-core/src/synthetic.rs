use crate::{EventRecord, Outcome, Severity};
use chrono::{Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scenario {
    Baseline,
    BruteForce,
    PasswordSpray,
    LateralMovement,
    Mixed,
}

#[derive(Debug, Clone)]
struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }
    fn pick<'a>(&mut self, values: &'a [&'a str]) -> &'a str {
        values[(self.next() as usize) % values.len()]
    }
}

pub fn generate_events(count: usize, seed: u64, scenario: Scenario) -> Vec<EventRecord> {
    let users = ["ana", "marcos", "lucia", "diego", "svc-backup", "ines"];
    let hosts = ["ws-014", "ws-022", "ws-031", "srv-files", "dc-01", "vpn-01"];
    let ips = [
        "10.20.4.17",
        "10.20.4.22",
        "10.20.8.9",
        "198.51.100.42",
        "203.0.113.77",
    ];
    let mut rng = XorShift64::new(seed);
    let base = Utc.with_ymd_and_hms(2026, 7, 30, 8, 0, 0).unwrap();
    let mut events = Vec::with_capacity(count);

    for index in 0..count {
        let timestamp = base + Duration::seconds(index as i64 * 13 + (rng.next() % 7) as i64);
        let user = rng.pick(&users).to_owned();
        let host = rng.pick(&hosts).to_owned();
        let ip = rng.pick(&ips).to_owned();
        let success = !rng.next().is_multiple_of(8);
        events.push(EventRecord {
            id: format!("evt-{seed:08x}-{index:08}"),
            timestamp,
            source: if index % 5 == 0 {
                "vpn"
            } else {
                "windows-security"
            }
            .into(),
            event_type: "authentication".into(),
            user: Some(user),
            host: Some(host),
            source_ip: Some(ip),
            outcome: if success {
                Outcome::Success
            } else {
                Outcome::Failure
            },
            severity: if success {
                Severity::Info
            } else {
                Severity::Low
            },
            message: if success {
                "Interactive sign-in accepted"
            } else {
                "Invalid password"
            }
            .into(),
            attributes: BTreeMap::from([("synthetic".into(), "true".into())]),
        });
    }

    match scenario {
        Scenario::Baseline => {}
        Scenario::BruteForce => inject_brute_force(&mut events, base, seed),
        Scenario::PasswordSpray => inject_spray(&mut events, base, seed),
        Scenario::LateralMovement => inject_lateral(&mut events, base, seed),
        Scenario::Mixed => {
            inject_brute_force(&mut events, base, seed);
            inject_spray(&mut events, base + Duration::minutes(25), seed);
            inject_lateral(&mut events, base + Duration::minutes(50), seed);
        }
    }
    events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp).then(a.id.cmp(&b.id)));
    events.truncate(count);
    events
}

fn anomaly_event(
    id: String,
    timestamp: chrono::DateTime<Utc>,
    user: &str,
    host: &str,
    ip: &str,
    outcome: Outcome,
    severity: Severity,
) -> EventRecord {
    EventRecord {
        id,
        timestamp,
        source: "windows-security".into(),
        event_type: "authentication".into(),
        user: Some(user.into()),
        host: Some(host.into()),
        source_ip: Some(ip.into()),
        outcome,
        severity,
        message: if outcome == Outcome::Success {
            "Sign-in accepted after failures"
        } else {
            "Invalid password"
        }
        .into(),
        attributes: BTreeMap::from([
            ("synthetic".into(), "true".into()),
            ("scenario_marker".into(), "true".into()),
        ]),
    }
}

fn inject_brute_force(events: &mut Vec<EventRecord>, start: chrono::DateTime<Utc>, seed: u64) {
    for attempt in 0..7 {
        events.push(anomaly_event(
            format!("scenario-{seed}-brute-{attempt}"),
            start + Duration::seconds(30 + attempt * 18),
            "ana",
            "ws-014",
            "198.51.100.42",
            Outcome::Failure,
            Severity::High,
        ));
    }
    events.push(anomaly_event(
        format!("scenario-{seed}-brute-success"),
        start + Duration::seconds(180),
        "ana",
        "ws-014",
        "198.51.100.42",
        Outcome::Success,
        Severity::Critical,
    ));
}

fn inject_spray(events: &mut Vec<EventRecord>, start: chrono::DateTime<Utc>, seed: u64) {
    for (attempt, user) in ["ana", "marcos", "lucia", "diego", "ines", "svc-backup"]
        .iter()
        .enumerate()
    {
        events.push(anomaly_event(
            format!("scenario-{seed}-spray-{attempt}"),
            start + Duration::seconds(attempt as i64 * 25),
            user,
            "vpn-01",
            "203.0.113.77",
            Outcome::Failure,
            Severity::High,
        ));
    }
}

fn inject_lateral(events: &mut Vec<EventRecord>, start: chrono::DateTime<Utc>, seed: u64) {
    for (step, host) in ["ws-022", "srv-files", "dc-01"].iter().enumerate() {
        events.push(anomaly_event(
            format!("scenario-{seed}-lateral-{step}"),
            start + Duration::seconds(step as i64 * 90),
            "svc-backup",
            host,
            "10.20.8.9",
            Outcome::Success,
            Severity::High,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic_and_ordered() {
        let a = generate_events(100, 77, Scenario::Mixed);
        let b = generate_events(100, 77, Scenario::Mixed);
        assert_eq!(a, b);
        assert!(
            a.windows(2)
                .all(|pair| pair[0].timestamp <= pair[1].timestamp)
        );
    }
}
