import type { NormalizedMap } from '../lib'
import type { ToolRouter } from '@/types/toolRouter'
import type { RouterMode } from '@/types/router'
import type { Tool } from '@/types/tool'

type ToolRouterState = {
  items: NormalizedMap<ToolRouter>
  toolsByRouter: Record<string, Tool[]>
  modesByRouter: Record<string, RouterMode[]>
  toolsByMode: Record<string, Tool[]>
  /** Reverse lookup: modeId → routerId for O(1) owner resolution */
  modeToRouter: Record<string, string>
  loading: boolean
  error: string | null
}

export type { ToolRouterState }
