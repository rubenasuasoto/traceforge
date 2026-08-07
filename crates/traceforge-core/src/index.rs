use crate::event::EventRecord;
use crate::query::{Expr, MatchValue, ParseError, parse_query};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

type PostingList = Vec<usize>;

#[derive(Debug, Default, Clone)]
struct TrieNode {
    children: BTreeMap<char, usize>,
    postings: PostingList,
}

#[derive(Debug, Default, Clone)]
struct PrefixTrie {
    nodes: Vec<TrieNode>,
}

impl PrefixTrie {
    fn new() -> Self {
        Self {
            nodes: vec![TrieNode::default()],
        }
    }

    fn insert(&mut self, key: &str, event_id: usize) {
        let mut node_id = 0;
        push_unique(&mut self.nodes[node_id].postings, event_id);
        for ch in key.chars() {
            let child = if let Some(child) = self.nodes[node_id].children.get(&ch) {
                *child
            } else {
                let next = self.nodes.len();
                self.nodes.push(TrieNode::default());
                self.nodes[node_id].children.insert(ch, next);
                next
            };
            node_id = child;
            push_unique(&mut self.nodes[node_id].postings, event_id);
        }
    }

    fn find(&self, prefix: &str) -> &[usize] {
        let mut node_id = 0;
        for ch in prefix.chars() {
            let Some(next) = self.nodes[node_id].children.get(&ch) else {
                return &[];
            };
            node_id = *next;
        }
        &self.nodes[node_id].postings
    }
}

