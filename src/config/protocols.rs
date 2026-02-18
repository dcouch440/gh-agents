//! Protocol configuration and role definitions loaded from `config/protocols/`.
//!
//! Each protocol folder contains a `config.yaml` with per-role agent settings
//! (model, temperature, token limits) and role subdirectories with the LLM
//! contract files (`system.md`, `prompt.md`, `response.json`).
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
}

// ---------------------------------------------------------------------------
// Role definition types
// ---------------------------------------------------------------------------

/// Raw compile-time content for a protocol role's LLM contract.
///
/// Each field holds the content of a file from `config/protocols/<proto>/<role>/`:
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
        pub const TASK_DESCRIPTION: &str = "Workforce.task_description";
        pub const TEAM_ROSTER: &str = "Workforce.team_roster";
        pub const PREVIOUS_OUTPUTS: &str = "Workforce.previous_outputs";
    }

    /// Variables for belief capture runtime extraction prompts.
    pub mod belief_capture {
        pub const EXTRACTION_FOCUS: &str = "BeliefCapture.extraction_focus";
        pub const TAG_VOCABULARY: &str = "BeliefCapture.tag_vocabulary";
        pub const CONTRADICTION_HANDLING: &str = "BeliefCapture.contradiction_handling";
        pub const SOURCE_STEP_NAME: &str = "BeliefCapture.source_step_name";
        pub const SOURCE_TYPE: &str = "BeliefCapture.source_type";
        pub const SOURCE_CONTENT: &str = "BeliefCapture.source_content";
    }

    /// Variables for chat belief extraction prompts.
    pub mod chat_belief {
        pub const NODE_NAME: &str = "ChatBelief.node_name";
        pub const NODE_ARCHETYPE: &str = "ChatBelief.node_archetype";
        pub const CONVERSATION: &str = "ChatBelief.conversation";
        pub const BOARD_BELIEFS: &str = "ChatBelief.board_beliefs";
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

    /// Variables assembled by the platform (config, context, runtime state).
    pub mod system {
        pub const DOC_NAME: &str = "System.doc_name";
        pub const SELECTED_CONTEXT: &str = "System.selected_context";
        pub const REQUESTED_DOCUMENTS: &str = "System.requested_documents";
        pub const AVAILABLE_CAPABILITIES: &str = "System.available_capabilities";
        pub const CONTEXT_DOCUMENTS_INSTRUCTION: &str = "System.context_documents_instruction";
        pub const CURRENT_CONFIG: &str = "System.current_config";
        pub const GRAPH_CONTEXT: &str = "System.graph_context";
        pub const BOARD_CONTEXT: &str = "System.board_context";
        pub const ARCHETYPE_BLOCK: &str = "System.archetype_block";
        pub const ASSISTANT_NOTES: &str = "System.assistant_notes";
        pub const BOARD_OVERVIEW: &str = "System.board_overview";
        pub const DISPATCH_STATUS: &str = "System.dispatch_status";
    }
}

// ---------------------------------------------------------------------------
// Config statics — parsed once on first access
// ---------------------------------------------------------------------------

pub static MEETING: Lazy<ProtocolConfig> = Lazy::new(|| {
    serde_yaml::from_str(include_str!("../../config/protocols/meeting/config.yaml"))
        .expect("Failed to parse config/protocols/meeting/config.yaml")
});

pub static NODE_ASSISTANT: Lazy<ProtocolConfig> = Lazy::new(|| {
    serde_yaml::from_str(include_str!(
        "../../config/protocols/node_assistant/config.yaml"
    ))
    .expect("Failed to parse config/protocols/node_assistant/config.yaml")
});

pub static BELIEF_CAPTURE: Lazy<ProtocolConfig> = Lazy::new(|| {
    serde_yaml::from_str(include_str!(
        "../../config/protocols/belief_capture/config.yaml"
    ))
    .expect("Failed to parse config/protocols/belief_capture/config.yaml")
});

pub static WORKFORCE: Lazy<ProtocolConfig> = Lazy::new(|| {
    serde_yaml::from_str(include_str!("../../config/protocols/workforce/config.yaml"))
        .expect("Failed to parse config/protocols/workforce/config.yaml")
});

pub static AGENT_DESIGNER: Lazy<ProtocolConfig> = Lazy::new(|| {
    serde_yaml::from_str(include_str!(
        "../../config/protocols/agent_designer/config.yaml"
    ))
    .expect("Failed to parse config/protocols/agent_designer/config.yaml")
});

pub static DISPATCH: Lazy<ProtocolConfig> = Lazy::new(|| {
    serde_yaml::from_str(include_str!("../../config/protocols/dispatch/config.yaml"))
        .expect("Failed to parse config/protocols/dispatch/config.yaml")
});

// ---------------------------------------------------------------------------
// Role statics — compile-time embedded content
// ---------------------------------------------------------------------------

