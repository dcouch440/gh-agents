//! Test utilities for creating AppState with mock repositories.
//!
//! Provides helper functions and builders for easier test setup.

use std::sync::Arc;

use crate::db::traits::{
    MockAgentExecutionRepo, MockContentVersionRepo, MockDocumentRepo, MockOutputSchemaRepo,
    MockPromptTemplateRepo, MockProtocolRepo, MockResultRepo, MockRoomRepo, MockSystemConfigRepo,
    MockTokenLedgerRepo, MockToolCapabilityRepo, MockUserRepo, MockWorkflowRepo,
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