fn push_unique(postings: &mut PostingList, value: usize) {
    if postings.last() != Some(&value) {
        postings.push(value);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub operator: String,
    pub detail: String,
    pub input_candidates: usize,
    pub output_candidates: usize,
    pub operations: usize,
    pub complexity: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub strategy: String,
    pub total_events: usize,
    pub candidates: usize,
    pub operations: usize,
    pub steps: Vec<PlanStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub query: String,
    pub matches: Vec<EventRecord>,
    pub plan: ExecutionPlan,
}

#[derive(Debug, Clone)]
pub struct SearchIndex {
    events: Vec<EventRecord>,
    exact: HashMap<String, PostingList>,
    trie: PrefixTrie,
    temporal: Vec<(i64, usize)>,
}

impl SearchIndex {
    pub fn build(mut events: Vec<EventRecord>) -> Self {
        events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp).then(a.id.cmp(&b.id)));
        let mut exact = HashMap::<String, PostingList>::new();
        let mut trie = PrefixTrie::new();
        let mut temporal = Vec::with_capacity(events.len());

        for (event_id, event) in events.iter().enumerate() {
            temporal.push((event.timestamp.timestamp_millis(), event_id));
            for (field, value) in indexed_fields(event) {
                let key = format!("{}={}", field, value.to_ascii_lowercase());
                push_unique(exact.entry(key.clone()).or_default(), event_id);
                trie.insert(&key, event_id);
            }
            for token in tokenize(&event.message) {
                let key = format!("text={token}");
                push_unique(exact.entry(key.clone()).or_default(), event_id);
                trie.insert(&key, event_id);
            }
        }

        Self {
            events,
            exact,
            trie,
            temporal,
        }
    }

    pub fn events(&self) -> &[EventRecord] {
        &self.events
    }

    pub fn query(&self, query: &str, limit: usize) -> Result<QueryResult, ParseError> {
        let expr = parse_query(query)?;
        let mut plan = ExecutionPlan {
            strategy: "indexed-posting-lists".into(),
            total_events: self.events.len(),
            ..ExecutionPlan::default()
        };
        let postings = self.evaluate(&expr, &mut plan);
        plan.candidates = postings.len();
        plan.operations = plan.steps.iter().map(|step| step.operations).sum();
        let matches = postings
            .into_iter()
            .take(limit)
            .map(|id| self.events[id].clone())
            .collect();
        Ok(QueryResult {
            query: query.into(),
            matches,
            plan,
        })
    }

    pub fn linear_scan(&self, expr: &Expr) -> Vec<usize> {
        self.events
            .iter()
            .enumerate()
            .filter_map(|(id, event)| matches_expr(event, expr).then_some(id))
            .collect()
    }

    fn evaluate(&self, expr: &Expr, plan: &mut ExecutionPlan) -> PostingList {
        match expr {
            Expr::Term { field, value } => self.evaluate_term(field.as_deref(), value, plan),
            Expr::And(left, right) => {
                let left = self.evaluate(left, plan);
                let right = self.evaluate(right, plan);
                let output = intersect(&left, &right);
                plan.steps.push(PlanStep {
                    operator: "INTERSECT".into(),
                    detail: "linear merge of two ordered posting lists".into(),
                    input_candidates: left.len() + right.len(),
                    output_candidates: output.len(),
                    operations: left.len() + right.len(),
                    complexity: "O(n + m)".into(),
                });
                output
            }
            Expr::Or(left, right) => {
                let left = self.evaluate(left, plan);
                let right = self.evaluate(right, plan);
                let output = union(&left, &right);
                plan.steps.push(PlanStep {
                    operator: "UNION".into(),
                    detail: "deduplicating merge of ordered posting lists".into(),
                    input_candidates: left.len() + right.len(),
                    output_candidates: output.len(),
                    operations: left.len() + right.len(),
                    complexity: "O(n + m)".into(),
                });
                output
            }
            Expr::Not(inner) => {
                let inner = self.evaluate(inner, plan);
                let universe: Vec<_> = (0..self.events.len()).collect();
                let output = difference(&universe, &inner);
                plan.steps.push(PlanStep {
                    operator: "DIFFERENCE".into(),
                    detail: "subtract posting list from event universe".into(),
                    input_candidates: universe.len() + inner.len(),
                    output_candidates: output.len(),
                    operations: universe.len() + inner.len(),
                    complexity: "O(n + m)".into(),
                });
                output
            }
        }
    }

    fn evaluate_term(
        &self,
        field: Option<&str>,
        value: &MatchValue,
        plan: &mut ExecutionPlan,
    ) -> PostingList {
        match value {
            MatchValue::TimeRange { start, end } => {
                let low = self
                    .temporal
                    .partition_point(|(time, _)| *time < start.timestamp_millis());
                let high = self
                    .temporal
                    .partition_point(|(time, _)| *time <= end.timestamp_millis());
                let mut result: Vec<_> =
                    self.temporal[low..high].iter().map(|(_, id)| *id).collect();
                result.sort_unstable();
                plan.steps.push(PlanStep {
                    operator: "TIME_RANGE".into(),
                    detail: format!("binary search temporal index [{low}, {high})"),
                    input_candidates: self.events.len(),
                    output_candidates: result.len(),
                    operations: log2_steps(self.events.len()) * 2 + result.len(),
                    complexity: "O(log n + k)".into(),
                });
                result
            }
            MatchValue::Exact(value) => {
                let effective_field = if field == Some("message") {
                    "text"
                } else {
                    field.unwrap_or("text")
                };
                if effective_field == "text" && value.chars().any(char::is_whitespace) {
                    let terms = tokenize(value);
                    let mut candidates = terms
                        .first()
                        .and_then(|term| self.exact.get(&format!("text={term}")))
                        .cloned()
                        .unwrap_or_default();
                    for term in terms.iter().skip(1) {
                        let postings = self
                            .exact
                            .get(&format!("text={term}"))
                            .map(Vec::as_slice)
                            .unwrap_or(&[]);
                        candidates = intersect(&candidates, postings);
                    }
                    let before = candidates.len();
                    let needle = value.to_ascii_lowercase();
                    candidates.retain(|id| {
                        self.events[*id]
                            .message
                            .to_ascii_lowercase()
                            .contains(&needle)
                    });
                    plan.steps.push(PlanStep {
                        operator: "PHRASE_VERIFY".into(),
                        detail: format!(
                            "intersect {} token lists, then verify phrase",
                            terms.len()
                        ),
                        input_candidates: before,
                        output_candidates: candidates.len(),
                        operations: before + terms.len(),
                        complexity: "O(sum postings + candidates)".into(),
                    });
                    return candidates;
                }
                let key = format!("{}={}", effective_field, value.to_ascii_lowercase());
                let result = self.exact.get(&key).cloned().unwrap_or_default();
                plan.steps.push(PlanStep {
                    operator: "POSTING_LOOKUP".into(),
                    detail: key,
                    input_candidates: self.events.len(),
                    output_candidates: result.len(),
                    operations: 1,
                    complexity: "O(1) average + O(k)".into(),
                });
                result
            }
            MatchValue::Prefix(prefix) => {
                let key = format!(
                    "{}={}",
                    field.unwrap_or("text"),
                    prefix.to_ascii_lowercase()
                );
                let result = self.trie.find(&key).to_vec();
                plan.steps.push(PlanStep {
                    operator: "TRIE_PREFIX".into(),
                    detail: key,
                    input_candidates: self.events.len(),
                    output_candidates: result.len(),
                    operations: prefix.chars().count() + result.len(),
                    complexity: "O(p + k)".into(),
                });
                result
            }
        }
    }
}

