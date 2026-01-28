//! Agent role system

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{info, warn};

// Re-export CommunicationStyle from types for convenience
pub use crate::types::CommunicationStyle;

/// Role category for organization and selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleCategory {
    /// Analysis roles: extract information, find patterns
    Analysis,
    /// Planning roles: decompose work, prioritize, map dependencies
    Planning,
    /// Implementation roles: write code, execute tasks
    Implementation,
    /// Communication roles: summarize, format, report
    Communication,
}

impl RoleCategory {
    /// Get all categories
    pub fn all() -> &'static [RoleCategory] {
        &[
            RoleCategory::Analysis,
            RoleCategory::Planning,
            RoleCategory::Implementation,
            RoleCategory::Communication,
        ]
    }

    /// Display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            RoleCategory::Analysis => "Analysis",
            RoleCategory::Planning => "Planning",
            RoleCategory::Implementation => "Implementation",
            RoleCategory::Communication => "Communication",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            RoleCategory::Analysis => "🔍",
            RoleCategory::Planning => "📋",
            RoleCategory::Implementation => "💻",
            RoleCategory::Communication => "💬",
        }
    }
}

/// Unique identifier for a role
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoleId(pub String);

impl RoleId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Create ID for a custom role (prefixed to avoid collision)
    pub fn new_custom(name: &str) -> Self {
        let slug = name.to_lowercase().replace(' ', "-");
        Self(format!("custom-{}", slug))
    }
}

/// Output format for role responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    /// Structured plan with tickets/slices
    Plan,
    /// Code with report
    CodeAndReport,
    /// Simple result/answer
    Result,
    /// Summary document
    Summary,
    /// Custom format (described in string)
    Custom(String),
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Result
    }
}

/// A role defines agent behavior, file access, and delegation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Unique identifier
    pub id: RoleId,

    /// Display name
    pub name: String,

    /// Category for organization
    pub category: RoleCategory,

    /// Human-readable description
    pub description: String,

    /// System prompt for this role (used if template is None)
    pub system_prompt: String,

    /// Communication style
    pub style: CommunicationStyle,

    /// Files this role MUST read before working
    /// Supports variables: {ticket}, {domain}, etc.
    pub required_reading: Vec<String>,

    /// Role IDs this role can delegate to
    pub can_delegate_to: Vec<RoleId>,

    /// Expected output format
    pub output_format: OutputFormat,

    /// Maximum delegation depth (0 = cannot delegate)
    pub max_delegation_depth: u8,

    /// Optional template for custom roles (user fills in variables)
    pub template: Option<RoleTemplate>,

    /// Whether this is a user-created role
    #[serde(default)]
    pub is_custom: bool,
}

/// Template for custom roles with user-fillable variables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleTemplate {
    /// The prompt template with {variable} placeholders
    pub prompt: String,
    /// Variables that user fills in at spawn time
    pub variables: Vec<TemplateVariable>,
}

/// A variable in a role template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    /// Variable name (used in {name} placeholders)
    pub name: String,
    /// Type of input expected
    pub var_type: VariableType,
    /// Human-readable label for UI
    pub label: String,
    /// Whether the variable must be filled
    #[serde(default = "default_true")]
    pub required: bool,
    /// Optional placeholder/hint text
    pub placeholder: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Type of template variable (affects UI input widget)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableType {
    /// Free-form text input
    Text,
    /// File path selector (multi-select)
    Files,
    /// Numeric input
    Number,
    /// Selection from predefined options
    Choice(Vec<String>),
}

impl Role {
    /// Check if this role can delegate to another role
    pub fn can_delegate(&self, target: &RoleId) -> bool {
        self.max_delegation_depth > 0 && self.can_delegate_to.contains(target)
    }

    /// Resolve required_reading paths with variables
    pub fn resolve_required_reading(&self, vars: &HashMap<String, String>) -> Vec<PathBuf> {
        self.required_reading
            .iter()
            .map(|path| {
                let mut resolved = path.clone();
                for (key, value) in vars {
                    resolved = resolved.replace(&format!("{{{}}}", key), value);
                }
                PathBuf::from(resolved)
            })
            .collect()
    }
}

