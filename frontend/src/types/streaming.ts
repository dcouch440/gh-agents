type ToolStatus = 'running' | 'complete'

type ToolIndicatorData = {
  toolId: string
  toolName: string
  status: ToolStatus
}

type MessageSegment =
  | { type: 'text'; content: string }
  | { type: 'tool'; toolId: string; toolName: string; status: ToolStatus }
  | { type: 'doc_update'; docId: string; title: string }
  | { type: 'panel_render'; content: string; submitLabel: string }

// SSE event types matching backend StreamChunk enum
// Note: 'done' is intercepted by createSSEStream and routed to onDone callback
type StreamEventType = 'token' | 'message' | 'content' | 'tool_start' | 'tool_end' | 'doc_update' | 'panel_render' | 'error'

export const SSE_EVENT = {
  TOKEN: 'token',
  MESSAGE: 'message',
  CONTENT: 'content',
  TOOL_START: 'tool_start',
  TOOL_END: 'tool_end',
  DOC_UPDATE: 'doc_update',
  PANEL_RENDER: 'panel_render',
  ERROR: 'error',
} as const

const isContentEvent = (eventType: string): boolean =>
  eventType === SSE_EVENT.TOKEN || eventType === SSE_EVENT.MESSAGE || eventType === SSE_EVENT.CONTENT

export { isContentEvent }
export type { ToolStatus, ToolIndicatorData, MessageSegment, StreamEventType }
