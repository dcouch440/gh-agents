//! Protocol configuration and role definitions loaded from `config/`.
//!
//! Layout:
//! - `config/assistant/`          — shared conversational layer (all protocols)
//! - `config/designer/`           — shared Agent Designer pre-lifecycle
//! - `config/archetype/<name>/`   — archetype-specific (archetype, builder, agent)
//! - `config/services/<name>/`    — utility LLM services (belief extraction, etc.)
//! - `config/system/`             — platform config synced to DB at runtime
//!
//! All content is embedded at compile time via `include_str!()`. YAML configs
//! are parsed once on first access via `Lazy`. Role definitions are resolved
//! at runtime by substituting `{{.var}}` template variables.

use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::server::hub::protocols::template_resolve::resolve_template;
use crate::server::hub::protocols::text_utils::{collapse_blank_lines, strip_comments};

// ---------------------------------------------------------------------------
// Config types (from config.yaml)
// ---------------------------------------------------------------------------

/// Top-level protocol configuration parsed from `config.yaml`.
#[derive(Debug, Deserialize)]
pub struct ProtocolConfig {
    pub agents: HashMap<String, AgentConfig>,
}

/// Per-role agent configuration: model selection and execution parameters.
#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    pub model_id: String,
    pub max_tokens: u32,
    pub temperature: f32,
    #[serde(default = "default_max_rounds")]
    pub max_rounds: u32,
    #[serde(default = "default_context_budget")]
    pub context_budget: usize,
    /// Reasoning effort for providers that support it.
    ///
    /// Omitted means "send no `reasoning_effort`", so the provider applies its
    /// own default. On the DeepInfra profile all three tiers resolve to the
    /// same model, so this is what actually separates an orchestrator agent
    /// from a utility one.
    #[serde(default)]
    pub effort: Option<crate::llm::ReasoningEffort>,
}

fn default_max_rounds() -> u32 {
    1
}

fn default_context_budget() -> usize {
    480_000
}

impl ProtocolConfig {
    /// Get the configuration for a specific agent role.
    ///
    /// # Panics
    /// Panics if the role is not defined in the config. This is intentional —
    /// a missing role is a configuration bug caught at startup.
    pub fn agent(&self, role: &str) -> &AgentConfig {
        self.agents
            .get(role)
            .unwrap_or_else(|| panic!("Unknown agent role '{role}' in protocol config"))
    }

    /// Resolve tier markers (`tier:1`, `tier:2`, `tier:3`) in `model_id` fields
    /// to the concrete model IDs from the active provider profile in `constants.rs`.
    ///
    /// Called once during `Lazy` initialization so every consumer sees real model IDs.
    fn resolve_model_tiers(&mut self) {
        for agent in self.agents.values_mut() {
            agent.model_id = match agent.model_id.as_str() {
                "tier:1" => crate::constants::MODEL_TIER1.to_string(),
                "tier:2" => crate::constants::MODEL_TIER2.to_string(),
                "tier:3" => crate::constants::MODEL_TIER3.to_string(),
                other => other.to_string(),
            };
        }
    }
}

// ---------------------------------------------------------------------------
// Role definition types
// ---------------------------------------------------------------------------

/// Raw compile-time content for a role's LLM contract.
///
/// - `system` — `system.md`: persona and instructions (system prompt)
/// - `prompt` — `prompt.md`: user message template with `{{.var}}` placeholders
/// - `response` — `response.json`: optional JSON schema for structured output
pub struct RoleDefinition {
    pub system: &'static str,
    pub prompt: &'static str,
    pub response: Option<&'static str>,
}

