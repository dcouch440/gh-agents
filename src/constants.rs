//! App-wide constants for nexor.

// ── Environment Variable Keys ─────────────────────────────────────────────

pub const ENV_ANTHROPIC_API_KEY: &str = "ANTHROPIC_API_KEY";
pub const ENV_ANTHROPIC_MODEL: &str = "ANTHROPIC_MODEL";
pub const ENV_XAI_API_KEY: &str = "XAI_API_KEY";
pub const ENV_XAI_MODEL: &str = "XAI_MODEL";
pub const ENV_GITHUB_TOKEN: &str = "GITHUB_TOKEN";
pub const ENV_DATABASE_URL: &str = "DATABASE_URL";
pub const ENV_DB_MAX_CONNECTIONS: &str = "DB_MAX_CONNECTIONS";
pub const ENV_JWT_SECRET: &str = "JWT_SECRET";
pub const ENV_RUST_ENV: &str = "RUST_ENV";
pub const ENV_CORS_ORIGINS: &str = "CORS_ORIGINS";
pub const ENV_NEXOR_STATIC_DIR: &str = "NEXOR_STATIC_DIR";

// ── Model IDs ──────────────────────────────────────────────────────────────
// Anthropic Claude model identifiers. Update these when migrating to new
// model versions — every runtime reference should use these constants.

/// Primary orchestrator model (Opus tier).
pub const MODEL_OPUS: &str = "claude-opus-4-5-20251101";

/// Primary worker model (Sonnet tier).
pub const MODEL_SONNET: &str = "claude-sonnet-4-20250514";

/// Primary utility model (Haiku tier).
pub const MODEL_HAIKU: &str = "claude-3-5-haiku-20241022";

// ── Well-known IDs ────────────────────────────────────────────────────────

/// Default agent UUID (seeded by migration 0012).
/// Used when a workflow step is created without an explicit agent_id.
pub const DEFAULT_AGENT_ID: uuid::Uuid = uuid::Uuid::from_u128(1);

/// Documenter assistant system agent UUID (seeded by migration 0022).
pub const DOCUMENTER_ASSISTANT_AGENT_ID: uuid::Uuid = uuid::Uuid::from_u128(2);

// ── Defaults ───────────────────────────────────────────────────────────────

/// Default model used when no tier/config is specified.
pub const DEFAULT_MODEL: &str = MODEL_SONNET;

// ── Token Limits (tier defaults) ────────────────────────────────────────────

pub const DEFAULT_MAX_TOKENS_ORCHESTRATOR: u32 = 16384;
pub const DEFAULT_MAX_TOKENS_WORKER: u32 = 8192;
pub const DEFAULT_MAX_TOKENS_UTILITY: u32 = 4096;

// ── Token Limits (task-specific) ────────────────────────────────────────────

pub const MAX_TOKENS_FILE_READ: u32 = 1024;
pub const MAX_TOKENS_SUMMARIZE: u32 = 256;
pub const MAX_TOKENS_TITLE: u32 = 32;
pub const MAX_TOKENS_CONTEXT: u32 = 256;
pub const MAX_TOKENS_INDEXER: u32 = 512;
pub const MAX_TOKENS_COMPILER: u32 = 256;
pub const MAX_TOKENS_PLANNER: u32 = 8192;

// ── Temperature ─────────────────────────────────────────────────────────────

pub const DEFAULT_TEMPERATURE: f32 = 0.7;
pub const TEMPERATURE_TECHNICAL: f32 = 0.3;
pub const TEMPERATURE_FORMAL: f32 = 0.4;
pub const TEMPERATURE_FRIENDLY: f32 = 0.6;
pub const TEMPERATURE_CASUAL: f32 = 0.7;

// ── Task Execution ──────────────────────────────────────────────────────────

/// Default maximum retries before a task is permanently failed.
pub const TASK_MAX_RETRIES: u32 = 3;
/// Maximum tool call rounds per task execution.
pub const TASK_MAX_TOOL_ROUNDS: u32 = 15;
/// Consecutive tool errors before the executor bails out early.
pub const TASK_MAX_CONSECUTIVE_TOOL_ERRORS: u32 = 3;

// ── Pipeline ────────────────────────────────────────────────────────────────