/// Library of predefined roles
pub struct RoleLibrary {
    roles: HashMap<RoleId, Role>,
    by_category: HashMap<RoleCategory, Vec<RoleId>>,
}

impl RoleLibrary {
    /// Create library with default predefined roles
    pub fn new() -> Self {
        let mut library = Self {
            roles: HashMap::new(),
            by_category: HashMap::new(),
        };

        // Initialize category indexes
        for category in RoleCategory::all() {
            library.by_category.insert(*category, Vec::new());
        }

        // Add default roles
        library.add_default_roles();

        library
    }

    fn add_default_roles(&mut self) {
        // === Analysis Roles ===
        self.add_role(Role {
            id: RoleId::new("complaint-finder"),
            name: "Complaint Finder".to_string(),
            category: RoleCategory::Analysis,
            description: "Extracts user frustrations and pain points".to_string(),
            system_prompt: include_str!("prompts/complaint_finder.txt").to_string(),
            style: CommunicationStyle::Formal,
            required_reading: vec![],
            can_delegate_to: vec![],
            output_format: OutputFormat::Summary,
            max_delegation_depth: 0,
            template: None,
            is_custom: false,
        });

        self.add_role(Role {
            id: RoleId::new("risk-assessor"),
            name: "Risk Assessor".to_string(),
            category: RoleCategory::Analysis,
            description: "Finds potential failures and edge cases".to_string(),
            system_prompt: include_str!("prompts/risk_assessor.txt").to_string(),
            style: CommunicationStyle::Formal,
            required_reading: vec![],
            can_delegate_to: vec![],
            output_format: OutputFormat::Summary,
            max_delegation_depth: 0,
            template: None,
            is_custom: false,
        });

        // === Planning Roles ===
        self.add_role(Role {
            id: RoleId::new("orchestrator"),
            name: "Orchestrator".to_string(),
            category: RoleCategory::Planning,
            description: "Decomposes work into actionable tickets".to_string(),
            system_prompt: include_str!("prompts/orchestrator.txt").to_string(),
            style: CommunicationStyle::Formal,
            required_reading: vec![
                "PRD.md".to_string(),
                "ROADMAP.md".to_string(),
                "PROGRESS.md".to_string(),
            ],
            can_delegate_to: vec![RoleId::new("worker"), RoleId::new("utility")],
            output_format: OutputFormat::Plan,
            max_delegation_depth: 2,
            template: None,
            is_custom: false,
        });

        self.add_role(Role {
            id: RoleId::new("scope-definer"),
            name: "Scope Definer".to_string(),
            category: RoleCategory::Planning,
            description: "Breaks work into concrete deliverables".to_string(),
            system_prompt: include_str!("prompts/scope_definer.txt").to_string(),
            style: CommunicationStyle::Formal,
            required_reading: vec!["PRD.md".to_string()],
            can_delegate_to: vec![],
            output_format: OutputFormat::Plan,
            max_delegation_depth: 0,
            template: None,
            is_custom: false,
        });

        // === Implementation Roles ===
        self.add_role(Role {
            id: RoleId::new("worker"),
            name: "Builder".to_string(),
            category: RoleCategory::Implementation,
            description: "Writes code to spec".to_string(),
            system_prompt: include_str!("prompts/worker.txt").to_string(),
            style: CommunicationStyle::Technical,
            required_reading: vec![
                "decomp/{ticket}.md".to_string(),
                "CONVENTIONS.md".to_string(),
            ],
            can_delegate_to: vec![RoleId::new("utility")],
            output_format: OutputFormat::CodeAndReport,
            max_delegation_depth: 1,
            template: None,
            is_custom: false,
        });

        self.add_role(Role {
            id: RoleId::new("reviewer"),
            name: "Reviewer".to_string(),
            category: RoleCategory::Implementation,
            description: "Validates quality and correctness".to_string(),
            system_prompt: include_str!("prompts/reviewer.txt").to_string(),
            style: CommunicationStyle::Technical,
            required_reading: vec!["CONVENTIONS.md".to_string()],
            can_delegate_to: vec![],
            output_format: OutputFormat::Summary,
            max_delegation_depth: 0,
            template: None,
            is_custom: false,
        });

        // === Communication Roles ===
        self.add_role(Role {
            id: RoleId::new("utility"),
            name: "Helper".to_string(),
            category: RoleCategory::Communication,
            description: "Performs focused helper tasks".to_string(),
            system_prompt: include_str!("prompts/utility.txt").to_string(),
            style: CommunicationStyle::Casual,
            required_reading: vec![],
            can_delegate_to: vec![],
            output_format: OutputFormat::Result,
            max_delegation_depth: 0,
            template: None,
            is_custom: false,
        });

        self.add_role(Role {
            id: RoleId::new("summarizer"),
            name: "Summarizer".to_string(),
            category: RoleCategory::Communication,
            description: "Condenses information clearly".to_string(),
            system_prompt: include_str!("prompts/summarizer.txt").to_string(),
            style: CommunicationStyle::Casual,
            required_reading: vec![],
            can_delegate_to: vec![],
            output_format: OutputFormat::Summary,
            max_delegation_depth: 0,
            template: None,
            is_custom: false,
        });
    }