/// Strip comments from an embedded prompt, once per distinct file.
///
/// The one chokepoint every prompt is sent through. Prompt files carry their
/// own reasoning inline and it is billed on every call unless it is removed
/// here — `config/runtime_agent/system.md` goes out once per agent per step.
///
/// Keyed by the raw text's address, which is stable because every caller
/// passes a `&'static str` from `include_str!`. `stripped_prompts_are_cached`
/// covers that assumption.
fn prompt_text(raw: &'static str) -> &'static str {
    static CACHE: OnceLock<Mutex<HashMap<usize, &'static str>>> = OnceLock::new();

    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = raw.as_ptr() as usize;

    let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(text) = guard.get(&key) {
        return text;
    }
    // Leaked deliberately: prompts are `include_str!` constants, so there is
    // one per file for the life of the process and nothing to reclaim.
    let text: &'static str = Box::leak(strip_comments(raw).into_boxed_str());
    guard.insert(key, text);
    text
}

/// Fully resolved LLM context for a protocol role.
///
/// Produced by [`RoleDefinition::resolve`] after template variable substitution.
pub struct ProtocolContext {
    /// Resolved system prompt (`system.md` with all variables substituted).
    pub system_prompt: String,
    /// Resolved user prompt (`prompt.md` with all variables substituted).
    pub user_prompt: String,
    /// Parsed response schema (`response.json` with variables substituted), if any.
    pub response_schema: Option<serde_json::Value>,
}

impl RoleDefinition {
    /// The system prompt as the model receives it — comments stripped.
    ///
    /// Use this, never `.system`, anywhere text is about to be sent. The raw
    /// field stays public because the template-variable audit has to scan what
    /// is actually written in the file, comments included.
    pub fn system_text(&self) -> &'static str {
        prompt_text(self.system)
    }

    /// The user-message template as the model receives it — comments stripped.
    ///
    /// Same contract as [`system_text`](Self::system_text), for `prompt.md`.
    pub fn prompt_text(&self) -> &'static str {
        prompt_text(self.prompt)
    }

    /// Resolve all `{{.var}}` template variables and produce a ready-to-use context.
    ///
    /// Empty variable values produce blank lines that are collapsed automatically.
    /// Unknown variables are left as-is.
    ///
    /// Comments are stripped from the template, before substitution, and never
    /// from the resolved text. Variable values carry content this process did
    /// not write — chat messages, board state, the file hunks handed to
    /// `MERGE_HUNK` — and stripping after substitution treats an `<!--` in one
    /// of those as a prompt comment: its content is deleted from the prompt
    /// silently, or, with no `-->` after it, `strip_comments` panics the
    /// request. A hunk is a slice of lines, so an HTML comment split across the
    /// slice boundary is unterminated by construction.
    pub fn resolve(&self, vars: &HashMap<String, String>) -> ProtocolContext {
        let system_prompt = collapse_blank_lines(&resolve_template(self.system_text(), vars));
        let user_prompt = collapse_blank_lines(&resolve_template(self.prompt_text(), vars));
        let response_schema = self.response.map(|r| {
            let resolved = resolve_template(r, vars);
            serde_json::from_str(&resolved)
                .expect("Failed to parse response.json after variable resolution")
        });
        ProtocolContext {
            system_prompt,
            user_prompt,
            response_schema,
        }
    }
}

// ---------------------------------------------------------------------------
// Template variable names — single source of truth for all {{.X}} keys
// ---------------------------------------------------------------------------

pub mod vars {
    /// Variables originating from human input.
    pub mod user {
        pub const PROMPT: &str = "User.prompt";
        pub const GATEKEEPER_INPUT: &str = "User.gatekeeper_input";
    }

    /// Variables for workforce runtime agent prompts.
    pub mod workforce {
        pub const AGENT_NAME: &str = "Workforce.agent_name";
        pub const ROLE_DESCRIPTION: &str = "Workforce.role_description";
    }

    /// Variables for chat belief extraction prompts.
    pub mod chat_belief {
        pub const NODE_NAME: &str = "ChatBelief.node_name";
        pub const NODE_ARCHETYPE: &str = "ChatBelief.node_archetype";
        pub const CONVERSATION: &str = "ChatBelief.conversation";
        pub const BOARD_BELIEFS: &str = "ChatBelief.board_beliefs";
    }

    /// Variables for question extraction prompts.
    pub mod question_extraction {
        pub const NODE_NAME: &str = "QuestionExtraction.node_name";
        pub const CONVERSATION: &str = "QuestionExtraction.conversation";
    }

    /// Variables for workspace merge conflict resolution prompts.
    pub mod merge {
        pub const FILE_PATH: &str = "Merge.file_path";
        pub const FILE_TYPE: &str = "Merge.file_type";
        pub const LINE_RANGE: &str = "Merge.line_range";
        pub const CONTEXT_BLOCK: &str = "Merge.context_block";
        pub const BASE_HUNK: &str = "Merge.base_hunk";
        pub const VERSION_A_HUNK: &str = "Merge.version_a_hunk";
        pub const VERSION_B_HUNK: &str = "Merge.version_b_hunk";
        pub const STEP_A_NAME: &str = "Merge.step_a_name";
        pub const STEP_A_DESCRIPTION: &str = "Merge.step_a_description";
        pub const STEP_B_NAME: &str = "Merge.step_b_name";
        pub const STEP_B_DESCRIPTION: &str = "Merge.step_b_description";
        pub const DIFF_SUMMARY: &str = "Merge.diff_summary";
        pub const CONTENT_A: &str = "Merge.content_a";
        pub const CONTENT_B: &str = "Merge.content_b";
    }