/// Maximum retries per pipeline stage before the run is failed.
pub const PIPELINE_MAX_STAGE_RETRIES: u32 = 1;

// ── Timeouts ────────────────────────────────────────────────────────────────

/// Default timeout for task execution, tool calls, and approvals (5 min).
pub const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Per-verification-agent LLM call timeout (60s). On timeout, treat as approved.
pub const VERIFICATION_AGENT_TIMEOUT_SECS: u64 = 60;

// ── Retry / Backoff ─────────────────────────────────────────────────────────

/// Initial retry backoff delay.
pub const RETRY_INITIAL_BACKOFF_MS: u64 = 500;
/// Maximum retry backoff delay.
pub const RETRY_MAX_BACKOFF_SECS: u64 = 60;
/// Default maximum number of retries for LLM calls.
pub const RETRY_MAX_ATTEMPTS: u32 = 5;
/// Jitter factor applied to backoff delays.
pub const RETRY_JITTER_FACTOR: f64 = 0.25;

// ── Rate Limiting ─────────────────────────────────────────────────────────

/// Maximum concurrent LLM API calls across all agents.
pub const RATE_LIMIT_MAX_CONCURRENT_CALLS: usize = 5;
/// Maximum requests per minute to Anthropic API (0 = unlimited).
pub const RATE_LIMIT_REQUESTS_PER_MINUTE: usize = 25;
/// Initial global backoff delay when any agent gets rate limited.
pub const RATE_LIMIT_GLOBAL_BACKOFF_INITIAL_MS: u64 = 2000;
/// Maximum global backoff delay.
pub const RATE_LIMIT_GLOBAL_BACKOFF_MAX_MS: u64 = 60000;

// ── WebSocket Limits ──────────────────────────────────────────────────────────

/// Maximum concurrent WebSocket connections (global).
pub const WS_MAX_CONNECTIONS: usize = 2000;
/// Maximum concurrent WebSocket connections per IP address.
pub const WS_MAX_CONNECTIONS_PER_IP: usize = 20;
/// Maximum WebSocket frame size (1 MB).
pub const WS_MAX_FRAME_SIZE: usize = 1_048_576;
/// Maximum WebSocket message size (4 MB).
pub const WS_MAX_MESSAGE_SIZE: usize = 4_194_304;
/// Seconds without a pong before a connection is considered dead.
pub const WS_PONG_TIMEOUT_SECS: u64 = 90;

// ── Channel Buffer Sizes ────────────────────────────────────────────────────

/// Buffer size for agent command/response channels.
pub const CHANNEL_AGENT: usize = 32;
/// Buffer size for the orchestrator message queue.
pub const CHANNEL_ORCHESTRATOR: usize = 100;

// ── Content Truncation Limits (chars) ───────────────────────────────────────

/// Small content threshold — files under this size are returned directly.
pub const TRUNCATE_SMALL_FILE: usize = 2_000;
/// Input limit for Haiku summarization.
pub const TRUNCATE_SUMMARIZE_INPUT: usize = 10_000;
/// Input limit for Haiku summary requests.
pub const TRUNCATE_SUMMARY_INPUT: usize = 8_000;
/// Input limit for Haiku title generation.
pub const TRUNCATE_TITLE_INPUT: usize = 2_000;
/// Content limit for indexer file analysis.
pub const TRUNCATE_INDEX_INPUT: usize = 4_000;
/// Context file size cap sent to the orchestrator.
pub const TRUNCATE_CONTEXT_FILE: usize = 50_000;
/// Tool result display truncation.
pub const TRUNCATE_TOOL_RESULT: usize = 10_000;
/// Error message truncation for logging.
pub const TRUNCATE_ERROR_LOG: usize = 200;

// ── Input Validation Limits ─────────────────────────────────────────────

/// Maximum length for title fields (tasks, documents, sessions).
pub const MAX_TITLE_LENGTH: usize = 200;
/// Maximum length for description / content fields.
pub const MAX_DESCRIPTION_LENGTH: usize = 10_000;
/// Maximum length for chat messages.
pub const MAX_CHAT_MESSAGE_LENGTH: usize = 5_000;
/// Maximum length for persona prompts.
pub const MAX_PROMPT_LENGTH: usize = 20_000;

// ── Query / Pagination ──────────────────────────────────────────────────────

