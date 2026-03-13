//! Test utilities for creating AppState with mock repositories.
//!
//! Provides helper functions and builders for easier test setup.

use std::sync::Arc;

use crate::db::traits::{
    MockAgentExecutionRepo, MockAgentRepo, MockAuthConfigRepo, MockChatMessageRepo,
    MockContentVersionRepo, MockDocumentRepo, MockOutputSchemaRepo, MockPromptTemplateRepo,
    MockProtocolRepo, MockResultRepo, MockRoomRepo, MockSessionRepo, MockSystemConfigRepo,
    MockSystemFileRepo, MockTokenLedgerRepo, MockToolCapabilityRepo, MockToolRepo, MockUserRepo,
    MockWorkflowRepo,
};

use super::Repos;

/// Create a Repos instance with all mock implementations.
///
/// Each mock is created with `new()` - tests can customize expectations
/// on individual repos after calling this.
pub fn default_mock_repos() -> Repos {
    Repos::new(
        Arc::new(MockUserRepo::new()),
        Arc::new(MockDocumentRepo::new()),
        Arc::new(MockOutputSchemaRepo::new()),
        Arc::new(MockPromptTemplateRepo::new()),
        Arc::new(MockWorkflowRepo::new()),
        Arc::new(MockAgentExecutionRepo::new()),
        Arc::new(MockTokenLedgerRepo::new()),
        Arc::new(MockResultRepo::new()),
        Arc::new(MockRoomRepo::new()),
        Arc::new(MockToolCapabilityRepo::new()),
        Arc::new(MockSystemConfigRepo::new()),
        Arc::new(MockProtocolRepo::new()),
        Arc::new(MockContentVersionRepo::new()),
        Arc::new(MockAgentRepo::new()),
        Arc::new(MockToolRepo::new()),
        Arc::new(MockSessionRepo::new()),
        Arc::new(MockChatMessageRepo::new()),
        Arc::new(MockAuthConfigRepo::new()),
        Arc::new(MockSystemFileRepo::new()),
    )
}

/// Builder for customizing mock repos in tests.
///
/// Start with `MockReposBuilder::new()` which creates default mocks,
/// then override specific repos as needed.
pub struct MockReposBuilder {
    repos: Repos,
}

impl MockReposBuilder {
    /// Create a new builder with default mock implementations.
    pub fn new() -> Self {
        Self {
            repos: default_mock_repos(),
        }
    }

    /// Override the users repository.
    pub fn with_users(mut self, repo: Arc<dyn crate::db::traits::UserRepo>) -> Self {
        self.repos.users = repo;
        self
    }

    /// Override the documents repository.
    pub fn with_documents(mut self, repo: Arc<dyn crate::db::traits::DocumentRepo>) -> Self {
        self.repos.documents = repo;
        self
    }

    /// Override the output schemas repository.
    pub fn with_output_schemas(
        mut self,
        repo: Arc<dyn crate::db::traits::OutputSchemaRepo>,
    ) -> Self {
        self.repos.output_schemas = repo;
        self
    }

    /// Override the prompt templates repository.
    pub fn with_prompt_templates(
        mut self,
        repo: Arc<dyn crate::db::traits::PromptTemplateRepo>,
    ) -> Self {
        self.repos.prompt_templates = repo;
        self
    }

    /// Override the workflows repository.
    pub fn with_workflows(mut self, repo: Arc<dyn crate::db::traits::WorkflowRepo>) -> Self {
        self.repos.workflows = repo;
        self
    }

    /// Override the agent executions repository.
    pub fn with_agent_executions(
        mut self,
        repo: Arc<dyn crate::db::traits::AgentExecutionRepo>,
    ) -> Self {
        self.repos.agent_executions = repo;
        self
    }

    /// Override the token ledger repository.
    pub fn with_token_ledger(mut self, repo: Arc<dyn crate::db::traits::TokenLedgerRepo>) -> Self {
        self.repos.token_ledger = repo;
        self
    }

    /// Override the results repository.
    pub fn with_results(mut self, repo: Arc<dyn crate::db::traits::ResultRepo>) -> Self {
        self.repos.results = repo;
        self
    }

    /// Override the rooms repository.
    pub fn with_rooms(mut self, repo: Arc<dyn crate::db::traits::RoomRepo>) -> Self {
        self.repos.rooms = repo;
        self
    }

    /// Override the tool capabilities repository.
    pub fn with_tool_capabilities(
        mut self,
        repo: Arc<dyn crate::db::traits::ToolCapabilityRepo>,
    ) -> Self {
        self.repos.tool_capabilities = repo;
        self
    }

    /// Override the content versions repository.
    pub fn with_content_versions(
        mut self,
        repo: Arc<dyn crate::db::traits::ContentVersionRepo>,
    ) -> Self {
        self.repos.content_versions = repo;
        self
    }

    /// Override the agents repository.
    pub fn with_agents(mut self, repo: Arc<dyn crate::db::traits::AgentRepo>) -> Self {
        self.repos.agents = repo;
        self
    }

    /// Override the tools repository.
    pub fn with_tools(mut self, repo: Arc<dyn crate::db::traits::ToolRepo>) -> Self {
        self.repos.tools = repo;
        self
    }

    /// Override the sessions repository.
    pub fn with_sessions(mut self, repo: Arc<dyn crate::db::traits::SessionRepo>) -> Self {
        self.repos.sessions = repo;
        self
    }

    /// Override the chat messages repository.
    pub fn with_chat_messages(mut self, repo: Arc<dyn crate::db::traits::ChatMessageRepo>) -> Self {
        self.repos.chat_messages = repo;
        self
    }

    /// Override the auth config repository.
    pub fn with_auth_config(mut self, repo: Arc<dyn crate::db::traits::AuthConfigRepo>) -> Self {
        self.repos.auth_config = repo;
        self
    }

    /// Override the system files repository.
    pub fn with_system_files(mut self, repo: Arc<dyn crate::db::traits::SystemFileRepo>) -> Self {
        self.repos.system_files = repo;
        self
    }

    /// Build the Repos instance.
    pub fn build(self) -> Repos {
        self.repos
    }
}

impl Default for MockReposBuilder {
    fn default() -> Self {
        Self::new()
    }
}
