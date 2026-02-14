import { toArray, nmGet } from '../lib'
import type { ToolRouter } from '@/types/toolRouter'
import type { RouterMode } from '@/types/router'
import type { Tool } from '@/types/tool'
import type { ToolRouterState } from './types'

const selectAll = (s: ToolRouterState): ToolRouter[] => toArray(s.items)

const selectById =
  (id: string) =>
  (s: ToolRouterState): ToolRouter | undefined =>
    nmGet(s.items, id)

const selectRouterTools =
  (routerId: string) =>
  (s: ToolRouterState): Tool[] =>
    s.toolsByRouter[routerId] ?? []

const selectModes =
  (routerId: string) =>
  (s: ToolRouterState): RouterMode[] =>
    s.modesByRouter[routerId] ?? []

const selectModeTools =
  (modeId: string) =>
  (s: ToolRouterState): Tool[] =>
    s.toolsByMode[modeId] ?? []

const selectLoading = (s: ToolRouterState): boolean => s.loading

const selectError = (s: ToolRouterState): string | null => s.error

export { selectAll, selectById, selectRouterTools, selectModes, selectModeTools, selectLoading, selectError }
