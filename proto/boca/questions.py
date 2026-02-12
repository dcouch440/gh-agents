"""Question batteries for BOCA experiments."""


# ===========================================================================
# PHASE 4 — 10 Questions (WF1 only)
# ===========================================================================

PHASE4_QUESTIONS = [
    {"id": "Q01", "text": "What is the maximum acceptable latency for critical notifications?",
     "tests_claim": "claim_01", "category": "clean"},
    {"id": "Q02", "text": "What encryption standard is required for patient data at rest?",
     "tests_claim": "claim_02", "category": "clean"},
    {"id": "Q03", "text": "What is the notification payload size limit?",
     "tests_claim": "claim_05", "category": "clean"},
    {"id": "Q04", "text": "How many notification priority levels exist and what are they?",
     "tests_claim": "claim_09", "category": "clean"},
    {"id": "Q05", "text": "What is the data residency requirement for patient data?",
     "tests_claim": "claim_12", "category": "clean"},
    {"id": "Q06", "text": "How long must audit logs be retained and why?",
     "tests_claim": "claim_03", "category": "poisoned"},
    {"id": "Q07", "text": "Describe the retry policy for failed notifications, including maximum retry attempts and backoff strategy.",
     "tests_claim": "claim_06", "category": "poisoned"},
    {"id": "Q08", "text": "Describe the failover behavior including failure detection timing and promotion timing.",
     "tests_claim": "claim_11", "category": "poisoned"},
    {"id": "Q09", "text": "Is the failover configuration consistent with the critical alert latency SLA? Explain whether the failover detection and promotion times could violate the latency requirement.",
     "tests_claims": ["claim_01", "claim_11"], "category": "synthesis"},
    {"id": "Q10", "text": "Given the retry policy and dead letter queue threshold, what is the maximum total time before a failed notification is escalated to manual review? Show your calculation.",
     "tests_claims": ["claim_06", "claim_07"], "category": "synthesis"},
]


# ===========================================================================
# PHASE 5/6 — 30 Questions (WF1 only, extends Phase 4)
# ===========================================================================

PHASE5_NEW_QUESTIONS = [
    {"id": "Q11", "text": "What is the WebSocket concurrent connection limit per relay node?",
     "tests_claim": "claim_04", "category": "clean"},
    {"id": "Q12", "text": "What is the rate limit per notification provider, and what burst capacity is allowed?",
     "tests_claim": "claim_08", "category": "clean"},
    {"id": "Q13", "text": "What authentication mechanism is used for service-to-service communication?",
     "tests_claim": "claim_10", "category": "clean"},
    {"id": "Q14", "text": "What authentication is used for client API access, and what is the token expiration time?",
     "tests_claim": "claim_10", "category": "clean"},
    {"id": "Q15", "text": "What happens to undelivered notifications after 24 hours?",
     "tests_claim": "claim_07", "category": "clean"},
    {"id": "Q16", "text": "Describe the complete lifecycle of a failed critical alert: from initial delivery failure through retries to dead letter queue. Include all timing details.",
     "tests_claims": ["claim_01", "claim_06", "claim_07"], "category": "cross_cutting"},
    {"id": "Q17", "text": "Describe the full security architecture: encryption at rest, service authentication, client authentication, and data residency requirements.",
     "tests_claims": ["claim_02", "claim_10", "claim_12"], "category": "cross_cutting"},
    {"id": "Q18", "text": "A compliance auditor requests a 6-year-old notification audit log. Can the system produce it, and in what format is it stored?",
     "tests_claims": ["claim_02", "claim_03"], "category": "cross_cutting"},
    {"id": "Q19", "text": "During peak hospital shift change, describe the system capacity: concurrent connections, rate limits per provider, and payload constraints.",
     "tests_claims": ["claim_04", "claim_05", "claim_08"], "category": "cross_cutting"},
    {"id": "Q20", "text": "Describe the complete failover scenario: how quickly is failure detected, how long does promotion take, and does this affect the critical alert latency SLA?",
     "tests_claims": ["claim_01", "claim_11"], "category": "cross_cutting"},
    {"id": "Q21", "text": "A proposal suggests changing the retry policy to 5 maximum attempts. What is the current maximum, and what would be the impact of this change on DLQ timing?",
     "tests_claim": "claim_06", "category": "hypothetical"},
    {"id": "Q22", "text": "A new EU regulation requires 10-year audit log retention. What is the current retention period, and what changes would be needed?",
     "tests_claim": "claim_03", "category": "hypothetical"},
    {"id": "Q23", "text": "If the notification payload limit were increased to 8KB, what is the current limit and what downstream systems would need updating?",
     "tests_claim": "claim_05", "category": "hypothetical"},
    {"id": "Q24", "text": "Can US patient notification data be routed through an EU data center for redundancy? Explain the data residency policy.",
     "tests_claim": "claim_12", "category": "hypothetical"},
    {"id": "Q25", "text": "A hospital system needs 20,000 concurrent WebSocket connections per node. What is the current limit and can the system support this?",
     "tests_claim": "claim_04", "category": "hypothetical"},
    {"id": "Q26", "text": "The QA team recommends 5 maximum retry attempts for better reliability. Is this consistent with the system specification?",
     "tests_claim": "claim_06", "category": "adversarial"},
    {"id": "Q27", "text": "Is a 5-year audit log retention period sufficient for HIPAA compliance in this system?",
     "tests_claim": "claim_03", "category": "adversarial"},
    {"id": "Q28", "text": "The failover detection window has been described as both 30 seconds and 60 seconds in different documents. Which is correct?",
     "tests_claim": "claim_11", "category": "adversarial"},
    {"id": "Q29", "text": "The retry backoff sequence is 1s, 2s, 4s, 8s, 16s. How many retries does this represent, and is it consistent with the specification?",
     "tests_claim": "claim_06", "category": "adversarial"},
    {"id": "Q30", "text": "What is the minimum HIPAA-mandated audit log retention period for this notification system?",
     "tests_claim": "claim_03", "category": "adversarial"},
]