    /// Add a role to the library
    pub fn add_role(&mut self, role: Role) {
        let id = role.id.clone();
        let category = role.category;
        self.roles.insert(id.clone(), role);
        self.by_category
            .entry(category)
            .or_insert_with(Vec::new)
            .push(id);
    }

    /// Get a role by ID
    pub fn get(&self, id: &RoleId) -> Option<&Role> {
        self.roles.get(id)
    }

    /// List all roles in a category
    pub fn list_by_category(&self, category: RoleCategory) -> Vec<&Role> {
        self.by_category
            .get(&category)
            .map(|ids| ids.iter().filter_map(|id| self.roles.get(id)).collect())
            .unwrap_or_default()
    }

    /// List all roles
    pub fn list_all(&self) -> Vec<&Role> {
        self.roles.values().collect()
    }

    /// Get roles grouped by category (for UI)
    pub fn grouped_by_category(&self) -> Vec<(RoleCategory, Vec<&Role>)> {
        RoleCategory::all()
            .iter()
            .map(|cat| (*cat, self.list_by_category(*cat)))
            .collect()
    }

    /// Add a custom role created by user
    pub fn add_custom_role(&mut self, mut role: Role) {
        role.is_custom = true;
        self.add_role(role);
    }

    /// Remove a custom role
    pub fn remove_custom_role(&mut self, id: &RoleId) -> Option<Role> {
        if let Some(role) = self.roles.get(id) {
            if !role.is_custom {
                return None; // Can't remove predefined roles
            }
        }

        if let Some(role) = self.roles.remove(id) {
            if let Some(ids) = self.by_category.get_mut(&role.category) {
                ids.retain(|r| r != id);
            }
            Some(role)
        } else {
            None
        }
    }

    /// List only custom roles
    pub fn list_custom(&self) -> Vec<&Role> {
        self.roles.values().filter(|r| r.is_custom).collect()
    }
}

impl Default for RoleLibrary {
    fn default() -> Self {
        Self::new()
    }
}

/// Loads required reading files for a role
pub struct RequiredReadingLoader {
    base_path: PathBuf,
}

impl RequiredReadingLoader {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Load all required reading files for a role
    pub async fn load_for_role(
        &self,
        role: &Role,
        vars: &HashMap<String, String>,
    ) -> Vec<LoadedFile> {
        let paths = role.resolve_required_reading(vars);
        let mut loaded = Vec::new();

        for path in paths {
            let full_path = self.base_path.join(&path);

            match fs::read_to_string(&full_path).await {
                Ok(content) => {
                    info!(path = ?path, "Loaded required reading");
                    loaded.push(LoadedFile {
                        path: path.to_string_lossy().to_string(),
                        content,
                    });
                }
                Err(e) => {
                    warn!(
                        path = ?path,
                        error = ?e,
                        "Failed to load required reading file"
                    );
                }
            }
        }

        loaded
    }
}

/// A loaded file ready for context injection
#[derive(Debug, Clone)]
pub struct LoadedFile {
    pub path: String,
    pub content: String,
}

/// Central manager for roles and their context
pub struct RoleManager {
    library: RoleLibrary,
    loader: RequiredReadingLoader,
}

