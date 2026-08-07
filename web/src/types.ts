export type Language = 'en' | 'es'

export interface EventRecord {
  id: string
  timestamp: string
  source: string
  event_type: string
  user?: string
  host?: string
  source_ip?: string
  outcome: 'success' | 'failure' | 'unknown'
  severity: 'info' | 'low' | 'medium' | 'high' | 'critical'
  message: string
  attributes: Record<string, string>
}

export interface PlanStep {
  operator: string
  detail: string
  input_candidates: number
  output_candidates: number
  operations: number
  complexity: string
}

export interface QueryResult {
  query: string
  matches: EventRecord[]
  plan: {
    strategy: string
    total_events: number
    candidates: number
    operations: number
    steps: PlanStep[]
  }
}

export interface Detection {
  id: string
  kind: string
  severity: EventRecord['severity']
  started_at: string
  ended_at: string
  entities: string[]
  event_ids: string[]
  explanation: string
  evidence: Record<string, string>
}

export interface GraphNode { id: string; kind: string; label: string; component: number }
export interface GraphEdge { source: string; target: string; observations: number; risk: number }
export interface GraphPayload { nodes: GraphNode[]; edges: GraphEdge[] }
export interface Stats { events: number; nodes: number; edges: number; localOnly: boolean; engine: string }

