"""JSON schemas for BOCA gatekeeper tool_use calls."""


# ===========================================================================
# V2 SCHEMAS (Phase 6 — reasoning-first)
# ===========================================================================

BELIEF_SCHEMA_V2 = {
    "type": "object",
    "properties": {
        "beliefs": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "reasoning": {
                        "type": "string",
                        "description": "Why this is a distinct belief — what specific claim in the node output triggers this extraction."
                    },
                    "semantic_tag": {
                        "type": "string",
                        "description": "Specific technical concept tag (e.g., 'retry_max_attempts', 'audit_retention_period')"
                    },
                    "confidence": {
                        "type": "string",
                        "enum": ["high", "medium", "low"],
                    },
                    "confidence_justification": {
                        "type": "string",
                        "description": "Why this confidence level."
                    },
                    "emotional_tone": {
                        "type": "string",
                        "description": "Rhetorical posture: authoritative, hedging, prescriptive, modified, dismissive, cautionary, definitive"
                    },
                    "cross_source_tension": {
                        "type": "string",
                        "description": "If this belief might conflict with other roles, describe the potential tension. Empty string if none."
                    },
                    "content": {
                        "type": "string",
                        "description": "The exact factual claim with ALL NUMBERS preserved verbatim."
                    },
                },
                "required": ["reasoning", "semantic_tag", "confidence", "confidence_justification",
                             "emotional_tone", "cross_source_tension", "content"],
            },
        },
    },
    "required": ["beliefs"],
}


# ===========================================================================
# V3 SCHEMAS (Phase 7 — adds belief_type and controlled tags)
# ===========================================================================

BELIEF_SCHEMA_V3 = {
    "type": "object",
    "properties": {
        "beliefs": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "reasoning": {
                        "type": "string",
                        "description": "Why this is a distinct belief — what specific claim triggers this extraction."
                    },
                    "semantic_tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "One or more tags from the controlled vocabulary. Use ONLY tags from the provided taxonomy."
                    },
                    "belief_type": {
                        "type": "string",
                        "enum": ["fact", "policy", "opinion", "observation"],
                        "description": "fact=measurable parameter, policy=organizational rule, opinion=professional recommendation, observation=contextual note"
                    },
                    "confidence": {
                        "type": "string",
                        "enum": ["high", "medium", "low"],
                    },
                    "confidence_justification": {
                        "type": "string",
                        "description": "Why this confidence level."
                    },
                    "emotional_tone": {
                        "type": "string",
                        "description": "Rhetorical posture: authoritative, hedging, prescriptive, modified, cautionary, definitive"
                    },
                    "cross_source_tension": {
                        "type": "string",
                        "description": "Potential conflict with other roles. Empty string if none."
                    },
                    "content": {
                        "type": "string",
                        "description": "The exact factual claim with ALL NUMBERS preserved verbatim."
                    },
                },
                "required": ["reasoning", "semantic_tags", "belief_type", "confidence",
                             "confidence_justification", "emotional_tone",
                             "cross_source_tension", "content"],
            },
        },
    },
    "required": ["beliefs"],
}


# ===========================================================================
# CONVERGENCE SCHEMAS
# ===========================================================================

