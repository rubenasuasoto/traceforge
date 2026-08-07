use serde::Serialize;
use traceforge_core::{
    DetectorConfig, EntityGraph, InputFormat, PathMode, Scenario, SearchIndex, detect_all,
    generate_events, ingest_bytes,
};
use wasm_bindgen::prelude::*;

const WEB_MAX_BYTES: usize = 50 * 1024 * 1024;
const WEB_MAX_EVENTS: usize = 100_000;

#[derive(Serialize)]
struct GraphPayload {
    nodes: Vec<traceforge_core::graph::GraphNode>,
    edges: Vec<traceforge_core::graph::GraphEdge>,
}

#[wasm_bindgen]
pub struct TraceForgeEngine {
    index: SearchIndex,
    graph: EntityGraph,
}

#[wasm_bindgen]
impl TraceForgeEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::from_events(generate_events(2_000, 42, Scenario::Mixed))
    }

    pub fn generate(
        &mut self,
        count: usize,
        seed: u64,
        scenario: &str,
    ) -> Result<JsValue, JsValue> {
        if count > WEB_MAX_EVENTS {
            return Err(js_error("event limit is 100000"));
        }
        *self = Self::from_events(generate_events(count, seed, parse_scenario(scenario)?));
        self.stats()
    }

    pub fn load_jsonl(&mut self, contents: &str) -> Result<JsValue, JsValue> {
        self.load(contents, InputFormat::Jsonl)
    }

    pub fn load_csv(&mut self, contents: &str) -> Result<JsValue, JsValue> {
        self.load(contents, InputFormat::Csv)
    }

    fn load(&mut self, contents: &str, format: InputFormat) -> Result<JsValue, JsValue> {
        if contents.len() > WEB_MAX_BYTES {
            return Err(js_error("file limit is 50 MB"));
        }
        let report = ingest_bytes(contents.as_bytes(), format);
        if report.events.len() > WEB_MAX_EVENTS {
            return Err(js_error("event limit is 100000"));
        }
        let serializable = report.clone();
        *self = Self::from_events(report.events);
        to_js(&serializable)
    }

    pub fn query(&self, query: &str, limit: usize) -> Result<JsValue, JsValue> {
        to_js(
            &self
                .index
                .query(query, limit)
                .map_err(|error| js_error(&error.to_string()))?,
        )
    }

    pub fn detections(&self) -> Result<JsValue, JsValue> {
        to_js(&detect_all(self.index.events(), &DetectorConfig::default()))
    }

    pub fn graph(&self) -> Result<JsValue, JsValue> {
        to_js(&GraphPayload {
            nodes: self.graph.nodes(),
            edges: self.graph.edges(),
        })
    }

    pub fn path(&self, from: &str, to: &str, risk_weighted: bool) -> Result<JsValue, JsValue> {
        to_js(&self.graph.path(
            from,
            to,
            if risk_weighted {
                PathMode::Risk
            } else {
                PathMode::Hops
            },
        ))
    }

    pub fn stats(&self) -> Result<JsValue, JsValue> {
        to_js(&serde_json::json!({
            "events": self.index.events().len(),
            "nodes": self.graph.nodes().len(),
            "edges": self.graph.edges().len(),
            "localOnly": true,
            "engine": "traceforge-core/0.1.0"
        }))
    }
}

impl TraceForgeEngine {
    fn from_events(events: Vec<traceforge_core::EventRecord>) -> Self {
        let index = SearchIndex::build(events);
        let graph = EntityGraph::build(index.events());
        Self { index, graph }
    }
}

impl Default for TraceForgeEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_scenario(value: &str) -> Result<Scenario, JsValue> {
    match value {
        "baseline" => Ok(Scenario::Baseline),
        "brute-force" => Ok(Scenario::BruteForce),
        "password-spray" => Ok(Scenario::PasswordSpray),
        "lateral-movement" => Ok(Scenario::LateralMovement),
        "mixed" => Ok(Scenario::Mixed),
        _ => Err(js_error("unknown scenario")),
    }
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(|error| js_error(&error.to_string()))
}

fn js_error(message: &str) -> JsValue {
    JsValue::from_str(message)
}