/// Default number of results when no limit is specified.
pub const DEFAULT_QUERY_LIMIT: i64 = 100;
/// Maximum allowed query limit.
pub const MAX_QUERY_LIMIT: i64 = 1000;
/// Default max results for search operations.
pub const DEFAULT_SEARCH_RESULTS: usize = 20;

// ── Scheduler ───────────────────────────────────────────────────────────────

/// Scheduler poll interval in milliseconds.
pub const SCHEDULER_POLL_INTERVAL_MS: u64 = 100;
/// Scheduler batch size (tasks per poll).
pub const SCHEDULER_BATCH_SIZE: usize = 5;
/// Scheduler agent wait timeout in milliseconds.
pub const SCHEDULER_AGENT_WAIT_MS: u64 = 500;

// ── Orchestrator ────────────────────────────────────────────────────────────

/// Message count that triggers automatic summarization.
pub const SUMMARIZE_THRESHOLD: usize = 20;
/// Number of recent messages to keep when summarizing (the rest are condensed).
pub const SUMMARIZE_KEEP_RECENT: usize = 10;

// ── Routing Complexity Thresholds ───────────────────────────────────────────

pub const COMPLEXITY_HIGH_FILES: usize = 5;
pub const COMPLEXITY_HIGH_DESC_LEN: usize = 500;
pub const COMPLEXITY_MEDIUM_FILES: usize = 2;
pub const COMPLEXITY_MEDIUM_DESC_LEN: usize = 200;

// ── Delegation ──────────────────────────────────────────────────────────────

pub const DEFAULT_MAX_DELEGATION_DEPTH: u8 = 2;

// ── API Route Paths ────────────────────────────────────────────────────
// All paths relative to the /api nest. Used in server/mod.rs route definitions.

pub mod routes {
    // Auth
    pub const HEALTH: &str = "/health";
    pub const AUTH_SETUP: &str = "/auth/setup";
    pub const AUTH_LOGIN: &str = "/auth/login";
    pub const AUTH_REGISTER: &str = "/auth/register";
    pub const AUTH_ME: &str = "/auth/me";

    // Tasks
    pub const TASKS: &str = "/tasks";
    pub const TASK: &str = "/tasks/:id";

    // Agents
    pub const AGENTS: &str = "/agents";
    pub const AGENT: &str = "/agents/:id";
    pub const AGENT_TOOLS: &str = "/agents/:id/tools";
    pub const AGENT_CONTEXT: &str = "/agents/:id/context";
    pub const AGENT_MODES: &str = "/agents/:id/modes";
    pub const AGENT_MODE: &str = "/agent-modes/:id";

    // Tools
    pub const TOOLS: &str = "/tools";
    pub const TOOL: &str = "/tools/:id";

    // Pipeline stages
    pub const PIPELINE_STAGE_RENDER: &str = "/pipelines/:id/stages/:stage_number/render";
    pub const PIPELINE_STAGE_SIDE_TASKS: &str = "/pipelines/:id/stages/:stage_number/side-tasks";
    pub const PIPELINE_STAGE_SIDE_TASK: &str =
        "/pipelines/:id/stages/:stage_number/side-tasks/:side_task_id";

    // Pipeline runs
    pub const PIPELINE_RUNS: &str = "/pipeline-runs";
    pub const PIPELINE_RUN: &str = "/pipeline-runs/:run_id";
    pub const PIPELINE_RUN_APPROVE: &str = "/pipeline-runs/:run_id/approve";
    pub const PIPELINE_RUN_CANCEL: &str = "/pipeline-runs/:run_id/cancel";
    pub const PIPELINE_RUN_TREE: &str = "/pipeline-runs/:run_id/tree";

    // Agent executions
    pub const AGENT_EXECUTION_CANCEL: &str = "/agent-executions/:execution_id/cancel";

    // Config
    pub const CONFIG: &str = "/config";

    // Chat
    pub const CHAT: &str = "/chat";
    pub const CHAT_HISTORY: &str = "/chat/history";
    pub const CHAT_STREAM: &str = "/chat/:message_id/stream";

    // Modes
    pub const MODES: &str = "/modes";