impl RoleManager {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            library: RoleLibrary::new(),
            loader: RequiredReadingLoader::new(project_root),
        }
    }

    /// Get the role library for browsing/selection
    pub fn library(&self) -> &RoleLibrary {
        &self.library
    }

    /// Get a specific role
    pub fn get_role(&self, id: &RoleId) -> Option<&Role> {
        self.library.get(id)
    }

    /// Build complete context for a role
    pub async fn build_context_for_role(
        &self,
        role: &Role,
        vars: &HashMap<String, String>,
    ) -> RoleContext {
        let files = self.loader.load_for_role(role, vars).await;

        RoleContext {
            role: role.clone(),
            loaded_files: files,
        }
    }

    /// Get default role for a tier (backwards compatibility)
    pub fn default_role_for_tier(&self, tier: crate::types::AgentTier) -> Option<&Role> {
        match tier {
            crate::types::AgentTier::Orchestrator => self.library.get(&RoleId::new("orchestrator")),
            crate::types::AgentTier::Worker => self.library.get(&RoleId::new("worker")),
            crate::types::AgentTier::Utility => self.library.get(&RoleId::new("utility")),
        }
    }
}

/// Complete context for executing with a role
#[derive(Debug, Clone)]
pub struct RoleContext {
    pub role: Role,
    pub loaded_files: Vec<LoadedFile>,
}

impl RoleContext {
    /// Build the full system prompt including loaded files
    pub fn build_system_prompt(&self) -> String {
        let mut prompt = self.role.system_prompt.clone();

        if !self.loaded_files.is_empty() {
            prompt.push_str("\n\n---\n\n## Required Reading\n\n");
            for file in &self.loaded_files {
                prompt.push_str(&format!(
                    "### {}\n\n```\n{}\n```\n\n",
                    file.path, file.content
                ));
            }
        }

        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_category_all() {
        let categories = RoleCategory::all();
        assert_eq!(categories.len(), 4);
    }

    #[test]
    fn role_resolves_variables() {
        let role = Role {
            id: RoleId::new("worker"),
            name: "Worker".to_string(),
            category: RoleCategory::Implementation,
            description: "Implements tickets".to_string(),
            system_prompt: "You are a worker.".to_string(),
            style: CommunicationStyle::Technical,
            required_reading: vec![
                "decomp/{ticket}.md".to_string(),
                "CONVENTIONS.md".to_string(),
            ],
            can_delegate_to: vec![RoleId::new("utility")],
            output_format: OutputFormat::CodeAndReport,
            max_delegation_depth: 1,
            template: None,
            is_custom: false,
        };

        let mut vars = HashMap::new();
        vars.insert("ticket".to_string(), "M3/3.4".to_string());

        let paths = role.resolve_required_reading(&vars);
        assert_eq!(paths[0], PathBuf::from("decomp/M3/3.4.md"));
        assert_eq!(paths[1], PathBuf::from("CONVENTIONS.md"));
    }

    #[test]
    fn role_delegation_check() {
        let role = Role {
            id: RoleId::new("orchestrator"),
            name: "Orchestrator".to_string(),
            category: RoleCategory::Planning,
            description: "Plans work".to_string(),
            system_prompt: "You are an orchestrator.".to_string(),
            style: CommunicationStyle::Formal,
            required_reading: vec![],
            can_delegate_to: vec![RoleId::new("worker")],
            output_format: OutputFormat::Plan,
            max_delegation_depth: 2,
            template: None,
            is_custom: false,
        };

        assert!(role.can_delegate(&RoleId::new("worker")));
        assert!(!role.can_delegate(&RoleId::new("unknown")));
    }

    #[test]
    fn library_has_default_roles() {
        let library = RoleLibrary::new();

        // Check key roles exist
        assert!(library.get(&RoleId::new("orchestrator")).is_some());
        assert!(library.get(&RoleId::new("worker")).is_some());
        assert!(library.get(&RoleId::new("utility")).is_some());
    }

    #[test]
    fn library_lists_by_category() {
        let library = RoleLibrary::new();

        let planning = library.list_by_category(RoleCategory::Planning);
        assert!(!planning.is_empty());

        let impl_roles = library.list_by_category(RoleCategory::Implementation);
        assert!(!impl_roles.is_empty());
    }

    #[test]
    fn library_grouped_for_ui() {
        let library = RoleLibrary::new();

        let grouped = library.grouped_by_category();
        assert_eq!(grouped.len(), 4); // 4 categories
    }

    #[test]
    fn library_custom_role_management() {
        let mut library = RoleLibrary::new();

        let custom_role = Role {
            id: RoleId::new_custom("my role"),
            name: "My Custom Role".to_string(),
            category: RoleCategory::Implementation,
            description: "Custom".to_string(),
            system_prompt: "Custom prompt".to_string(),
            style: CommunicationStyle::Casual,
            required_reading: vec![],
            can_delegate_to: vec![],
            output_format: OutputFormat::Result,
            max_delegation_depth: 0,
            template: None,
            is_custom: false, // Will be set to true by add_custom_role
        };

        library.add_custom_role(custom_role);

        let custom_roles = library.list_custom();
        assert_eq!(custom_roles.len(), 1);
        assert!(custom_roles[0].is_custom);
    }

    #[test]
    fn library_cannot_remove_predefined_roles() {
        let mut library = RoleLibrary::new();

        let result = library.remove_custom_role(&RoleId::new("orchestrator"));
        assert!(result.is_none()); // Cannot remove predefined roles
    }

    #[tokio::test]
    async fn loads_required_reading() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();

        // Create a test file
        let conventions_path = dir.path().join("CONVENTIONS.md");
        fs::write(&conventions_path, "# Conventions\nUse snake_case")
            .await
            .unwrap();

        let loader = RequiredReadingLoader::new(dir.path().to_path_buf());

        let role = Role {
            id: RoleId::new("test"),
            name: "Test".to_string(),
            category: RoleCategory::Implementation,
            description: "Test role".to_string(),
            system_prompt: "Test".to_string(),
            style: CommunicationStyle::Technical,
            required_reading: vec!["CONVENTIONS.md".to_string()],
            can_delegate_to: vec![],
            output_format: OutputFormat::Result,
            max_delegation_depth: 0,
            template: None,
            is_custom: false,
        };

        let files = loader.load_for_role(&role, &HashMap::new()).await;
        assert_eq!(files.len(), 1);
        assert!(files[0].content.contains("snake_case"));
    }

