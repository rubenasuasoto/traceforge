import type { Language } from './types'

export const copy = {
  en: {
    subtitle: 'Algorithmic log investigation', local: 'Local-only processing', import: 'Import JSONL / CSV',
    scenario: 'Synthetic scenario', mixed: 'Mixed attack chain', baseline: 'Baseline activity', brute: 'Brute force', spray: 'Password spraying', lateral: 'Lateral movement',
    seed: 'Seed', events: 'Events', generate: 'Generate scenario', dataset: 'Dataset', engine: 'Engine', entities: 'Entities', relations: 'Relations',
    query: 'Query', run: 'Run query', hint: 'Fields: user, host, ip, outcome, severity, type · AND, OR, NOT · prefix* · timestamp:[start TO end]',
    results: 'Results', detections: 'Detections', graph: 'Entity graph', algorithm: 'How it was resolved',
    matches: 'matches', candidates: 'candidates', operations: 'estimated operations', timeline: 'Event timeline',
    time: 'Time', source: 'Source', identity: 'Identity', host: 'Host', result: 'Result', severity: 'Severity', message: 'Message',
    noResults: 'No events match this query.', noDetections: 'No rule crossed its deterministic threshold.',
    graphHelp: 'Select two entities to inspect a route.', start: 'Start entity', end: 'End entity', hops: 'Fewest hops', risk: 'Risk-weighted', findPath: 'Find path',
    privacy: 'Files never leave this browser. Maximum 50 MB / 100,000 events.', syntax: 'Query syntax', offline: 'Works offline after the first load.', loading: 'Loading Rust/WASM engine…',
    previous: 'Previous', next: 'Next', page: 'Page', of: 'of', sample: 'Synthetic demo', invalidFile: 'Use a JSONL or CSV file under 50 MB.',
  },
  es: {
    subtitle: 'Investigación algorítmica de logs', local: 'Procesamiento solo local', import: 'Importar JSONL / CSV',
    scenario: 'Escenario sintético', mixed: 'Cadena de ataque mixta', baseline: 'Actividad base', brute: 'Fuerza bruta', spray: 'Password spraying', lateral: 'Movimiento lateral',
    seed: 'Semilla', events: 'Eventos', generate: 'Generar escenario', dataset: 'Dataset', engine: 'Motor', entities: 'Entidades', relations: 'Relaciones',
    query: 'Consulta', run: 'Ejecutar consulta', hint: 'Campos: user, host, ip, outcome, severity, type · AND, OR, NOT · prefijo* · timestamp:[inicio TO fin]',
    results: 'Resultados', detections: 'Detecciones', graph: 'Grafo de entidades', algorithm: 'Cómo se resolvió',
    matches: 'coincidencias', candidates: 'candidatos', operations: 'operaciones estimadas', timeline: 'Línea temporal',
    time: 'Hora', source: 'Fuente', identity: 'Identidad', host: 'Equipo', result: 'Resultado', severity: 'Severidad', message: 'Mensaje',
    noResults: 'Ningún evento coincide con la consulta.', noDetections: 'Ninguna regla superó su umbral determinista.',
    graphHelp: 'Selecciona dos entidades para inspeccionar una ruta.', start: 'Entidad inicial', end: 'Entidad final', hops: 'Menos saltos', risk: 'Ponderada por riesgo', findPath: 'Buscar ruta',
    privacy: 'Los archivos nunca salen del navegador. Máximo 50 MB / 100.000 eventos.', syntax: 'Sintaxis de consulta', offline: 'Funciona sin red tras la primera carga.', loading: 'Cargando motor Rust/WASM…',
    previous: 'Anterior', next: 'Siguiente', page: 'Página', of: 'de', sample: 'Demo sintética', invalidFile: 'Usa un archivo JSONL o CSV de menos de 50 MB.',
  },
} satisfies Record<Language, Record<string, string>>