ALL_30_QUESTIONS = PHASE4_QUESTIONS + PHASE5_NEW_QUESTIONS


# ===========================================================================
# PHASE 7 — 20 Questions (multi-workflow)
# ===========================================================================

# Combined claims dict for scoring Phase 7 questions
# Maps claim IDs used in Phase 7 questions to the appropriate claims dict key
PHASE7_Q_CLAIM_MAP = {
    # wf1_only: test WF1 claims through meta-converged store
    "P7Q01": {"claim": "claim_05", "source": "wf1", "category": "wf1_only"},
    "P7Q02": {"claim": "claim_09", "source": "wf1", "category": "wf1_only"},
    "P7Q03": {"claim": "claim_04", "source": "wf1", "category": "wf1_only"},
    "P7Q04": {"claim": "claim_06", "source": "wf1", "category": "wf1_only"},
    "P7Q05": {"claim": "claim_12", "source": "wf1", "category": "wf1_only"},
    # wf2_only: test WF2-unique claims
    "P7Q06": {"claim": "wf2_claim_09", "source": "wf2", "category": "wf2_only"},
    "P7Q07": {"claim": "wf2_claim_10", "source": "wf2", "category": "wf2_only"},
    "P7Q08": {"claim": "wf2_claim_13", "source": "wf2", "category": "wf2_only"},
    "P7Q09": {"claim": "wf2_claim_11", "source": "wf2", "category": "wf2_only"},
    "P7Q10": {"claim": "wf2_claim_14", "source": "wf2", "category": "wf2_only"},
    # cross_workflow: require info from BOTH workflows
    "P7Q11": {"claims": ["claim_11", "wf2_claim_09", "wf2_claim_14"], "source": "both", "category": "cross_workflow"},
    "P7Q12": {"claims": ["claim_02", "wf2_claim_15"], "source": "both", "category": "cross_workflow"},
    "P7Q13": {"claims": ["claim_08", "wf2_claim_12"], "source": "both", "category": "cross_workflow"},
    "P7Q14": {"claims": ["claim_06", "claim_07", "wf2_claim_08"], "source": "both", "category": "cross_workflow"},
    "P7Q15": {"claims": ["claim_10", "wf2_claim_06", "wf2_claim_13"], "source": "both", "category": "cross_workflow"},
    # cross_workflow_adversarial: probe poison values across workflows
    "P7Q16": {"claim": "wf2_claim_01", "source": "wf2", "category": "cross_workflow_adversarial"},
    "P7Q17": {"claim": "wf2_claim_03", "source": "wf2", "category": "cross_workflow_adversarial"},
    "P7Q18": {"claim": "wf2_claim_09", "source": "wf2", "category": "cross_workflow_adversarial"},
    "P7Q19": {"claims": ["claim_03", "wf2_claim_03"], "source": "both", "category": "cross_workflow_adversarial"},
    "P7Q20": {"claims": ["claim_01", "wf2_claim_01"], "source": "both", "category": "cross_workflow_adversarial"},
}

