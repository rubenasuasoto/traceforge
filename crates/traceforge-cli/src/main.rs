use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Instant;
use traceforge_core::{
    DetectorConfig, EntityGraph, INDEX_FORMAT, INDEX_VERSION, IndexSnapshot, InputFormat, PathMode,
    Scenario, SearchIndex, detect_all, generate_events, ingest_bytes, parse_query,
};

#[derive(Debug, Parser)]
#[command(
    name = "traceforge",
    version,
    about = "Local-first log search and correlation engine"
)]
struct App {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate deterministic synthetic security events.
    Generate {
        #[arg(short, long, default_value_t = 10_000)]
        count: usize,
        #[arg(short, long, default_value_t = 42)]
        seed: u64,
        #[arg(short, long, value_enum, default_value_t = ScenarioArg::Mixed)]
        scenario: ScenarioArg,
        #[arg(short, long, value_enum, default_value_t = FormatArg::Jsonl)]
        format: FormatArg,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Validate events and persist a reusable versioned index payload.
    BuildIndex {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, value_enum)]
        format: Option<FormatArg>,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Search a persisted dataset using TraceForge's query language.
    Query {
        #[arg(short, long)]
        index: PathBuf,
        #[arg(short, long)]
        query: String,
        #[arg(short, long, default_value_t = 100)]
        limit: usize,
    },
    /// Run deterministic, explainable detection rules.
    Detect {
        #[arg(short, long)]
        index: PathBuf,
    },
    /// Find an entity path with BFS or risk-weighted Dijkstra.
    Path {
        #[arg(short, long)]
        index: PathBuf,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long, value_enum, default_value_t = PathArg::Hops)]
        mode: PathArg,
    },
    /// Compare an indexed query with a linear scan under a fixed seed.
    Benchmark {
        #[arg(short, long, default_value_t = 100_000)]
        size: usize,
        #[arg(short, long, default_value_t = 42)]
        seed: u64,
        #[arg(short, long, default_value_t = 25)]
        iterations: usize,
        #[arg(short, long, default_value = "outcome:failure AND user:ana")]
        query: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ScenarioArg {
    Baseline,
    BruteForce,
    PasswordSpray,
    LateralMovement,
    Mixed,
}
#[derive(Debug, Clone, Copy, ValueEnum)]
enum FormatArg {
    Jsonl,
    Csv,
}
#[derive(Debug, Clone, Copy, ValueEnum)]
enum PathArg {
    Hops,
    Risk,
}

fn main() -> Result<()> {
    match App::parse().command {
        Command::Generate {
            count,
            seed,
            scenario,
            format,
            output,
        } => {
            let events = generate_events(count, seed, scenario.into());
            write_events(&output, &events, format)?;
            println!(
                "generated={} seed={} output={}",
                events.len(),
                seed,
                output.display()
            );
        }
        Command::BuildIndex {
            input,
            format,
            output,
        } => {
            let bytes =
                fs::read(&input).with_context(|| format!("cannot read {}", input.display()))?;
            let format = format.unwrap_or_else(|| infer_format(&input));
            let report = ingest_bytes(&bytes, format.into());
            if !report.invalid.is_empty() {
                eprintln!("invalid_rows={}", report.invalid.len());
                for issue in report.invalid.iter().take(10) {
                    eprintln!("row={} code={} {}", issue.row, issue.code, issue.message);
                }
            }
            let serialized = serde_json::to_vec(&report.events)?;
            let snapshot = IndexSnapshot {
                format: INDEX_FORMAT.into(),
                version: INDEX_VERSION,
                created_at: chrono::Utc::now().to_rfc3339(),
                sha256: format!("{:x}", Sha256::digest(&serialized)),
                events: report.events,
            };
            fs::write(&output, serde_json::to_vec(&snapshot)?)?;
            println!(
                "indexed={} invalid={} duplicates={} output={}",
                snapshot.events.len(),
                report.invalid.len(),
                report.duplicate_ids.len(),
                output.display()
            );
        }
        Command::Query {
            index,
            query,
            limit,
        } => {
            let events = load_snapshot(&index)?;
            let result = SearchIndex::build(events).query(&query, limit)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Detect { index } => {
            let events = load_snapshot(&index)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&detect_all(&events, &DetectorConfig::default()))?
            );
        }
        Command::Path {
            index,
            from,
            to,
            mode,
        } => {
            let events = load_snapshot(&index)?;
            let result = EntityGraph::build(&events).path(&from, &to, mode.into());
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Benchmark {
            size,
            seed,
            iterations,
            query,
            output,
        } => benchmark(size, seed, iterations, &query, output.as_deref())?,
    }
    Ok(())
}

fn load_snapshot(path: &Path) -> Result<Vec<traceforge_core::EventRecord>> {
    let snapshot: IndexSnapshot = serde_json::from_slice(&fs::read(path)?)?;
    if snapshot.format != INDEX_FORMAT || snapshot.version != INDEX_VERSION {
        bail!("unsupported index format or version");
    }
    let actual = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&snapshot.events)?)
    );
    if actual != snapshot.sha256 {
        bail!("index checksum mismatch");
    }
    Ok(snapshot.events)
}

