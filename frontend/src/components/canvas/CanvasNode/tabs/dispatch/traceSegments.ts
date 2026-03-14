import type { DispatchTraceEvent } from '@/stores/dispatchStore'

// ── Segment Types ────────────────────────────────────────────────────────────

type DispatchTextSegment = { type: 'text'; content: string }

type DispatchToolSegment = {
  type: 'tool'
  toolId: string
  toolName: string
  input: Record<string, unknown>
  result: unknown
  status: 'running' | 'complete'
}

type DispatchErrorSegment = { type: 'error'; error: string }

type DispatchPhaseSegment = { type: 'phase'; label: string }

type DispatchSystemPromptSegment = { type: 'system_prompt'; content: string; agentName: string | null }

type DispatchSegment =
  | DispatchTextSegment
  | DispatchToolSegment
  | DispatchErrorSegment
  | DispatchPhaseSegment
  | DispatchSystemPromptSegment

// ── Builder ──────────────────────────────────────────────────────────────────

const buildDispatchSegments = (trace: DispatchTraceEvent[]): DispatchSegment[] => {
  const segments: DispatchSegment[] = []
  let textBuffer = ''

  const flushText = () => {
    if (textBuffer.length > 0) {
      segments.push({ type: 'text', content: textBuffer })
      textBuffer = ''
    }
  }

  for (const event of trace) {
    switch (event.type) {
      case 'token': {
        textBuffer += event.content
        break
      }
      case 'tool_start': {
        flushText()
        segments.push({
          type: 'tool',
          toolId: event.toolId,
          toolName: event.toolName,
          input: event.input,
          result: null,
          status: 'running',
        })
        break
      }
      case 'tool_end': {
        flushText()
        // Find matching tool_start segment and update it (scan backwards)
        for (let j = segments.length - 1; j >= 0; j--) {
          const seg = segments[j]
          if (seg?.type === 'tool' && seg.toolId === event.toolId) {
            segments[j] = { ...seg, result: event.result, status: 'complete' }
            break
          }
        }
        break
      }
      case 'error': {
        flushText()
        segments.push({ type: 'error', error: event.error })
        break
      }
      case 'phase_marker': {
        flushText()
        segments.push({ type: 'phase', label: event.label })
        break
      }
      case 'system_prompt': {
        flushText()
        segments.push({ type: 'system_prompt', content: event.content, agentName: event.agentName })
        break
      }
    }
  }

  flushText()
  return segments
}

export { buildDispatchSegments }
export type { DispatchSegment, DispatchTextSegment, DispatchToolSegment, DispatchErrorSegment, DispatchPhaseSegment, DispatchSystemPromptSegment }