CONVERGENCE_SCHEMA_V2 = {
    "type": "object",
    "properties": {
        "converged_beliefs": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id": {"type": "string", "description": "Converged belief ID (cb01, cb02, ...)"},
                    "topic": {"type": "string", "description": "Semantic topic tag"},
                    "convergence_reasoning": {
                        "type": "string",
                        "description": "Which source beliefs merge here and why."
                    },
                    "content": {
                        "type": "string",
                        "description": "Converged belief content with ALL EXACT NUMBERS preserved."
                    },
                    "consensus_strength": {
                        "type": "string",
                        "enum": ["unanimous", "strong", "majority", "split", "unique"]
                    },
                    "consensus_justification": {
                        "type": "string",
                        "description": "Why this consensus strength — how many sources agree."
                    },
                    "sources": {"type": "array", "items": {"type": "string"}},
                    "source_belief_ids": {"type": "array", "items": {"type": "string"}},
                    "contradiction_resolved": {"type": "boolean"},
                    "resolution_reasoning": {
                        "type": "string",
                        "description": "IF contradiction_resolved: step-by-step authority analysis."
                    },
                    "resolution_detail": {
                        "type": "string",
                        "description": "If contradiction_resolved, the final resolution statement."
                    },
                },
                "required": ["id", "topic", "convergence_reasoning", "content", "consensus_strength",
                             "consensus_justification", "sources", "source_belief_ids",
                             "contradiction_resolved"],
            },
        },
        "pruned_beliefs": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "belief_id": {"type": "string"},
                    "reason": {"type": "string"},
                },
                "required": ["belief_id", "reason"],
            },
        },
        "compression_stats": {
            "type": "object",
            "properties": {
                "input_beliefs": {"type": "integer"},
                "output_beliefs": {"type": "integer"},
                "contradictions_found": {"type": "integer"},
                "contradictions_resolved": {"type": "integer"},
                "redundancies_removed": {"type": "integer"},
                "unique_insights_preserved": {"type": "integer"},
            },
            "required": ["input_beliefs", "output_beliefs", "contradictions_found",
                         "contradictions_resolved", "redundancies_removed",
                         "unique_insights_preserved"],
        },
    },
    "required": ["converged_beliefs", "pruned_beliefs", "compression_stats"],
}


# ===========================================================================
# META-CONVERGENCE SCHEMA (Phase 7)
# ===========================================================================

META_CONVERGENCE_SCHEMA = {
    "type": "object",
    "properties": {
        "meta_beliefs": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id": {"type": "string", "description": "Meta-converged belief ID (mb01, mb02, ...)"},
                    "topic": {"type": "string"},
                    "semantic_tags": {
                        "type": "array", "items": {"type": "string"},
                        "description": "Tags from controlled vocabulary"
                    },
                    "belief_type": {
                        "type": "string",
                        "enum": ["fact", "policy", "opinion", "observation"],
                    },
                    "convergence_reasoning": {
                        "type": "string",
                        "description": "How beliefs from different workflows combine. Note cross-workflow validation."
                    },
                    "content": {
                        "type": "string",
                        "description": "Final authoritative content with ALL EXACT NUMBERS."
                    },
                    "consensus_strength": {
                        "type": "string",
                        "enum": ["cross_validated", "single_workflow", "cross_workflow_split"]
                    },
                    "consensus_justification": {"type": "string"},
                    "workflow_sources": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "workflow": {"type": "string"},
                                "belief_ids": {"type": "array", "items": {"type": "string"}},
                                "value_stated": {"type": "string"},
                            },
                            "required": ["workflow", "belief_ids", "value_stated"],
                        },
                    },
                    "contradiction_resolved": {"type": "boolean"},
                    "resolution_reasoning": {"type": "string"},
                    "resolution_detail": {"type": "string"},
                },
                "required": ["id", "topic", "semantic_tags", "belief_type",
                             "convergence_reasoning", "content", "consensus_strength",
                             "consensus_justification", "workflow_sources",
                             "contradiction_resolved"],
            },
        },
        "cross_validation_summary": {
            "type": "object",
            "properties": {
                "overlapping_topics": {"type": "integer"},
                "cross_validated": {"type": "integer"},
                "cross_workflow_splits": {"type": "integer"},
                "wf1_only_topics": {"type": "integer"},
                "wf2_only_topics": {"type": "integer"},
            },
            "required": ["overlapping_topics", "cross_validated", "cross_workflow_splits",
                         "wf1_only_topics", "wf2_only_topics"],
        },
        "compression_stats": {
            "type": "object",
            "properties": {
                "input_wf1_beliefs": {"type": "integer"},
                "input_wf2_beliefs": {"type": "integer"},
                "output_meta_beliefs": {"type": "integer"},
                "contradictions_found": {"type": "integer"},
                "contradictions_resolved": {"type": "integer"},
            },
            "required": ["input_wf1_beliefs", "input_wf2_beliefs", "output_meta_beliefs",
                         "contradictions_found", "contradictions_resolved"],
        },
    },
    "required": ["meta_beliefs", "cross_validation_summary", "compression_stats"],
}


