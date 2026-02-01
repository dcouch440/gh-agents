//! Agent clustering — named groups of agents that share context.

use std::collections::HashMap;
use uuid::Uuid;

use super::agent::AgentId;
use super::channels::FileContent;

/// Unique identifier for a cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClusterId(pub Uuid);

impl Default for ClusterId {
    fn default() -> Self {
        Self::new()
    }
}

impl ClusterId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// A named group of agents that share context.
#[derive(Debug, Clone)]
pub struct Cluster {
    pub id: ClusterId,
    pub name: String,
    pub description: String,
    pub members: Vec<AgentId>,
    pub shared_context: ClusterContext,
}

/// Shared context injected into every task assigned to cluster members.
#[derive(Debug, Clone, Default)]
pub struct ClusterContext {
    /// Shared conventions text appended to TaskContext.conventions.
    pub conventions: String,
    /// Files all cluster members should see, appended to TaskContext.files.
    pub shared_files: Vec<FileContent>,
}

/// Manages all clusters and agent-to-cluster mappings.
#[derive(Debug, Default)]
pub struct ClusterManager {
    clusters: HashMap<ClusterId, Cluster>,
    /// Reverse index: which cluster does an agent belong to?
    agent_to_cluster: HashMap<AgentId, ClusterId>,
}

impl ClusterManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new cluster, returning its ID.
    pub fn create_cluster(&mut self, name: String, description: String) -> ClusterId {
        let id = ClusterId::new();
        let cluster = Cluster {
            id,
            name,
            description,
            members: Vec::new(),
            shared_context: ClusterContext::default(),
        };
        self.clusters.insert(id, cluster);
        id
    }

    /// Create a cluster with a specific ID (for reconstruction from DB).
    pub fn create_cluster_with_id(&mut self, id: ClusterId, name: String, description: String) {
        let cluster = Cluster {
            id,
            name,
            description,
            members: Vec::new(),
            shared_context: ClusterContext::default(),
        };
        self.clusters.insert(id, cluster);
    }

    /// Add an agent to a cluster. Removes from previous cluster if any.
    pub fn add_agent(&mut self, cluster_id: ClusterId, agent_id: AgentId) -> Result<(), ClusterError> {
        if !self.clusters.contains_key(&cluster_id) {
            return Err(ClusterError::NotFound(cluster_id));
        }

        // Remove from old cluster if assigned
        if let Some(old_id) = self.agent_to_cluster.remove(&agent_id) {
            if let Some(old_cluster) = self.clusters.get_mut(&old_id) {
                old_cluster.members.retain(|id| *id != agent_id);
            }
        }

        // Safety: cluster existence verified by contains_key check above
        let cluster = self.clusters.get_mut(&cluster_id).expect("cluster existence verified above");
        if !cluster.members.contains(&agent_id) {
            cluster.members.push(agent_id.clone());
        }
        self.agent_to_cluster.insert(agent_id, cluster_id);
        Ok(())
    }

    /// Remove an agent from its cluster.
    pub fn remove_agent(&mut self, cluster_id: ClusterId, agent_id: AgentId) -> Result<(), ClusterError> {
        let cluster = self.clusters.get_mut(&cluster_id).ok_or(ClusterError::NotFound(cluster_id))?;
        cluster.members.retain(|id| *id != agent_id);
        self.agent_to_cluster.remove(&agent_id);
        Ok(())
    }

    /// Get a cluster by ID.
    pub fn get_cluster(&self, id: &ClusterId) -> Option<&Cluster> {
        self.clusters.get(id)
    }

    /// List all clusters.
    pub fn list_clusters(&self) -> Vec<&Cluster> {
        self.clusters.values().collect()
    }

    /// Get the cluster an agent belongs to (if any).
    pub fn get_agent_cluster(&self, agent_id: &AgentId) -> Option<&Cluster> {
        self.agent_to_cluster.get(agent_id).and_then(|cid| self.clusters.get(cid))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClusterError {
    #[error("Cluster not found: {0:?}")]
    NotFound(ClusterId),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(n: u128) -> AgentId {
        AgentId(Uuid::from_u128(n))
    }

    #[test]
    fn create_and_list() {
        let mut mgr = ClusterManager::new();
        let id = mgr.create_cluster("frontend".into(), "Frontend team".into());
        let clusters = mgr.list_clusters();
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].name, "frontend");
        assert_eq!(clusters[0].id, id);
    }

    #[test]
    fn add_and_remove_agent() {
        let mut mgr = ClusterManager::new();
        let cid = mgr.create_cluster("c1".into(), "".into());
        let a = agent(1);

        mgr.add_agent(cid, a.clone()).unwrap();
        assert_eq!(mgr.get_cluster(&cid).unwrap().members.len(), 1);
        assert_eq!(mgr.get_agent_cluster(&a).unwrap().id, cid);

        mgr.remove_agent(cid, a.clone()).unwrap();
        assert!(mgr.get_cluster(&cid).unwrap().members.is_empty());
        assert!(mgr.get_agent_cluster(&a).is_none());
    }

    #[test]
    fn add_agent_moves_between_clusters() {
        let mut mgr = ClusterManager::new();
        let c1 = mgr.create_cluster("c1".into(), "".into());
        let c2 = mgr.create_cluster("c2".into(), "".into());
        let a = agent(1);

        mgr.add_agent(c1, a.clone()).unwrap();
        assert_eq!(mgr.get_agent_cluster(&a).unwrap().id, c1);

        mgr.add_agent(c2, a.clone()).unwrap();
        assert_eq!(mgr.get_agent_cluster(&a).unwrap().id, c2);
        assert!(mgr.get_cluster(&c1).unwrap().members.is_empty());
    }

    #[test]
    fn add_to_nonexistent_cluster_fails() {
        let mut mgr = ClusterManager::new();
        let bad_id = ClusterId::new();
        let result = mgr.add_agent(bad_id, agent(1));
        assert!(result.is_err());
    }

    #[test]
    fn remove_from_nonexistent_cluster_fails() {
        let mut mgr = ClusterManager::new();
        let bad_id = ClusterId::new();
        let result = mgr.remove_agent(bad_id, agent(1));
        assert!(result.is_err());
    }

    #[test]
    fn duplicate_add_is_idempotent() {
        let mut mgr = ClusterManager::new();
        let cid = mgr.create_cluster("c".into(), "".into());
        let a = agent(1);

        mgr.add_agent(cid, a.clone()).unwrap();
        mgr.add_agent(cid, a).unwrap();
        assert_eq!(mgr.get_cluster(&cid).unwrap().members.len(), 1);
    }
}
