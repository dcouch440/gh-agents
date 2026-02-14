import { Collections } from '@/utils/collections'
import type { ChatMessageData } from '@/components/chat'
import type { SSEEvent } from '@/api'
import type { ChatMessage, MessageSegment } from '@/types'
import { SSE_EVENT, isContentEvent } from '@/types'
import { getStep, updateStep } from './_store'

type ToolEventPayload = { name: string; id: string }
type DocEventPayload = { doc_id: string; title: string }
type PanelEventPayload = { content: string; submit_label: string }

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

const appendTextToken = (
  segments: readonly MessageSegment[],
  messages: readonly ChatMessageData[],
  text: string,
): { segments: MessageSegment[]; messages: ChatMessageData[] } => {
  const newSegments: MessageSegment[] = segments.slice()
  const lastSeg = newSegments[newSegments.length - 1]
  if (lastSeg?.type === 'text') {
    newSegments[newSegments.length - 1] = { ...lastSeg, content: lastSeg.content + text }
  } else {
    newSegments.push({ type: 'text', content: text })
  }

  const newMessages: ChatMessageData[] = messages.slice()
  const last = newMessages[newMessages.length - 1]
  if (last?.role === 'assistant') {
    newMessages[newMessages.length - 1] = { ...last, content: last.content + text }
  }

  return { segments: newSegments, messages: newMessages }
}

const buildToolSegments = (
  segments: readonly MessageSegment[],
  toolId: string,
  toolName: string,
): MessageSegment[] => [
  ...segments,
  { type: 'tool', toolId, toolName, status: 'running' as const },
]

const completeToolInSegments = (
  segments: readonly MessageSegment[],
  toolId: string,
): MessageSegment[] =>
  Collections.mapBy(segments, (s) =>
    s.type === 'tool' && s.toolId === toolId ? { ...s, status: 'complete' as const } : s,
  )

const buildDocSegments = (
  segments: readonly MessageSegment[],
  docId: string,
  title: string,
): MessageSegment[] => [...segments, { type: 'doc_update', docId, title }]

const applyStreamError = (
  messages: readonly ChatMessageData[],
  error: string,
): ChatMessageData[] => {
  const newMessages = messages.slice()
  const last = newMessages[newMessages.length - 1]
  if (last?.role === 'assistant') {
    newMessages[newMessages.length - 1] = { ...last, content: last.content || `Error: ${error}` }
  }
  return newMessages
}

const mapHistory = (history: readonly ChatMessage[]): ChatMessageData[] =>
  Collections.mapBy(history, (m) => ({
    id: m.id,
    role: m.role,
    content: m.content,
  }))

const parseTokenText = (data: string): string => {
  try {
    const parsed = JSON.parse(data) as unknown
    if (typeof parsed === 'string') return parsed
  } catch {
    // raw text
  }
  return data
}

// ---------------------------------------------------------------------------
// Store actions
// ---------------------------------------------------------------------------

const appendMessage = (stepId: string, message: ChatMessageData): void => {
  const step = getStep(stepId)
  updateStep(stepId, { messages: [...step.messages, message] })
}

const streamToken = (stepId: string, text: string): void => {
  const step = getStep(stepId)
  const { segments, messages } = appendTextToken(step.streamingSegments, step.messages, text)
  updateStep(stepId, { streamingSegments: segments, messages })
}

const addTool = (stepId: string, toolId: string, toolName: string): void => {
  const step = getStep(stepId)
  updateStep(stepId, { streamingSegments: buildToolSegments(step.streamingSegments, toolId, toolName) })
}

const completeTool = (stepId: string, toolId: string): void => {
  const step = getStep(stepId)
  updateStep(stepId, { streamingSegments: completeToolInSegments(step.streamingSegments, toolId) })
}

const addDoc = (stepId: string, docId: string, title: string): void => {
  const step = getStep(stepId)
  updateStep(stepId, { streamingSegments: buildDocSegments(step.streamingSegments, docId, title) })
}

const setPanel = (stepId: string, content: string, submitLabel: string): void => {
  updateStep(stepId, { activePanel: { content, submitLabel } })
}

const dismissPanel = (stepId: string): void => {
  updateStep(stepId, { activePanel: null })
}

const finalizeStream = (stepId: string): void => {
  updateStep(stepId, { streamingSegments: [] })
}

const handleStreamError = (stepId: string, error: string): void => {
  const step = getStep(stepId)
  updateStep(stepId, {
    messages: applyStreamError(step.messages, error),
    streamingSegments: [],
    error,
  })
}

const handleSSEEvent = (stepId: string, event: SSEEvent): number => {
  switch (event.event) {
    case SSE_EVENT.TOKEN:
    case SSE_EVENT.MESSAGE:
    case SSE_EVENT.CONTENT: {
      const text = parseTokenText(event.data)
      streamToken(stepId, text)
      return text.length
    }
    case SSE_EVENT.TOOL_START: {
      const data = JSON.parse(event.data) as ToolEventPayload
      addTool(stepId, data.id, data.name)
      return 0
    }
    case SSE_EVENT.TOOL_END: {
      const data = JSON.parse(event.data) as ToolEventPayload
      completeTool(stepId, data.id)
      return 0
    }
    case SSE_EVENT.DOC_UPDATE: {
      const data = JSON.parse(event.data) as DocEventPayload
      addDoc(stepId, data.doc_id, data.title)
      return 0
    }
    case SSE_EVENT.PANEL_RENDER: {
      const data = JSON.parse(event.data) as PanelEventPayload
      setPanel(stepId, data.content, data.submit_label)
      return 0
    }
    case SSE_EVENT.ERROR: {
      handleStreamError(stepId, event.data)
      return 0
    }
    default:
      return 0
  }
}

const buildDeduplicatingHandler = (
  stepId: string,
  dedupeAfter: number,
  onEvent: (event: SSEEvent) => void,
  trackLength: (len: number) => void,
): ((event: SSEEvent) => void) => {
  let replayedLength = 0
  return (evt: SSEEvent) => {
    if (isContentEvent(evt.event)) {
      const text = parseTokenText(evt.data)
      replayedLength += text.length
      if (replayedLength <= dedupeAfter) return
      const overlap = dedupeAfter - (replayedLength - text.length)
      const newText = overlap > 0 ? text.slice(overlap) : text
      if (newText) {
        trackLength(newText.length)
        streamToken(stepId, newText)
      }
    } else {
      onEvent(evt)
    }
  }
}

export {
  // Pure helpers (exported for testing)
  appendTextToken,
  buildToolSegments,
  completeToolInSegments,
  buildDocSegments,
  applyStreamError,
  mapHistory,
  parseTokenText,
  // Store actions
  appendMessage,
  streamToken,
  addTool,
  completeTool,
  addDoc,
  setPanel,
  dismissPanel,
  finalizeStream,
  handleStreamError,
  handleSSEEvent,
  buildDeduplicatingHandler,
}
