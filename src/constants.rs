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
