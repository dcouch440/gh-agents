//! Board state types — variant-agnostic snapshots of workflow and node state.
//!
//! These types capture all data needed to render `<board_state>` XML across
//! all 4 layers of the manager node stack. The rendering layer selects what
//! to include based on [`BoardStateVariant`].

use uuid::Uuid;

// ============================================================================
// Variant & Scope
// ============================================================================

/// Which layer is requesting the board state.
///
/// Each variant controls the zoom level and detail included in the
/// rendered `<board_state>` XML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardStateVariant {
    /// L1 — Manager Assistant: all nodes compressed, `<asking>` tags, no ids.
    ManagerAssistant,
    /// L2 — Manager Builder: all nodes with ids + capabilities.
    ManagerBuilder,
    /// L3 — Node Assistant: own node with agents, ports — no agent ids.
    NodeAssistant,
    /// L4 — Dispatch/Node Builder: own node, full detail with ids.
    Dispatch,
}

/// Scope of the board state — which nodes to include.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// All visible nodes in the workflow (L1, L2).
    AllNodes,
    /// Only the current node (L3, L4).
    OwnNode,
}

impl BoardStateVariant {
    /// Whether this variant includes all nodes or just the current one.
    pub fn scope(&self) -> Scope {
        match self {
            Self::ManagerAssistant | Self::ManagerBuilder => Scope::AllNodes,
            Self::NodeAssistant | Self::Dispatch => Scope::OwnNode,
        }
    }

    /// Whether node UUIDs appear in the output.
    pub fn include_node_ids(&self) -> bool {
        matches!(self, Self::ManagerBuilder | Self::Dispatch)
    }

    /// Whether agent UUIDs appear in the output.
    pub fn include_agent_ids(&self) -> bool {
        matches!(self, Self::Dispatch)
    }

    /// Whether the `<status>` tag is rendered (compressed node status from question extraction).
    pub fn include_compressed_status(&self) -> bool {
        matches!(self, Self::ManagerAssistant | Self::ManagerBuilder)
    }

    /// Whether the `<asking>` tag is rendered for nodes awaiting user input.
    pub fn include_asking(&self) -> bool {
        matches!(self, Self::ManagerAssistant)
    }

    /// Whether `task` appears as an attribute on `<node>`.
    pub fn include_task_attr(&self) -> bool {
        matches!(self, Self::NodeAssistant | Self::Dispatch)
    }

    /// Whether capabilities are shown (attribute or element).
    pub fn include_capabilities(&self) -> bool {
        !matches!(self, Self::ManagerAssistant)
    }

    /// Whether individual `<agent>` child elements are rendered inside `<node>`.
    /// L1 shows agents as a summary attribute instead.
    pub fn include_agent_children(&self) -> bool {
        matches!(self, Self::ManagerBuilder | Self::NodeAssistant | Self::Dispatch)
    }

    /// Whether agent role descriptions are included in agent elements.
    pub fn include_agent_descriptions(&self) -> bool {
        matches!(self, Self::ManagerBuilder | Self::NodeAssistant | Self::Dispatch)
    }

    /// Whether incoming port/context details are included.
    pub fn include_ports(&self) -> bool {
        matches!(self, Self::NodeAssistant | Self::Dispatch)
    }

    /// Whether ports include schema and json_path detail (L4 only).
    pub fn include_port_schemas(&self) -> bool {
        matches!(self, Self::Dispatch)
    }

    /// Whether assistant notes are included (L4 only).
    pub fn include_notes(&self) -> bool {
        matches!(self, Self::Dispatch)
    }
}

// ============================================================================
// Snapshot Types
// ============================================================================

/// Snapshot of the entire board — workflow metadata plus all nodes.
#[derive(Debug, Clone)]
pub struct BoardSnapshot {
    pub workflow_name: String,
    pub workflow_id: Uuid,
    pub nodes: Vec<NodeSnapshot>,
    /// Union of all capabilities across all mission briefs (L2 uses this).
    pub available_capabilities: Vec<String>,
}

/// Snapshot of a single node on the board.
#[derive(Debug, Clone)]
pub struct NodeSnapshot {
    pub id: Uuid,
    /// Stable readable identifier (e.g. "workforce-1") for LLM-facing references.
    pub ref_id: Option<String>,
    pub name: String,
    pub protocol: String,
    pub status: String,
    pub task: String,
    pub capabilities: Vec<String>,
    pub failure_mode: String,
    /// Compressed summary line (e.g. "3 agents, all dependencies set").
    pub summary: String,
    /// Compressed semantic status from question extraction (L1/L2 renders as `<status>`).
    pub compressed_status: Option<String>,
    pub agents: Vec<AgentSnapshot>,
    pub input_ports: Vec<InputPortSnapshot>,
    pub output_ports: Vec<OutputPortSnapshot>,
    pub incoming_context: Vec<IncomingContextSnapshot>,
    /// Assistant notes content (L4).
    pub notes: String,
    /// Pending question for the user (L1 renders as `<asking>`).
    pub asking: Option<String>,
    /// Comma-separated upstream node names.
    pub receives: Option<String>,
}

/// Snapshot of an agent in a workforce roster.
#[derive(Debug, Clone)]
pub struct AgentSnapshot {
    pub id: Uuid,
    pub name: String,
    pub role_description: String,
    pub capabilities: Vec<String>,
    pub receives_from: Vec<String>,
}

/// Snapshot of an incoming context connection (L3 style).
#[derive(Debug, Clone)]
pub struct IncomingContextSnapshot {
    pub name: String,
    pub source_mode: String,
    pub status: String,
    pub preview: Option<String>,
    pub word_count: Option<usize>,
}

/// Snapshot of a typed input port with schema (L4 style).
#[derive(Debug, Clone)]
pub struct InputPortSnapshot {
    pub port_name: String,
    pub from_node: String,
    pub schema: Option<String>,
    pub json_path: Option<String>,
}

/// Snapshot of a typed output port with schema (L4 style).
#[derive(Debug, Clone)]
pub struct OutputPortSnapshot {
    pub port_name: String,
    pub to_node: String,
    pub schema: Option<String>,
}
