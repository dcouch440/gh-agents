//! Compare two prompt outputs and identify differences.

use nexor::prompts::schemas::DecompositionOutput;
use std::collections::HashSet;

/// Compare two prompt outputs and identify differences
pub struct PromptDiff {
    pub structural_changes: Vec<StructuralChange>,
    pub content_changes: Vec<ContentChange>,
    pub behavioral_changes: Vec<BehavioralChange>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum StructuralChange {
    FieldAdded(String),
    FieldRemoved(String),
    TypeChanged {
        field: String,
        old: String,
        new: String,
    },
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ContentChange {
    SliceCountChanged { old: usize, new: usize },
    SliceAdded(String),
    SliceRemoved(String),
    SliceRenamed { old: String, new: String },
    ThinkingChanged,
    RisksChanged,
    QuestionsChanged,
}

#[derive(Debug, Clone)]
pub enum BehavioralChange {
    TierAssignmentChanged {
        task: String,
        old: String,
        new: String,
    },
    DependencyAdded {
        slice: String,
        dependency: String,
    },
    DependencyRemoved {
        slice: String,
        dependency: String,
    },
    ComplexityEstimateChanged {
        task: String,
        old: String,
        new: String,
    },
    AcceptanceCriteriaChanged {
        slice: String,
    },
}

impl PromptDiff {
    /// Compare two decomposition outputs
    pub fn compare_decompositions(old: &DecompositionOutput, new: &DecompositionOutput) -> Self {
        let mut diff = Self {
            structural_changes: Vec::new(),
            content_changes: Vec::new(),
            behavioral_changes: Vec::new(),
        };

        // Check thinking changes
        if old.thinking != new.thinking {
            diff.content_changes.push(ContentChange::ThinkingChanged);
        }

        // Check risks changes
        if old.risks != new.risks {
            diff.content_changes.push(ContentChange::RisksChanged);
        }

        // Check questions changes
        if old.questions != new.questions {
            diff.content_changes.push(ContentChange::QuestionsChanged);
        }

        // Check slice count
        if old.slices.len() != new.slices.len() {
            diff.content_changes.push(ContentChange::SliceCountChanged {
                old: old.slices.len(),
                new: new.slices.len(),
            });
        }

        // Find added/removed slices
        let old_titles: HashSet<_> = old.slices.iter().map(|s| &s.title).collect();
        let new_titles: HashSet<_> = new.slices.iter().map(|s| &s.title).collect();

        for title in new_titles.difference(&old_titles) {
            diff.content_changes
                .push(ContentChange::SliceAdded((*title).clone()));
        }

        for title in old_titles.difference(&new_titles) {
            diff.content_changes
                .push(ContentChange::SliceRemoved((*title).clone()));
        }

        // Check behavioral changes in matching slices
        for old_slice in &old.slices {
            if let Some(new_slice) = new.slices.iter().find(|s| s.title == old_slice.title) {
                // Check dependency changes
                let old_deps: HashSet<_> = old_slice.dependencies.iter().collect();
                let new_deps: HashSet<_> = new_slice.dependencies.iter().collect();

                for dep in new_deps.difference(&old_deps) {
                    diff.behavioral_changes
                        .push(BehavioralChange::DependencyAdded {
                            slice: old_slice.title.clone(),
                            dependency: (*dep).clone(),
                        });
                }

                for dep in old_deps.difference(&new_deps) {
                    diff.behavioral_changes
                        .push(BehavioralChange::DependencyRemoved {
                            slice: old_slice.title.clone(),
                            dependency: (*dep).clone(),
                        });
                }

                // Check acceptance criteria changes
                if old_slice.acceptance_criteria != new_slice.acceptance_criteria {
                    diff.behavioral_changes
                        .push(BehavioralChange::AcceptanceCriteriaChanged {
                            slice: old_slice.title.clone(),
                        });
                }

                // Check task changes
                for old_task in &old_slice.tasks {
                    if let Some(new_task) =
                        new_slice.tasks.iter().find(|t| t.title == old_task.title)
                    {
                        // Check tier changes
                        if old_task.tier != new_task.tier {
                            diff.behavioral_changes
                                .push(BehavioralChange::TierAssignmentChanged {
                                    task: old_task.title.clone(),
                                    old: format!("{:?}", old_task.tier),
                                    new: format!("{:?}", new_task.tier),
                                });
                        }

                        // Check complexity changes
                        if old_task.estimated_complexity != new_task.estimated_complexity {
                            diff.behavioral_changes.push(
                                BehavioralChange::ComplexityEstimateChanged {
                                    task: old_task.title.clone(),
                                    old: format!("{:?}", old_task.estimated_complexity),
                                    new: format!("{:?}", new_task.estimated_complexity),
                                },
                            );
                        }
                    }
                }
            }
        }

        diff
    }

    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        !self.structural_changes.is_empty()
            || !self.content_changes.is_empty()
            || !self.behavioral_changes.is_empty()
    }

    /// Check if there are breaking changes
    pub fn has_breaking_changes(&self) -> bool {
        // Structural changes are always breaking
        if !self.structural_changes.is_empty() {
            return true;
        }

        // Slice removals are breaking
        for change in &self.content_changes {
            if matches!(change, ContentChange::SliceRemoved(_)) {
                return true;
            }
        }

        // Tier changes might be breaking
        for change in &self.behavioral_changes {
            if matches!(change, BehavioralChange::TierAssignmentChanged { .. }) {
                return true;
            }
        }

        false
    }

    /// Generate a human-readable diff report
    pub fn report(&self) -> String {
        let mut lines = Vec::new();

        if !self.structural_changes.is_empty() {
            lines.push("## Structural Changes".to_string());
            for change in &self.structural_changes {
                match change {
                    StructuralChange::FieldAdded(f) => {
                        lines.push(format!("- Added field: \"{}\"", f));
                    }
                    StructuralChange::FieldRemoved(f) => {
                        lines.push(format!("- Removed field: \"{}\"", f));
                    }
                    StructuralChange::TypeChanged { field, old, new } => {
                        lines.push(format!(
                            "- Type changed for \"{}\": {} -> {}",
                            field, old, new
                        ));
                    }
                }
            }
            lines.push(String::new());
        }

        if !self.content_changes.is_empty() {
            lines.push("## Content Changes".to_string());
            for change in &self.content_changes {
                match change {
                    ContentChange::SliceCountChanged { old, new } => {
                        lines.push(format!("- Slice count: {} -> {}", old, new));
                    }
                    ContentChange::SliceAdded(title) => {
                        lines.push(format!("- Added slice: \"{}\"", title));
                    }
                    ContentChange::SliceRemoved(title) => {
                        lines.push(format!("- Removed slice: \"{}\"", title));
                    }
                    ContentChange::SliceRenamed { old, new } => {
                        lines.push(format!("- Renamed: \"{}\" -> \"{}\"", old, new));
                    }
                    ContentChange::ThinkingChanged => {
                        lines.push("- Thinking content changed".to_string());
                    }
                    ContentChange::RisksChanged => {
                        lines.push("- Risks changed".to_string());
                    }
                    ContentChange::QuestionsChanged => {
                        lines.push("- Questions changed".to_string());
                    }
                }
            }
            lines.push(String::new());
        }

        if !self.behavioral_changes.is_empty() {
            lines.push("## Behavioral Changes".to_string());
            for change in &self.behavioral_changes {
                match change {
                    BehavioralChange::TierAssignmentChanged { task, old, new } => {
                        lines.push(format!(
                            "- Tier changed for \"{}\": {} -> {}",
                            task, old, new
                        ));
                    }
                    BehavioralChange::DependencyAdded { slice, dependency } => {
                        lines.push(format!(
                            "- Added dependency: \"{}\" now depends on \"{}\"",
                            slice, dependency
                        ));
                    }
                    BehavioralChange::DependencyRemoved { slice, dependency } => {
                        lines.push(format!(
                            "- Removed dependency: \"{}\" no longer depends on \"{}\"",
                            slice, dependency
                        ));
                    }
                    BehavioralChange::ComplexityEstimateChanged { task, old, new } => {
                        lines.push(format!(
                            "- Complexity changed for \"{}\": {} -> {}",
                            task, old, new
                        ));
                    }
                    BehavioralChange::AcceptanceCriteriaChanged { slice } => {
                        lines.push(format!("- Acceptance criteria changed for \"{}\"", slice));
                    }
                }
            }
        }

        if lines.is_empty() {
            "No changes detected.".to_string()
        } else {
            lines.join("\n")
        }
    }

    /// Get a summary of changes
    pub fn summary(&self) -> String {
        let structural = self.structural_changes.len();
        let content = self.content_changes.len();
        let behavioral = self.behavioral_changes.len();
        let total = structural + content + behavioral;

        if total == 0 {
            "No changes".to_string()
        } else {
            format!(
                "{} changes ({} structural, {} content, {} behavioral)",
                total, structural, content, behavioral
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexor::prompts::schemas::{ComplexityOutput, SliceOutput, TaskOutput, TierOutput};

    fn create_base_output() -> DecompositionOutput {
        DecompositionOutput {
            thinking: "Analyzing...".to_string(),
            slices: vec![SliceOutput {
                title: "Slice A".to_string(),
                description: "First slice".to_string(),
                tasks: vec![TaskOutput {
                    title: "Task 1".to_string(),
                    tier: TierOutput::Worker,
                    estimated_complexity: ComplexityOutput::Low,
                    context_files: vec![],
                }],
                dependencies: vec![],
                acceptance_criteria: vec!["Done".to_string()],
            }],
            questions: vec![],
            risks: vec![],
        }
    }

    #[test]
    fn test_no_changes_detected() {
        let old = create_base_output();
        let new = old.clone();

        let diff = PromptDiff::compare_decompositions(&old, &new);
        assert!(!diff.has_changes());
        assert_eq!(diff.report(), "No changes detected.");
    }

    #[test]
    fn test_detects_slice_count_change() {
        let old = DecompositionOutput {
            thinking: "test".to_string(),
            slices: vec![],
            questions: vec![],
            risks: vec![],
        };

        let new = DecompositionOutput {
            thinking: "test".to_string(),
            slices: vec![SliceOutput {
                title: "New slice".to_string(),
                description: "desc".to_string(),
                tasks: vec![],
                dependencies: vec![],
                acceptance_criteria: vec!["done".to_string()],
            }],
            questions: vec![],
            risks: vec![],
        };

        let diff = PromptDiff::compare_decompositions(&old, &new);
        assert!(diff.has_changes());
        assert!(diff
            .content_changes
            .iter()
            .any(|c| matches!(c, ContentChange::SliceAdded(_))));
        assert!(diff
            .content_changes
            .iter()
            .any(|c| matches!(c, ContentChange::SliceCountChanged { .. })));
    }

    #[test]
    fn test_detects_slice_added() {
        let old = create_base_output();
        let mut new = old.clone();
        new.slices.push(SliceOutput {
            title: "Slice B".to_string(),
            description: "Second slice".to_string(),
            tasks: vec![],
            dependencies: vec![],
            acceptance_criteria: vec!["Done".to_string()],
        });

        let diff = PromptDiff::compare_decompositions(&old, &new);
        assert!(diff.has_changes());
        assert!(diff
            .content_changes
            .iter()
            .any(|c| matches!(c, ContentChange::SliceAdded(t) if t == "Slice B")));
    }

    #[test]
    fn test_detects_slice_removed() {
        let old = create_base_output();
        let mut new = old.clone();
        new.slices.clear();

        let diff = PromptDiff::compare_decompositions(&old, &new);
        assert!(diff.has_changes());
        assert!(diff.has_breaking_changes());
        assert!(diff
            .content_changes
            .iter()
            .any(|c| matches!(c, ContentChange::SliceRemoved(t) if t == "Slice A")));
    }

    #[test]
    fn test_detects_dependency_added() {
        let mut old = create_base_output();
        old.slices.push(SliceOutput {
            title: "Slice B".to_string(),
            description: "Second".to_string(),
            tasks: vec![],
            dependencies: vec![],
            acceptance_criteria: vec!["Done".to_string()],
        });

        let mut new = old.clone();
        new.slices[1].dependencies = vec!["Slice A".to_string()];

        let diff = PromptDiff::compare_decompositions(&old, &new);
        assert!(diff.has_changes());
        assert!(diff.behavioral_changes.iter().any(
            |c| matches!(c, BehavioralChange::DependencyAdded { slice, dependency }
                if slice == "Slice B" && dependency == "Slice A")
        ));
    }

    #[test]
    fn test_detects_dependency_removed() {
        let mut old = create_base_output();
        old.slices.push(SliceOutput {
            title: "Slice B".to_string(),
            description: "Second".to_string(),
            tasks: vec![],
            dependencies: vec!["Slice A".to_string()],
            acceptance_criteria: vec!["Done".to_string()],
        });

        let mut new = old.clone();
        new.slices[1].dependencies = vec![];

        let diff = PromptDiff::compare_decompositions(&old, &new);
        assert!(diff.has_changes());
        assert!(diff.behavioral_changes.iter().any(
            |c| matches!(c, BehavioralChange::DependencyRemoved { slice, dependency }
                if slice == "Slice B" && dependency == "Slice A")
        ));
    }

    #[test]
    fn test_detects_tier_change() {
        let old = create_base_output();
        let mut new = old.clone();
        new.slices[0].tasks[0].tier = TierOutput::Utility;

        let diff = PromptDiff::compare_decompositions(&old, &new);
        assert!(diff.has_changes());
        assert!(diff.has_breaking_changes());
        assert!(diff.behavioral_changes.iter().any(
            |c| matches!(c, BehavioralChange::TierAssignmentChanged { task, .. } if task == "Task 1")
        ));
    }

    #[test]
    fn test_detects_complexity_change() {
        let old = create_base_output();
        let mut new = old.clone();
        new.slices[0].tasks[0].estimated_complexity = ComplexityOutput::High;

        let diff = PromptDiff::compare_decompositions(&old, &new);
        assert!(diff.has_changes());
        assert!(diff.behavioral_changes.iter().any(
            |c| matches!(c, BehavioralChange::ComplexityEstimateChanged { task, .. } if task == "Task 1")
        ));
    }

    #[test]
    fn test_detects_thinking_change() {
        let old = create_base_output();
        let mut new = old.clone();
        new.thinking = "Different thinking".to_string();

        let diff = PromptDiff::compare_decompositions(&old, &new);
        assert!(diff.has_changes());
        assert!(diff
            .content_changes
            .iter()
            .any(|c| matches!(c, ContentChange::ThinkingChanged)));
    }

    #[test]
    fn test_detects_risks_change() {
        let old = create_base_output();
        let mut new = old.clone();
        new.risks = vec!["New risk".to_string()];

        let diff = PromptDiff::compare_decompositions(&old, &new);
        assert!(diff.has_changes());
        assert!(diff
            .content_changes
            .iter()
            .any(|c| matches!(c, ContentChange::RisksChanged)));
    }

    #[test]
    fn test_detects_acceptance_criteria_change() {
        let old = create_base_output();
        let mut new = old.clone();
        new.slices[0].acceptance_criteria = vec!["Different criteria".to_string()];

        let diff = PromptDiff::compare_decompositions(&old, &new);
        assert!(diff.has_changes());
        assert!(diff.behavioral_changes.iter().any(
            |c| matches!(c, BehavioralChange::AcceptanceCriteriaChanged { slice } if slice == "Slice A")
        ));
    }

    #[test]
    fn test_report_format() {
        let old = create_base_output();
        let mut new = old.clone();
        new.slices.push(SliceOutput {
            title: "New Slice".to_string(),
            description: "Added".to_string(),
            tasks: vec![],
            dependencies: vec![],
            acceptance_criteria: vec!["Done".to_string()],
        });

        let diff = PromptDiff::compare_decompositions(&old, &new);
        let report = diff.report();

        assert!(report.contains("## Content Changes"));
        assert!(report.contains("Added slice: \"New Slice\""));
    }

    #[test]
    fn test_summary() {
        let old = create_base_output();
        let mut new = old.clone();
        new.thinking = "Changed".to_string();
        new.slices[0].tasks[0].tier = TierOutput::Utility;

        let diff = PromptDiff::compare_decompositions(&old, &new);
        let summary = diff.summary();

        assert!(summary.contains("changes"));
        assert!(summary.contains("content"));
        assert!(summary.contains("behavioral"));
    }

    #[test]
    fn test_no_changes_summary() {
        let old = create_base_output();
        let new = old.clone();

        let diff = PromptDiff::compare_decompositions(&old, &new);
        assert_eq!(diff.summary(), "No changes");
    }
}
