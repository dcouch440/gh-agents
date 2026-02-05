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
  IconButton,
  Tooltip,
} from "@mui/material";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import {PageHeader} from "@/components/primitives";
import {SplitPane} from "@/components/primitives/SplitPane";
import {CodeEditor} from "@/components/primitives/CodeEditor";
import {MarkdownPreview} from "@/components/primitives/MarkdownPreview";
import {EditorToolbar} from "@/components/primitives/EditorToolbar";
import {ToggleGroup} from "@/components/primitives/ToggleGroup";
import {ChatPanel} from "@/components/chat/ChatPanel";
import {DocumentSelector} from "@/components/DocumentSelector";
import {OutputSchemaFormDialog} from "./OutputSchemaFormDialog";
import {useSplitPane} from "@/hooks/useSplitPane";
import {useSendSessionMessage} from "@/hooks/useChatMutations";
import {useOutputSchemaContext} from "@/hooks/useOutputSchemaContext";
import {api} from "@/api";
import type {ChatMessageData} from "@/components/chat/ChatPanel";
import type {SSEEvent} from "@/api";
import type {DraftConfig} from "@/types";
import {extractVariables} from "@/utils/variables";

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
  agentId: string | null; // Set only after save (or when loading saved session)
  saving: boolean;
  dirty: boolean;
  error: string | null;
  variableSimulation: {
    enabled: boolean;
    mockData: Record<string, string>; // variable name → JSON string
    activeTab: string; // "system" or variable name
  };
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
  | {type: "CLEAR_MESSAGES"}
  | {type: "SET_STREAMING"; value: boolean}
  | {type: "SET_SESSION"; sessionId: string}
  | {type: "SET_SESSION_LOADING"; value: boolean}
  | {type: "SET_AGENT_ID"; agentId: string}
  | {type: "SET_SAVING"; value: boolean}
  | {type: "SET_DIRTY"; value: boolean}
  | {type: "SET_ERROR"; value: string | null}
  | {type: "TOGGLE_VARIABLE_SIMULATION"}
  | {type: "SET_VARIABLE_TAB"; tab: string}
  | {type: "SET_VARIABLE_MOCK_DATA"; variable: string; jsonText: string}
  | {type: "SYNC_VARIABLES"; variables: string[]}
  | {
      type: "HYDRATE_DRAFT_SESSION";
      payload: {
        systemPrompt: string;
        modelId: string;
        maxTokens: number;
        temperature: number;
        messages: ChatMessageData[];
        sessionId: string;
      };
    }
  | {
      type: "HYDRATE_SAVED_SESSION";
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
        agentId: string;
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
  agentId: null,
  saving: false,
  dirty: false,
  error: null,
  variableSimulation: {
    enabled: false,
    mockData: {},
    activeTab: "system",
  },
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
    case "CLEAR_MESSAGES":
      return {...state, messages: []};
    case "SET_STREAMING":
      return {...state, streaming: action.value};
    case "SET_SESSION":
      return {...state, sessionId: action.sessionId, sessionLoading: false};
    case "SET_SESSION_LOADING":
      return {...state, sessionLoading: action.value};
    case "SET_AGENT_ID":
      return {...state, agentId: action.agentId};
    case "SET_SAVING":
      return {...state, saving: action.value};
    case "SET_DIRTY":
      return {...state, dirty: action.value};
    case "SET_ERROR":
      return {...state, error: action.value};
    case "HYDRATE_DRAFT_SESSION":
      return {
        ...state,
        ...action.payload,
        sessionLoading: false,
        dirty: false,
        error: null,
      };
    case "HYDRATE_SAVED_SESSION":
      return {
        ...state,
        ...action.payload,
        sessionLoading: false,
        dirty: false,
        error: null,
      };
    case "TOGGLE_VARIABLE_SIMULATION":
      return {
        ...state,
        variableSimulation: {
          ...state.variableSimulation,
          enabled: !state.variableSimulation.enabled,
          // Reset to system tab and clear mock data when toggling off
          activeTab: !state.variableSimulation.enabled
            ? state.variableSimulation.activeTab
            : "system",
          mockData: !state.variableSimulation.enabled
            ? state.variableSimulation.mockData
            : {},
        },
      };
    case "SET_VARIABLE_TAB":
      return {
        ...state,
        variableSimulation: {
          ...state.variableSimulation,
          activeTab: action.tab,
        },
      };
    case "SET_VARIABLE_MOCK_DATA":
      return {
        ...state,
        variableSimulation: {
          ...state.variableSimulation,
          mockData: {
            ...state.variableSimulation.mockData,
            [action.variable]: action.jsonText,
          },
        },
      };
    case "SYNC_VARIABLES": {
      // When variables change (systemPrompt edited), update mockData
      // Preserve existing mock data for variables that still exist
      // Remove mock data for variables that no longer exist
      const newMockData: Record<string, string> = {};
      for (const variable of action.variables) {
        newMockData[variable] = state.variableSimulation.mockData[variable] || "";
      }
      return {
        ...state,
        variableSimulation: {
          ...state.variableSimulation,
          mockData: newMockData,
          // Reset to system tab if current tab variable no longer exists
          activeTab:
            action.variables.includes(state.variableSimulation.activeTab) ||
            state.variableSimulation.activeTab === "system"
              ? state.variableSimulation.activeTab
              : "system",
        },
      };
    }
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
  const {schemas = []} = useOutputSchemaContext();
  const [showDocumentSelector, setShowDocumentSelector] = useState(false);
  const [showSchemaDialog, setShowSchemaDialog] = useState(false);
  const contentRef = useRef("");
  const justNavigatedRef = useRef(false);

  // Load existing session or create new one on mount
  useEffect(() => {
    let cancelled = false;

    const loadExistingSession = async () => {
      try {
        // Fetch session and history
        const [session, history] = await Promise.all([
          api.sessions.get(urlSessionId!),
          api.sessions.getHistory(urlSessionId!),
        ]);

        if (cancelled) return;

        // Convert chat history to ChatMessageData format
        const messages: ChatMessageData[] = history.map((msg) => ({
          id: msg.id,
          role: msg.role,
          content: msg.content,
        }));

        if (session.agent_id) {
          // Saved session with agent - load agent config
          const [agent, contextDocs] = await Promise.all([
            api.agents.get(session.agent_id),
            api.agents.getContext(session.agent_id),
          ]);

          // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
          if (cancelled) return;

          dispatch({
            type: "HYDRATE_SAVED_SESSION",
            payload: {
              name: agent.name,
              systemPrompt: agent.system_prompt,
              modelId: getShorthandModelId(agent.model_id),
              maxTokens: agent.model_max_tokens,
              temperature: agent.model_temperature,
              outputSchemaId: agent.output_schema_id,
              selectedDocumentIds: contextDocs.documents.map((d) => d.id),
              messages,
              sessionId: session.id,
              agentId: agent.id,
            },
          });
        } else if (session.draft_config) {
          // Draft session without agent - load from draft_config
          const draft = session.draft_config;
          dispatch({
            type: "HYDRATE_DRAFT_SESSION",
            payload: {
              systemPrompt: draft.system_prompt,
              modelId: getShorthandModelId(draft.model_id),
              maxTokens: draft.model_max_tokens,
              temperature: draft.model_temperature,
              messages,
              sessionId: session.id,
            },
          });
        } else {
          throw new Error("Session has no agent or draft config");
        }
      } catch (err) {
        if (!cancelled) {
          dispatch({type: "SET_SESSION_LOADING", value: false});
          dispatch({
            type: "SET_ERROR",
            value:
              err instanceof Error ? err.message : "Failed to load session",
          });
        }
      }
    };

    const createNewSession = async () => {
      try {
        // Create default draft config
        const draftConfig: DraftConfig = {
          system_prompt: "",
          model_id: getFullModelId(initialState.modelId),
          model_max_tokens: initialState.maxTokens,
          model_temperature: initialState.temperature,
        };

        // Create session with draft config (no agent)
        const session = await api.sessions.create({
          mode_id: "workshop",
          title: "Agent Workshop",
          draft_config: draftConfig,
        });

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
    };
  }, [urlSessionId]);

  // Sync config changes to session's draft_config (debounced)
  // Only sync for draft sessions (no agentId)
  useEffect(() => {
    if (!state.sessionId || state.agentId) return;

    const timeoutId = setTimeout(() => {
      const draftConfig: DraftConfig = {
        system_prompt: state.systemPrompt,
        model_id: getFullModelId(state.modelId),
        model_max_tokens: state.maxTokens,
        model_temperature: state.temperature,
      };
      void api.sessions.updateConfig(state.sessionId, draftConfig);
    }, 500);

    return () => {
      clearTimeout(timeoutId);
    };
  }, [
    state.sessionId,
    state.agentId,
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

  // Sync variables when system prompt changes (for variable simulation)
  useEffect(() => {
    if (!state.variableSimulation.enabled) return;

    const variables = extractVariables(state.systemPrompt);
    dispatch({type: "SYNC_VARIABLES", variables});
  }, [state.systemPrompt, state.variableSimulation.enabled]);

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
    if (!state.name.trim() || !state.sessionId) return;
    dispatch({type: "SET_SAVING", value: true});
    dispatch({type: "SET_ERROR", value: null});

    if (state.agentId) {
      // Update existing agent
      api.agents
        .update(state.agentId, {
          name: state.name.trim(),
          system_prompt: state.systemPrompt || undefined,
          model_id: getFullModelId(state.modelId),
          model_max_tokens: state.maxTokens,
          model_temperature: state.temperature,
          output_schema_id: state.outputSchemaId ?? undefined,
        })
        .then(() => {
          // Update agent context documents
          return api.agents.setContext(state.agentId!, state.selectedDocumentIds);
        })
        .then(() => {
          dispatch({type: "SET_DIRTY", value: false});
        })
        .catch((err: unknown) => {
          dispatch({
            type: "SET_ERROR",
            value: err instanceof Error ? err.message : "Failed to update agent",
          });
        })
        .finally(() => {
          dispatch({type: "SET_SAVING", value: false});
        });
    } else {
      // Create new agent from draft session
      api.sessions
        .saveAgent(state.sessionId, {
          name: state.name.trim(),
          context_document_ids: state.selectedDocumentIds.length > 0
            ? state.selectedDocumentIds
            : undefined,
        })
        .then((response) => {
          dispatch({type: "SET_AGENT_ID", agentId: response.agent_id});
          dispatch({type: "SET_DIRTY", value: false});
          // Navigate to session URL if not already there
          if (!urlSessionId) {
            justNavigatedRef.current = true;
            void navigate(`/agents/workshop/${state.sessionId}`, {replace: true});
          }
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
    }
  }, [
    state.name,
    state.sessionId,
    state.agentId,
    state.systemPrompt,
    state.modelId,
    state.maxTokens,
    state.temperature,
    state.outputSchemaId,
    state.selectedDocumentIds,
    urlSessionId,
    navigate,
  ]);

  const handleClearMessages = useCallback(() => {
    if (!state.sessionId) return;

    void api.sessions.clearMessages(state.sessionId).then(() => {
      dispatch({type: "CLEAR_MESSAGES"});
    });
  }, [state.sessionId]);

  const chatDisabled =
    state.saving || state.sessionLoading || !state.sessionId || sseStreaming;

  const isSaved = state.agentId !== null;

  return (
    <Box
      sx={{
        display: "flex",
        flexDirection: "column",
        height: "calc(100vh - 120px)",
        pt: 2,
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
              ? isSaved
                ? "Updating..."
                : "Saving..."
              : isSaved
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
            <Box sx={{display: "flex", flexDirection: "column", height: "100%"}}>
              <Box
                sx={{
                  display: "flex",
                  justifyContent: "flex-end",
                  px: 1,
                  py: 0.5,
                  borderBottom: 1,
                  borderColor: "divider",
                }}
              >
                <Tooltip title="Clear messages">
                  <span>
                    <IconButton
                      size="small"
                      onClick={handleClearMessages}
                      disabled={state.messages.length === 0 || state.streaming}
                    >
                      <DeleteOutlineIcon fontSize="small" />
                    </IconButton>
                  </span>
                </Tooltip>
              </Box>
              <Box sx={{flex: 1, minHeight: 0}}>
                <ChatPanel
                  messages={state.messages}
                  onSend={handleSend}
                  streaming={state.streaming}
                  disabled={chatDisabled}
                />
              </Box>
            </Box>
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
                <Box sx={{gridColumn: "1 / -1"}}>
                  <FormControl size="small" fullWidth>
                    <InputLabel id="ws-schema-label">
                      Output Schema (Optional)
                    </InputLabel>
                    <Box sx={{display: "grid", gridTemplateColumns: "1fr auto", gap: 1}}>
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
                        {schemas.map((schema) => (
                          <MenuItem key={schema.id} value={schema.id}>
                            {schema.name}
                          </MenuItem>
                        ))}
                      </Select>
                      <Button
                        variant="outlined"
                        size="small"
                        onClick={() => setShowSchemaDialog(true)}
                        disabled={state.saving}
                        sx={{minWidth: 80}}
                      >
                        New
                      </Button>
                    </Box>
                  </FormControl>
                </Box>
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

      <OutputSchemaFormDialog
        open={showSchemaDialog}
        onClose={() => setShowSchemaDialog(false)}
        onSave={(schemaId) => {
          dispatch({type: "SET_OUTPUT_SCHEMA", schemaId});
          dispatch({type: "SET_DIRTY", value: true});
        }}
      />

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
