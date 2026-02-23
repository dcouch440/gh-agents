import { assistantSessionStore } from '.'
import { store } from './_store'
import {
  appendTextToken,
  buildToolSegments,
  completeToolInSegments,
  buildDocSegments,
  applyStreamError,
  mapHistory,
  parseTokenText,
} from './streaming'
import type { ChatMessage } from '@/types'

describe('assistantSessionStore', () => {
  const STEP = 'step-001'

  beforeEach(() => {
    store.setState({ byStep: {} })
  })

  describe('pure helpers', () => {
    const assistantMsg = { id: 'a1', role: 'assistant' as const, content: '' }

    describe('appendTextToken', () => {
      it('creates a new text segment when segments are empty', () => {
        const { segments } = appendTextToken([], [assistantMsg], 'Hello')
        expect(segments).toEqual([{ type: 'text', content: 'Hello' }])
      })

      it('appends to existing text segment', () => {
        const first = appendTextToken([], [assistantMsg], 'Hello')
        const { segments } = appendTextToken(first.segments, first.messages, ' world')
        expect(segments).toEqual([{ type: 'text', content: 'Hello world' }])
      })

      it('creates new text segment after tool segment', () => {
        const withText = appendTextToken([], [assistantMsg], 'Before')
        const withTool = buildToolSegments(withText.segments, 't1', 'think')
        const { segments } = appendTextToken(withTool, withText.messages, 'After')

        expect(segments).toHaveLength(3)
        expect(segments[0]).toEqual({ type: 'text', content: 'Before' })
        expect(segments[1]).toEqual({ type: 'tool', toolId: 't1', toolName: 'think', status: 'running' })
        expect(segments[2]).toEqual({ type: 'text', content: 'After' })
      })

      it('updates last assistant message content', () => {
        const first = appendTextToken([], [assistantMsg], 'Hello')
        const { messages } = appendTextToken(first.segments, first.messages, ' world')

        const lastMsg = messages[messages.length - 1]
        expect(lastMsg?.content).toBe('Hello world')
      })
    })

    describe('buildToolSegments', () => {
      it('adds a running tool segment', () => {
        const segments = buildToolSegments([], 't1', 'update_prompt')
        expect(segments).toEqual([
          { type: 'tool', toolId: 't1', toolName: 'update_prompt', status: 'running' },
        ])
      })
    })

    describe('completeToolInSegments', () => {
      it('updates matching tool segment to complete', () => {
        const withTool = buildToolSegments([], 't1', 'update_prompt')
        const segments = completeToolInSegments(withTool, 't1')

        expect(segments).toEqual([
          { type: 'tool', toolId: 't1', toolName: 'update_prompt', status: 'complete' },
        ])
      })

      it('only updates the matching tool by id', () => {
        let segs = buildToolSegments([], 't1', 'update_prompt')
        segs = buildToolSegments(segs, 't2', 'read_context')
        const result = completeToolInSegments(segs, 't1')

        expect(result[0]).toEqual(expect.objectContaining({ toolId: 't1', status: 'complete' }))
        expect(result[1]).toEqual(expect.objectContaining({ toolId: 't2', status: 'running' }))
      })
    })

    describe('buildDocSegments', () => {
      it('adds a doc_update segment', () => {
        const segments = buildDocSegments([], 'd1', 'API Reference')
        expect(segments).toEqual([{ type: 'doc_update', docId: 'd1', title: 'API Reference' }])
      })
    })

    describe('applyStreamError', () => {
      it('sets error text on empty assistant message', () => {
        const messages = applyStreamError([assistantMsg], 'Timeout')
        expect(messages[0]?.content).toBe('Error: Timeout')
      })

      it('preserves existing content on assistant message', () => {
        const withContent = { ...assistantMsg, content: 'Partial response' }
        const messages = applyStreamError([withContent], 'Timeout')
        expect(messages[0]?.content).toBe('Partial response')
      })
    })

    describe('mapHistory', () => {
      it('maps ChatMessage to ChatMessageData', () => {
        const history: ChatMessage[] = [
          { id: 'msg-1', role: 'user', content: 'hello', timestamp: '2025-01-01T00:00:00Z', source_type: null },
          { id: 'msg-2', role: 'assistant', content: 'hi', timestamp: '2025-01-01T00:00:01Z', source_type: null },
        ]
        const result = mapHistory(history)

        expect(result).toEqual([
          { id: 'msg-1', role: 'user', content: 'hello', source_type: null, panelMeta: undefined },
          { id: 'msg-2', role: 'assistant', content: 'hi', source_type: null, panelMeta: undefined },
        ])
      })

      it('reconstructs panelMeta as submitted for panel_render messages', () => {
        const history: ChatMessage[] = [
          { id: 'msg-1', role: 'assistant', content: '# Panel', timestamp: '2025-01-01T00:00:00Z', source_type: 'panel_render' },
        ]
        const result = mapHistory(history)

        expect(result[0]?.panelMeta).toEqual({ submitLabel: 'Submit', submitted: true })
      })
    })

    describe('parseTokenText', () => {
      it('parses JSON string', () => {
        expect(parseTokenText('"Hello"')).toBe('Hello')
      })

      it('returns raw text for non-JSON', () => {
        expect(parseTokenText('Hello')).toBe('Hello')
      })
    })
  })

  describe('store actions', () => {
    const assistantMsg = { id: 'a1', role: 'assistant' as const, content: '' }
    const userMsg = { id: 'u1', role: 'user' as const, content: 'hello' }

    beforeEach(() => {
      assistantSessionStore.initEmpty(STEP)
      assistantSessionStore.appendMessage(STEP, userMsg)
      assistantSessionStore.appendMessage(STEP, assistantMsg)
    })

    it('streamToken appends text and updates assistant message', () => {
      assistantSessionStore.streamToken(STEP, 'Hello')
      assistantSessionStore.streamToken(STEP, ' world')

      const step = store.getState().byStep[STEP]!
      expect(step.streamingSegments).toEqual([{ type: 'text', content: 'Hello world' }])
      expect(step.messages[1]?.content).toBe('Hello world')
    })

    it('addTool adds a running tool segment', () => {
      assistantSessionStore.addTool(STEP, 't1', 'update_prompt')

      const step = store.getState().byStep[STEP]!
      expect(step.streamingSegments).toEqual([
        { type: 'tool', toolId: 't1', toolName: 'update_prompt', status: 'running' },
      ])
    })

    it('completeTool marks tool as complete', () => {
      assistantSessionStore.addTool(STEP, 't1', 'update_prompt')
      assistantSessionStore.completeTool(STEP, 't1')

      const step = store.getState().byStep[STEP]!
      expect(step.streamingSegments).toEqual([
        { type: 'tool', toolId: 't1', toolName: 'update_prompt', status: 'complete' },
      ])
    })

    it('addDoc adds a doc_update segment', () => {
      assistantSessionStore.addDoc(STEP, 'd1', 'API Reference')

      const step = store.getState().byStep[STEP]!
      expect(step.streamingSegments).toEqual([
        { type: 'doc_update', docId: 'd1', title: 'API Reference' },
      ])
    })

    it('addPanelMessage inserts panel message before last assistant', () => {
      assistantSessionStore.addPanelMessage(STEP, '# Panel\n- [ ] Option A', 'Submit')

      const step = store.getState().byStep[STEP]!
      // Panel message inserted before the last assistant message (index 1)
      expect(step.messages).toHaveLength(3)
      expect(step.messages[1]?.source_type).toBe('panel_render')
      expect(step.messages[1]?.panelMeta).toEqual({ submitLabel: 'Submit', submitted: false })
      // Original assistant message stays at the end
      expect(step.messages[2]?.id).toBe('a1')
      // Adds a panel_render segment
      expect(step.streamingSegments).toContainEqual(
        expect.objectContaining({ type: 'panel_render', content: '# Panel\n- [ ] Option A' }),
      )
    })

    it('submitPanel marks panel message as submitted', () => {
      assistantSessionStore.addPanelMessage(STEP, '# Panel', 'Submit')

      const step = store.getState().byStep[STEP]!
      const panelMsg = step.messages.find((m) => m.source_type === 'panel_render')!
      assistantSessionStore.submitPanel(STEP, panelMsg.id)

      const updated = store.getState().byStep[STEP]!
      const updatedPanel = updated.messages.find((m) => m.id === panelMsg.id)!
      expect(updatedPanel.panelMeta?.submitted).toBe(true)
    })

    it('finalizeStream clears streaming segments', () => {
      assistantSessionStore.streamToken(STEP, 'Final content')
      assistantSessionStore.addTool(STEP, 't1', 'think')
      assistantSessionStore.finalizeStream(STEP)

      const step = store.getState().byStep[STEP]!
      expect(step.streamingSegments).toEqual([])
    })

    it('finalizeStream preserves message content', () => {
      assistantSessionStore.streamToken(STEP, 'Complete response')
      assistantSessionStore.finalizeStream(STEP)

      const step = store.getState().byStep[STEP]!
      expect(step.messages[1]?.content).toBe('Complete response')
    })

    it('handleStreamError clears segments and sets error', () => {
      assistantSessionStore.streamToken(STEP, 'Partial')
      assistantSessionStore.handleStreamError(STEP, 'Connection lost')

      const step = store.getState().byStep[STEP]!
      expect(step.streamingSegments).toEqual([])
      expect(step.error).toBe('Connection lost')
    })

    it('handleStreamError sets error text on empty assistant message', () => {
      assistantSessionStore.handleStreamError(STEP, 'Timeout')

      const step = store.getState().byStep[STEP]!
      expect(step.messages[1]?.content).toBe('Error: Timeout')
    })

    it('handleStreamError preserves existing assistant content', () => {
      assistantSessionStore.streamToken(STEP, 'Partial response')
      assistantSessionStore.handleStreamError(STEP, 'Timeout')

      const step = store.getState().byStep[STEP]!
      expect(step.messages[1]?.content).toBe('Partial response')
    })

    it('resetStep removes the step entry', () => {
      assistantSessionStore.resetStep(STEP)
      expect(store.getState().byStep[STEP]).toBeUndefined()
    })

    describe('full streaming lifecycle', () => {
      it('handles a complete streaming sequence with tools', () => {
        assistantSessionStore.streamToken(STEP, "I'll create docs.\n\n")
        assistantSessionStore.addTool(STEP, 't1', 'update_prompt')
        assistantSessionStore.completeTool(STEP, 't1')
        assistantSessionStore.addDoc(STEP, 'd1', 'API Reference')
        assistantSessionStore.streamToken(STEP, '\n\nCreated the document.')
        assistantSessionStore.finalizeStream(STEP)

        const step = store.getState().byStep[STEP]!
        expect(step.streamingSegments).toEqual([])
        expect(step.messages[1]?.content).toBe("I'll create docs.\n\n\n\nCreated the document.")
      })
    })
  })

  describe('session lifecycle', () => {
    it('initEmpty sets isLoading to false', () => {
      assistantSessionStore.initEmpty(STEP)
      const step = store.getState().byStep[STEP]!
      expect(step.isLoading).toBe(false)
      expect(step.session).toBeNull()
    })

    it('initStep sets isLoading to true', () => {
      assistantSessionStore.initStep(STEP)
      const step = store.getState().byStep[STEP]!
      expect(step.isLoading).toBe(true)
    })

    it('setSession sets all fields', () => {
      const session = { id: 's1', mode_id: 'step_chat', agent_id: null, draft_config: null, title: 'Test', created_at: '', updated_at: '' }
      const messages = [{ id: 'm1', role: 'user' as const, content: 'hello' }]

      assistantSessionStore.setSession(STEP, session, messages)

      const step = store.getState().byStep[STEP]!
      expect(step.session).toBe(session)
      expect(step.messages).toBe(messages)
      expect(step.isLoading).toBe(false)
      expect(step.error).toBeNull()
    })

    it('setError sets error and loading false', () => {
      assistantSessionStore.initStep(STEP)
      assistantSessionStore.setError(STEP, 'Network error')

      const step = store.getState().byStep[STEP]!
      expect(step.error).toBe('Network error')
      expect(step.isLoading).toBe(false)
    })
  })
})
