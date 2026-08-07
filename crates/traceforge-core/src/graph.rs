use crate::EventRecord;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PathMode {
    Hops,
    Risk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub component: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub observations: u32,
    pub risk: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResult {
    pub found: bool,
    pub mode: PathMode,
    pub nodes: Vec<String>,
    pub cost: u32,
    pub visited: usize,
    pub explanation: String,
}

#[derive(Debug, Clone)]
struct EdgeData {
    observations: u32,
    risk: u32,
}

#[derive(Debug, Clone)]
pub struct EntityGraph {
    labels: Vec<String>,
    lookup: BTreeMap<String, usize>,
    adjacency: Vec<BTreeMap<usize, EdgeData>>,
    components: Vec<usize>,
}

impl EntityGraph {
    pub fn build(events: &[EventRecord]) -> Self {
        let mut graph = Self {
            labels: Vec::new(),
            lookup: BTreeMap::new(),
            adjacency: Vec::new(),
            components: Vec::new(),
        };
        for event in events {
            let entities: Vec<String> = [
                event.user.as_ref().map(|value| format!("user:{value}")),
                event.host.as_ref().map(|value| format!("host:{value}")),
                event.source_ip.as_ref().map(|value| format!("ip:{value}")),
            ]
            .into_iter()
            .flatten()
            .collect();
            for left in 0..entities.len() {
                for right in (left + 1)..entities.len() {
                    graph.connect(&entities[left], &entities[right], event.severity.score());
                }
            }
        }
        graph.rebuild_components();
        graph
    }

    fn ensure_node(&mut self, label: &str) -> usize {
        if let Some(id) = self.lookup.get(label) {
            return *id;
        }
        let id = self.labels.len();
        self.labels.push(label.into());
        self.lookup.insert(label.into(), id);
        self.adjacency.push(BTreeMap::new());
        id
    }

    fn connect(&mut self, left: &str, right: &str, risk: u32) {
        let a = self.ensure_node(left);
        let b = self.ensure_node(right);
        for (from, to) in [(a, b), (b, a)] {
            let edge = self.adjacency[from].entry(to).or_insert(EdgeData {
                observations: 0,
                risk: 0,
            });
            edge.observations += 1;
            edge.risk = edge.risk.max(risk);
        }
    }

    fn rebuild_components(&mut self) {
        let mut union_find = UnionFind::new(self.labels.len());
        for (from, edges) in self.adjacency.iter().enumerate() {
            for to in edges.keys() {
                union_find.union(from, *to);
            }
        }
        self.components = (0..self.labels.len())
            .map(|id| union_find.find(id))
            .collect();
    }

    pub fn nodes(&self) -> Vec<GraphNode> {
        self.labels
            .iter()
            .enumerate()
            .map(|(id, label)| {
                let (kind, value) = label.split_once(':').unwrap_or(("entity", label));
                GraphNode {
                    id: label.clone(),
                    kind: kind.into(),
                    label: value.into(),
                    component: self.components[id],
                }
            })
            .collect()
    }

    pub fn edges(&self) -> Vec<GraphEdge> {
        let mut edges = Vec::new();
        for (from, neighbours) in self.adjacency.iter().enumerate() {
            for (to, data) in neighbours {
                if from < *to {
                    edges.push(GraphEdge {
                        source: self.labels[from].clone(),
                        target: self.labels[*to].clone(),
                        observations: data.observations,
                        risk: data.risk,
                    });
                }
            }
        }
        edges
    }

    pub fn path(&self, from: &str, to: &str, mode: PathMode) -> PathResult {
        let Some(&start) = self.lookup.get(from) else {
            return self.not_found(mode, "source entity does not exist", 0);
        };
        let Some(&goal) = self.lookup.get(to) else {
            return self.not_found(mode, "target entity does not exist", 0);
        };
        if self.components[start] != self.components[goal] {
            return self.not_found(mode, "entities are in disconnected components", 0);
        }
        match mode {
            PathMode::Hops => self.bfs(start, goal),
            PathMode::Risk => self.dijkstra(start, goal),
        }
    }

    fn bfs(&self, start: usize, goal: usize) -> PathResult {
        let mut queue = VecDeque::from([start]);
        let mut previous = vec![None; self.labels.len()];
        let mut seen = vec![false; self.labels.len()];
        seen[start] = true;
        let mut visited = 0;
        while let Some(node) = queue.pop_front() {
            visited += 1;
            if node == goal {
                break;
            }
            for next in self.adjacency[node].keys() {
                if !seen[*next] {
                    seen[*next] = true;
                    previous[*next] = Some(node);
                    queue.push_back(*next);
                }
            }
        }
        let ids = reconstruct(&previous, start, goal);
        PathResult {
            found: !ids.is_empty(),
            mode: PathMode::Hops,
            cost: ids.len().saturating_sub(1) as u32,
            nodes: ids.into_iter().map(|id| self.labels[id].clone()).collect(),
            visited,
            explanation: "BFS minimizes the number of entity-to-entity hops; O(V + E).".into(),
        }
    }

    fn dijkstra(&self, start: usize, goal: usize) -> PathResult {
        let mut distance = vec![u32::MAX; self.labels.len()];
        let mut previous = vec![None; self.labels.len()];
        let mut queue = IndexedMinHeap::new(self.labels.len());
        distance[start] = 0;
        queue.push_or_decrease(start, 0);
        let mut visited = 0;
        while let Some((node, cost)) = queue.pop_min() {
            visited += 1;
            if node == goal {
                break;
            }
            if cost != distance[node] {
                continue;
            }
            for (next, edge) in &self.adjacency[node] {
                // High-risk observations are cheaper, so the selected route exposes the
                // strongest suspicious chain rather than the safest network route.
                let edge_cost = 11_u32.saturating_sub(edge.risk).max(1);
                let candidate = cost.saturating_add(edge_cost);
                if candidate < distance[*next] {
                    distance[*next] = candidate;
                    previous[*next] = Some(node);
                    queue.push_or_decrease(*next, candidate);
                }
            }
        }
        let ids = reconstruct(&previous, start, goal);
        PathResult {
            found: !ids.is_empty(),
            mode: PathMode::Risk,
            cost: distance[goal],
            nodes: ids.into_iter().map(|id| self.labels[id].clone()).collect(),
            visited,
            explanation: "Dijkstra uses an indexed min-heap and inverse severity cost (11 - risk); O((V + E) log V).".into(),
        }
    }

    fn not_found(&self, mode: PathMode, explanation: &str, visited: usize) -> PathResult {
        PathResult {
            found: false,
            mode,
            nodes: Vec::new(),
            cost: 0,
            visited,
            explanation: explanation.into(),
        }
    }
}

fn reconstruct(previous: &[Option<usize>], start: usize, goal: usize) -> Vec<usize> {
    if start == goal {
        return vec![start];
    }
    if previous[goal].is_none() {
        return Vec::new();
    }
    let mut path = vec![goal];
    let mut current = goal;
    while current != start {
        let Some(parent) = previous[current] else {
            return Vec::new();
        };
        path.push(parent);
        current = parent;
    }
    path.reverse();
    path
}

#[derive(Debug)]
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl UnionFind {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
            rank: vec![0; size],
        }
    }
    fn find(&mut self, value: usize) -> usize {
        if self.parent[value] != value {
            self.parent[value] = self.find(self.parent[value]);
        }
        self.parent[value]
    }
    fn union(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a == b {
            return;
        }
        if self.rank[a] < self.rank[b] {
            self.parent[a] = b;
        } else if self.rank[a] > self.rank[b] {
            self.parent[b] = a;
        } else {
            self.parent[b] = a;
            self.rank[a] += 1;
        }
    }
}

