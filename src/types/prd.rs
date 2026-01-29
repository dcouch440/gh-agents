//! PRD (Product Requirements Document) types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a PRD
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PRDId(pub Uuid);

impl PRDId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for PRDId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PRDId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// PRD lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PRDStatus {
    #[default]
    Draft,
    Review,
    Approved,
    Archived,
}

impl std::fmt::Display for PRDStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PRDStatus::Draft => write!(f, "draft"),
            PRDStatus::Review => write!(f, "review"),
            PRDStatus::Approved => write!(f, "approved"),
            PRDStatus::Archived => write!(f, "archived"),
        }
    }
}

impl std::str::FromStr for PRDStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(PRDStatus::Draft),
            "review" => Ok(PRDStatus::Review),
            "approved" => Ok(PRDStatus::Approved),
            "archived" => Ok(PRDStatus::Archived),
            _ => Err(format!("unknown PRD status: {}", s)),
        }
    }
}

/// Project scale estimate based on milestone count
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectScale {
    Feature,
    Project,
    Epic,
}

impl std::fmt::Display for ProjectScale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectScale::Feature => write!(f, "Feature"),
            ProjectScale::Project => write!(f, "Project"),
            ProjectScale::Epic => write!(f, "Epic"),
        }
    }
}

/// A technical decision made during planning
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TechnicalDecision {
    pub area: String,
    pub decision: String,
    pub rationale: String,
}

/// A data model sketch for the PRD
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataModelSketch {
    pub name: String,
    pub fields: Vec<String>,
    pub description: String,
}

/// A milestone in the PRD
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MilestoneSpec {
    pub title: String,
    pub description: String,
    pub deliverables: Vec<String>,
    pub dependencies: Vec<String>,
}

/// Product Requirements Document
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PRDDocument {
    pub id: PRDId,
    pub title: String,
    pub status: PRDStatus,
    pub vision: String,
    pub problem_statement: String,
    pub target_users: String,
    pub success_criteria: Vec<String>,
    pub technical_decisions: Vec<TechnicalDecision>,
    pub data_models: Vec<DataModelSketch>,
    pub milestones: Vec<MilestoneSpec>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PRDDocument {
    /// Create a new empty PRD
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: PRDId::new(),
            title: title.into(),
            status: PRDStatus::Draft,
            vision: String::new(),
            problem_statement: String::new(),
            target_users: String::new(),
            success_criteria: vec![],
            technical_decisions: vec![],
            data_models: vec![],
            milestones: vec![],
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if PRD has enough content to be finalized
    pub fn is_complete(&self) -> bool {
        !self.vision.is_empty() && !self.milestones.is_empty()
    }

    /// Estimate project scale based on milestone count
    pub fn estimated_scale(&self) -> ProjectScale {
        match self.milestones.len() {
            0..=2 => ProjectScale::Feature,
            3..=6 => ProjectScale::Project,
            _ => ProjectScale::Epic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prd_id_generates_unique() {
        let id1 = PRDId::new();
        let id2 = PRDId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn prd_status_default_is_draft() {
        assert_eq!(PRDStatus::default(), PRDStatus::Draft);
    }

    #[test]
    fn prd_status_roundtrip() {
        let status = PRDStatus::Approved;
        let s = status.to_string();
        let parsed: PRDStatus = s.parse().unwrap();
        assert_eq!(parsed, status);
    }

    #[test]
    fn is_complete_returns_false_for_empty() {
        let prd = PRDDocument::new("Test");
        assert!(!prd.is_complete());
    }

    #[test]
    fn is_complete_returns_true_with_vision_and_milestones() {
        let mut prd = PRDDocument::new("Test");
        prd.vision = "A great product".into();
        prd.milestones.push(MilestoneSpec {
            title: "M1".into(),
            description: "First milestone".into(),
            deliverables: vec![],
            dependencies: vec![],
        });
        assert!(prd.is_complete());
    }

    #[test]
    fn estimated_scale_feature() {
        let prd = PRDDocument::new("Test");
        assert_eq!(prd.estimated_scale(), ProjectScale::Feature);
    }

    #[test]
    fn estimated_scale_project() {
        let mut prd = PRDDocument::new("Test");
        for i in 0..4 {
            prd.milestones.push(MilestoneSpec {
                title: format!("M{}", i),
                description: String::new(),
                deliverables: vec![],
                dependencies: vec![],
            });
        }
        assert_eq!(prd.estimated_scale(), ProjectScale::Project);
    }

    #[test]
    fn estimated_scale_epic() {
        let mut prd = PRDDocument::new("Test");
        for i in 0..8 {
            prd.milestones.push(MilestoneSpec {
                title: format!("M{}", i),
                description: String::new(),
                deliverables: vec![],
                dependencies: vec![],
            });
        }
        assert_eq!(prd.estimated_scale(), ProjectScale::Epic);
    }

    #[test]
    fn serialization_roundtrip() {
        let mut prd = PRDDocument::new("Test PRD");
        prd.vision = "Build something great".into();
        prd.technical_decisions.push(TechnicalDecision {
            area: "Backend".into(),
            decision: "Use Rust".into(),
            rationale: "Performance".into(),
        });

        let json = serde_json::to_string(&prd).unwrap();
        let parsed: PRDDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, prd);
    }
}