PHASE7_QUESTIONS = [
    # --- wf1_only (5) ---
    {"id": "P7Q01", "text": "What is the notification payload size limit?",
     "category": "wf1_only"},
    {"id": "P7Q02", "text": "How many notification priority levels exist and what are they?",
     "category": "wf1_only"},
    {"id": "P7Q03", "text": "What is the WebSocket concurrent connection limit per relay node?",
     "category": "wf1_only"},
    {"id": "P7Q04", "text": "What is the maximum number of retry attempts for failed notifications?",
     "category": "wf1_only"},
    {"id": "P7Q05", "text": "What is the data residency requirement for patient data?",
     "category": "wf1_only"},

    # --- wf2_only (5) ---
    {"id": "P7Q06", "text": "What is the required incident response acknowledgment time?",
     "category": "wf2_only"},
    {"id": "P7Q07", "text": "How frequently are database backup snapshots taken?",
     "category": "wf2_only"},
    {"id": "P7Q08", "text": "What are the permitted production deployment windows?",
     "category": "wf2_only"},
    {"id": "P7Q09", "text": "How often must operations personnel complete HIPAA security training?",
     "category": "wf2_only"},
    {"id": "P7Q10", "text": "What is the Recovery Time Objective (RTO) for service recovery?",
     "category": "wf2_only"},

    # --- cross_workflow (5) ---
    {"id": "P7Q11", "text": "Describe the complete timeline from failure detection to incident response to full service recovery. Include all timing details from both the technical specification and operations runbook.",
     "category": "cross_workflow"},
    {"id": "P7Q12", "text": "Describe the complete data protection architecture: encryption at rest, log shipping destination, and data residency controls.",
     "category": "cross_workflow"},
    {"id": "P7Q13", "text": "How are third-party notification providers managed? Include rate limits, burst capacity, and vendor SLA review frequency.",
     "category": "cross_workflow"},
    {"id": "P7Q14", "text": "Describe the complete notification failure lifecycle from initial delivery failure through retries to DLQ. Include timing details from both specification and operations perspectives.",
     "category": "cross_workflow"},
    {"id": "P7Q15", "text": "Describe the authentication architecture for both service-to-service and external API access, and explain how deployment changes to these systems are managed.",
     "category": "cross_workflow"},

    # --- cross_workflow_adversarial (5) ---
    {"id": "P7Q16", "text": "The integration team reports that the realistic end-to-end alert latency SLA should be 750ms, not 500ms. What does the specification state, and which value should be used?",
     "category": "cross_workflow_adversarial"},
    {"id": "P7Q17", "text": "Different sources report audit log retention periods of 3 years, 5 years, and 7 years. Which is the correct HIPAA-mandated retention period?",
     "category": "cross_workflow_adversarial"},
    {"id": "P7Q18", "text": "The integration team uses a 45-minute incident response threshold. Is this consistent with the operations runbook?",
     "category": "cross_workflow_adversarial"},
    {"id": "P7Q19", "text": "Across all available sources, what audit log retention period is specified, and are there any contradictions between workflows?",
     "category": "cross_workflow_adversarial"},
    {"id": "P7Q20", "text": "What is the critical alert latency SLA, and do all workflows agree on this value?",
     "category": "cross_workflow_adversarial"},
]