/// Min-heap with an O(1) node-to-position map, implemented for TraceForge.
#[derive(Debug)]
struct IndexedMinHeap {
    heap: Vec<(usize, u32)>,
    positions: Vec<Option<usize>>,
}

impl IndexedMinHeap {
    fn new(size: usize) -> Self {
        Self {
            heap: Vec::new(),
            positions: vec![None; size],
        }
    }
    fn push_or_decrease(&mut self, node: usize, priority: u32) {
        if let Some(position) = self.positions[node] {
            if priority < self.heap[position].1 {
                self.heap[position].1 = priority;
                self.sift_up(position);
            }
            return;
        }
        let position = self.heap.len();
        self.heap.push((node, priority));
        self.positions[node] = Some(position);
        self.sift_up(position);
    }
    fn pop_min(&mut self) -> Option<(usize, u32)> {
        if self.heap.is_empty() {
            return None;
        }
        let last = self.heap.len() - 1;
        self.swap(0, last);
        let item = self.heap.pop().unwrap();
        self.positions[item.0] = None;
        if !self.heap.is_empty() {
            self.sift_down(0);
        }
        Some(item)
    }
    fn sift_up(&mut self, mut position: usize) {
        while position > 0 {
            let parent = (position - 1) / 2;
            if self.heap[parent].1 <= self.heap[position].1 {
                break;
            }
            self.swap(parent, position);
            position = parent;
        }
    }
    fn sift_down(&mut self, mut position: usize) {
        loop {
            let left = position * 2 + 1;
            let right = left + 1;
            let mut smallest = position;
            if left < self.heap.len() && self.heap[left].1 < self.heap[smallest].1 {
                smallest = left;
            }
            if right < self.heap.len() && self.heap[right].1 < self.heap[smallest].1 {
                smallest = right;
            }
            if smallest == position {
                break;
            }
            self.swap(position, smallest);
            position = smallest;
        }
    }
    fn swap(&mut self, a: usize, b: usize) {
        self.heap.swap(a, b);
        self.positions[self.heap[a].0] = Some(a);
        self.positions[self.heap[b].0] = Some(b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::{Scenario, generate_events};

    #[test]
    fn bfs_and_dijkstra_find_paths() {
        let graph = EntityGraph::build(&generate_events(200, 7, Scenario::Mixed));
        let nodes = graph.nodes();
        let user = nodes.iter().find(|node| node.kind == "user").unwrap();
        let host = nodes
            .iter()
            .find(|node| node.kind == "host" && node.component == user.component)
            .unwrap();
        assert!(graph.path(&user.id, &host.id, PathMode::Hops).found);
        assert!(graph.path(&user.id, &host.id, PathMode::Risk).found);
    }

    #[test]
    fn missing_entities_return_an_explanation() {
        let graph = EntityGraph::build(&[]);
        let result = graph.path("user:nobody", "host:none", PathMode::Hops);
        assert!(!result.found);
        assert!(result.explanation.contains("source"));
    }

    #[test]
    fn indexed_heap_decreases_priority() {
        let mut heap = IndexedMinHeap::new(4);
        heap.push_or_decrease(1, 9);
        heap.push_or_decrease(2, 4);
        heap.push_or_decrease(1, 2);
        assert_eq!(heap.pop_min(), Some((1, 2)));
    }
}