    /// Variables assembled by the platform (config, context, runtime state).
    pub mod system {
        pub const DOC_NAME: &str = "System.doc_name";
        pub const SELECTED_CONTEXT: &str = "System.selected_context";
        pub const REQUESTED_DOCUMENTS: &str = "System.requested_documents";
        pub const AVAILABLE_CAPABILITIES: &str = "System.available_capabilities";
        pub const CONTEXT_DOCUMENTS_INSTRUCTION: &str = "System.context_documents_instruction";
        pub const BOARD_STATE: &str = "System.board_state";
        pub const GRAPH_CONTEXT: &str = "System.graph_context";
        pub const BOARD_CONTEXT: &str = "System.board_context";
        pub const ARCHETYPE_BLOCK: &str = "System.archetype_block";
        pub const PLAN: &str = "System.plan";
        pub const BOARD_OVERVIEW: &str = "System.board_overview";
        pub const DISPATCH_STATUS: &str = "System.dispatch_status";
        pub const RUN_CONTEXT: &str = "System.run_context";
    }
}

// ---------------------------------------------------------------------------
// Config statics — parsed once on first access
// ---------------------------------------------------------------------------

/// Parse a protocol config YAML and resolve tier markers to concrete model IDs.
fn load_protocol_config(yaml: &str, name: &str) -> ProtocolConfig {
    let mut config: ProtocolConfig =
        serde_yaml::from_str(yaml).unwrap_or_else(|e| panic!("Failed to parse {name}: {e}"));
    config.resolve_model_tiers();
    config
}

/// Shared node assistant config (conversational layer, all protocols).
pub static ASSISTANT: Lazy<ProtocolConfig> = Lazy::new(|| {
    load_protocol_config(
        include_str!("../../config/assistant/config.yaml"),
        "config/assistant/config.yaml",
    )
});

/// Workforce runtime agent config.
pub static WORKFORCE: Lazy<ProtocolConfig> = Lazy::new(|| {
    load_protocol_config(
        include_str!("../../config/runtime_agent/config.yaml"),
        "config/runtime_agent/config.yaml",
    )
});

/// Manager assistant config (L1 — conversational layer for the manager node).
pub static MANAGER_ASSISTANT: Lazy<ProtocolConfig> = Lazy::new(|| {
    load_protocol_config(
        include_str!("../../config/manager/assistant/config.yaml"),
        "config/manager/assistant/config.yaml",
    )
});

/// Manager builder config (L2 — background topology + dispatch agent).
pub static MANAGER_BUILDER: Lazy<ProtocolConfig> = Lazy::new(|| {
    load_protocol_config(
        include_str!("../../config/manager/builder/config.yaml"),
        "config/manager/builder/config.yaml",
    )
});

/// System node agent config (replaces builder + designer).
pub static SYSTEM_NODE_AGENT: Lazy<ProtocolConfig> = Lazy::new(|| {
    load_protocol_config(
        include_str!("../../config/system_agent/config.yaml"),
        "config/system_agent/config.yaml",
    )
});

/// Workspace merge conflict resolution config.
pub static MERGE: Lazy<ProtocolConfig> = Lazy::new(|| {
    load_protocol_config(
        include_str!("../../config/services/merge/config.yaml"),
        "config/services/merge/config.yaml",
    )
});

/// Workflow agent config (designs workflow topology via conversation).
pub static WORKFLOW_AGENT: Lazy<ProtocolConfig> = Lazy::new(|| {
    load_protocol_config(
        include_str!("../../config/workflow_agent/config.yaml"),
        "config/workflow_agent/config.yaml",
    )
});

// ---------------------------------------------------------------------------
// Role statics — compile-time embedded content
// ---------------------------------------------------------------------------

pub mod roles {
    use super::RoleDefinition;