pub mod roles {
    use super::RoleDefinition;

    pub static MEETING_GATEKEEPER: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/protocols/meeting/gatekeeper/system.md"),
        prompt: include_str!("../../config/protocols/meeting/gatekeeper/prompt.md"),
        response: Some(include_str!(
            "../../config/protocols/meeting/gatekeeper/response.json"
        )),
    };

    pub static NODE_ASSISTANT_BASE: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/protocols/node_assistant/base/system.md"),
        prompt: include_str!("../../config/protocols/node_assistant/base/prompt.md"),
        response: None,
    };

    /// Belief capture archetype block, injected via `{{.System.archetype_block}}`.
    pub const NODE_ASSISTANT_BELIEF_CAPTURE_BLOCK: &str =
        include_str!("../../config/protocols/node_assistant/belief_capture/block.md");

    /// Room archetype block, injected via `{{.System.archetype_block}}`.
    pub const NODE_ASSISTANT_ROOM_BLOCK: &str =
        include_str!("../../config/protocols/node_assistant/room/block.md");

    /// Workforce archetype block, injected via `{{.System.archetype_block}}`.
    pub const NODE_ASSISTANT_WORKFORCE_BLOCK: &str =
        include_str!("../../config/protocols/node_assistant/workforce/block.md");

    /// Dispatch background agent system prompt template.
    pub const DISPATCH_SYSTEM: &str = include_str!("../../config/protocols/dispatch/system.md");

    /// Workforce runtime agent prompt template.
    pub static WORKFORCE_AGENT: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/protocols/workforce/agent/system.md"),
        prompt: include_str!("../../config/protocols/workforce/agent/prompt.md"),
        response: None,
    };

    pub static BELIEF_CAPTURE_EXTRACTOR: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/protocols/belief_capture/extractor/system.md"),
        prompt: include_str!("../../config/protocols/belief_capture/extractor/prompt.md"),
        response: Some(include_str!(
            "../../config/protocols/belief_capture/extractor/response.json"
        )),
    };

    /// Chat belief extractor: reads chat conversations and extracts user beliefs.
    pub static CHAT_BELIEF_EXTRACTOR: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/protocols/chat_belief_extraction/system.md"),
        prompt: include_str!("../../config/protocols/chat_belief_extraction/prompt.md"),
        response: Some(include_str!(
            "../../config/protocols/chat_belief_extraction/response.json"
        )),
    };

    /// Agent Designer: generates optimized prompt pairs for task force agents.
    pub static AGENT_DESIGNER_DESIGNER: RoleDefinition = RoleDefinition {
        system: include_str!("../../config/protocols/agent_designer/designer/system.md"),
        prompt: include_str!("../../config/protocols/agent_designer/designer/prompt.md"),
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
    fn meeting_config_parses_gatekeeper() {
        let cfg = &*MEETING;
        let gk = cfg.agent("gatekeeper");
        assert_eq!(gk.model_id, "claude-sonnet-4-20250514");
        assert_eq!(gk.temperature, 0.3);
        assert_eq!(gk.max_tokens, 4096);
        assert_eq!(gk.max_rounds, 1);
        assert_eq!(gk.context_budget, 480_000);
    }

    #[test]
    #[should_panic(expected = "Unknown agent role")]
    fn unknown_role_panics() {
        MEETING.agent("nonexistent");
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
        assert!(!roles::MEETING_GATEKEEPER.system.is_empty());
        assert!(!roles::MEETING_GATEKEEPER.prompt.is_empty());
        assert!(roles::MEETING_GATEKEEPER.response.is_some());

        assert!(!roles::NODE_ASSISTANT_BASE.system.is_empty());
        assert!(!roles::NODE_ASSISTANT_BASE.prompt.is_empty());
        assert!(roles::NODE_ASSISTANT_BASE.response.is_none());

        assert!(!roles::NODE_ASSISTANT_BELIEF_CAPTURE_BLOCK.is_empty());
        assert!(!roles::NODE_ASSISTANT_ROOM_BLOCK.is_empty());
        assert!(!roles::NODE_ASSISTANT_WORKFORCE_BLOCK.is_empty());

        assert!(!roles::WORKFORCE_AGENT.system.is_empty());
        assert!(!roles::WORKFORCE_AGENT.prompt.is_empty());
        assert!(roles::WORKFORCE_AGENT.response.is_none());

        assert!(!roles::BELIEF_CAPTURE_EXTRACTOR.system.is_empty());
        assert!(!roles::BELIEF_CAPTURE_EXTRACTOR.prompt.is_empty());
        assert!(roles::BELIEF_CAPTURE_EXTRACTOR.response.is_some());

        assert!(!roles::CHAT_BELIEF_EXTRACTOR.system.is_empty());
        assert!(!roles::CHAT_BELIEF_EXTRACTOR.prompt.is_empty());
        assert!(roles::CHAT_BELIEF_EXTRACTOR.response.is_some());

        assert!(!roles::AGENT_DESIGNER_DESIGNER.system.is_empty());
        assert!(!roles::AGENT_DESIGNER_DESIGNER.prompt.is_empty());
        assert!(roles::AGENT_DESIGNER_DESIGNER.response.is_none());
    }

    #[test]
    fn node_assistant_config_parses() {
        let cfg = &*NODE_ASSISTANT;
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
        assert_eq!(agent.max_rounds, 15);
        assert_eq!(agent.context_budget, 480_000);
    }

    #[test]
    fn belief_capture_config_parses() {
        let cfg = &*BELIEF_CAPTURE;
        let extractor = cfg.agent("extractor");
        assert_eq!(extractor.temperature, 0.2);
        assert_eq!(extractor.max_rounds, 1);
        assert_eq!(extractor.context_budget, 200_000);
    }

    #[test]
    fn agent_designer_config_parses() {
        let cfg = &*AGENT_DESIGNER;
        let designer = cfg.agent("designer");
        assert_eq!(designer.model_id, "claude-sonnet-4-5-20250929");
        assert_eq!(designer.temperature, 0.4);
        assert_eq!(designer.max_tokens, 16384);
        assert_eq!(designer.max_rounds, 1);
        assert_eq!(designer.context_budget, 480_000);
    }

    #[test]
    fn dispatch_config_parses() {
        let cfg = &*DISPATCH;
        let dispatcher = cfg.agent("dispatcher");
        assert_eq!(dispatcher.model_id, "claude-sonnet-4-20250514");
        assert_eq!(dispatcher.temperature, 0.3);
        assert_eq!(dispatcher.max_tokens, 8192);
        assert_eq!(dispatcher.max_rounds, 15);
        assert_eq!(dispatcher.context_budget, 200_000);
    }

    #[test]
    fn node_assistant_base_resolves_with_archetype_block() {
        let mut vars = HashMap::new();
        vars.insert(
            vars::system::BOARD_CONTEXT.to_string(),
            "Nodes: A -> B -> [SELECTED] C".to_string(),
        );
        vars.insert(
            vars::system::ARCHETYPE_BLOCK.to_string(),
            roles::NODE_ASSISTANT_WORKFORCE_BLOCK.to_string(),
        );
        vars.insert(vars::system::CURRENT_CONFIG.to_string(), String::new());

        let ctx = roles::NODE_ASSISTANT_BASE.resolve(&vars);
        assert!(
            ctx.system_prompt.contains("help the user design this node"),
            "should contain focused identity"
        );
        assert!(
            ctx.system_prompt.contains("Nodes: A -> B -> [SELECTED] C"),
            "should contain board context"
        );
        assert!(
            ctx.system_prompt.contains("archetype_context"),
            "should contain workforce block"
        );
    }

    #[test]
    fn gatekeeper_response_json_parses_directly() {
        let raw = roles::MEETING_GATEKEEPER.response.unwrap();
        let schema: serde_json::Value =
            serde_json::from_str(raw).expect("gatekeeper response.json should be valid JSON");
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["speakers"].is_object());
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
            vars::system::CURRENT_CONFIG,
            vars::system::GRAPH_CONTEXT,
            vars::system::BOARD_CONTEXT,
            vars::system::ARCHETYPE_BLOCK,
            vars::system::ASSISTANT_NOTES,
            vars::system::BOARD_OVERVIEW,
            vars::system::DISPATCH_STATUS,
            vars::workforce::AGENT_NAME,
            vars::workforce::ROLE_DESCRIPTION,
            vars::workforce::TASK_DESCRIPTION,
            vars::workforce::TEAM_ROSTER,
            vars::workforce::PREVIOUS_OUTPUTS,
            vars::belief_capture::EXTRACTION_FOCUS,
            vars::belief_capture::TAG_VOCABULARY,
            vars::belief_capture::CONTRADICTION_HANDLING,
            vars::belief_capture::SOURCE_STEP_NAME,
            vars::belief_capture::SOURCE_TYPE,
            vars::belief_capture::SOURCE_CONTENT,
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
        ]);

        let all_roles: &[(&str, &RoleDefinition)] = &[
            ("gatekeeper", &roles::MEETING_GATEKEEPER),
            ("node_assistant", &roles::NODE_ASSISTANT_BASE),
            ("workforce_agent", &roles::WORKFORCE_AGENT),
            ("belief_capture_extractor", &roles::BELIEF_CAPTURE_EXTRACTOR),
            ("chat_belief_extractor", &roles::CHAT_BELIEF_EXTRACTOR),
            ("agent_designer", &roles::AGENT_DESIGNER_DESIGNER),
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
