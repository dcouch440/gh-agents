//! Detects signs of confusion or uncertainty in LLM output.

/// Detects signs of confusion or uncertainty in LLM output.
pub struct ConfusionDetector {
    hedging_phrases: Vec<&'static str>,
    contradiction_patterns: Vec<(&'static str, &'static str)>,
}

impl Default for ConfusionDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfusionDetector {
    pub fn new() -> Self {
        Self {
            hedging_phrases: vec![
                "I'm not sure",
                "I think",
                "I believe",
                "maybe",
                "perhaps",
                "might be",
                "could be",
                "not certain",
                "unclear",
                "I don't know",
                "hard to say",
                "it depends",
                "I'm confused",
                "I don't understand",
                "possibly",
                "probably",
                "seems like",
                "appears to be",
                "not entirely sure",
                "best guess",
            ],
            contradiction_patterns: vec![
                ("should", "should not"),
                ("must", "must not"),
                ("will", "won't"),
                ("can", "cannot"),
                ("yes", "no"),
                ("always", "never"),
                ("do", "don't"),
                ("is", "isn't"),
                ("are", "aren't"),
            ],
        }
    }

    /// Analyze text for signs of confusion
    pub fn analyze(&self, text: &str) -> ConfusionAnalysis {
        let lower = text.to_lowercase();
        let mut hedges_found = Vec::new();
        let mut contradictions_found = Vec::new();

        // Find hedging language
        for phrase in &self.hedging_phrases {
            if lower.contains(&phrase.to_lowercase()) {
                hedges_found.push(phrase.to_string());
            }
        }

        // Find contradictions (simplified - looks for both terms in same text)
        for (term_a, term_b) in &self.contradiction_patterns {
            if lower.contains(*term_a) && lower.contains(*term_b) {
                // More sophisticated analysis would check sentence context
                contradictions_found.push(format!("{} vs {}", term_a, term_b));
            }
        }

        // Calculate confidence score
        let hedge_score = hedges_found.len();
        let contradiction_score = contradictions_found.len() * 2;
        let total_issues = hedge_score + contradiction_score;

        let confidence = if total_issues == 0 {
            ConfidenceLevel::High
        } else if total_issues <= 2 {
            ConfidenceLevel::Medium
        } else {
            ConfidenceLevel::Low
        };

        ConfusionAnalysis {
            hedges_found,
            contradictions_found,
            confidence,
            should_review: confidence == ConfidenceLevel::Low,
            issue_count: total_issues,
        }
    }

    /// Quick check if output seems confused
    pub fn seems_confused(&self, text: &str) -> bool {
        self.analyze(text).confidence == ConfidenceLevel::Low
    }

    /// Check if text contains specific hedging patterns
    pub fn has_hedging(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        self.hedging_phrases
            .iter()
            .any(|p| lower.contains(&p.to_lowercase()))
    }