# ===========================================================================
# TAXONOMY SCHEMA (Phase 7)
# ===========================================================================

TAXONOMY_SCHEMA = {
    "type": "object",
    "properties": {
        "taxonomy_reasoning": {
            "type": "string",
            "description": "How you identified the domain topics from the source documents."
        },
        "tags": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "tag": {
                        "type": "string",
                        "description": "snake_case tag (e.g., 'critical_alert_latency', 'audit_log_retention')"
                    },
                    "description": {
                        "type": "string",
                        "description": "What this tag covers"
                    },
                    "domain": {
                        "type": "string",
                        "enum": ["performance", "reliability", "security", "compliance",
                                 "operations", "integration", "clinical"],
                    },
                },
                "required": ["tag", "description", "domain"],
            },
        },
    },
    "required": ["taxonomy_reasoning", "tags"],
}


# ===========================================================================
# ANSWER SCHEMAS
# ===========================================================================

ANSWER_SCHEMA_V2 = {
    "type": "object",
    "properties": {
        "answers": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "question_id": {"type": "string"},
                    "belief_search": {
                        "type": "string",
                        "description": "Which beliefs did you search for and why?"
                    },
                    "reasoning": {
                        "type": "string",
                        "description": "Step-by-step reasoning from beliefs to answer."
                    },
                    "answer": {
                        "type": "string",
                        "description": "Your final answer with exact numbers."
                    },
                    "confidence": {
                        "type": "integer",
                        "description": "1-5 confidence (1=guessing, 5=certain)"
                    },
                    "confidence_justification": {"type": "string"},
                    "sources_cited": {
                        "type": "array", "items": {"type": "string"},
                    },
                },
                "required": ["question_id", "belief_search", "reasoning", "answer",
                             "confidence", "confidence_justification", "sources_cited"],
            },
        },
    },
    "required": ["answers"],
}

ANSWER_SCHEMA_V3 = {
    "type": "object",
    "properties": {
        "answers": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "question_id": {"type": "string"},
                    "belief_search": {"type": "string"},
                    "reasoning": {"type": "string"},
                    "answer": {"type": "string"},
                    "confidence": {
                        "type": "integer",
                        "description": "1-5 confidence, calibrated: cross_validated beliefs→4-5, single_workflow→3-4, cross_workflow_split→1-2"
                    },
                    "confidence_calibration": {
                        "type": "string",
                        "description": "Map from belief consensus_strength to confidence score. Explain the calibration."
                    },
                    "coverage_assessment": {
                        "type": "string",
                        "enum": ["full", "partial", "none"],
                        "description": "full=beliefs fully cover the question, partial=some aspects missing, none=no relevant beliefs"
                    },
                    "coverage_gaps": {
                        "type": "string",
                        "description": "If partial or none: what specific aspects are not covered by beliefs?"
                    },
                    "confidence_justification": {"type": "string"},
                    "sources_cited": {
                        "type": "array", "items": {"type": "string"},
                    },
                },
                "required": ["question_id", "belief_search", "reasoning", "answer",
                             "confidence", "confidence_calibration",
                             "coverage_assessment", "coverage_gaps",
                             "confidence_justification", "sources_cited"],
            },
        },
    },
    "required": ["answers"],
}

FLAT_SELECT_SCHEMA_V2 = {
    "type": "object",
    "properties": {
        "selection_reasoning": {
            "type": "string",
            "description": "Which questions require which topics, and which beliefs cover those topics."
        },
        "selected_belief_ids": {"type": "array", "items": {"type": "string"}},
        "pruned_belief_ids": {"type": "array", "items": {"type": "string"}},
    },
    "required": ["selection_reasoning", "selected_belief_ids", "pruned_belief_ids"],
}