    // Sessions
    pub const SESSIONS: &str = "/sessions";
    pub const SESSION: &str = "/sessions/:session_id";
    pub const SESSION_CHAT: &str = "/sessions/:session_id/chat";
    pub const SESSION_HISTORY: &str = "/sessions/:session_id/history";
    pub const SESSION_CHAT_STREAM: &str = "/sessions/:session_id/chat/:message_id/stream";
    pub const SESSION_CONFIG: &str = "/sessions/:session_id/config";
    pub const SESSION_MESSAGES: &str = "/sessions/:session_id/messages";
    pub const SESSION_SAVE_AGENT: &str = "/sessions/:session_id/save-agent";

    // Documents
    pub const DOCUMENTS: &str = "/documents";
    pub const DOCUMENTS_SEARCH: &str = "/documents/search";
    pub const DOCUMENT: &str = "/documents/:id";

    // Output schemas
    pub const OUTPUT_SCHEMAS: &str = "/output-schemas";
    pub const OUTPUT_SCHEMA: &str = "/output-schemas/:id";

    // Prompt templates
    pub const PROMPT_TEMPLATES: &str = "/prompt-templates";
    pub const PROMPT_TEMPLATE: &str = "/prompt-templates/:id";

    // Archetypes
    pub const ARCHETYPES: &str = "/archetypes";

    // Workflows
    pub const WORKFLOWS: &str = "/workflows";
    pub const WORKFLOW: &str = "/workflows/:id";
    pub const WORKFLOW_STEPS: &str = "/workflows/:id/steps";
    pub const WORKFLOW_STEP: &str = "/workflows/:wid/steps/:sid";
    pub const WORKFLOW_EDGES: &str = "/workflows/:id/edges";
    pub const WORKFLOW_EDGE: &str = "/workflows/:wid/edges/:eid";
    pub const WORKFLOW_STEP_DOCUMENTS: &str = "/workflows/:wid/steps/:sid/documents";
    pub const WORKFLOW_RUN: &str = "/workflows/:id/run";
    pub const WORKFLOW_EXECUTIONS: &str = "/workflows/:id/executions";
    pub const WORKFLOW_STEP_CHAT_SESSION: &str = "/workflows/:wid/steps/:sid/chat/session";
    pub const WORKFLOW_STEP_CHAT_MESSAGES: &str = "/workflows/:wid/steps/:sid/chat/messages";
    pub const WORKFLOW_STEP_CHAT_DEBUG: &str = "/workflows/:wid/steps/:sid/chat/debug";
    pub const WORKFLOW_STEP_CONFIG: &str = "/workflows/:wid/steps/:sid/config";
    pub const WORKFLOW_STEP_LAST_RUN: &str = "/workflows/:wid/steps/:sid/last-run";
    pub const WORKFLOW_NOTES: &str = "/workflows/:id/notes";

    // Workshop (node-by-node execution)
    pub const WORKFLOW_WORKSHOP: &str = "/workflows/:id/workshop";
    pub const WORKFLOW_WORKSHOP_STEP_EXECUTE: &str =
        "/workflows/:id/workshop/steps/:step_id/execute";

    // Execution History (per-step results for specific runs)
    pub const WORKFLOW_EXECUTION_STEPS: &str = "/workflows/:wid/executions/:eid/steps";
    pub const WORKFLOW_EXECUTION_STEP: &str = "/workflows/:wid/executions/:eid/steps/:sid";

    // Run Templates (frozen workflow snapshots)
    pub const WORKFLOW_TEMPLATES: &str = "/workflows/:id/templates";
    pub const WORKFLOW_TEMPLATE: &str = "/workflows/:id/templates/:template_id";
    pub const WORKFLOW_REBASE: &str = "/workflows/:id/rebase";

    // Workflow Collections
    pub const COLLECTIONS: &str = "/collections";
    pub const COLLECTION: &str = "/collections/:id";
    pub const COLLECTION_RUN: &str = "/collections/:id/run";
    pub const COLLECTION_RUN_STATUS: &str = "/collections/runs/:run_id/status";

    // Pipeline stage members
    pub const PIPELINE_STAGE_MEMBERS: &str = "/pipelines/:pid/stages/:num/members";
    pub const PIPELINE_STAGE_MEMBER: &str = "/pipelines/:pid/stages/:num/members/:mid";

