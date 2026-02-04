import {useReducer, useEffect, useRef, useCallback, useState} from "react";
import {useNavigate, useParams} from "react-router-dom";
import {
  Box,
  TextField,
  Button,
  Select,
  MenuItem,
  FormControl,
  InputLabel,
  Alert,
} from "@mui/material";
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
import {useOutputSchemaContext} from "@/hooks/useOutputSchemaContext";
import {api} from "@/api";
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
  outputSchemaId: string | null;
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
  | {type: "SET_OUTPUT_SCHEMA"; schemaId: string | null}
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
  | {type: "SET_ERROR"; value: string | null}
  | {
      type: "HYDRATE_SESSION";
      payload: {
        name: string;
        systemPrompt: string;
        modelId: string;
        maxTokens: number;
        temperature: number;
        outputSchemaId: string | null;
        selectedDocumentIds: string[];
        messages: ChatMessageData[];
        sessionId: string;
        tempAgentId: string;
      };
    };

const initialState: WorkshopState = {
  name: "",
  systemPrompt: "",
  modelId: "sonnet",
  maxTokens: 4096,
  temperature: 0.7,
  outputSchemaId: null,
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
    case "SET_OUTPUT_SCHEMA":
      return {...state, outputSchemaId: action.schemaId, dirty: true};
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
    case "HYDRATE_SESSION":
      return {
        ...state,
        ...action.payload,
        sessionLoading: false,
        dirty: false,
        error: null,
      };
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

const getShorthandModelId = (fullId: string): string => {
  const reverseMap: Record<string, string> = {
    "claude-opus-4-5-20251101": "opus",
    "claude-sonnet-4-20250514": "sonnet",
    "claude-3-5-haiku-20241022": "haiku",
  };
  return reverseMap[fullId] ?? "sonnet";
};

// ── Component ────────────────────────────────────────────────────────────────

function AgentWorkshopPage() {
  const navigate = useNavigate();
  const {sessionId: urlSessionId} = useParams<{sessionId?: string}>();
  const [state, dispatch] = useReducer(reducer, initialState);
  const {splitPercent, handleMouseDown} = useSplitPane({
    initial: 40,
    min: 25,
    max: 75,
  });
  const {send, streaming: sseStreaming} = useSendSessionMessage();
  const {schemas} = useOutputSchemaContext();
  const [showDocumentSelector, setShowDocumentSelector] = useState(false);
  const contentRef = useRef("");
  const savedRef = useRef(false);
  const justNavigatedRef = useRef(false);

  // Load existing session or create new one on mount
  useEffect(() => {
    let cancelled = false;
    let tempAgentIdForCleanup: string | null = null;

    const loadExistingSession = async () => {
      try {
        // Fetch session, agent, history, and context in parallel
        const [session, history] = await Promise.all([
          api.sessions.get(urlSessionId!),
          api.sessions.getHistory(urlSessionId!),
        ]);

        if (cancelled) return;

        if (!session.agent_id) {
          throw new Error("Session has no linked agent");
        }

        // Fetch agent and context
        const [agent, contextDocs] = await Promise.all([
          api.agents.get(session.agent_id),
          api.agents.getContext(session.agent_id),
        ]);

        // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
        if (cancelled) return;

        // Convert chat history to ChatMessageData format
        const messages: ChatMessageData[] = history.map((msg) => ({
          id: msg.id,
          role: msg.role,
          content: msg.content,
        }));

        // Hydrate state
        dispatch({
          type: "HYDRATE_SESSION",
          payload: {
            name: agent.name.replace("[Workshop Draft] ", ""),
            systemPrompt: agent.system_prompt,
            modelId: getShorthandModelId(agent.model_id),
            maxTokens: agent.model_max_tokens,
            temperature: agent.model_temperature,
            outputSchemaId: agent.output_schema_id,
            selectedDocumentIds: contextDocs.documents.map((d) => d.id),
            messages,
            sessionId: session.id,
            tempAgentId: agent.id,
          },
        });
      } catch (err) {
        if (!cancelled) {
          dispatch({type: "SET_SESSION_LOADING", value: false});
          dispatch({
            type: "SET_ERROR",
            value: err instanceof Error ? err.message : "Failed to load session",
          });
        }
      }
    };

    const createNewSession = async () => {
      try {
        const agent = await api.agents.create({
          name: "[Workshop Draft]",
          system_prompt: "",
          model_id: getFullModelId(initialState.modelId),
          model_max_tokens: 4096,
          model_temperature: 0.7,
        });

        if (cancelled) return;
        tempAgentIdForCleanup = agent.id;
        dispatch({type: "SET_TEMP_AGENT", agentId: agent.id});

        const session = await api.sessions.create({
          mode_id: "workshop",
          agent_id: agent.id,
          title: "Agent Workshop",
        });

        // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
        if (!cancelled) {
          dispatch({type: "SET_SESSION", sessionId: session.id});
        }
      } catch (err) {
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
      }
    };

    // Skip loading if we just navigated after save (data is already in state)
    if (justNavigatedRef.current) {
      justNavigatedRef.current = false;
      return;
    }

    if (urlSessionId) {
      void loadExistingSession();
    } else {
      void createNewSession();
    }

    return () => {
      cancelled = true;
      // Clean up temporary agent if user navigates away without saving
      if (tempAgentIdForCleanup && !savedRef.current) {
        void api.agents.delete(tempAgentIdForCleanup);
      }
    };
  }, [urlSessionId]);

  // Update temporary agent when config changes
  useEffect(() => {
    if (!state.tempAgentId) return;

    const timeoutId = setTimeout(() => {
      void api.agents.update(state.tempAgentId, {
        system_prompt: state.systemPrompt || undefined,
        model_id: getFullModelId(state.modelId),
        model_max_tokens: state.maxTokens,
        model_temperature: state.temperature,
        output_schema_id: state.outputSchemaId ?? undefined,
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
    state.outputSchemaId,
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
        if (
          event.event === "token" ||
          event.event === "message" ||
          event.event === "content"
        ) {
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
          return api.agents.setContext(
            state.tempAgentId,
            state.selectedDocumentIds,
          );
        }
      })
      .then(() => {
        savedRef.current = true;
        dispatch({type: "SET_DIRTY", value: false});
        // If we don't have a sessionId in URL yet, update URL without reload
        if (!urlSessionId && state.sessionId) {
          justNavigatedRef.current = true;
          void navigate(`/agents/workshop/${state.sessionId}`, {replace: true});
        }
        // Otherwise, stay on the same URL (already persistent)
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
  }, [
    state.name,
    state.tempAgentId,
    state.selectedDocumentIds,
    state.sessionId,
    urlSessionId,
    navigate,
  ]);

  const chatDisabled =
    state.saving || state.sessionLoading || !state.sessionId || sseStreaming;

  return (
    <Box
      sx={{
        display: "flex",
        flexDirection: "column",
        height: "calc(100vh - 120px)",
        pt: 2
      }}
    >
      <PageHeader title="Agent Workshop">
        <Box sx={{display: "flex", gap: 2, alignItems: "center"}}>
          <TextField
            size="small"
            placeholder="Agent name..."
            value={state.name}
            onChange={(e) =>
              dispatch({type: "SET_NAME", value: e.target.value})
            }
            disabled={state.saving}
            sx={{minWidth: 200}}
          />
          <Button
            variant="contained"
            onClick={handleSave}
            disabled={state.saving || !state.name.trim()}
            size="small"
          >
            {state.saving
              ? urlSessionId
                ? "Updating..."
                : "Saving..."
              : urlSessionId
                ? "Update"
                : "Save"}
          </Button>
        </Box>
      </PageHeader>

      {state.error ? (
        <Alert severity="error" sx={{mb: 2}}>
          {state.error}
        </Alert>
      ) : null}

      <Box
        sx={{
          flex: 1,
          display: "flex",
          minHeight: 0,
          flexDirection: "column",
        }}
      >
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
            <Box
              sx={{display: "flex", flexDirection: "column", height: "100%"}}
            >
              <EditorToolbar>
                <ToggleGroup
                  options={EDITOR_MODES}
                  value={state.editorMode}
                  onChange={(v) =>
                    dispatch({type: "SET_EDITOR_MODE", value: v as EditorMode})
                  }
                />
              </EditorToolbar>
              <Box sx={{flex: 1, overflow: "auto"}}>
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
              </Box>
              <Box
                sx={{
                  p: 2.5,
                  borderTop: 1,
                  borderColor: "divider",
                  bgcolor: "background.paper",
                  display: "grid",
                  gridTemplateColumns: "repeat(2, 1fr)",
                  gap: 2,
                  flexShrink: 0,
                }}
              >
                <FormControl size="small" fullWidth>
                  <InputLabel id="ws-model-label">Model</InputLabel>
                  <Select
                    labelId="ws-model-label"
                    id="ws-model"
                    value={state.modelId}
                    label="Model"
                    onChange={(e) =>
                      dispatch({type: "SET_MODEL_ID", value: e.target.value})
                    }
                    disabled={state.saving}
                  >
                    <MenuItem value="opus">Opus</MenuItem>
                    <MenuItem value="sonnet">Sonnet</MenuItem>
                    <MenuItem value="haiku">Haiku</MenuItem>
                  </Select>
                </FormControl>
                <TextField
                  id="ws-tokens"
                  label="Max Tokens"
                  type="number"
                  size="small"
                  inputProps={{min: 1}}
                  value={state.maxTokens}
                  onChange={(e) =>
                    dispatch({
                      type: "SET_MAX_TOKENS",
                      value: Number(e.target.value),
                    })
                  }
                  disabled={state.saving}
                  fullWidth
                />
                <TextField
                  id="ws-temp"
                  label="Temperature"
                  type="number"
                  size="small"
                  inputProps={{min: 0, max: 2, step: 0.1}}
                  value={state.temperature}
                  onChange={(e) =>
                    dispatch({
                      type: "SET_TEMPERATURE",
                      value: Number(e.target.value),
                    })
                  }
                  disabled={state.saving}
                  fullWidth
                />
                <FormControl size="small" fullWidth sx={{gridColumn: "1 / -1"}}>
                  <InputLabel id="ws-schema-label">
                    Output Schema (Optional)
                  </InputLabel>
                  <Select
                    labelId="ws-schema-label"
                    id="ws-schema"
                    value={state.outputSchemaId ?? ""}
                    label="Output Schema (Optional)"
                    onChange={(e) =>
                      dispatch({
                        type: "SET_OUTPUT_SCHEMA",
                        schemaId: e.target.value || null,
                      })
                    }
                    disabled={state.saving}
                  >
                    <MenuItem value="">
                      <em>None</em>
                    </MenuItem>
                    {schemas?.map((schema) => (
                      <MenuItem key={schema.id} value={schema.id}>
                        {schema.name}
                      </MenuItem>
                    ))}
                  </Select>
                </FormControl>
                <Box sx={{gridColumn: "1 / -1"}}>
                  <FormControl fullWidth>
                    <InputLabel
                      shrink
                      sx={{position: "relative", transform: "none", mb: 1}}
                    >
                      Agent Context Documents
                    </InputLabel>
                    <Button
                      variant="outlined"
                      onClick={() => setShowDocumentSelector(true)}
                      disabled={state.saving}
                      fullWidth
                      size="small"
                    >
                      {state.selectedDocumentIds.length > 0
                        ? `${state.selectedDocumentIds.length} document${
                            state.selectedDocumentIds.length === 1 ? "" : "s"
                          } selected`
                        : "Select documents"}
                    </Button>
                  </FormControl>
                </Box>
              </Box>
            </Box>
          }
        />
      </Box>

      <DocumentSelector
        selectedIds={state.selectedDocumentIds}
        onSelectionChange={(ids) =>
          dispatch({type: "SET_SELECTED_DOCUMENTS", documentIds: ids})
        }
        open={showDocumentSelector}
        onClose={() => setShowDocumentSelector(false)}
      />
    </Box>
  );
}

export {AgentWorkshopPage};