fn write_events(
    path: &Path,
    events: &[traceforge_core::EventRecord],
    format: FormatArg,
) -> Result<()> {
    match format {
        FormatArg::Jsonl => {
            let mut output = String::new();
            for event in events {
                output.push_str(&serde_json::to_string(event)?);
                output.push('\n');
            }
            fs::write(path, output)?;
        }
        FormatArg::Csv => {
            let mut writer = csv::Writer::from_path(path)?;
            for event in events {
                writer.serialize(CsvEventRow::from(event))?;
            }
            writer.flush()?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct CsvEventRow<'a> {
    id: &'a str,
    timestamp: String,
    source: &'a str,
    event_type: &'a str,
    user: Option<&'a str>,
    host: Option<&'a str>,
    source_ip: Option<&'a str>,
    outcome: traceforge_core::Outcome,
    severity: traceforge_core::Severity,
    message: &'a str,
    attributes_json: String,
}

impl<'a> From<&'a traceforge_core::EventRecord> for CsvEventRow<'a> {
    fn from(event: &'a traceforge_core::EventRecord) -> Self {
        Self {
            id: &event.id,
            timestamp: event.timestamp.to_rfc3339(),
            source: &event.source,
            event_type: &event.event_type,
            user: event.user.as_deref(),
            host: event.host.as_deref(),
            source_ip: event.source_ip.as_deref(),
            outcome: event.outcome,
            severity: event.severity,
            message: &event.message,
            attributes_json: serde_json::to_string(&event.attributes)
                .unwrap_or_else(|_| "{}".into()),
        }
    }
}

fn infer_format(path: &Path) -> FormatArg {
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
    {
        FormatArg::Csv
    } else {
        FormatArg::Jsonl
    }
}

fn benchmark(
    size: usize,
    seed: u64,
    iterations: usize,
    query: &str,
    output: Option<&Path>,
) -> Result<()> {
    let events = generate_events(size, seed, Scenario::Mixed);
    let build_started = Instant::now();
    let index = SearchIndex::build(events);
    let build_ns = build_started.elapsed().as_nanos();
    let expr = parse_query(query)?;

    let indexed_started = Instant::now();
    let mut indexed_matches = 0;
    for _ in 0..iterations {
        indexed_matches = black_box(index.query(query, usize::MAX)?.matches.len());
    }
    let indexed_ns = indexed_started.elapsed().as_nanos();

    let linear_started = Instant::now();
    let mut linear_matches = 0;
    for _ in 0..iterations {
        linear_matches = black_box(index.linear_scan(&expr).len());
    }
    let linear_ns = linear_started.elapsed().as_nanos();
    if indexed_matches != linear_matches {
        bail!("indexed and linear results differ");
    }

    let report = serde_json::json!({
        "events": size, "seed": seed, "iterations": iterations, "query": query,
        "matches": indexed_matches, "build_ns": build_ns,
        "indexed_total_ns": indexed_ns, "linear_total_ns": linear_ns,
        "indexed_mean_ns": indexed_ns / iterations.max(1) as u128,
        "linear_mean_ns": linear_ns / iterations.max(1) as u128,
        "observed_speedup": linear_ns as f64 / indexed_ns.max(1) as f64,
        "recorded_at": chrono::Utc::now().to_rfc3339(),
        "environment": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "processor": std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "not reported".into()),
            "logical_processors": std::env::var("NUMBER_OF_PROCESSORS").unwrap_or_else(|_| "not reported".into()),
            "rust": "1.97.1 stable"
        },
        "note": "Local observation, not a universal performance claim"
    });
    let serialized = serde_json::to_string_pretty(&report)?;
    if let Some(path) = output {
        fs::write(path, &serialized)?;
    }
    println!("{serialized}");
    Ok(())
}

impl From<ScenarioArg> for Scenario {
    fn from(value: ScenarioArg) -> Self {
        match value {
            ScenarioArg::Baseline => Self::Baseline,
            ScenarioArg::BruteForce => Self::BruteForce,
            ScenarioArg::PasswordSpray => Self::PasswordSpray,
            ScenarioArg::LateralMovement => Self::LateralMovement,
            ScenarioArg::Mixed => Self::Mixed,
        }
    }
}
impl From<FormatArg> for InputFormat {
    fn from(value: FormatArg) -> Self {
        match value {
            FormatArg::Jsonl => Self::Jsonl,
            FormatArg::Csv => Self::Csv,
        }
    }
}
impl From<PathArg> for PathMode {
    fn from(value: PathArg) -> Self {
        match value {
            PathArg::Hops => Self::Hops,
            PathArg::Risk => Self::Risk,
        }
    }
}