    // Agent executions
    pub const AGENT_EXECUTIONS: &str = "/agent-executions";
    pub const AGENT_EXECUTION: &str = "/agent-executions/:id";
    pub const AGENT_EXECUTION_MESSAGES: &str = "/agent-executions/:id/messages";
    pub const AGENT_EXECUTION_MESSAGE_STREAM: &str =
        "/agent-executions/:id/messages/:stream_id/stream";
    pub const AGENT_EXECUTION_APPROVE: &str = "/agent-executions/:id/approve";
    pub const AGENT_EXECUTION_EXEMPLARY: &str = "/agent-executions/:id/exemplary";

    // Costs
    pub const COSTS: &str = "/costs";

    // Results
    pub const RESULTS: &str = "/results";
    pub const RESULT: &str = "/results/:id";

    // Stats
    pub const STATS: &str = "/stats";

    // Tool routers
    pub const TOOL_ROUTERS: &str = "/tool-routers";
    pub const TOOL_ROUTER: &str = "/tool-routers/:id";
    pub const TOOL_ROUTER_TOOLS: &str = "/tool-routers/:id/tools";

    // Router modes
    pub const ROUTER_MODES: &str = "/tool-routers/:router_id/modes";
    pub const ROUTER_MODE: &str = "/router-modes/:id";
    pub const ROUTER_MODE_TOOLS: &str = "/router-modes/:id/tools";

    // Session context & requests
    pub const SESSION_CONTEXT: &str = "/sessions/:session_id/context";
    pub const SESSION_REQUESTS: &str = "/sessions/:session_id/requests";

    // Context response
    pub const CONTEXT_RESPONSE: &str = "/context-response";

    // Rooms
    pub const ROOMS: &str = "/rooms";
    pub const ROOM: &str = "/rooms/:id";
    pub const PIPELINE_ROOMS: &str = "/pipelines/:id/rooms";
    pub const ROOM_MEMBERS: &str = "/rooms/:id/members";
    pub const ROOM_MEMBER: &str = "/rooms/:id/members/:agent_id";
    pub const ROOM_SESSIONS: &str = "/rooms/:id/sessions";
    pub const ROOM_SESSION: &str = "/room-sessions/:id";
    pub const ROOM_SESSION_MESSAGES: &str = "/room-sessions/:id/messages";
    pub const ROOM_SESSION_TRANSCRIPT: &str = "/room-sessions/:id/transcript";
    pub const ROOM_SESSION_CLOSE: &str = "/room-sessions/:id/close";
    pub const ROOM_SESSION_OUTPUTS: &str = "/room-sessions/:id/outputs";

    // Step Ports
    pub const STEP_INPUTS: &str = "/workflows/:wid/steps/:sid/inputs";
    pub const STEP_INPUT: &str = "/workflows/:wid/steps/:sid/inputs/:pid";
    pub const STEP_OUTPUTS: &str = "/workflows/:wid/steps/:sid/outputs";
    pub const STEP_OUTPUT: &str = "/workflows/:wid/steps/:sid/outputs/:pid";

    // Document Definitions
    pub const STEP_DOCUMENT_DEFS: &str = "/workflows/:wid/steps/:sid/document-defs";
    pub const STEP_DOCUMENT_DEF: &str = "/workflows/:wid/steps/:sid/document-defs/:did";

    // Agent Roster
    pub const STEP_AGENT_ROSTER: &str = "/workflows/:wid/steps/:sid/agent-roster";
    pub const STEP_ROSTER_AGENT: &str = "/workflows/:wid/steps/:sid/agent-roster/:rid";

    // Room Step Members (design-time)
    pub const STEP_ROOM_MEMBERS: &str = "/workflows/:wid/steps/:sid/room-members";

    // Routing Rules
    pub const STEP_ROUTING_RULES: &str = "/workflows/:wid/steps/:sid/routing-rules";
    pub const STEP_ROUTING_RULE: &str = "/workflows/:wid/steps/:sid/routing-rules/:rid";

    // System Config
    pub const SYSTEM_CONFIGS: &str = "/system-config";
    pub const SYSTEM_CONFIG: &str = "/system-config/:key";

