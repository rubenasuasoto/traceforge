import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Activity, Braces, Database, FileUp, GitBranch, Languages, LockKeyhole, Play, Route, Search, ShieldAlert, TerminalSquare } from 'lucide-react'
import init, { TraceForgeEngine } from './wasm-pkg/traceforge_wasm'
import { copy } from './i18n'
import type { Detection, GraphPayload, Language, QueryResult, Stats } from './types'

type View = 'results' | 'detections' | 'graph'
type WasmEngine = TraceForgeEngine
const PAGE_SIZE = 12
const DEFAULT_QUERY = 'outcome:failure AND (user:ana OR severity:high)'

function App() {
  const [language, setLanguage] = useState<Language>(() => (localStorage.getItem('traceforge-language') as Language) || (navigator.language.startsWith('es') ? 'es' : 'en'))
  const [engine, setEngine] = useState<WasmEngine | null>(null)
  const [stats, setStats] = useState<Stats | null>(null)
  const [result, setResult] = useState<QueryResult | null>(null)
  const [detections, setDetections] = useState<Detection[]>([])
  const [graph, setGraph] = useState<GraphPayload>({ nodes: [], edges: [] })
  const [query, setQuery] = useState(DEFAULT_QUERY)
  const [queryError, setQueryError] = useState('')
  const [view, setView] = useState<View>('results')
  const [scenario, setScenario] = useState('mixed')
  const [seed, setSeed] = useState(42)
  const [eventCount, setEventCount] = useState(4000)
  const [page, setPage] = useState(1)
  const [busy, setBusy] = useState(true)
  const [fileLabel, setFileLabel] = useState('')
  const inputRef = useRef<HTMLInputElement>(null)
  const t = copy[language]

  const refreshPanels = useCallback((active: WasmEngine, nextQuery = query) => {
    try {
      setStats(active.stats() as Stats)
      setResult(active.query(nextQuery, 10_000) as QueryResult)
      setDetections(active.detections() as Detection[])
      setGraph(active.graph() as GraphPayload)
      setQueryError('')
      setPage(1)
    } catch (error) {
      setQueryError(String(error))
    }
  }, [query])

  useEffect(() => {
    let active = true
    init().then(() => {
      if (!active) return
      const instance = new TraceForgeEngine()
      setEngine(instance)
      setStats(instance.stats() as Stats)
      setResult(instance.query(DEFAULT_QUERY, 10_000) as QueryResult)
      setDetections(instance.detections() as Detection[])
      setGraph(instance.graph() as GraphPayload)
      setBusy(false)
    }).catch((error) => { setQueryError(String(error)); setBusy(false) })
    return () => { active = false }
  }, []) // WASM initializes exactly once.

  useEffect(() => {
    localStorage.setItem('traceforge-language', language)
    document.documentElement.lang = language
  }, [language])

  const runQuery = useCallback(() => {
    if (!engine) return
    try {
      setResult(engine.query(query, 10_000) as QueryResult)
      setQueryError('')
      setPage(1)
      setView('results')
    } catch (error) { setQueryError(String(error)) }
  }, [engine, query])

  const generate = () => {
    if (!engine) return
    setBusy(true)
    try {
      engine.generate(eventCount, BigInt(seed), scenario)
      setFileLabel('')
      refreshPanels(engine)
    } finally { setBusy(false) }
  }

  const importFile = async (file?: File) => {
    if (!file || !engine) return
    const extension = file.name.toLowerCase().split('.').at(-1)
    if (file.size > 50 * 1024 * 1024 || !['jsonl', 'csv'].includes(extension ?? '')) {
      setQueryError(t.invalidFile)
      return
    }
    setBusy(true)
    try {
      const contents = await file.text()
      if (extension === 'csv') engine.load_csv(contents)
      else engine.load_jsonl(contents)
      setFileLabel(file.name)
      refreshPanels(engine)
    } catch (error) { setQueryError(String(error)) }
    finally { setBusy(false); if (inputRef.current) inputRef.current.value = '' }
  }

  if (!engine || !stats) {
    return <main className="boot"><div className="brand-mark"><Activity /></div><p>{t.loading}</p>{queryError && <p role="alert">{queryError}</p>}</main>
  }

  const pageCount = Math.max(1, Math.ceil((result?.matches.length ?? 0) / PAGE_SIZE))
  const rows = result?.matches.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE) ?? []

  return (
    <div className="app-shell">
      <header className="topbar">
        <a href="#workbench" className="brand" aria-label="TraceForge home"><span className="brand-glyph"><Activity size={19} /></span><span>TRACEFORGE</span><small>{t.subtitle}</small></a>
        <div className="top-actions">
          <span className="local-badge"><LockKeyhole size={14} /> {t.local}</span>
          <button className="language" onClick={() => setLanguage(language === 'es' ? 'en' : 'es')} aria-label={language === 'es' ? 'Switch to English' : 'Cambiar a español'}><Languages size={16} /> {language.toUpperCase()}</button>
          <input ref={inputRef} className="sr-only" type="file" accept=".jsonl,.csv,application/x-ndjson,text/csv" onChange={(event) => void importFile(event.target.files?.[0])} />
          <button className="button secondary" onClick={() => inputRef.current?.click()}><FileUp size={16} /> {t.import}</button>
        </div>
      </header>

      <div className="workspace">
        <aside className="sidebar" aria-label={t.scenario}>
          <div className="section-label"><Database size={14} /> {t.dataset}</div>
          <div className="dataset-card"><span className="status-dot" /> <div><strong>{fileLabel || t.sample}</strong><small>{stats.events.toLocaleString(language)} {t.events.toLowerCase()}</small></div></div>
          <label>{t.scenario}<select value={scenario} onChange={(event) => setScenario(event.target.value)}><option value="mixed">{t.mixed}</option><option value="baseline">{t.baseline}</option><option value="brute-force">{t.brute}</option><option value="password-spray">{t.spray}</option><option value="lateral-movement">{t.lateral}</option></select></label>
          <div className="two-fields"><label>{t.seed}<input type="number" min="1" value={seed} onChange={(event) => setSeed(Number(event.target.value))} /></label><label>{t.events}<input type="number" min="100" max="100000" step="100" value={eventCount} onChange={(event) => setEventCount(Math.min(100000, Math.max(100, Number(event.target.value))))} /></label></div>
          <button className="button primary full" onClick={generate} disabled={busy}><Play size={15} /> {t.generate}</button>
          <div className="sidebar-note"><ShieldAlert size={16} /><p>{t.privacy}<small>{t.offline}</small></p></div>
          <div className="engine-version"><TerminalSquare size={14} /><span>{stats.engine}<small>Rust → WebAssembly</small></span></div>
        </aside>

        <main id="workbench" className="main-panel">
          <section className="metrics" aria-label="Dataset statistics">
            <Metric icon={<Braces />} label={t.events} value={stats.events.toLocaleString(language)} />
            <Metric icon={<GitBranch />} label={t.entities} value={stats.nodes.toLocaleString(language)} />
            <Metric icon={<Route />} label={t.relations} value={stats.edges.toLocaleString(language)} />
            <Metric icon={<Activity />} label={t.detections} value={detections.length.toLocaleString(language)} accent />
          </section>

          <section className="query-panel">
            <div className="panel-heading"><div><span className="section-index">01</span><h1>{t.query}</h1></div><button className="text-button" onClick={() => setQuery('timestamp:[2026-07-30T08:00:00Z TO 2026-07-30T09:00:00Z] AND outcome:failure')}>{t.syntax}</button></div>
            <div className={`query-box ${queryError ? 'has-error' : ''}`}><Search aria-hidden="true" /><input aria-label={t.query} value={query} onChange={(event) => setQuery(event.target.value)} onKeyDown={(event) => { if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') runQuery() }} spellCheck="false" /><button className="button primary" onClick={runQuery}><Play size={15} /> {t.run}</button></div>
            <p className="query-hint">{queryError ? <span role="alert">{queryError}</span> : t.hint}</p>
          </section>

          <nav className="view-tabs" aria-label="Workbench views">
            {(['results', 'detections', 'graph'] as View[]).map((item) => <button key={item} className={view === item ? 'active' : ''} onClick={() => setView(item)}>{t[item]} <span>{item === 'results' ? result?.matches.length ?? 0 : item === 'detections' ? detections.length : stats.nodes}</span></button>)}
          </nav>

          {view === 'results' && <ResultsView result={result} rows={rows} language={language} page={page} pageCount={pageCount} setPage={setPage} />}
          {view === 'detections' && <DetectionView detections={detections} language={language} />}
          {view === 'graph' && <GraphView engine={engine} graph={graph} language={language} />}
        </main>
      </div>
      <footer><span>TraceForge v0.1.0</span><span>Apache-2.0 · Synthetic data · No telemetry</span></footer>
    </div>
  )
}

function Metric({ icon, label, value, accent = false }: { icon: React.ReactNode; label: string; value: string; accent?: boolean }) {
  return <article className={`metric ${accent ? 'accent' : ''}`}><span>{icon}</span><div><small>{label}</small><strong>{value}</strong></div></article>
}

function ResultsView({ result, rows, language, page, pageCount, setPage }: { result: QueryResult | null; rows: QueryResult['matches']; language: Language; page: number; pageCount: number; setPage: (page: number) => void }) {
  const t = copy[language]
  const timeline = useMemo(() => {
    const buckets = Array.from({ length: 32 }, () => 0)
    for (let i = 0; i < (result?.matches.length ?? 0); i++) buckets[i % buckets.length]++
    const max = Math.max(1, ...buckets)
    return buckets.map((value) => Math.max(5, Math.round(value / max * 100)))
  }, [result])
  return <section className="view-grid">
    <div className="results-column">
      <div className="timeline-card"><div className="card-title"><span>{t.timeline}</span><strong>{result?.matches.length ?? 0} {t.matches}</strong></div><div className="timeline" aria-label={`${result?.matches.length ?? 0} ${t.matches}`}>{timeline.map((height, index) => <span key={index} style={{ height: `${height}%` }} />)}</div></div>
      <div className="table-card"><div className="table-scroll"><table><thead><tr><th>{t.time}</th><th>{t.source}</th><th>{t.identity}</th><th>{t.host}</th><th>{t.result}</th><th>{t.severity}</th><th>{t.message}</th></tr></thead><tbody>{rows.map((event) => <tr key={event.id}><td className="mono">{new Date(event.timestamp).toLocaleTimeString(language)}</td><td>{event.source}</td><td>{event.user || '—'}</td><td>{event.host || '—'}</td><td><span className={`outcome ${event.outcome}`}>{event.outcome}</span></td><td><span className={`severity ${event.severity}`}>{event.severity}</span></td><td title={event.message}>{event.message}</td></tr>)}</tbody></table></div>{!rows.length && <p className="empty">{t.noResults}</p>}<div className="pagination"><button disabled={page === 1} onClick={() => setPage(page - 1)}>{t.previous}</button><span>{t.page} {page} {t.of} {pageCount}</span><button disabled={page === pageCount} onClick={() => setPage(page + 1)}>{t.next}</button></div></div>
    </div>
    <AlgorithmPanel result={result} language={language} />
  </section>
}

function AlgorithmPanel({ result, language }: { result: QueryResult | null; language: Language }) {
  const t = copy[language]
  return <aside className="algorithm-card"><div className="card-title"><span>{t.algorithm}</span><Braces size={17} /></div><div className="plan-summary"><strong>{result?.plan.strategy || '—'}</strong><div><span>{result?.plan.candidates ?? 0}<small>{t.candidates}</small></span><span>{result?.plan.operations ?? 0}<small>{t.operations}</small></span></div></div><ol className="plan-steps">{result?.plan.steps.map((step, index) => <li key={`${step.operator}-${index}`}><span>{String(index + 1).padStart(2, '0')}</span><div><strong>{step.operator}</strong><p>{step.detail}</p><small>{step.complexity} · {step.output_candidates} out</small></div></li>)}</ol></aside>
}

function DetectionView({ detections, language }: { detections: Detection[]; language: Language }) {
  const t = copy[language]
  if (!detections.length) return <p className="empty large">{t.noDetections}</p>
  return <section className="detection-grid">{detections.map((item) => <article className="detection-card" key={item.id}><header><span className={`severity ${item.severity}`}>{item.severity}</span><time>{new Date(item.started_at).toLocaleString(language)}</time></header><h2>{item.kind.replaceAll('-', ' ')}</h2><p>{item.explanation}</p><div className="entity-tags">{item.entities.slice(0, 6).map((entity) => <span key={entity}>{entity}</span>)}</div><footer><span>{item.event_ids.length} events</span><span>{Object.entries(item.evidence).map(([key, value]) => `${key}: ${value}`).join(' · ')}</span></footer></article>)}</section>
}

function GraphView({ engine, graph, language }: { engine: WasmEngine; graph: GraphPayload; language: Language }) {
  const t = copy[language]
  const [from, setFrom] = useState(graph.nodes[0]?.id ?? '')
  const [to, setTo] = useState(graph.nodes.at(-1)?.id ?? '')
  const [risk, setRisk] = useState(false)
  const [path, setPath] = useState<string[]>([])
  useEffect(() => { if (!graph.nodes.some((node) => node.id === from)) setFrom(graph.nodes[0]?.id ?? ''); if (!graph.nodes.some((node) => node.id === to)) setTo(graph.nodes.at(-1)?.id ?? ''); setPath([]) }, [graph, from, to])
  const positions = useMemo(() => new Map(graph.nodes.map((node, index) => { const angle = index / Math.max(1, graph.nodes.length) * Math.PI * 2 - Math.PI / 2; const radius = 36 + (index % 3) * 6; return [node.id, { x: 50 + Math.cos(angle) * radius, y: 50 + Math.sin(angle) * radius }] })), [graph])
  const pathEdges = new Set(path.slice(1).map((node, index) => [path[index], node].sort().join('|')))
  const runPath = () => { const found = engine.path(from, to, risk) as { nodes: string[] }; setPath(found.nodes) }
  return <section className="graph-layout"><div className="graph-canvas"><svg viewBox="0 0 100 100" role="img" aria-label={t.graph}>{graph.edges.map((edge) => { const a = positions.get(edge.source); const b = positions.get(edge.target); if (!a || !b) return null; const active = pathEdges.has([edge.source, edge.target].sort().join('|')); return <line key={`${edge.source}-${edge.target}`} x1={a.x} y1={a.y} x2={b.x} y2={b.y} className={active ? 'active-edge' : ''} /> })}{graph.nodes.map((node) => { const point = positions.get(node.id)!; const active = path.includes(node.id); return <g key={node.id} transform={`translate(${point.x} ${point.y})`} className={`node ${node.kind} ${active ? 'active-node' : ''}`}><circle r="2.4" /><title>{node.id}</title></g> })}</svg><div className="graph-legend"><span><i className="user" /> user</span><span><i className="host" /> host</span><span><i className="ip" /> ip</span></div></div><aside className="path-panel"><div className="card-title"><span>{t.graphHelp}</span><Route size={17} /></div><label>{t.start}<select value={from} onChange={(event) => setFrom(event.target.value)}>{graph.nodes.map((node) => <option key={node.id}>{node.id}</option>)}</select></label><label>{t.end}<select value={to} onChange={(event) => setTo(event.target.value)}>{graph.nodes.map((node) => <option key={node.id}>{node.id}</option>)}</select></label><div className="segmented"><button className={!risk ? 'active' : ''} onClick={() => setRisk(false)}>{t.hops}</button><button className={risk ? 'active' : ''} onClick={() => setRisk(true)}>{t.risk}</button></div><button className="button primary full" onClick={runPath}><Route size={15} /> {t.findPath}</button>{path.length > 0 && <ol className="path-list">{path.map((node) => <li key={node}>{node}</li>)}</ol>}</aside></section>
}

export default App
