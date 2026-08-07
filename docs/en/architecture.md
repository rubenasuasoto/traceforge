# Architecture and algorithm map

TraceForge intentionally separates portable algorithms from environment adapters. `traceforge-core` does not depend on WASM, browser APIs or CLI argument parsing.

## Data flow

1. Ingestion parses JSONL or the documented CSV projection.
2. Missing IDs receive a stable fingerprint; events are sorted by UTC timestamp and ID.
3. `SearchIndex` builds exact posting lists, a prefix trie and a temporal vector.
4. The query parser produces an AST. Evaluation recursively combines ordered lists.
5. Detection rules consume the same ordered events with bounded sliding windows.
6. `EntityGraph` connects user, host and source-IP nodes observed in one event.
7. The CLI owns files; the WASM adapter owns browser limits and serialization.

## Data structures

| Structure | Implementation | Purpose | Cost |
| --- | --- | --- | --- |
| Posting list | sorted `Vec<usize>` | exact terms | lookup O(1) average + output k |
| Intersection/union/difference | handwritten two-pointer merge | boolean AST | O(n + m) |
| Prefix trie | nodes with `BTreeMap<char, node>` and postings | prefix terms | O(prefix + output) |
| Temporal index | sorted `(epoch_ms, event_id)` vector | inclusive time range | O(log n + k) |
| Union-Find | rank + path compression | connected components | amortized inverse Ackermann |
| BFS | `VecDeque` | minimum-hop path | O(V + E) |
| Indexed min-heap | vector heap + node-position map | decrease-key Dijkstra | O((V + E) log V) |
| Detection window | `VecDeque<&EventRecord>` | time-bounded rules | amortized O(n) |
| Top-k | bounded `BinaryHeap<Reverse<...>>` | entity rankings | O(n log k) |

## Risk path semantics

Each co-observed pair stores observation count and maximum event severity. Dijkstra edge cost is `max(1, 11 - risk_score)`. A critical edge therefore costs 1 and an informational edge costs 10. The selected path is the strongest suspicious chain under this model, not a physical network route.

## Native/WASM parity

Both adapters instantiate `SearchIndex`, `EntityGraph` and `detect_all` from `traceforge-core`. WASM methods return serde-compatible objects and enforce the 50 MB / 100,000 event browser boundary. No TypeScript implementation of the algorithms exists.

## Index persistence

The CLI `.tfi` payload records a format name, integer version, creation timestamp, normalized events and SHA-256 checksum. Indices with a mismatched format, version or checksum are rejected. Version 1 rebuilds in-memory indices on load; the stable payload is designed for correctness and compatibility before zero-copy performance.

