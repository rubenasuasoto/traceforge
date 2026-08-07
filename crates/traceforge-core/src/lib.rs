//! TraceForge's portable algorithmic core.
//!
//! The public API returns serializable values so the native CLI and WebAssembly
//! adapter exercise the same implementation.

pub mod detect;
pub mod event;
pub mod graph;
pub mod index;
pub mod ingest;
pub mod query;
pub mod synthetic;

pub use detect::{Detection, DetectionKind, DetectorConfig, detect_all};
pub use event::{EventRecord, Outcome, Severity};
pub use graph::{EntityGraph, PathMode, PathResult};
pub use index::{ExecutionPlan, QueryResult, SearchIndex};
pub use ingest::{IngestIssue, IngestReport, InputFormat, ingest_bytes};
pub use query::{Expr, ParseError, parse_query};
pub use synthetic::{Scenario, generate_events};

use serde::{Deserialize, Serialize};

/// Versioned payload persisted by `build-index`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexSnapshot {
    pub format: String,
    pub version: u32,
    pub created_at: String,
    pub events: Vec<EventRecord>,
    pub sha256: String,
}

pub const INDEX_FORMAT: &str = "traceforge-index";
pub const INDEX_VERSION: u32 = 1;
