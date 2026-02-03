import { useReducer } from 'react'
import { useNavigate } from 'react-router-dom'
import { PageHeader } from '@/components/primitives'
import { SplitPane } from '@/components/primitives/SplitPane'
import { CodeEditor } from '@/components/primitives/CodeEditor'
import { MarkdownPreview } from '@/components/primitives/MarkdownPreview'
import { EditorToolbar } from '@/components/primitives/EditorToolbar'
import { ToggleGroup } from '@/components/primitives/ToggleGroup'
import { ChatPanel } from '@/components/chat/ChatPanel'
import { useSplitPane } from '@/hooks/useSplitPane'
import { api } from '@/api'
import { ROUTES } from '@/constants'
import type { ChatMessageData } from '@/components/chat/ChatPanel'

// ── State ────────────────────────────────────────────────────────────────────

type EditorMode = 'edit' | 'preview'

type WorkshopState = {
  name: string
  systemPrompt: string
  modelId: string
  maxTokens: number
  temperature: number
  editorMode: EditorMode
  messages: ChatMessageData[]
  streaming: boolean
  saving: boolean
  error: string | null
}

type WorkshopAction =
  | { type: 'SET_NAME'; value: string }
  | { type: 'SET_SYSTEM_PROMPT'; value: string }
  | { type: 'SET_MODEL_ID'; value: string }
  | { type: 'SET_MAX_TOKENS'; value: number }
  | { type: 'SET_TEMPERATURE'; value: number }
  | { type: 'SET_EDITOR_MODE'; value: EditorMode }
  | { type: 'ADD_MESSAGE'; message: ChatMessageData }
  | { type: 'SET_SAVING'; value: boolean }
  | { type: 'SET_ERROR'; value: string | null }

const initialState: WorkshopState = {
  name: '',
  systemPrompt: '',
  modelId: 'sonnet',
  maxTokens: 4096,
  temperature: 0.7,
  editorMode: 'edit',
  messages: [],
  streaming: false,
  saving: false,
  error: null,
}

const reducer = (state: WorkshopState, action: WorkshopAction): WorkshopState => {
  switch (action.type) {
    case 'SET_NAME':
      return { ...state, name: action.value }
    case 'SET_SYSTEM_PROMPT':
      return { ...state, systemPrompt: action.value }
    case 'SET_MODEL_ID':
      return { ...state, modelId: action.value }
    case 'SET_MAX_TOKENS':
      return { ...state, maxTokens: action.value }
    case 'SET_TEMPERATURE':
      return { ...state, temperature: action.value }
    case 'SET_EDITOR_MODE':
      return { ...state, editorMode: action.value }
    case 'ADD_MESSAGE':
      return { ...state, messages: [...state.messages, action.message] }
    case 'SET_SAVING':
      return { ...state, saving: action.value }
    case 'SET_ERROR':
      return { ...state, error: action.value }
  }
}

// ── Editor mode toggle options ───────────────────────────────────────────────

const EDITOR_MODES = [
  { value: 'edit', label: 'Edit' },
  { value: 'preview', label: 'Preview' },
]

// ── Component ────────────────────────────────────────────────────────────────

function AgentWorkshopPage() {
  const navigate = useNavigate()
  const [state, dispatch] = useReducer(reducer, initialState)
  const { splitPercent, handleMouseDown } = useSplitPane({ initial: 40, min: 25, max: 75 })

  const handleSend = (message: string) => {
    const id = `msg-${Date.now()}`
    dispatch({ type: 'ADD_MESSAGE', message: { id, role: 'user', content: message } })
    // TODO: Part 3 — wire to useSendSessionMessage for SSE streaming
  }

  const handleSave = () => {
    if (!state.name.trim()) return
    dispatch({ type: 'SET_SAVING', value: true })
    dispatch({ type: 'SET_ERROR', value: null })
    api.agents.create({
      name: state.name.trim(),
      system_prompt: state.systemPrompt || undefined,
      model_id: state.modelId,
      model_max_tokens: state.maxTokens,
      model_temperature: state.temperature,
    })
      .then(() => { void navigate(ROUTES.AGENTS) })
      .catch((err: unknown) => {
        dispatch({ type: 'SET_ERROR', value: err instanceof Error ? err.message : 'Failed to save agent' })
      })
      .finally(() => { dispatch({ type: 'SET_SAVING', value: false }) })
  }

  return (
    <div className="workshop">
      <PageHeader title="Agent Workshop">
        <input
          className="form-input workshop__name-input"
          type="text"
          placeholder="Agent name..."
          value={state.name}
          onChange={(e) => dispatch({ type: 'SET_NAME', value: e.target.value })}
          disabled={state.saving}
        />
        <button
          type="button"
          className="btn btn--primary"
          onClick={handleSave}
          disabled={state.saving || !state.name.trim()}
        >
          {state.saving ? 'Saving...' : 'Save'}
        </button>
      </PageHeader>

      {state.error ? <div className="error-message">{state.error}</div> : null}

      <div className="workshop__body">
        <SplitPane
          splitPercent={splitPercent}
          onMouseDown={handleMouseDown}
          left={
            <ChatPanel
              messages={state.messages}
              onSend={handleSend}
              streaming={state.streaming}
              disabled={state.saving}
            />
          }
          right={
            <div className="workshop__editor">
              <EditorToolbar>
                <ToggleGroup
                  options={EDITOR_MODES}
                  value={state.editorMode}
                  onChange={(v) => dispatch({ type: 'SET_EDITOR_MODE', value: v as EditorMode })}
                />
              </EditorToolbar>
              <div className="workshop__editor-content">
                {state.editorMode === 'edit' ? (
                  <CodeEditor
                    value={state.systemPrompt}
                    onChange={(v) => dispatch({ type: 'SET_SYSTEM_PROMPT', value: v })}
                    language="markdown"
                    placeholder="Write the agent's system prompt..."
                    readOnly={state.saving}
                  />
                ) : (
                  <MarkdownPreview content={state.systemPrompt} />
                )}
              </div>
              <div className="workshop__config">
                <div className="workshop__config-field">
                  <label className="form-label" htmlFor="ws-model">Model</label>
                  <select
                    id="ws-model"
                    className="form-select"
                    value={state.modelId}
                    onChange={(e) => dispatch({ type: 'SET_MODEL_ID', value: e.target.value })}
                    disabled={state.saving}
                  >
                    <option value="opus">Opus</option>
                    <option value="sonnet">Sonnet</option>
                    <option value="haiku">Haiku</option>
                  </select>
                </div>
                <div className="workshop__config-field">
                  <label className="form-label" htmlFor="ws-tokens">Max Tokens</label>
                  <input
                    id="ws-tokens"
                    className="form-input"
                    type="number"
                    min={1}
                    value={state.maxTokens}
                    onChange={(e) => dispatch({ type: 'SET_MAX_TOKENS', value: Number(e.target.value) })}
                    disabled={state.saving}
                  />
                </div>
                <div className="workshop__config-field">
                  <label className="form-label" htmlFor="ws-temp">Temperature</label>
                  <input
                    id="ws-temp"
                    className="form-input"
                    type="number"
                    min={0}
                    max={2}
                    step={0.1}
                    value={state.temperature}
                    onChange={(e) => dispatch({ type: 'SET_TEMPERATURE', value: Number(e.target.value) })}
                    disabled={state.saving}
                  />
                </div>
              </div>
            </div>
          }
        />
      </div>
    </div>
  )
}

export { AgentWorkshopPage }
