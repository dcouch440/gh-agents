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

// ── Channel Buffer Sizes ────────────────────────────────────────────────────

/// Buffer size for agent command/response channels.
pub const CHANNEL_AGENT: usize = 32;
/// Buffer size for the orchestrator message queue.
pub const CHANNEL_ORCHESTRATOR: usize = 100;
/// Buffer size for broadcast channels (feed, task, agent, session updates).
pub const CHANNEL_BROADCAST: usize = 100;
/// Buffer size for high-throughput broadcast channels (feed, routing).
pub const CHANNEL_BROADCAST_HIGH: usize = 256;
/// Buffer size for low-throughput broadcast channels (agents, sessions).
pub const CHANNEL_BROADCAST_LOW: usize = 64;

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

    // Tools
    pub const TOOLS: &str = "/tools";
    pub const TOOL: &str = "/tools/:id";

    // Pipeline stages
    pub const PIPELINE_STAGE_RENDER: &str = "/pipelines/:id/stages/:stage_number/render";
    pub const PIPELINE_STAGE_SIDE_TASKS: &str = "/pipelines/:id/stages/:stage_number/side-tasks";
    pub const PIPELINE_STAGE_SIDE_TASK: &str = "/pipelines/:id/stages/:stage_number/side-tasks/:side_task_id";

    // Pipeline runs
    pub const PIPELINE_RUNS: &str = "/pipeline-runs";
    pub const PIPELINE_RUN: &str = "/pipeline-runs/:run_id";
    pub const PIPELINE_RUN_APPROVE: &str = "/pipeline-runs/:run_id/approve";

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

    // Stats
    pub const STATS: &str = "/stats";

    // Context response
    pub const CONTEXT_RESPONSE: &str = "/context-response";

    // WebSocket
    pub const WS: &str = "/ws";
}

// ── Grok / xAI ──────────────────────────────────────────────────────────────

/// Base URL for the xAI API.
pub const XAI_DEFAULT_BASE_URL: &str = "https://api.x.ai";
/// Model optimized for agentic search with server-side tool use.
pub const XAI_RESEARCH_MODEL: &str = "grok-4-1-fast";
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