    #[tokio::test]
    async fn handles_missing_file() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let loader = RequiredReadingLoader::new(dir.path().to_path_buf());

        let role = Role {
            id: RoleId::new("test"),
            name: "Test".to_string(),
            category: RoleCategory::Implementation,
            description: "Test role".to_string(),
            system_prompt: "Test".to_string(),
            style: CommunicationStyle::Technical,
            required_reading: vec!["DOES_NOT_EXIST.md".to_string()],
            can_delegate_to: vec![],
            output_format: OutputFormat::Result,
            max_delegation_depth: 0,
            template: None,
            is_custom: false,
        };

        let files = loader.load_for_role(&role, &HashMap::new()).await;
        assert_eq!(files.len(), 0); // Missing file not included, but no panic
    }

    #[tokio::test]
    async fn role_manager_provides_roles() {
        let manager = RoleManager::new(PathBuf::from("."));

        assert!(manager.get_role(&RoleId::new("worker")).is_some());
        assert!(manager.get_role(&RoleId::new("orchestrator")).is_some());
    }

    #[test]
    fn role_context_builds_prompt() {
        let role = Role {
            id: RoleId::new("test"),
            name: "Test".to_string(),
            category: RoleCategory::Implementation,
            description: "Test".to_string(),
            system_prompt: "You are a test agent.".to_string(),
            style: CommunicationStyle::Technical,
            required_reading: vec![],
            can_delegate_to: vec![],
            output_format: OutputFormat::Result,
            max_delegation_depth: 0,
            template: None,
            is_custom: false,
        };

        let context = RoleContext {
            role,
            loaded_files: vec![LoadedFile {
                path: "CONVENTIONS.md".to_string(),
                content: "Use snake_case".to_string(),
            }],
        };

        let prompt = context.build_system_prompt();
        assert!(prompt.contains("You are a test agent"));
        assert!(prompt.contains("## Required Reading"));
        assert!(prompt.contains("CONVENTIONS.md"));
        assert!(prompt.contains("snake_case"));
    }
}
