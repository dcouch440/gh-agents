import {useReducer, useEffect, useRef, useCallback, useState} from "react";
import {useNavigate} from "react-router-dom";
import {PageHeader} from "@/components/primitives";
import {SplitPane} from "@/components/primitives/SplitPane";
import {CodeEditor} from "@/components/primitives/CodeEditor";
import {MarkdownPreview} from "@/components/primitives/MarkdownPreview";
import {EditorToolbar} from "@/components/primitives/EditorToolbar";
import {ToggleGroup} from "@/components/primitives/ToggleGroup";
import {ChatPanel} from "@/components/chat/ChatPanel";
import {DocumentSelector} from "@/components/DocumentSelector";
import {useSplitPane} from "@/hooks/useSplitPane";
import {useSendSessionMessage} from "@/hooks/useChatMutations";
import {api} from "@/api";
import {ROUTES} from "@/constants";
import type {ChatMessageData} from "@/components/chat/ChatPanel";
import type {SSEEvent} from "@/api";

// ── State ────────────────────────────────────────────────────────────────────

type EditorMode = "edit" | "preview";

type WorkshopState = {
  name: string;
  systemPrompt: string;
  modelId: string;
  maxTokens: number;
  temperature: number;
  selectedDocumentIds: string[];
  editorMode: EditorMode;
  messages: ChatMessageData[];
  streaming: boolean;
  sessionId: string | null;
  sessionLoading: boolean;
  tempAgentId: string | null;
  saving: boolean;
  dirty: boolean;
  error: string | null;
};

type WorkshopAction =
  | {type: "SET_NAME"; value: string}
  | {type: "SET_SYSTEM_PROMPT"; value: string}
  | {type: "SET_MODEL_ID"; value: string}
  | {type: "SET_MAX_TOKENS"; value: number}
  | {type: "SET_TEMPERATURE"; value: number}
  | {type: "SET_SELECTED_DOCUMENTS"; documentIds: string[]}
  | {type: "SET_EDITOR_MODE"; value: EditorMode}
  | {type: "ADD_MESSAGE"; message: ChatMessageData}
  | {type: "UPDATE_LAST_ASSISTANT"; content: string}
  | {type: "SET_STREAMING"; value: boolean}
  | {type: "SET_SESSION"; sessionId: string}
  | {type: "SET_SESSION_LOADING"; value: boolean}
  | {type: "SET_TEMP_AGENT"; agentId: string}
  | {type: "SET_SAVING"; value: boolean}
  | {type: "SET_DIRTY"; value: boolean}
  | {type: "SET_ERROR"; value: string | null};

const initialState: WorkshopState = {
  name: "",
  systemPrompt: "",
  modelId: "sonnet",
  maxTokens: 4096,
  temperature: 0.7,
  selectedDocumentIds: [],
  editorMode: "edit",
  messages: [],
  streaming: false,
  sessionId: null,
  sessionLoading: true,
  tempAgentId: null,
  saving: false,
  dirty: false,
  error: null,
};

const reducer = (
  state: WorkshopState,
  action: WorkshopAction,
): WorkshopState => {
  switch (action.type) {
    case "SET_NAME":
      return {...state, name: action.value, dirty: true};
    case "SET_SYSTEM_PROMPT":
      return {...state, systemPrompt: action.value, dirty: true};
    case "SET_MODEL_ID":
      return {...state, modelId: action.value, dirty: true};
    case "SET_MAX_TOKENS":
      return {...state, maxTokens: action.value, dirty: true};
    case "SET_TEMPERATURE":
      return {...state, temperature: action.value, dirty: true};
    case "SET_SELECTED_DOCUMENTS":
      return {...state, selectedDocumentIds: action.documentIds, dirty: true};
    case "SET_EDITOR_MODE":
      return {...state, editorMode: action.value};
    case "ADD_MESSAGE":
      return {...state, messages: [...state.messages, action.message]};
    case "UPDATE_LAST_ASSISTANT": {
      const msgs = [...state.messages];
      const lastIdx = msgs.length - 1;
      if (lastIdx >= 0 && msgs[lastIdx].role === "assistant") {
        msgs[lastIdx] = {...msgs[lastIdx], content: action.content};
      }
      return {...state, messages: msgs};
    }
    case "SET_STREAMING":
      return {...state, streaming: action.value};
    case "SET_SESSION":
      return {...state, sessionId: action.sessionId, sessionLoading: false};
    case "SET_SESSION_LOADING":
      return {...state, sessionLoading: action.value};
    case "SET_TEMP_AGENT":
      return {...state, tempAgentId: action.agentId};
    case "SET_SAVING":
      return {...state, saving: action.value};
    case "SET_DIRTY":
      return {...state, dirty: action.value};
    case "SET_ERROR":
      return {...state, error: action.value};
  }
};

// ── Constants ────────────────────────────────────────────────────────────────