    // Protocols
    pub const PROTOCOL_TYPES: &str = "/protocols/types";
    pub const PROTOCOLS: &str = "/protocols";
    pub const PROTOCOL: &str = "/protocols/:id";
    pub const PROTOCOL_PORTS: &str = "/protocols/:id/ports";
    pub const PROTOCOL_PORT: &str = "/protocols/:protocol_id/ports/:port_id";
    pub const PROTOCOL_PREVIEW: &str = "/protocols/:id/preview";
    pub const PROTOCOL_APPLY: &str = "/protocols/:id/apply/:step_id";
    pub const PROTOCOL_UNAPPLY: &str = "/protocols/:protocol_id/unapply/:step_id";
    pub const PROTOCOL_DOCUMENT_DEFS: &str = "/protocols/:id/document-defs";
    pub const PROTOCOL_DOCUMENT_DEF: &str = "/protocols/:pid/document-defs/:did";
    pub const PROTOCOL_EXECUTIONS: &str = "/protocols/:id/executions";

    // WebSocket
    pub const WS: &str = "/ws";
}

// ── Ollama / Local LLM ────────────────────────────────────────────────────

/// Environment toggle for Ollama provider initialization.
pub const ENV_OLLAMA_ENABLED: &str = "NEXOR_OLLAMA_ENABLED";
/// Base URL for the Ollama API (default: localhost:11434).
pub const ENV_OLLAMA_BASE_URL: &str = "OLLAMA_BASE_URL";
/// Model to use with Ollama (required when enabled).
pub const ENV_OLLAMA_MODEL: &str = "OLLAMA_MODEL";
/// Default Ollama API base URL.
pub const OLLAMA_DEFAULT_BASE_URL: &str = "http://localhost:11434";
/// Default timeout for Ollama requests (local models are slower).
pub const OLLAMA_DEFAULT_TIMEOUT_SECS: u64 = 300;

// ── Grok / xAI ──────────────────────────────────────────────────────────────

/// Base URL for the xAI API.
pub const XAI_DEFAULT_BASE_URL: &str = "https://api.x.ai";
/// Model optimized for agentic search with server-side tool use.
pub const XAI_RESEARCH_MODEL: &str = "grok-4-1-fast-non-reasoning";
/// Timeout for research requests (server-side search can take a while).
pub const XAI_RESEARCH_TIMEOUT_SECS: u64 = 120;
/// Max output tokens for research responses.
pub const XAI_RESEARCH_MAX_TOKENS: u32 = 4096;
/// Max agentic search turns Grok can take per request.
pub const XAI_RESEARCH_MAX_SEARCH_TURNS: u32 = 10;

// ── True Context Distiller ────────────────────────────────────────────────

/// Model used for the True Context distiller pre-pass.
/// Swap to XAI_RESEARCH_MODEL or any model ID to change the distiller backend.
pub const DISTILLER_MODEL: &str = MODEL_HAIKU;
/// Max output tokens for the distiller response.
pub const DISTILLER_MAX_TOKENS: u32 = 256;
/// Max input characters from chat history fed to the distiller.
pub const DISTILLER_MAX_INPUT_CHARS: usize = 4_000;
/// Max recent messages to feed the distiller.
pub const DISTILLER_MAX_MESSAGES: usize = 15;
/// Temperature for distiller (low — factual summarization).
pub const DISTILLER_TEMPERATURE: f32 = 0.2;

// ── Sandbox Defaults ────────────────────────────────────────────────────────

pub const SANDBOX_DEFAULT_IMAGE: &str = "alpine:latest";
pub const SANDBOX_DEFAULT_MEMORY: &str = "512m";
pub const SANDBOX_DEFAULT_CPUS: &str = "1.0";
pub const SANDBOX_COMPUTE_MEMORY: &str = "2g";
pub const SANDBOX_COMPUTE_CPUS: &str = "2.0";
pub const SANDBOX_READONLY_MEMORY: &str = "128m";
pub const SANDBOX_READONLY_CPUS: &str = "0.5";

// ── Container Defaults (persistent agent containers) ──────────────────────

