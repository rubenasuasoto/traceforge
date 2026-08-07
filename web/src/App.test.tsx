import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

vi.mock('./wasm-pkg/traceforge_wasm', () => ({
  default: vi.fn().mockResolvedValue({}),
  TraceForgeEngine: class {
    stats() { return { events: 3, nodes: 2, edges: 1, localOnly: true, engine: 'test' } }
    query() { return { matches: [], plan: { strategy: 'indexed-posting-lists', candidates: 0, operations: 1, steps: [] } } }
    detections() { return [] }
    graph() { return { nodes: [], edges: [] } }
  },
}))

import App from './App'

describe('TraceForge workbench', () => {
  it('renders the local-processing promise and query editor', async () => {
    render(<App />)
    expect(await screen.findByText('TRACEFORGE')).toBeInTheDocument()
    expect(screen.getByText(/Local-only processing|Procesamiento solo local/)).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: /Query|Consulta/ })).toBeInTheDocument()
  })
})

