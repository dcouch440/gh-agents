//! App-wide constants for nexor.

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

// ── Channel Buffer Sizes ────────────────────────────────────────────────────

/// Buffer size for agent command/response channels.
pub const CHANNEL_AGENT: usize = 32;
/// Buffer size for the orchestrator message queue.
pub const CHANNEL_ORCHESTRATOR: usize = 100;
/// Buffer size for broadcast channels (feed, task, agent, session updates).
pub const CHANNEL_BROADCAST: usize = 100;

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

// ── Sandbox Defaults ────────────────────────────────────────────────────────

pub const SANDBOX_DEFAULT_IMAGE: &str = "alpine:latest";
pub const SANDBOX_DEFAULT_MEMORY: &str = "512m";
pub const SANDBOX_DEFAULT_CPUS: &str = "1.0";
pub const SANDBOX_COMPUTE_MEMORY: &str = "2g";
pub const SANDBOX_COMPUTE_CPUS: &str = "2.0";
pub const SANDBOX_READONLY_MEMORY: &str = "128m";
pub const SANDBOX_READONLY_CPUS: &str = "0.5";