    /// Get hedging phrases found in text
    pub fn find_hedges(&self, text: &str) -> Vec<String> {
        let lower = text.to_lowercase();
        self.hedging_phrases
            .iter()
            .filter(|p| lower.contains(&p.to_lowercase()))
            .map(|p| p.to_string())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct ConfusionAnalysis {
    pub hedges_found: Vec<String>,
    pub contradictions_found: Vec<String>,
    pub confidence: ConfidenceLevel,
    pub should_review: bool,
    pub issue_count: usize,
}

impl ConfusionAnalysis {
    pub fn summary(&self) -> String {
        if self.hedges_found.is_empty() && self.contradictions_found.is_empty() {
            "No signs of confusion detected.".to_string()
        } else {
            let mut parts = Vec::new();
            if !self.hedges_found.is_empty() {
                parts.push(format!("Hedging: {:?}", self.hedges_found));
            }
            if !self.contradictions_found.is_empty() {
                parts.push(format!("Contradictions: {:?}", self.contradictions_found));
            }
            parts.push(format!("Confidence: {:?}", self.confidence));
            parts.join("; ")
        }
    }

    pub fn is_confident(&self) -> bool {
        self.confidence == ConfidenceLevel::High
    }

    pub fn needs_attention(&self) -> bool {
        self.confidence == ConfidenceLevel::Low || !self.contradictions_found.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::High => write!(f, "High"),
            Self::Medium => write!(f, "Medium"),
            Self::Low => write!(f, "Low"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_detector() {
        let detector = ConfusionDetector::new();
        assert!(!detector.hedging_phrases.is_empty());
        assert!(!detector.contradiction_patterns.is_empty());
    }

    #[test]
    fn test_default_detector() {
        let detector = ConfusionDetector::default();
        assert!(!detector.hedging_phrases.is_empty());
    }

    #[test]
    fn test_detects_hedging() {
        let detector = ConfusionDetector::new();

        let confident = "Create a users table with id, email, and password columns.";
        let hedging =
            "I think we should maybe create a users table, but I'm not sure about the columns.";

        assert!(!detector.seems_confused(confident));
        assert!(detector.seems_confused(hedging));
    }

    #[test]
    fn test_confidence_levels() {
        let detector = ConfusionDetector::new();

        let analysis = detector.analyze("This is a clear statement.");
        assert_eq!(analysis.confidence, ConfidenceLevel::High);

        let analysis = detector.analyze("I think this might work.");
        assert!(matches!(
            analysis.confidence,
            ConfidenceLevel::Medium | ConfidenceLevel::Low
        ));
    }

    #[test]
    fn test_high_confidence_clear_text() {
        let detector = ConfusionDetector::new();
        let analysis = detector.analyze("Create the user authentication module.");

        assert_eq!(analysis.confidence, ConfidenceLevel::High);
        assert!(analysis.hedges_found.is_empty());
        assert!(analysis.contradictions_found.is_empty());
        assert!(!analysis.should_review);
    }

    #[test]
    fn test_low_confidence_hedging() {
        let detector = ConfusionDetector::new();
        let analysis = detector.analyze(
            "I'm not sure, but maybe we should possibly try this, though I don't know if it will work.",
        );

        assert_eq!(analysis.confidence, ConfidenceLevel::Low);
        assert!(!analysis.hedges_found.is_empty());
        assert!(analysis.should_review);
    }

    #[test]
    fn test_detects_contradictions() {
        let detector = ConfusionDetector::new();
        let analysis = detector.analyze("You should do this, but you should not do that.");

        assert!(!analysis.contradictions_found.is_empty());
    }

    #[test]
    fn test_has_hedging() {
        let detector = ConfusionDetector::new();

        assert!(!detector.has_hedging("This is clear."));
        assert!(detector.has_hedging("I think this is right."));
        assert!(detector.has_hedging("Maybe we should try this."));
        assert!(detector.has_hedging("Perhaps this approach works."));
    }

    #[test]
    fn test_find_hedges() {
        let detector = ConfusionDetector::new();

        let hedges = detector.find_hedges("I think this might be right, but maybe not.");
        assert!(hedges.contains(&"I think".to_string()));
        assert!(hedges.contains(&"maybe".to_string()));
        assert!(hedges.contains(&"might be".to_string()));
    }

    #[test]
    fn test_find_hedges_empty() {
        let detector = ConfusionDetector::new();
        let hedges = detector.find_hedges("This is a clear statement.");
        assert!(hedges.is_empty());
    }

    #[test]
    fn test_analysis_summary_no_issues() {
        let analysis = ConfusionAnalysis {
            hedges_found: vec![],
            contradictions_found: vec![],
            confidence: ConfidenceLevel::High,
            should_review: false,
            issue_count: 0,
        };

        assert!(analysis.summary().contains("No signs of confusion"));
    }

    #[test]
    fn test_analysis_summary_with_issues() {
        let analysis = ConfusionAnalysis {
            hedges_found: vec!["maybe".to_string()],
            contradictions_found: vec!["yes vs no".to_string()],
            confidence: ConfidenceLevel::Low,
            should_review: true,
            issue_count: 3,
        };

        let summary = analysis.summary();
        assert!(summary.contains("Hedging"));
        assert!(summary.contains("Contradictions"));
        assert!(summary.contains("Confidence"));
    }

    #[test]
    fn test_analysis_is_confident() {
        let high = ConfusionAnalysis {
            hedges_found: vec![],
            contradictions_found: vec![],
            confidence: ConfidenceLevel::High,
            should_review: false,
            issue_count: 0,
        };
        assert!(high.is_confident());

        let low = ConfusionAnalysis {
            hedges_found: vec!["maybe".to_string()],
            contradictions_found: vec![],
            confidence: ConfidenceLevel::Low,
            should_review: true,
            issue_count: 3,
        };
        assert!(!low.is_confident());
    }

    #[test]
    fn test_analysis_needs_attention() {
        let low = ConfusionAnalysis {
            hedges_found: vec!["maybe".to_string()],
            contradictions_found: vec![],
            confidence: ConfidenceLevel::Low,
            should_review: true,
            issue_count: 3,
        };
        assert!(low.needs_attention());

        let with_contradiction = ConfusionAnalysis {
            hedges_found: vec![],
            contradictions_found: vec!["yes vs no".to_string()],
            confidence: ConfidenceLevel::Medium,
            should_review: false,
            issue_count: 2,
        };
        assert!(with_contradiction.needs_attention());

        let clear = ConfusionAnalysis {
            hedges_found: vec![],
            contradictions_found: vec![],
            confidence: ConfidenceLevel::High,
            should_review: false,
            issue_count: 0,
        };
        assert!(!clear.needs_attention());
    }

    #[test]
    fn test_confidence_level_display() {
        assert_eq!(format!("{}", ConfidenceLevel::High), "High");
        assert_eq!(format!("{}", ConfidenceLevel::Medium), "Medium");
        assert_eq!(format!("{}", ConfidenceLevel::Low), "Low");
    }

    #[test]
    fn test_medium_confidence() {
        let detector = ConfusionDetector::new();

        // Just one hedge should be medium confidence
        let analysis = detector.analyze("I think this is the right approach.");
        assert!(matches!(
            analysis.confidence,
            ConfidenceLevel::Medium | ConfidenceLevel::High
        ));
    }

    #[test]
    fn test_case_insensitive() {
        let detector = ConfusionDetector::new();

        assert!(detector.has_hedging("MAYBE this works"));
        assert!(detector.has_hedging("I THINK so"));
        assert!(detector.has_hedging("Perhaps THIS IS RIGHT"));
    }

    #[test]
    fn test_issue_count() {
        let detector = ConfusionDetector::new();

        let clear = detector.analyze("This is clear.");
        assert_eq!(clear.issue_count, 0);

        let hedging = detector.analyze("I think maybe this could be right.");
        assert!(hedging.issue_count > 0);
    }
}