    /// Shared assistant base template (all protocols inject archetype blocks).
    pub static ASSISTANT_BASE: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/assistant/system.md"),
        prompt: include_str!("../../config/assistant/prompt.md"),
        response: None,
    };

    /// Workforce archetype block, injected via `{{.System.archetype_block}}`.
    pub const WORKFORCE_ARCHETYPE: &str = include_str!("../../config/archetype.md");

    /// Raw prompt files for the two roles that are a bare system prompt with
    /// no `prompt.md` and no template variables, so they have no
    /// `RoleDefinition` to hang `system_text()` off.
    ///
    /// Private on purpose. Both used to be `pub const` read straight into a
    /// request, which is how they shipped their own comments to the model on
    /// every call. The accessors below are the only way to reach them, so a
    /// fourth prompt added here cannot repeat it.
    mod raw {
        pub(super) const SYSTEM_NODE_AGENT_SYSTEM: &str =
            include_str!("../../config/system_agent/system.md");
        pub(super) const WORKFLOW_AGENT_SYSTEM: &str =
            include_str!("../../config/workflow_agent/system.md");
    }

    /// System node agent system prompt (designs runtime agent teams).
    pub fn system_node_agent_system() -> &'static str {
        super::prompt_text(raw::SYSTEM_NODE_AGENT_SYSTEM)
    }

    /// Workflow agent system prompt (designs workflow topology via conversation).
    pub fn workflow_agent_system() -> &'static str {
        super::prompt_text(raw::WORKFLOW_AGENT_SYSTEM)
    }

    /// Workforce runtime agent prompt template.
    pub static WORKFORCE_AGENT: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/runtime_agent/system.md"),
        prompt: include_str!("../../config/runtime_agent/prompt.md"),
        response: None,
    };

    /// Chat belief extractor: reads chat conversations and extracts user beliefs.
    pub static BELIEF_EXTRACTOR: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/services/belief_extraction/system.md"),
        prompt: include_str!("../../config/services/belief_extraction/prompt.md"),
        response: Some(include_str!(
            "../../config/services/belief_extraction/response.json"
        )),
    };

    /// Run results summarizer: distills step output into a 2-4 sentence summary.
    pub const RUN_RESULTS_SUMMARIZER: &str =
        include_str!("../../config/services/run_results/system.md");

    /// Board overview summarizer: distills all assistant notes into a board-wide summary.
    pub const BOARD_OVERVIEW_SUMMARIZER: &str =
        include_str!("../../config/services/board_overview/system.md");

    /// Manager assistant base template (L1 — conversational layer).
    pub static MANAGER_ASSISTANT_BASE: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/manager/assistant/system.md"),
        prompt: include_str!("../../config/manager/assistant/prompt.md"),
        response: None,
    };

    /// Manager builder system prompt (L2 — topology + dispatch agent).
    pub const MANAGER_BUILDER_SYSTEM: &str = include_str!("../../config/manager/builder/system.md");

    /// Question extraction (Tier 3 compresses node responses into status + question).
    pub static QUESTION_EXTRACTOR: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/services/question_extraction/system.md"),
        prompt: include_str!("../../config/services/question_extraction/prompt.md"),
        response: None,
    };

    /// Workspace merge: standard conflict hunk resolution.
    pub static MERGE_HUNK: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/services/merge/system.md"),
        prompt: include_str!("../../config/services/merge/hunk_prompt.md"),
        response: None,
    };

    /// Workspace merge: delete-modify conflict resolution.
    pub static MERGE_DELETE_MODIFY: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/services/merge/system.md"),
        prompt: include_str!("../../config/services/merge/delete_modify_prompt.md"),
        response: None,
    };

    /// Workspace merge: new-new conflict resolution (both agents created same file).
    pub static MERGE_NEW_NEW: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/services/merge/system.md"),
        prompt: include_str!("../../config/services/merge/new_new_prompt.md"),
        response: None,
    };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::*;

    // ── Config tests ─────────────────────────────────────────────────────

    #[test]
    #[should_panic(expected = "Unknown agent role")]
    fn unknown_role_panics() {
        ASSISTANT.agent("nonexistent");
    }

    // ── RoleDefinition / ProtocolContext tests ───────────────────────────

    #[test]
    fn resolve_substitutes_system_and_prompt() {
        let role = RoleDefinition {
            system: "Hello {{.name}}, you are a {{.role}}.",
            prompt: "Task: {{.task}}",
            response: None,
        };
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        vars.insert("role".to_string(), "researcher".to_string());
        vars.insert("task".to_string(), "find data".to_string());

        let ctx = role.resolve(&vars);
        assert_eq!(ctx.system_prompt, "Hello Alice, you are a researcher.");
        assert_eq!(ctx.user_prompt, "Task: find data");
        assert!(ctx.response_schema.is_none());
    }

    #[test]
    fn resolve_empty_var_collapses_blank_lines() {
        let role = RoleDefinition {
            system: "line1",
            prompt: "{{.Workforce.agent_name}}\n\n{{.System.selected_context}}",
            response: None,
        };
        let mut vars = HashMap::new();
        vars.insert("Workforce.agent_name".to_string(), "Analyst".to_string());
        vars.insert("System.selected_context".to_string(), String::new());

        let ctx = role.resolve(&vars);
        assert_eq!(ctx.user_prompt, "Analyst");
    }

    #[test]
    fn resolve_response_json_parses() {
        let role = RoleDefinition {
            system: "sys",
            prompt: "prompt",
            response: Some(r#"{"type": "object", "required": [{{.Protocol.fields}}]}"#),
        };
        let mut vars = HashMap::new();
        vars.insert(
            "Protocol.fields".to_string(),
            r#""name", "age""#.to_string(),
        );

        let ctx = role.resolve(&vars);
        let schema = ctx.response_schema.unwrap();
        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 2);
    }

    #[test]
    fn resolve_no_response_returns_none() {
        let role = RoleDefinition {
            system: "sys",
            prompt: "prompt",
            response: None,
        };
        let ctx = role.resolve(&HashMap::new());
        assert!(ctx.response_schema.is_none());
    }

    /// Every prompt reaches the model through the stripper.
    ///
    /// The failure this exists to catch is additive, not a regression: three
    /// prompts once bypassed processing entirely and shipped their own
    /// reasoning to the model on every call. Nothing structurally prevents a
    /// fourth from being added the same way, so the guarantee is asserted
    /// rather than assumed. A prompt added to `roles` and not listed here is
    /// the case this misses — keep the list complete.
    #[test]
    fn no_prompt_reaches_the_model_with_comments_in_it() {
        let sent: Vec<(&str, &str)> = vec![
            ("system_agent", roles::system_node_agent_system()),
            ("workflow_agent", roles::workflow_agent_system()),
            ("runtime_agent", roles::WORKFORCE_AGENT.system_text()),
            ("assistant", roles::ASSISTANT_BASE.system_text()),
            ("belief_extractor", roles::BELIEF_EXTRACTOR.system_text()),
            (
                "manager_assistant",
                roles::MANAGER_ASSISTANT_BASE.system_text(),
            ),
            ("merge_hunk", roles::MERGE_HUNK.system_text()),
        ];

        for (name, text) in sent {
            assert!(
                !text.contains("<!--"),
                "{name} ships an unstripped comment to the model"
            );
            assert!(!text.is_empty(), "{name} stripped down to nothing");
        }
    }

    /// Every `<tool_call>` in every example is a call the agent could actually
    /// send.
    ///
    /// The three agent prompts argue this at length in their own comments and
    /// have twice regressed to bare-shell examples: `run_command` takes a
    /// `command` STRING, so a heredoc shown outside the JSON envelope depicts a
    /// call that arrives as unparsable arguments or as `run_command {}`. The
    /// examples are the part of a prompt a model copies hardest, so a malformed
    /// one costs a production round-trip the agent cannot correct.
    ///
    /// The nested assertion is the one that actually catches things: the system
    /// node agent has no file tools, so every agent definition it writes goes
    /// through a heredoc inside that JSON string, double-escaped. A swallowed
    /// quote there is invisible on the page and is exactly what
    /// `write_validation_errors` fires on in production.
    #[test]
    fn every_example_tool_call_is_sendable_json() {
        let call_re = Regex::new(r#"(?s)<tool_call name="[^"]+">\s*(\{.*?\})\s*</tool_call>"#)
            .expect("tool_call regex");
        let heredoc_re = Regex::new(r"(?s)<< 'EOF'\n(.*?)\nEOF").expect("heredoc regex");

        let prompts: Vec<(&str, &str)> = vec![
            ("system_agent", roles::system_node_agent_system()),
            ("workflow_agent", roles::workflow_agent_system()),
            ("runtime_agent", roles::WORKFORCE_AGENT.system_text()),
        ];

        for (name, text) in prompts {
            let calls: Vec<_> = call_re.captures_iter(text).collect();
            assert!(
                calls.len() >= 5,
                "{name} has {} example tool calls — the regex stopped matching",
                calls.len()
            );

            for cap in calls {
                let raw = &cap[1];
                let parsed: serde_json::Value = serde_json::from_str(raw)
                    .unwrap_or_else(|e| panic!("{name} has an unsendable tool_call: {e}\n{raw}"));

                let Some(command) = parsed.get("command").and_then(|c| c.as_str()) else {
                    continue;
                };
                for h in heredoc_re.captures_iter(command) {
                    let body = h[1].trim();
                    if !body.starts_with('{') {
                        continue;
                    }
                    serde_json::from_str::<serde_json::Value>(body).unwrap_or_else(|e| {
                        panic!("{name} writes a malformed JSON file in a heredoc: {e}\n{body}")
                    });
                }
            }
        }
    }

    /// Variable values are content this process did not write — chat messages,
    /// board state, the file hunks handed to `MERGE_HUNK`. A `<!--` in one is
    /// not a prompt comment: it must survive substitution intact, and an
    /// unterminated one must not panic the request.
    #[test]
    fn resolve_leaves_comment_markers_in_variable_values_alone() {
        let role = RoleDefinition {
            system: "<!-- why this exists -->\nYou merge files.",
            prompt: "Hunk:\n{{.Merge.base_hunk}}",
            response: None,
        };
        let mut vars = HashMap::new();
        vars.insert(
            "Merge.base_hunk".to_string(),
            "<!-- section opener with no closer\n<div>kept</div>".to_string(),
        );

        let ctx = role.resolve(&vars);

        assert!(!ctx.system_prompt.contains("<!--"), "{}", ctx.system_prompt);
        assert!(ctx.system_prompt.contains("You merge files."));
        assert!(ctx
            .user_prompt
            .contains("<!-- section opener with no closer"));
        assert!(ctx.user_prompt.contains("<div>kept</div>"));
    }

    /// `prompt_text` keys on the raw string's address, which only holds
    /// because every caller passes an `include_str!` constant.
    #[test]
    fn stripped_prompts_are_cached() {
        let first = roles::WORKFORCE_AGENT.system_text();
        let second = roles::WORKFORCE_AGENT.system_text();
        assert!(
            std::ptr::eq(first, second),
            "system_text re-stripped instead of hitting the cache"
        );
    }

    #[test]
    fn all_role_statics_load() {
        assert!(!roles::ASSISTANT_BASE.system.is_empty());
        assert!(!roles::ASSISTANT_BASE.prompt.is_empty());
        assert!(roles::ASSISTANT_BASE.response.is_none());

        assert!(!roles::WORKFORCE_ARCHETYPE.is_empty());
        assert!(!roles::system_node_agent_system().is_empty());
        assert!(!roles::workflow_agent_system().is_empty());

        assert!(!roles::WORKFORCE_AGENT.system.is_empty());
        assert!(!roles::WORKFORCE_AGENT.prompt.is_empty());
        assert!(roles::WORKFORCE_AGENT.response.is_none());

        assert!(!roles::BELIEF_EXTRACTOR.system.is_empty());
        assert!(!roles::BELIEF_EXTRACTOR.prompt.is_empty());
        assert!(roles::BELIEF_EXTRACTOR.response.is_some());

        assert!(!roles::MANAGER_ASSISTANT_BASE.system.is_empty());
        assert!(!roles::MANAGER_ASSISTANT_BASE.prompt.is_empty());
        assert!(roles::MANAGER_ASSISTANT_BASE.response.is_none());

        assert!(!roles::MANAGER_BUILDER_SYSTEM.is_empty());

        assert!(!roles::MERGE_HUNK.system.is_empty());
        assert!(!roles::MERGE_HUNK.prompt.is_empty());
        assert!(roles::MERGE_HUNK.response.is_none());

        assert!(!roles::MERGE_DELETE_MODIFY.system.is_empty());
        assert!(!roles::MERGE_DELETE_MODIFY.prompt.is_empty());

        assert!(!roles::MERGE_NEW_NEW.system.is_empty());
        assert!(!roles::MERGE_NEW_NEW.prompt.is_empty());
    }

    #[test]
    fn assistant_config_parses() {
        let cfg = &*ASSISTANT;
        let assistant = cfg.agent("assistant");
        assert_eq!(assistant.temperature, 0.4);
        assert_eq!(assistant.max_rounds, 15);
        assert_eq!(assistant.context_budget, 480_000);
    }

    #[test]
    fn workforce_config_parses() {
        let cfg = &*WORKFORCE;
        let agent = cfg.agent("agent");
        assert_eq!(agent.temperature, 0.3);
        assert_eq!(agent.max_rounds, 60);
        assert_eq!(agent.context_budget, 950_000);
    }

    #[test]
    fn system_node_agent_config_parses() {
        let cfg = &*SYSTEM_NODE_AGENT;
        let system = cfg.agent("system");
        assert_eq!(system.model_id, crate::constants::MODEL_TIER2);
        assert_eq!(system.temperature, 0.3);
        assert_eq!(system.max_tokens, 8192);
        assert_eq!(system.max_rounds, 30);
        assert_eq!(system.context_budget, 480_000);
    }

    #[test]
    fn manager_assistant_config_parses() {
        let cfg = &*MANAGER_ASSISTANT;
        let assistant = cfg.agent("assistant");
        assert_eq!(assistant.model_id, crate::constants::MODEL_TIER1);
        assert_eq!(assistant.temperature, 0.4);
        assert_eq!(assistant.max_rounds, 15);
        assert_eq!(assistant.context_budget, 480_000);
    }

    #[test]
    fn merge_config_parses() {
        let cfg = &*MERGE;
        let resolver = cfg.agent("resolver");
        assert_eq!(resolver.model_id, crate::constants::MODEL_TIER3);
        assert_eq!(resolver.temperature, 0.0);
        assert_eq!(resolver.max_tokens, 4096);
        assert_eq!(resolver.max_rounds, 1);

        let complex = cfg.agent("complex_resolver");
        assert_eq!(complex.model_id, crate::constants::MODEL_TIER2);
        assert_eq!(complex.temperature, 0.0);
        assert_eq!(complex.max_tokens, 4096);
    }

    #[test]
    fn workflow_agent_config_parses() {
        let cfg = &*WORKFLOW_AGENT;
        let agent = cfg.agent("agent");
        assert!(!agent.model_id.is_empty());
        assert!(agent.temperature >= 0.0);
        assert!(agent.max_tokens > 0);
        assert!(agent.max_rounds > 0);
        assert!(agent.context_budget > 0);
    }

    #[test]
    fn manager_builder_config_parses() {
        let cfg = &*MANAGER_BUILDER;
        let dispatcher = cfg.agent("dispatcher");
        assert_eq!(dispatcher.model_id, crate::constants::MODEL_TIER1);
        assert_eq!(dispatcher.temperature, 0.3);
        assert_eq!(dispatcher.max_tokens, 8192);
        assert_eq!(dispatcher.max_rounds, 5);
        assert_eq!(dispatcher.context_budget, 300_000);
    }

    #[test]
    fn assistant_base_resolves_with_archetype_block() {
        let mut vars = HashMap::new();
        vars.insert(
            vars::system::BOARD_CONTEXT.to_string(),
            "Nodes: A -> B -> [SELECTED] C".to_string(),
        );
        vars.insert(
            vars::system::ARCHETYPE_BLOCK.to_string(),
            roles::WORKFORCE_ARCHETYPE.to_string(),
        );
        vars.insert(vars::system::BOARD_STATE.to_string(), String::new());

        let ctx = roles::ASSISTANT_BASE.resolve(&vars);
        assert!(
            ctx.system_prompt.contains("help the user design this node"),
            "should contain focused identity"
        );
        assert!(
            ctx.system_prompt.contains("Nodes: A -> B -> [SELECTED] C"),
            "should contain board context"
        );
        assert!(
            ctx.system_prompt.contains("<archetype"),
            "should contain workforce block"
        );
    }

    // ── Template variable validation ─────────────────────────────────────

    #[test]
    fn all_template_vars_are_known_constants() {
        use std::collections::HashSet;

        let known: HashSet<&str> = HashSet::from([
            vars::user::PROMPT,
            vars::user::GATEKEEPER_INPUT,
            vars::system::DOC_NAME,
            vars::system::SELECTED_CONTEXT,
            vars::system::REQUESTED_DOCUMENTS,
            vars::system::AVAILABLE_CAPABILITIES,
            vars::system::CONTEXT_DOCUMENTS_INSTRUCTION,
            vars::system::BOARD_STATE,
            vars::system::GRAPH_CONTEXT,
            vars::system::BOARD_CONTEXT,
            vars::system::ARCHETYPE_BLOCK,
            vars::system::PLAN,
            vars::system::BOARD_OVERVIEW,
            vars::system::DISPATCH_STATUS,
            vars::system::RUN_CONTEXT,
            vars::workforce::AGENT_NAME,
            vars::workforce::ROLE_DESCRIPTION,
            vars::chat_belief::NODE_NAME,
            vars::chat_belief::NODE_ARCHETYPE,
            vars::chat_belief::CONVERSATION,
            vars::chat_belief::BOARD_BELIEFS,
            vars::merge::FILE_PATH,
            vars::merge::FILE_TYPE,
            vars::merge::LINE_RANGE,
            vars::merge::CONTEXT_BLOCK,
            vars::merge::BASE_HUNK,
            vars::merge::VERSION_A_HUNK,
            vars::merge::VERSION_B_HUNK,
            vars::merge::STEP_A_NAME,
            vars::merge::STEP_A_DESCRIPTION,
            vars::merge::STEP_B_NAME,
            vars::merge::STEP_B_DESCRIPTION,
            vars::merge::DIFF_SUMMARY,
            vars::merge::CONTENT_A,
            vars::merge::CONTENT_B,
        ]);

        let all_roles: &[(&str, &RoleDefinition)] = &[
            ("assistant", &roles::ASSISTANT_BASE),
            ("workforce_agent", &roles::WORKFORCE_AGENT),
            ("belief_extractor", &roles::BELIEF_EXTRACTOR),
            ("manager_assistant", &roles::MANAGER_ASSISTANT_BASE),
            ("merge_hunk", &roles::MERGE_HUNK),
            ("merge_delete_modify", &roles::MERGE_DELETE_MODIFY),
            ("merge_new_new", &roles::MERGE_NEW_NEW),
        ];

        let re = regex::Regex::new(r"\{\{\.([^}]+)\}\}").unwrap();

        for (name, role) in all_roles {
            for (file, content) in [("system.md", role.system), ("prompt.md", role.prompt)] {
                for cap in re.captures_iter(content) {
                    let var = &cap[1];
                    assert!(
                        known.contains(var),
                        "Unknown template variable '{{{{.{var}}}}}' in {name}/{file}"
                    );
                }
            }
        }
    }
    // Effort is what separates the tiers on the DeepInfra profile, where all
    // three tier markers resolve to the same model id. If a shipped config
    // ever declares a tier without a matching effort, the distinction is lost
    // silently — the request simply omits the parameter.
    #[test]
    fn every_shipped_agent_declares_an_effort_matching_its_tier() {
        use crate::llm::ReasoningEffort;

        let expected = |tier: &str| match tier {
            "tier:1" => Some(ReasoningEffort::XHigh),
            "tier:2" => Some(ReasoningEffort::High),
            "tier:3" => Some(ReasoningEffort::None),
            _ => None,
        };

        // (raw yaml, label) for every config the binary embeds.
        let configs: Vec<(&str, &str)> = vec![
            (
                include_str!("../../config/assistant/config.yaml"),
                "assistant",
            ),
            (
                include_str!("../../config/workflow_agent/config.yaml"),
                "workflow_agent",
            ),
            (
                include_str!("../../config/runtime_agent/config.yaml"),
                "runtime_agent",
            ),
            (
                include_str!("../../config/system_agent/config.yaml"),
                "system_agent",
            ),
            (
                include_str!("../../config/manager/assistant/config.yaml"),
                "manager/assistant",
            ),
            (
                include_str!("../../config/manager/builder/config.yaml"),
                "manager/builder",
            ),
            (
                include_str!("../../config/services/merge/config.yaml"),
                "services/merge",
            ),
        ];

        for (raw, label) in configs {
            let parsed: ProtocolConfig =
                serde_yaml::from_str(raw).unwrap_or_else(|e| panic!("{label}: {e}"));
            for (role, agent) in &parsed.agents {
                let want = expected(&agent.model_id);
                assert!(
                    want.is_some(),
                    "{label}/{role}: unexpected model_id {}",
                    agent.model_id
                );
                assert_eq!(
                    agent.effort, want,
                    "{label}/{role} declares {} but effort {:?}",
                    agent.model_id, agent.effort
                );
            }
        }
    }

    #[test]
    fn an_omitted_effort_parses_as_none_rather_than_failing() {
        let raw =
            "agents:\n  a:\n    model_id: \"tier:1\"\n    max_tokens: 10\n    temperature: 0.1\n";
        let parsed: ProtocolConfig = serde_yaml::from_str(raw).unwrap();
        assert_eq!(parsed.agents["a"].effort, None);
    }

    #[test]
    fn effort_parses_from_its_wire_spelling() {
        let raw = "agents:\n  a:\n    model_id: \"tier:1\"\n    max_tokens: 10\n    temperature: 0.1\n    effort: xhigh\n";
        let parsed: ProtocolConfig = serde_yaml::from_str(raw).unwrap();
        assert_eq!(
            parsed.agents["a"].effort,
            Some(crate::llm::ReasoningEffort::XHigh)
        );
    }
}
