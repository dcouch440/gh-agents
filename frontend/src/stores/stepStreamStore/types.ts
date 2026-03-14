type StreamToolUse = {
  toolName: string
  toolId: string
  status: 'running' | 'completed'
  startedAt: string
}

type SourceStreamStatus = 'idle' | 'running' | 'completed' | 'failed'

type SourceStreamState = {
  sourceId: string
  sourceName: string
  stepId: string
  status: SourceStreamStatus
  streamBuffer: string
  toolUses: StreamToolUse[]
  error: string | null
  startedAt: string | null
  completedAt: string | null
}

type StepDesignState = {
  status: SourceStreamStatus
  designedCount: number
  totalCount: number
  lastAgentName: string | null
  designedAgentSlugs: ReadonlySet<string>
}

type StepStreamState = {
  sources: Record<string, SourceStreamState>
  designerStatus: SourceStreamStatus
  designStatusByStep: Record<string, StepDesignState>
  activeStepId: string | null
}

export type { StepStreamState, SourceStreamState, SourceStreamStatus, StreamToolUse, StepDesignState }