fn indexed_fields(event: &EventRecord) -> Vec<(&'static str, String)> {
    let mut values = vec![
        ("id", event.id.clone()),
        ("source", event.source.clone()),
        ("type", event.event_type.clone()),
        ("outcome", format!("{:?}", event.outcome)),
        ("severity", format!("{:?}", event.severity)),
    ];
    if let Some(value) = &event.user {
        values.push(("user", value.clone()));
    }
    if let Some(value) = &event.host {
        values.push(("host", value.clone()));
    }
    if let Some(value) = &event.source_ip {
        values.push(("ip", value.clone()));
    }
    values
}

fn tokenize(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_' && ch != '-')
        .filter(|token| !token.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn matches_expr(event: &EventRecord, expr: &Expr) -> bool {
    match expr {
        Expr::And(a, b) => matches_expr(event, a) && matches_expr(event, b),
        Expr::Or(a, b) => matches_expr(event, a) || matches_expr(event, b),
        Expr::Not(inner) => !matches_expr(event, inner),
        Expr::Term { field, value } => match value {
            MatchValue::TimeRange { start, end } => {
                event.timestamp >= *start && event.timestamp <= *end
            }
            MatchValue::Exact(value) => field
                .as_deref()
                .and_then(|field| (field != "message").then(|| event.field(field)).flatten())
                .map(|candidate| candidate.eq_ignore_ascii_case(value))
                .unwrap_or_else(|| {
                    if value.chars().any(char::is_whitespace) {
                        event.message.to_ascii_lowercase().contains(value)
                    } else {
                        tokenize(&event.message).iter().any(|token| token == value)
                    }
                }),
            MatchValue::Prefix(prefix) => field
                .as_deref()
                .and_then(|field| event.field(field))
                .map(|candidate| candidate.to_ascii_lowercase().starts_with(prefix))
                .unwrap_or_else(|| {
                    tokenize(&event.message)
                        .iter()
                        .any(|token| token.starts_with(prefix))
                }),
        },
    }
}

pub fn intersect(left: &[usize], right: &[usize]) -> Vec<usize> {
    let (mut a, mut b) = (0, 0);
    let mut result = Vec::new();
    while a < left.len() && b < right.len() {
        match left[a].cmp(&right[b]) {
            std::cmp::Ordering::Less => a += 1,
            std::cmp::Ordering::Greater => b += 1,
            std::cmp::Ordering::Equal => {
                result.push(left[a]);
                a += 1;
                b += 1;
            }
        }
    }
    result
}

pub fn union(left: &[usize], right: &[usize]) -> Vec<usize> {
    let (mut a, mut b) = (0, 0);
    let mut result = Vec::with_capacity(left.len() + right.len());
    while a < left.len() || b < right.len() {
        let next = if b >= right.len() || (a < left.len() && left[a] < right[b]) {
            let value = left[a];
            a += 1;
            value
        } else if a >= left.len() || right[b] < left[a] {
            let value = right[b];
            b += 1;
            value
        } else {
            let value = left[a];
            a += 1;
            b += 1;
            value
        };
        if result.last() != Some(&next) {
            result.push(next);
        }
    }
    result
}

pub fn difference(left: &[usize], right: &[usize]) -> Vec<usize> {
    let (mut a, mut b) = (0, 0);
    let mut result = Vec::new();
    while a < left.len() {
        while b < right.len() && right[b] < left[a] {
            b += 1;
        }
        if b >= right.len() || left[a] != right[b] {
            result.push(left[a]);
        }
        a += 1;
    }
    result
}

fn log2_steps(value: usize) -> usize {
    if value <= 1 {
        1
    } else {
        value.ilog2() as usize + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::{Scenario, generate_events};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn posting_intersection_matches_set_semantics(mut a in prop::collection::vec(0usize..200, 0..100), mut b in prop::collection::vec(0usize..200, 0..100)) {
            a.sort_unstable(); a.dedup(); b.sort_unstable(); b.dedup();
            let got = intersect(&a, &b);
            let expected: Vec<_> = a.iter().copied().filter(|value| b.binary_search(value).is_ok()).collect();
            prop_assert_eq!(got, expected);
        }
    }

    #[test]
    fn indexed_results_equal_linear_scan() {
        let index = SearchIndex::build(generate_events(600, 42, Scenario::Mixed));
        for query in [
            "outcome:failure AND user:ana",
            "host:ws-* OR severity:critical",
            "NOT outcome:success AND type:authentication",
            "message:\"invalid password\"",
        ] {
            let expr = parse_query(query).unwrap();
            let indexed: Vec<_> = index
                .query(query, usize::MAX)
                .unwrap()
                .matches
                .into_iter()
                .map(|event| event.id)
                .collect();
            let scanned: Vec<_> = index
                .linear_scan(&expr)
                .into_iter()
                .map(|id| index.events()[id].id.clone())
                .collect();
            assert_eq!(indexed, scanned, "query: {query}");
        }
    }
}