pub const CONTAINER_DEFAULT_IMAGE: &str = "nexor-agent:latest";
pub const CONTAINER_DEFAULT_MEMORY: &str = "2g";
pub const CONTAINER_DEFAULT_CPUS: &str = "2.0";
pub const CONTAINER_COMMAND_TIMEOUT_SECS: u64 = 300;
pub const CONTAINER_NAME_PREFIX: &str = "nexor-step";
/// Maximum bytes for stdout/stderr from a single container command (10 MB).
pub const CONTAINER_MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024;
/// Max age in seconds before orphaned containers are reaped at startup (1 hour).
pub const CONTAINER_REAPER_MAX_AGE_SECS: u64 = 3600;
/// Interval in seconds between periodic container reaper runs (5 minutes).
pub const CONTAINER_REAPER_INTERVAL_SECS: u64 = 300;
/// Maximum seconds to wait for running executions to drain during shutdown.
pub const SHUTDOWN_DRAIN_TIMEOUT_SECS: u64 = 30;
/// Maximum seconds for the entire container creation flow (create + clone + config).
pub const CONTAINER_CREATE_TIMEOUT_SECS: u64 = 600;
/// Maximum concurrent container creation operations (semaphore permits).
pub const CONTAINER_MAX_CONCURRENT_CREATES: usize = 10;
/// Maximum retry attempts for transient container creation failures.
pub const CONTAINER_RETRY_MAX_ATTEMPTS: u32 = 2;
/// Initial retry backoff delay for container creation (ms).
pub const CONTAINER_RETRY_INITIAL_BACKOFF_MS: u64 = 1000;
/// Maximum retry backoff delay for container creation (seconds).
pub const CONTAINER_RETRY_MAX_BACKOFF_SECS: u64 = 10;

// ── VPN / WireGuard Defaults ──────────────────────────────────────────────

/// Docker image for the WireGuard VPN sidecar container.
pub const VPN_SIDECAR_IMAGE: &str = "lscr.io/linuxserver/wireguard:latest";
/// Timeout in seconds for the VPN tunnel health check.
pub const VPN_HEALTH_CHECK_TIMEOUT_SECS: u64 = 30;
/// Interval in seconds between VPN health check polls.
pub const VPN_HEALTH_CHECK_INTERVAL_SECS: u64 = 2;
/// Name prefix for VPN sidecar containers.
pub const VPN_SIDECAR_NAME_PREFIX: &str = "nexor-vpn";
/// HTTP request timeout for wg-easy API calls (seconds).
pub const WGEASY_API_TIMEOUT_SECS: u64 = 10;
/// Initial retry backoff delay for VPN API calls (ms).
pub const VPN_RETRY_INITIAL_BACKOFF_MS: u64 = 200;
/// Maximum retry backoff delay for VPN API calls (seconds).
pub const VPN_RETRY_MAX_BACKOFF_SECS: u64 = 5;
/// Maximum number of retries for VPN API calls.
pub const VPN_RETRY_MAX_ATTEMPTS: u32 = 3;
/// Max age in seconds before orphaned VPN sidecars/peers are reaped at startup (1 hour).
pub const VPN_REAPER_MAX_AGE_SECS: u64 = 3600;
/// Default WireGuard gateway IP for connectivity health check.
pub const VPN_HEALTH_CHECK_GATEWAY: &str = "10.8.0.1";

/// iptables kill switch: blocks ALL traffic except through wg0, loopback,
/// and the WireGuard UDP handshake. Prevents traffic leak if tunnel drops.
pub const VPN_KILL_SWITCH_SCRIPT: &str = r#"
iptables -P OUTPUT DROP
iptables -A OUTPUT -o wg0 -j ACCEPT
iptables -A OUTPUT -o lo -j ACCEPT
iptables -A OUTPUT -p udp --dport 51820 -j ACCEPT
iptables -A OUTPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
iptables -P INPUT DROP
iptables -A INPUT -i wg0 -j ACCEPT
iptables -A INPUT -i lo -j ACCEPT
iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
"#;

/// Docker log driver for VPN sidecar. "none" prevents handshake metadata in logs.
pub const VPN_SIDECAR_LOG_DRIVER: &str = "none";

/// Timeout for external IP leak check (seconds).
pub const VPN_IP_LEAK_CHECK_TIMEOUT_SECS: u64 = 5;

/// URL for public IP verification through VPN tunnel.
pub const VPN_IP_CHECK_URL: &str = "https://api.ipify.org";

/// Interval in seconds between VPN watchdog health checks during execution.
pub const VPN_WATCHDOG_INTERVAL_SECS: u64 = 5;

/// Consecutive health check failures before the watchdog considers the tunnel dead.
pub const VPN_WATCHDOG_MAX_FAILURES: u32 = 3;