const EDITOR_MODES = [
  {value: "edit", label: "Edit"},
  {value: "preview", label: "Preview"},
];

const MODEL_ID_MAP: Record<string, string> = {
  opus: "claude-opus-4-5-20251101",
  sonnet: "claude-sonnet-4-20250514",
  haiku: "claude-3-5-haiku-20241022",
};

const getFullModelId = (shorthand: string): string => {
  return MODEL_ID_MAP[shorthand] ?? shorthand;
};

// ── Component ────────────────────────────────────────────────────────────────

function AgentWorkshopPage() {
  const navigate = useNavigate();
  const [state, dispatch] = useReducer(reducer, initialState);
  const {splitPercent, handleMouseDown} = useSplitPane({
    initial: 40,
    min: 25,
    max: 75,
  });
  const {send, streaming: sseStreaming} = useSendSessionMessage();
  const [showDocumentSelector, setShowDocumentSelector] = useState(false);
  const contentRef = useRef("");
  const savedRef = useRef(false);

  // Create a temporary agent and session on mount
  useEffect(() => {
    let cancelled = false;
    let tempAgentIdForCleanup: string | null = null;

    // First create a temporary agent
    api.agents
      .create({
        name: "[Workshop Draft]",
        system_prompt: "",
        model_id: getFullModelId(initialState.modelId),
        model_max_tokens: 4096,
        model_temperature: 0.7,
      })
      .then((agent) => {
        if (cancelled) return;
        tempAgentIdForCleanup = agent.id;
        dispatch({type: "SET_TEMP_AGENT", agentId: agent.id});

        // Then create a session linked to this agent
        return api.sessions.create({
          mode_id: "workshop",
          agent_id: agent.id,
          title: "Agent Workshop",
        });
      })
      .then((session) => {
        if (!cancelled && session) {
          dispatch({type: "SET_SESSION", sessionId: session.id});
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          dispatch({type: "SET_SESSION_LOADING", value: false});
          dispatch({
            type: "SET_ERROR",
            value:
              err instanceof Error
                ? err.message
                : "Failed to initialize workshop",
          });
        }
      });

    return () => {
      cancelled = true;
      // Clean up temporary agent if user navigates away without saving
      if (tempAgentIdForCleanup && !savedRef.current) {
        void api.agents.delete(tempAgentIdForCleanup);
      }
    };
  }, []);

  // Update temporary agent when config changes
  useEffect(() => {
    if (!state.tempAgentId) return;

    const timeoutId = setTimeout(() => {
      void api.agents.update(state.tempAgentId, {
        system_prompt: state.systemPrompt || undefined,
        model_id: getFullModelId(state.modelId),
        model_max_tokens: state.maxTokens,
        model_temperature: state.temperature,
      });
    }, 500);

    return () => {
      clearTimeout(timeoutId);
    };
  }, [
    state.tempAgentId,
    state.systemPrompt,
    state.modelId,
    state.maxTokens,
    state.temperature,
  ]);

  // Warn on unsaved navigation
  useEffect(() => {
    if (!state.dirty) return;
    const handler = (e: BeforeUnloadEvent) => {
      e.preventDefault();
    };
    window.addEventListener("beforeunload", handler);
    return () => {
      window.removeEventListener("beforeunload", handler);
    };
  }, [state.dirty]);

  const handleSend = useCallback(
    (message: string) => {
      if (!state.sessionId) return;

      // Add user message
      const userMsgId = `msg-${Date.now()}`;
      dispatch({
        type: "ADD_MESSAGE",
        message: {id: userMsgId, role: "user", content: message},
      });

      // Add empty assistant message placeholder
      const assistantMsgId = `msg-${Date.now() + 1}`;
      dispatch({
        type: "ADD_MESSAGE",
        message: {id: assistantMsgId, role: "assistant", content: ""},
      });
      dispatch({type: "SET_STREAMING", value: true});
      contentRef.current = "";

      const onEvent = (event: SSEEvent) => {
        if (event.event === "token" || event.event === "message" || event.event === "content") {
          // Backend sends event:"token" with JSON-encoded string data
          let text = event.data;
          try {
            // Try to parse as JSON in case it's double-encoded
            const parsed = JSON.parse(text) as unknown;
            if (typeof parsed === "string") {
              text = parsed;
            }
          } catch {
            // If parsing fails, use the raw data
          }
          contentRef.current += text;
          dispatch({
            type: "UPDATE_LAST_ASSISTANT",
            content: contentRef.current,
          });
        }
      };

      const onDone = () => {
        dispatch({type: "SET_STREAMING", value: false});
      };

      void send(state.sessionId, {message}, onEvent, onDone);
    },
    [state.sessionId, send],
  );

  const handleSave = useCallback(() => {
    if (!state.name.trim() || !state.tempAgentId) return;
    dispatch({type: "SET_SAVING", value: true});
    dispatch({type: "SET_ERROR", value: null});

    // Update the temporary agent's name to finalize it
    api.agents
      .update(state.tempAgentId, {
        name: state.name.trim(),
      })
      .then(() => {
        // Save agent context documents
        if (state.selectedDocumentIds.length > 0) {
          return api.agents.setContext(state.tempAgentId, state.selectedDocumentIds);
        }
      })
      .then(() => {
        savedRef.current = true;
        dispatch({type: "SET_DIRTY", value: false});
        void navigate(ROUTES.AGENTS);
      })
      .catch((err: unknown) => {
        dispatch({
          type: "SET_ERROR",
          value: err instanceof Error ? err.message : "Failed to save agent",
        });
      })
      .finally(() => {
        dispatch({type: "SET_SAVING", value: false});
      });
  }, [state.name, state.tempAgentId, state.selectedDocumentIds, navigate]);

  const chatDisabled =
    state.saving || state.sessionLoading || !state.sessionId || sseStreaming;

  return (
    <div className="workshop">
      <PageHeader title="Agent Workshop">
        <input
          className="form-input workshop__name-input"
          type="text"
          placeholder="Agent name..."
          value={state.name}
          onChange={(e) => dispatch({type: "SET_NAME", value: e.target.value})}
          disabled={state.saving}
        />
        <button
          type="button"
          className="btn btn--primary"
          onClick={handleSave}
          disabled={state.saving || !state.name.trim()}
        >
          {state.saving ? "Saving..." : "Save"}
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
              disabled={chatDisabled}
            />
          }
          right={
            <div className="workshop__editor">
              <EditorToolbar>
                <ToggleGroup
                  options={EDITOR_MODES}
                  value={state.editorMode}
                  onChange={(v) =>
                    dispatch({type: "SET_EDITOR_MODE", value: v as EditorMode})
                  }
                />
              </EditorToolbar>
              <div className="workshop__editor-content">
                {state.editorMode === "edit" ? (
                  <CodeEditor
                    value={state.systemPrompt}
                    onChange={(v) =>
                      dispatch({type: "SET_SYSTEM_PROMPT", value: v})
                    }
                    language="markdown"
                    placeholder="Write the agent's system prompt..."
                    readOnly={state.saving}
                    height="100%"
                  />
                ) : (
                  <MarkdownPreview content={state.systemPrompt} />
                )}
              </div>
              <div className="workshop__config">
                <div className="workshop__config-field">
                  <label className="form-label" htmlFor="ws-model">
                    Model
                  </label>
                  <select
                    id="ws-model"
                    className="form-select"
                    value={state.modelId}
                    onChange={(e) =>
                      dispatch({type: "SET_MODEL_ID", value: e.target.value})
                    }
                    disabled={state.saving}
                  >
                    <option value="opus">Opus</option>
                    <option value="sonnet">Sonnet</option>
                    <option value="haiku">Haiku</option>
                  </select>
                </div>
                <div className="workshop__config-field">
                  <label className="form-label" htmlFor="ws-tokens">
                    Max Tokens
                  </label>
                  <input
                    id="ws-tokens"
                    className="form-input"
                    type="number"
                    min={1}
                    value={state.maxTokens}
                    onChange={(e) =>
                      dispatch({
                        type: "SET_MAX_TOKENS",
                        value: Number(e.target.value),
                      })
                    }
                    disabled={state.saving}
                  />
                </div>
                <div className="workshop__config-field">
                  <label className="form-label" htmlFor="ws-temp">
                    Temperature
                  </label>
                  <input
                    id="ws-temp"
                    className="form-input"
                    type="number"
                    min={0}
                    max={2}
                    step={0.1}
                    value={state.temperature}
                    onChange={(e) =>
                      dispatch({
                        type: "SET_TEMPERATURE",
                        value: Number(e.target.value),
                      })
                    }
                    disabled={state.saving}
                  />
                </div>
                <div className="workshop__config-field workshop__config-field--full">
                  <label className="form-label">
                    Agent Context Documents
                  </label>
                  <button
                    type="button"
                    onClick={() => setShowDocumentSelector(true)}
                    disabled={state.saving}
                    className="btn btn--secondary"
                    style={{width: "100%", marginTop: "8px"}}
                  >
                    {state.selectedDocumentIds.length > 0
                      ? `${state.selectedDocumentIds.length} document${
                          state.selectedDocumentIds.length === 1 ? "" : "s"
                        } selected`
                      : "Select documents"}
                  </button>
                </div>
              </div>
            </div>
          }
        />
      </div>

      <DocumentSelector
        selectedIds={state.selectedDocumentIds}
        onSelectionChange={(ids) =>
          dispatch({type: "SET_SELECTED_DOCUMENTS", documentIds: ids})
        }
        open={showDocumentSelector}
        onClose={() => setShowDocumentSelector(false)}
      />
    </div>
  );
}

export {AgentWorkshopPage};
