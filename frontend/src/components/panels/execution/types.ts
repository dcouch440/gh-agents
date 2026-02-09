// Panel-local types for the execution feed

type RunHistoryItem = {
  id: string
  status: string
  startedAt: string
  completedAt: string | null
  durationMs: number | null
  error: string | null
}

export type { RunHistoryItem }
