# Arquitectura y mapa algorítmico

TraceForge separa deliberadamente los algoritmos portables de los adaptadores de entorno. `traceforge-core` no depende de WASM, APIs del navegador ni del parser de argumentos del CLI.

## Flujo de datos

1. La ingesta analiza JSONL o la proyección CSV documentada.
2. Los IDs ausentes reciben un fingerprint estable; los eventos se ordenan por fecha UTC e ID.
3. `SearchIndex` construye posting lists exactas, un trie y un vector temporal.
4. El parser produce un AST y su evaluación combina listas ordenadas recursivamente.
5. Las reglas consumen los mismos eventos mediante ventanas temporales acotadas.
6. `EntityGraph` conecta usuarios, equipos e IP de origen observados en un evento.
7. El CLI controla archivos; el adaptador WASM controla límites web y serialización.

## Estructuras de datos

| Estructura | Implementación | Uso | Coste |
| --- | --- | --- | --- |
| Posting list | `Vec<usize>` ordenado | términos exactos | lookup O(1) medio + salida k |
| Intersección/unión/diferencia | merge propio de dos punteros | AST booleano | O(n + m) |
| Trie | nodos con `BTreeMap<char, nodo>` y postings | prefijos | O(prefijo + salida) |
| Índice temporal | vector `(epoch_ms, event_id)` ordenado | rango inclusivo | O(log n + k) |
| Union-Find | rango y compresión de caminos | componentes | inversa de Ackermann amortizada |
| BFS | `VecDeque` | ruta con menos saltos | O(V + E) |
| Heap indexado | heap vectorial + mapa nodo/posición | decrease-key de Dijkstra | O((V + E) log V) |
| Ventana de detección | `VecDeque<&EventRecord>` | reglas temporales | O(n) amortizado |
| Top-k | `BinaryHeap<Reverse<...>>` acotado | rankings | O(n log k) |

## Semántica de riesgo

Cada pareja coobservada conserva número de observaciones y severidad máxima. Dijkstra usa `max(1, 11 - risk_score)`. Una relación crítica cuesta 1 y una informativa 10. La ruta resultante es la cadena sospechosa más fuerte bajo este modelo, no una ruta física de red.

## Paridad nativa/WASM

Ambos adaptadores instancian `SearchIndex`, `EntityGraph` y `detect_all` desde `traceforge-core`. WASM devuelve objetos compatibles con serde y aplica el límite de 50 MB / 100.000 eventos. No existe una segunda implementación algorítmica en TypeScript.

