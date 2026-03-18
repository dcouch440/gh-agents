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

use crate::server::hub::protocols::template_resolve::resolve_template;
use crate::server::hub::protocols::text_utils::collapse_blank_lines;

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
    /// Resolve all `{{.var}}` template variables and produce a ready-to-use context.
    ///
    /// Empty variable values produce blank lines that are collapsed automatically.
    /// Unknown variables are left as-is.
    pub fn resolve(&self, vars: &HashMap<String, String>) -> ProtocolContext {
        let system_prompt = collapse_blank_lines(&resolve_template(self.system, vars));
        let user_prompt = collapse_blank_lines(&resolve_template(self.prompt, vars));
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

    /// Variables for the Agent Designer pre-lifecycle prompt generation.
    pub mod designer {
        pub const ARCHETYPE: &str = "Designer.archetype";
        pub const CONTEXT_DESCRIPTION: &str = "Designer.context_description";
        pub const AGENT_DEFINITIONS: &str = "Designer.agent_definitions";
        pub const UPSTREAM_CONTEXT: &str = "Designer.upstream_context";
        pub const AVAILABLE_TOOLS: &str = "Designer.available_tools";
        pub const ARCHETYPE_GUIDANCE: &str = "Designer.archetype_guidance";
    }

    /// Template variables for the ReAct designer prompts.
    pub mod react_designer {
        pub const NODE_NAME: &str = "ReactDesigner.node_name";
        pub const PRIOR_DESIGN: &str = "ReactDesigner.prior_design";
        pub const STEP_ORDER: &str = "ReactDesigner.step_order";
        pub const TASK: &str = "ReactDesigner.task";
        pub const CURRENT_DESIGN_HANDOFF: &str = "ReactDesigner.current_design_handoff";
        pub const PREVIOUS_STEP: &str = "ReactDesigner.previous_step";
        pub const NEXT_STEP: &str = "ReactDesigner.next_step";
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
        include_str!("../../config/archetype/workforce/agent/config.yaml"),
        "config/archetype/workforce/agent/config.yaml",
    )
});

/// Shared Agent Designer config (pre-lifecycle prompt generation).
pub static DESIGNER: Lazy<ProtocolConfig> = Lazy::new(|| {
    load_protocol_config(
        include_str!("../../config/designer/config.yaml"),
        "config/designer/config.yaml",
    )
});

/// Workforce builder config (background configuration agent).
pub static WORKFORCE_BUILDER: Lazy<ProtocolConfig> = Lazy::new(|| {
    load_protocol_config(
        include_str!("../../config/archetype/workforce/builder/config.yaml"),
        "config/archetype/workforce/builder/config.yaml",
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

/// Workspace merge conflict resolution config.
pub static MERGE: Lazy<ProtocolConfig> = Lazy::new(|| {
    load_protocol_config(
        include_str!("../../config/services/merge/config.yaml"),
        "config/services/merge/config.yaml",
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
    pub const WORKFORCE_ARCHETYPE: &str =
        include_str!("../../config/archetype/workforce/archetype.md");

    /// Workforce builder system prompt (background configuration agent).
    pub const WORKFORCE_BUILDER_SYSTEM: &str =
        include_str!("../../config/archetype/workforce/builder/system.md");

    /// Workforce runtime agent prompt template.
    pub static WORKFORCE_AGENT: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/archetype/workforce/agent/system.md"),
        prompt: include_str!("../../config/archetype/workforce/agent/prompt.md"),
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

    /// Agent Designer: generates optimized prompt pairs for agents (one-shot).
    pub static DESIGNER: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/designer/system.md"),
        prompt: include_str!("../../config/designer/prompt.md"),
        response: None,
    };

    /// ReAct Agent Designer: multi-turn designer that writes configs to the store.
    pub static REACT_DESIGNER: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/designer/react_system.md"),
        prompt: include_str!("../../config/designer/react_prompt.md"),
        response: None,
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

    #[test]
    fn all_role_statics_load() {
        assert!(!roles::ASSISTANT_BASE.system.is_empty());
        assert!(!roles::ASSISTANT_BASE.prompt.is_empty());
        assert!(roles::ASSISTANT_BASE.response.is_none());

        assert!(!roles::WORKFORCE_ARCHETYPE.is_empty());
        assert!(!roles::WORKFORCE_BUILDER_SYSTEM.is_empty());

        assert!(!roles::WORKFORCE_AGENT.system.is_empty());
        assert!(!roles::WORKFORCE_AGENT.prompt.is_empty());
        assert!(roles::WORKFORCE_AGENT.response.is_none());

        assert!(!roles::BELIEF_EXTRACTOR.system.is_empty());
        assert!(!roles::BELIEF_EXTRACTOR.prompt.is_empty());
        assert!(roles::BELIEF_EXTRACTOR.response.is_some());

        assert!(!roles::DESIGNER.system.is_empty());
        assert!(!roles::DESIGNER.prompt.is_empty());
        assert!(roles::DESIGNER.response.is_none());

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
        assert_eq!(agent.max_rounds, 30);
        assert_eq!(agent.context_budget, 480_000);
    }

    #[test]
    fn designer_config_parses() {
        let cfg = &*DESIGNER;
        let designer = cfg.agent("designer");
        assert_eq!(designer.model_id, crate::constants::MODEL_TIER2);
        assert_eq!(designer.temperature, 0.4);
        assert_eq!(designer.max_tokens, 16384);
        assert_eq!(designer.max_rounds, 1);
        assert_eq!(designer.context_budget, 480_000);
    }

    #[test]
    fn workforce_builder_config_parses() {
        let cfg = &*WORKFORCE_BUILDER;
        let dispatcher = cfg.agent("dispatcher");
        assert_eq!(dispatcher.model_id, crate::constants::MODEL_TIER2);
        assert_eq!(dispatcher.temperature, 0.3);
        assert_eq!(dispatcher.max_tokens, 8192);
        assert!(dispatcher.max_rounds > 0);
        assert_eq!(dispatcher.context_budget, 200_000);
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
            vars::designer::ARCHETYPE,
            vars::designer::CONTEXT_DESCRIPTION,
            vars::designer::AGENT_DEFINITIONS,
            vars::designer::UPSTREAM_CONTEXT,
            vars::designer::AVAILABLE_TOOLS,
            vars::designer::ARCHETYPE_GUIDANCE,
            vars::react_designer::NODE_NAME,
            vars::react_designer::PRIOR_DESIGN,
            vars::react_designer::STEP_ORDER,
            vars::react_designer::TASK,
            vars::react_designer::CURRENT_DESIGN_HANDOFF,
            vars::react_designer::PREVIOUS_STEP,
            vars::react_designer::NEXT_STEP,
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
            ("designer", &roles::DESIGNER),
            ("manager_assistant", &roles::MANAGER_ASSISTANT_BASE),
            ("react_designer", &roles::REACT_DESIGNER),
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
}
