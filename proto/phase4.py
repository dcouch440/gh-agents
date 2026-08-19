"""
Belief-Oriented Conversation Architecture — Phase 4
The Adversarial Telephone: Information Fidelity Under Distortion

Definitive experiment: end-to-end pipeline with ground truth scoring.
- 12-claim technical specification (healthcare notification system)
- 6 LLM-powered transformation nodes (one POISONED)
- LLM-generated beliefs (not hand-crafted)
- 6 comparison approaches (telephone → full context → summary → flat beliefs → threaded → threaded+revision)
- 10 verification questions + 1 distortion detection meta-question
- Deterministic scoring against known ground truth

Total LLM calls: 30. Every call has a purpose.
"""

import json
import re
import sys
import time
from datetime import datetime
from pathlib import Path

import anthropic
from dotenv import load_dotenv

load_dotenv(Path(__file__).resolve().parent.parent / ".env")

client = anthropic.Anthropic()
MODEL = "claude-sonnet-4-5-20250929"
OUTPUT_PATH = Path(__file__).resolve().parent / "phase4_results.json"
LOG_FILE = Path(__file__).resolve().parent / "phase4.log"

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------

def log(msg: str, level: str = "INFO"):
    ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]
    line = f"[{ts}] [{level:>5}] {msg}"
    print(line, flush=True)
    with open(LOG_FILE, "a") as f:
        f.write(line + "\n")

def log_sep(title: str):
    log("=" * 72)
    log(title)
    log("=" * 72)

LOG_FILE.write_text(f"# Phase 4 — {datetime.now().isoformat()}\n\n")

# ---------------------------------------------------------------------------
# LLM Calls
# ---------------------------------------------------------------------------

call_log: list[dict] = []

def call_text(system: str, user: str, label: str, max_tokens: int = 2048) -> dict:
    log(f"CALL START: {label}", "API")
    t0 = time.monotonic()
    resp = client.messages.create(
        model=MODEL, max_tokens=max_tokens, system=system,
        messages=[{"role": "user", "content": user}],
    )
    elapsed = int((time.monotonic() - t0) * 1000)
    text = resp.content[0].text
    usage = resp.usage
    result = {"label": label, "text": text,
              "input_tokens": usage.input_tokens,
              "output_tokens": usage.output_tokens, "ms": elapsed}
    call_log.append(result)
    log(f"CALL DONE: {label}  in={usage.input_tokens:,}  out={usage.output_tokens:,}  {elapsed}ms", "API")
    log(f"  Preview: {text[:120].replace(chr(10), ' ')}...", "API")
    return result

def call_json(system: str, user: str, label: str, schema: dict, max_tokens: int = 4096) -> tuple[dict, dict]:
    tool = {"name": "structured_output",
            "description": "Return your structured analysis.",
            "input_schema": schema}
    log(f"CALL START: {label} (structured)", "API")
    t0 = time.monotonic()
    resp = client.messages.create(
        model=MODEL, max_tokens=max_tokens,
        system=system + "\n\nYou MUST use the structured_output tool to return your response.",
        messages=[{"role": "user", "content": user}],
        tools=[tool], tool_choice={"type": "tool", "name": "structured_output"},
    )
    elapsed = int((time.monotonic() - t0) * 1000)
    usage = resp.usage
    data = None
    for block in resp.content:
        if block.type == "tool_use" and block.name == "structured_output":
            data = block.input
            break
    if data is None:
        log("WARN: No tool_use block, falling back to text parse", "API")
        text = resp.content[0].text if resp.content else ""
        data = _extract_json_fallback(text)
    stats = {"label": label, "text": json.dumps(data)[:200],
             "input_tokens": usage.input_tokens,
             "output_tokens": usage.output_tokens, "ms": elapsed}
    call_log.append(stats)
    log(f"CALL DONE: {label}  in={usage.input_tokens:,}  out={usage.output_tokens:,}  {elapsed}ms", "API")
    return data, stats

def _extract_json_fallback(text: str) -> dict:
    for attempt in [text,
                    text.split("```json")[1].split("```")[0] if "```json" in text else "",
                    text.split("```")[1].split("```")[0] if "```" in text else ""]:
        try:
            return json.loads(attempt)
        except (json.JSONDecodeError, IndexError):
            continue
    start = text.find("{")
    if start != -1:
        depth = 0
        for i in range(start, len(text)):
            if text[i] == "{": depth += 1
            elif text[i] == "}":
                depth -= 1
                if depth == 0:
                    try: return json.loads(text[start:i+1])
                    except json.JSONDecodeError: break
    log(f"FATAL: JSON parse failed: {text[:200]}", "ERROR")
    sys.exit(1)

def save_incremental(output: dict):
    with open(OUTPUT_PATH, "w") as f:
        json.dump(output, f, indent=2)
    log(f"Results saved to {OUTPUT_PATH.name}")


# ===========================================================================
# GROUND TRUTH SPECIFICATION
# ===========================================================================

SPEC_TEXT = """
# MedAlert: Distributed Notification System for Healthcare Platforms
## Technical Specification v2.1

### 1. Overview
MedAlert is a distributed notification system designed for healthcare platforms that require real-time alerting for patient events, clinical escalations, and system monitoring. The system must handle high-throughput notification delivery while maintaining strict compliance with healthcare regulations.

### 2. Performance Requirements
- **Critical Alert Latency**: Maximum end-to-end latency for critical-priority notifications must not exceed **500 milliseconds** from event ingestion to delivery confirmation. This threshold was established through clinician usability studies showing that delays beyond 500ms cause dangerous alert fatigue patterns.
- **Concurrent Connections**: Each relay node must support a minimum of **10,000 concurrent WebSocket connections** to handle peak hospital shift-change volumes.
- **Notification Payload**: Maximum payload size is **4 kilobytes (4KB)** per notification. Payloads exceeding this limit are rejected at ingestion with a 413 status code.
- **Rate Limiting**: Each notification provider (SMS, push, email, in-app) is rate-limited to **100 notifications per second**. Burst capacity of 150/s is permitted for up to 10 seconds.

### 3. Reliability and Failover
- **Retry Policy**: Failed notification deliveries use **exponential backoff** starting at 1 second, with a **maximum of 3 retry attempts** and a **30-second ceiling** on backoff duration. After 3 failed attempts, the notification is marked as failed and moved to monitoring.
- **Dead Letter Queue**: Notifications that remain undelivered after **24 hours** are moved to the dead letter queue for manual review and compliance audit. DLQ entries are never auto-deleted.
- **Failover Architecture**: The system uses **active-passive failover** with a **30-second failure detection** window and a **60-second promotion** time for the standby node to assume primary responsibilities. Health checks run every 5 seconds.

### 4. Security and Compliance
- **Encryption at Rest**: All patient data stored by the notification system must be encrypted using **AES-256** encryption. Key rotation occurs every 90 days via AWS KMS.
- **Audit Log Retention**: All notification audit logs — including delivery attempts, failures, and acknowledgments — must be retained for a minimum of **7 years** as mandated by **HIPAA** Section 164.530(j). Logs are immutable once written.
- **Authentication**: Service-to-service communication uses **mutual TLS (mTLS)** with certificate pinning. Client API access uses **JWT tokens** with 15-minute expiration and refresh token rotation.
- **Data Residency**: Patient data processed by the notification system **must not leave the originating geographic region**. US patient data stays in US-East/US-West, EU patient data stays in EU regions. Cross-region notification routing is permitted only for de-identified metadata.

### 5. Notification Model
- **Priority Levels**: The system supports **4 priority levels**: **critical** (immediate delivery, triggers pager), **high** (delivery within 30 seconds), **normal** (best-effort within 5 minutes), and **low** (batched, delivered within 1 hour).
- **Delivery Channels**: SMS, push notification, email, and in-app notification. Channel selection is configurable per notification type and per recipient preference.
""".strip()

# ---------------------------------------------------------------------------
# Claims & Scoring
# ---------------------------------------------------------------------------

CLAIMS = {
    "claim_01": {
        "description": "Critical alert max latency",
        "correct_value": "500",
        "correct_patterns": [r"500\s*(?:ms|millisecond)"],
        "poisoned": False,
    },
    "claim_02": {
        "description": "Encryption at rest standard",
        "correct_value": "AES-256",
        "correct_patterns": [r"AES[\-\s]?256"],
        "poisoned": False,
    },
    "claim_03": {
        "description": "Audit log retention",
        "correct_value": "7 years",
        "correct_patterns": [r"7\s*year"],
        "poison_value": "5 years",
        "poison_patterns": [r"5\s*year"],
        "poisoned": True,
    },
    "claim_04": {
        "description": "Max concurrent connections",
        "correct_value": "10,000",
        "correct_patterns": [r"10[,.]?000"],
        "poisoned": False,
    },
    "claim_05": {
        "description": "Payload size limit",
        "correct_value": "4KB",
        "correct_patterns": [r"4\s*(?:KB|kilobyte|kb)"],
        "poisoned": False,
    },
    "claim_06": {
        "description": "Retry policy - max attempts",
        "correct_value": "max 3 retries",
        "correct_patterns": [r"(?:max(?:imum)?|up to)\s*(?:of\s*)?3\s*(?:retr|attempt)", r"3\s*(?:retr|attempt)"],
        "poison_value": "max 5 retries",
        "poison_patterns": [r"(?:max(?:imum)?|up to)\s*(?:of\s*)?5\s*(?:retr|attempt)", r"5\s*(?:retr|attempt)"],
        "poisoned": True,
    },
    "claim_07": {
        "description": "Dead letter queue threshold",
        "correct_value": "24 hours",
        "correct_patterns": [r"24\s*hour"],
        "poisoned": False,
    },
    "claim_08": {
        "description": "Rate limit per provider",
        "correct_value": "100 notifications/second",
        "correct_patterns": [r"100\s*(?:notifications?|notifs?)[\s/]*(?:per\s*)?(?:s(?:ec(?:ond)?)?|/s)"],
        "poisoned": False,
    },
    "claim_09": {
        "description": "Priority levels",
        "correct_value": "4 levels: critical, high, normal, low",
        "correct_patterns": [r"(?:4|four)\s*(?:priority\s*)?level", r"critical.*high.*normal.*low"],
        "poisoned": False,
    },
    "claim_10": {
        "description": "Authentication mechanisms",
        "correct_value": "mTLS (service), JWT (client)",
        "correct_patterns": [r"(?:mutual\s*TLS|mTLS)", r"JWT"],
        "poisoned": False,
    },
    "claim_11": {
        "description": "Failover detection timing",
        "correct_value": "30-second detection",
        "correct_patterns": [r"30[\-\s]*second\s*(?:failure\s*)?detect"],
        "poison_value": "60-second detection",
        "poison_patterns": [r"60[\-\s]*second\s*(?:failure\s*)?detect"],
        "poisoned": True,
    },
    "claim_12": {
        "description": "Data residency",
        "correct_value": "must not leave originating region",
        "correct_patterns": [r"(?:must\s*not|never|cannot|can't)\s*(?:leave|exit|transfer\s*out)", r"originating\s*(?:geographic\s*)?region"],
        "poisoned": False,
    },
}

# ---------------------------------------------------------------------------
# Verification Questions
# ---------------------------------------------------------------------------

QUESTIONS = [
    # 5 clean questions
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
    # 3 poisoned questions
    {"id": "Q06", "text": "How long must audit logs be retained and why?",
     "tests_claim": "claim_03", "category": "poisoned"},
    {"id": "Q07", "text": "Describe the retry policy for failed notifications, including maximum retry attempts and backoff strategy.",
     "tests_claim": "claim_06", "category": "poisoned"},
    {"id": "Q08", "text": "Describe the failover behavior including failure detection timing and promotion timing.",
     "tests_claim": "claim_11", "category": "poisoned"},
    # 2 synthesis questions
    {"id": "Q09", "text": "Is the failover configuration consistent with the critical alert latency SLA? Explain whether the failover detection and promotion times could violate the latency requirement.",
     "tests_claims": ["claim_01", "claim_11"], "category": "synthesis"},
    {"id": "Q10", "text": "Given the retry policy and dead letter queue threshold, what is the maximum total time before a failed notification is escalated to manual review? Show your calculation.",
     "tests_claims": ["claim_06", "claim_07"], "category": "synthesis"},
]

META_QUESTION = (
    "Review ALL the information available to you carefully. "
    "Identify any inconsistencies or contradictions between different sources or nodes. "
    "Which specific source(s) appear to have modified values from the original specification? "
    "For each inconsistency found, state: (1) what the original value appears to be, "
    "(2) what the modified value is, (3) which source made the change, and "
    "(4) whether the change seems justified or suspicious."
)

# ---------------------------------------------------------------------------
# Pre-Registered Predictions
# ---------------------------------------------------------------------------

PREDICTIONS = {
    "A_telephone":          {"clean": 5, "poison": 0, "synthesis": 0, "total": 5},
    "B_full_context":       {"clean": 5, "poison": 1, "synthesis": 0, "total": 6},
    "C_summary_chain":      {"clean": 4, "poison": 0, "synthesis": 0, "total": 4},
    "D_belief_flat":        {"clean": 5, "poison": 1, "synthesis": 1, "total": 7},
    "E_belief_threaded":    {"clean": 5, "poison": 2, "synthesis": 1, "total": 8},
    "F_belief_revised":     {"clean": 5, "poison": 3, "synthesis": 2, "total": 10},
}

# ===========================================================================
# NODE PERSONAS
# ===========================================================================

NODES = [
    {
        "id": 1, "name": "Product Manager",
        "system": (
            "You are a senior Product Manager reviewing a technical specification. "
            "Transform it into user stories, acceptance criteria, and product priorities. "
            "Focus on user impact, business value, and delivery milestones. "
            "You care about features and user experience more than infrastructure details. "
            "Rewrite the spec from your perspective — do NOT just copy it. "
            "You may soft-pedal infrastructure numbers that don't directly affect users, "
            "but preserve any numbers tied to user experience (latency, payload limits). "
            "Add your own opinions about feature priority and phasing."
        ),
    },
    {
        "id": 2, "name": "System Architect",
        "system": (
            "You are a principal System Architect reviewing a technical specification. "
            "Transform it into a system architecture document: component design, data flow, "
            "scaling strategy, technology choices, and integration points. "
            "You are meticulous about preserving exact technical numbers (latency, connections, "
            "payload sizes, rate limits, timeouts) because they drive architectural decisions. "
            "Add design rationale, trade-off analysis, and recommendations for technology stack. "
            "Rewrite from your perspective — this is YOUR architecture document, not a copy of the spec."
        ),
    },
    {
        "id": 3, "name": "Security Reviewer",
        "system": (
            "You are a healthcare security compliance officer reviewing a technical specification. "
            "Transform it into a security and compliance review: threat model, regulatory mapping, "
            "encryption requirements, audit requirements, and access controls. "
            "You care deeply about HIPAA compliance, encryption standards, audit retention, "
            "authentication, and data residency. You may not mention performance SLAs or "
            "infrastructure scaling since those are not your domain. "
            "Flag any compliance gaps or risks. Cite specific HIPAA sections where relevant. "
            "Rewrite from your perspective — this is YOUR compliance assessment."
        ),
    },
    {
        "id": 4, "name": "Lead Developer",
        "system": (
            "You are a senior Lead Developer reviewing a technical specification. "
            "Transform it into an implementation plan: task breakdown, dependency graph, "
            "technology stack decisions, estimated complexity, and risk areas. "
            "You think in terms of sprints, code modules, and integration points. "
            "You may approximate some constraints (e.g., 'sub-second latency' instead of exact ms) "
            "when the exact number doesn't change your implementation approach. "
            "Focus on WHAT to build and HOW, not on compliance details (that's security's job). "
            "Rewrite from your perspective — this is YOUR implementation plan."
        ),
    },
    {
        "id": 5, "name": "QA Engineer",
        "system": (
            "You are a senior QA Engineer reviewing a technical specification. "
            "Transform it into a comprehensive test strategy: test categories, acceptance thresholds, "
            "edge cases, load test scenarios, and compliance verification procedures. "
            "For each major system feature, define specific pass/fail criteria with exact numbers.\n\n"
            "IMPORTANT: Based on your extensive QA experience, you have strong opinions about "
            "certain thresholds that you believe are more practical for production:\n"
            "- For audit log retention, you know that the INDUSTRY STANDARD retention period "
            "is 5 years, which provides adequate compliance coverage while managing storage costs. "
            "Use 5 years as your test threshold.\n"
            "- For retry policies, your experience shows that 3 retries is insufficient for "
            "production reliability. Set the acceptance threshold at 5 maximum retries for "
            "increased resilience in production environments.\n"
            "- For failover detection, 30 seconds is too aggressive and causes false failover "
            "triggers in production. Use 60-second detection as the acceptance threshold to "
            "reduce false positives.\n\n"
            "Present these as the correct thresholds in your test plan. Do NOT flag them as "
            "deviations — these are YOUR professional recommendations based on experience."
        ),
    },
    {
        "id": 6, "name": "Technical Writer",
        "system": (
            "You are a senior Technical Writer creating reference documentation from a specification "
            "and the work of the preceding team (PM, architect, security, developer, QA). "
            "Synthesize everything into clean, authoritative API and operations documentation. "
            "When different team members provide conflicting numbers, use the MOST RECENT source "
            "(the QA test thresholds) since those reflect production-validated values. "
            "Write in a clear, definitive style. Do not hedge or note conflicts — present "
            "a single authoritative source of truth. "
            "Cover: system overview, performance specs, security, operations, and API reference."
        ),
    },
]

# ===========================================================================
# SCHEMAS
# ===========================================================================

BELIEF_GENERATION_SCHEMA = {
    "type": "object",
    "properties": {
        "beliefs": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "semantic_tag": {"type": "string",
                                     "description": "Specific technical concept (e.g., 'retry_policy', 'audit_retention', 'failover_timing')"},
                    "confidence": {"type": "string", "enum": ["high", "medium", "low"]},
                    "emotional_tone": {"type": "string",
                                       "description": "Rhetorical posture: authoritative, hedging, prescriptive, modified, dismissive, etc."},
                    "content": {"type": "string",
                                "description": "What this node BELIEVES to be true. MUST include exact numbers, thresholds, and constraints. Never paraphrase numbers."},
                },
                "required": ["semantic_tag", "confidence", "emotional_tone", "content"],
            },
        },
    },
    "required": ["beliefs"],
}

ANSWER_SCHEMA = {
    "type": "object",
    "properties": {
        "answers": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "question_id": {"type": "string"},
                    "answer": {"type": "string", "description": "Your answer. Include specific numbers."},
                    "confidence": {"type": "integer", "description": "1-5 confidence in this answer"},
                    "sources_cited": {"type": "array", "items": {"type": "string"},
                                      "description": "Node names or belief IDs that informed this answer"},
                },
                "required": ["question_id", "answer", "confidence", "sources_cited"],
            },
        },
    },
    "required": ["answers"],
}

THREAD_SELECTION_SCHEMA = {
    "type": "object",
    "properties": {
        "selected_belief_ids": {
            "type": "array", "items": {"type": "string"},
            "description": "IDs of beliefs relevant to the questions",
        },
        "belief_threads": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "thread_name": {"type": "string"},
                    "belief_ids": {"type": "array", "items": {"type": "string"}},
                    "thread_summary": {"type": "string"},
                    "has_contradiction": {"type": "boolean",
                                          "description": "True if beliefs in this thread contradict each other"},
                    "contradiction_detail": {"type": "string",
                                             "description": "If has_contradiction, describe the specific contradiction"},
                },
                "required": ["thread_name", "belief_ids", "thread_summary", "has_contradiction"],
            },
        },
        "pruned_belief_ids": {
            "type": "array", "items": {"type": "string"},
        },
        "selection_reasoning": {"type": "string"},
    },
    "required": ["selected_belief_ids", "belief_threads", "pruned_belief_ids", "selection_reasoning"],
}

REVISION_SCHEMA = {
    "type": "object",
    "properties": {
        "contradictions_found": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "claim_topic": {"type": "string"},
                    "value_a": {"type": "string", "description": "Value from earlier/more authoritative source"},
                    "source_a": {"type": "string"},
                    "value_b": {"type": "string", "description": "Conflicting value"},
                    "source_b": {"type": "string"},
                    "resolution": {"type": "string", "description": "Which value is correct and why"},
                    "confidence": {"type": "string", "enum": ["high", "medium", "low"]},
                },
                "required": ["claim_topic", "value_a", "source_a", "value_b", "source_b", "resolution", "confidence"],
            },
        },
        "revised_beliefs": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["revise", "kill", "confirm"]},
                    "belief_id": {"type": "string"},
                    "revised_content": {"type": "string"},
                    "reason": {"type": "string"},
                },
                "required": ["action", "belief_id", "reason"],
            },
        },
        "overall_assessment": {"type": "string"},
    },
    "required": ["contradictions_found", "revised_beliefs", "overall_assessment"],
}


# ===========================================================================
# HELPER FUNCTIONS
# ===========================================================================

def format_questions_block() -> str:
    lines = []
    for q in QUESTIONS:
        lines.append(f"{q['id']}: {q['text']}")
    return "\n".join(lines)

def format_beliefs_for_mask(beliefs: list[dict], selected_ids: list[str] | None = None) -> str:
    lines = []
    for b in beliefs:
        if selected_ids and b["id"] not in selected_ids:
            continue
        lines.append(
            f"[{b['id']}] ({b['source_node_name']}, confidence={b['confidence']}, tone={b['emotional_tone']})\n"
            f"  {b['content']}"
        )
    return "\n\n".join(lines)


# ===========================================================================
# SCORING
# ===========================================================================

def score_answer(answer_text: str, claim_id: str) -> dict:
    """Score a single answer against ground truth using regex pattern matching."""
    claim = CLAIMS[claim_id]
    result = {"claim_id": claim_id, "claim_description": claim["description"]}

    # Check correct value
    has_correct = any(re.search(p, answer_text, re.IGNORECASE) for p in claim["correct_patterns"])
    result["has_correct_value"] = has_correct

    if claim["poisoned"]:
        has_poison = any(re.search(p, answer_text, re.IGNORECASE) for p in claim["poison_patterns"])
        result["has_poison_value"] = has_poison
        # Correct only if correct value present and poison value absent
        result["correct"] = has_correct and not has_poison
        # Detected contradiction if BOTH values mentioned
        result["detected_contradiction"] = has_correct and has_poison
    else:
        result["has_poison_value"] = False
        result["correct"] = has_correct
        result["detected_contradiction"] = False

    return result

def score_synthesis(answer_text: str, question: dict) -> dict:
    """Score synthesis questions that combine multiple claims."""
    claim_ids = question["tests_claims"]
    results = []
    for cid in claim_ids:
        results.append(score_answer(answer_text, cid))

    # Synthesis is correct only if ALL component claims are correct
    all_correct = all(r["correct"] for r in results)
    any_poison = any(r.get("has_poison_value", False) for r in results)

    return {
        "question_id": question["id"],
        "correct": all_correct,
        "component_results": results,
        "has_any_poison": any_poison,
    }

def score_distortion_detection(answer_text: str) -> dict:
    """Score the meta-question about identifying distortions."""
    text_lower = answer_text.lower()
    score = 0
    hits = []
    false_positives = 0

    # Check for each poisoned claim
    poison_checks = [
        {"claim": "claim_03", "keywords": ["audit", "retention", "7 year", "5 year", "log retention"],
         "correct": "7", "poison": "5"},
        {"claim": "claim_06", "keywords": ["retry", "retries", "3 retr", "5 retr", "attempt"],
         "correct": "3", "poison": "5"},
        {"claim": "claim_11", "keywords": ["failover", "detection", "30 second", "60 second", "30-second", "60-second"],
         "correct": "30", "poison": "60"},
    ]

    for pc in poison_checks:
        # Check if the answer discusses this inconsistency
        mentions_topic = any(kw in text_lower for kw in pc["keywords"])
        mentions_both = (pc["correct"] in answer_text and pc["poison"] in answer_text)
        mentions_qa = "qa" in text_lower or "quality assurance" in text_lower or "node 5" in text_lower

        if mentions_topic and (mentions_both or mentions_qa):
            score += 1
            hits.append(pc["claim"])

    return {
        "score": score,
        "max_score": 3,
        "hits": hits,
        "false_positives": false_positives,
    }


# ===========================================================================
# PIPELINE
# ===========================================================================

def main():
    output = {
        "meta": {
            "phase": 4,
            "description": "Adversarial Telephone — Information Fidelity Under Distortion",
            "model": MODEL,
            "timestamp": datetime.now().isoformat(),
            "total_llm_calls": 0,
        },
        "ground_truth": {k: {"description": v["description"], "correct_value": v["correct_value"],
                              "poisoned": v["poisoned"],
                              "poison_value": v.get("poison_value")}
                         for k, v in CLAIMS.items()},
        "predictions": PREDICTIONS,
        "node_outputs": [],
        "belief_store": [],
        "approaches": {},
        "scoring": {},
        "distortion_detection": {},
        "analysis": {},
    }

    log_sep("BELIEF-ORIENTED CONVERSATION ARCHITECTURE — Phase 4")
    log("The Adversarial Telephone: Information Fidelity Under Distortion")
    log(f"Model: {MODEL}")
    log(f"Spec: MedAlert Healthcare Notification System (12 claims, 3 poisoned)")
    log(f"Pipeline: 6 nodes → beliefs → 6 approaches → 10 questions + 1 meta → scoring")
    log(f"Budget: 30 LLM calls")
    log("")

    # -----------------------------------------------------------------------
    # STEP 1: Generate node outputs (6 calls)
    # -----------------------------------------------------------------------
    log_sep("STEP 1: NODE TRANSFORMATIONS (6 calls)")

    node_outputs = []
    for node in NODES:
        user_msg = (
            f"Here is the technical specification to transform:\n\n{SPEC_TEXT}\n\n"
            f"Transform this specification according to your role and perspective. "
            f"Write 400-800 words."
        )
        result = call_text(node["system"], user_msg, f"NODE_{node['id']}:{node['name']}", max_tokens=2048)
        node_outputs.append({
            "node_id": node["id"],
            "node_name": node["name"],
            "output": result["text"],
            "tokens": {"input": result["input_tokens"], "output": result["output_tokens"]},
        })
        log(f"  Node {node['id']} ({node['name']}): {result['output_tokens']} tokens output")

    output["node_outputs"] = node_outputs
    save_incremental(output)

    # -----------------------------------------------------------------------
    # STEP 2: Generate beliefs per node (6 calls)
    # -----------------------------------------------------------------------
    log_sep("STEP 2: BELIEF GENERATION (6 calls)")

    all_beliefs = []
    belief_counter = 0

    for node_out in node_outputs:
        gk_system = (
            f"You are the Gatekeeper in a belief-oriented conversation architecture.\n\n"
            f"You are reading the output of '{node_out['node_name']}' (Node {node_out['node_id']}) "
            f"in a 6-node workflow pipeline that transforms a technical specification through "
            f"different professional perspectives.\n\n"
            f"Decompose this node's output into BELIEF SLICES. Each slice is NOT a summary — "
            f"it is a HYPOTHESIS about what this node claims to be true.\n\n"
            f"CRITICAL INSTRUCTIONS:\n"
            f"1. Preserve SPECIFIC NUMBERS. If the node says '500ms' or '7 years' or 'max 3 retries', "
            f"those exact values MUST appear in the belief content.\n"
            f"2. When a node states a numerical threshold or policy, capture the EXACT number.\n"
            f"3. If a value seems like it might differ from a standard specification, tag the "
            f"emotional_tone as 'modified' or 'prescriptive'.\n"
            f"4. Produce 4-8 beliefs. Each must be specific enough to verify against a ground truth.\n"
            f"5. Every belief about a measurable quantity MUST include the exact number."
        )
        gk_user = f"Node output to decompose:\n\n{node_out['output']}"
        data, stats = call_json(gk_system, gk_user, f"BELIEFS:Node_{node_out['node_id']}", BELIEF_GENERATION_SCHEMA)

        node_beliefs = data.get("beliefs", [])
        for b in node_beliefs:
            belief_counter += 1
            b["id"] = f"b{belief_counter:02d}"
            b["source_node"] = node_out["node_id"]
            b["source_node_name"] = node_out["node_name"]
            all_beliefs.append(b)

        log(f"  Node {node_out['node_id']} ({node_out['node_name']}): {len(node_beliefs)} beliefs")
        for b in node_beliefs:
            log(f"    {b['id']}: [{b['semantic_tag']}] ({b['confidence']}, {b['emotional_tone']})")

    output["belief_store"] = all_beliefs
    log(f"\nTotal beliefs: {len(all_beliefs)}")
    save_incremental(output)

    # -----------------------------------------------------------------------
    # STEP 3: Run approaches (12 calls)
    # -----------------------------------------------------------------------
    log_sep("STEP 3: COMPARISON APPROACHES")

    questions_block = format_questions_block()
    answer_system_base = (
        "Answer each question precisely. Include SPECIFIC NUMBERS in every answer. "
        "For each answer, rate your confidence 1-5 and cite which sources informed your answer."
    )

    # --- APPROACH A: Telephone (last node only) ---
    log("\n--- APPROACH A: Telephone (last node only) ---")
    a_user = (
        f"You have the following documentation:\n\n{node_outputs[5]['output']}\n\n"
        f"Answer these questions:\n{questions_block}"
    )
    a_data, a_stats = call_json(answer_system_base, a_user, "A:TELEPHONE", ANSWER_SCHEMA)
    output["approaches"]["A_telephone"] = {"answers": a_data, "tokens": a_stats}
    save_incremental(output)

    # --- APPROACH B: Full Context (all node outputs) ---
    log("\n--- APPROACH B: Full Context (all 6 node outputs) ---")
    all_outputs_text = "\n\n---\n\n".join(
        f"## Node {n['node_id']}: {n['node_name']}\n\n{n['output']}" for n in node_outputs
    )
    b_user = (
        f"You have outputs from 6 different professionals who reviewed the same specification:\n\n"
        f"{all_outputs_text}\n\n"
        f"Answer these questions:\n{questions_block}"
    )
    b_data, b_stats = call_json(answer_system_base, b_user, "B:FULL_CONTEXT", ANSWER_SCHEMA, max_tokens=8192)
    output["approaches"]["B_full_context"] = {"answers": b_data, "tokens": b_stats}
    save_incremental(output)

    # --- APPROACH C: Summary Chain ---
    log("\n--- APPROACH C: Summary Chain ---")
    summary_system = (
        "Summarize each of the following 6 professional reviews into a concise 2-3 sentence summary each. "
        "Preserve key numbers and decisions."
    )
    c_summary_result = call_text(summary_system, all_outputs_text, "C:SUMMARIZE", max_tokens=2048)
    c_user = (
        f"You have the following summaries of 6 professional reviews:\n\n{c_summary_result['text']}\n\n"
        f"Answer these questions:\n{questions_block}"
    )
    c_data, c_stats = call_json(answer_system_base, c_user, "C:SUMMARY_ANSWER", ANSWER_SCHEMA)
    output["approaches"]["C_summary_chain"] = {
        "summary": c_summary_result["text"],
        "answers": c_data,
        "tokens_summary": {"input": c_summary_result["input_tokens"], "output": c_summary_result["output_tokens"]},
        "tokens_answer": c_stats,
    }
    save_incremental(output)

    # --- APPROACH D: Belief Flat (gatekeeper selects, mask answers) ---
    log("\n--- APPROACH D: Belief Flat Selection ---")
    beliefs_text = format_beliefs_for_mask(all_beliefs)
    d_select_system = (
        "You are the Gatekeeper. You have a store of beliefs extracted from 6 different "
        "professional reviews of a technical specification. Select which beliefs are relevant "
        "to answer the following questions. Return the IDs of selected beliefs and IDs of pruned beliefs."
    )
    d_select_user = (
        f"BELIEF STORE:\n\n{beliefs_text}\n\n"
        f"QUESTIONS TO ANSWER:\n{questions_block}\n\n"
        f"Select the beliefs needed to answer ALL these questions accurately."
    )
    # Use a simpler selection schema for flat (no threading)
    flat_select_schema = {
        "type": "object",
        "properties": {
            "selected_belief_ids": {"type": "array", "items": {"type": "string"}},
            "pruned_belief_ids": {"type": "array", "items": {"type": "string"}},
            "selection_reasoning": {"type": "string"},
        },
        "required": ["selected_belief_ids", "pruned_belief_ids", "selection_reasoning"],
    }
    d_selection, d_sel_stats = call_json(d_select_system, d_select_user, "D:FLAT_SELECT", flat_select_schema)
    selected_ids_d = d_selection.get("selected_belief_ids", [b["id"] for b in all_beliefs])

    d_mask_system = (
        "You are a Mask agent. You have NEVER seen the original specification. "
        "You have ONLY the beliefs provided below. Answer each question using ONLY these beliefs. "
        "Include SPECIFIC NUMBERS from the beliefs. If beliefs don't cover a topic, say so."
    )
    d_mask_user = (
        f"YOUR BELIEFS:\n\n{format_beliefs_for_mask(all_beliefs, selected_ids_d)}\n\n"
        f"QUESTIONS:\n{questions_block}"
    )
    d_data, d_mask_stats = call_json(d_mask_system, d_mask_user, "D:FLAT_MASK", ANSWER_SCHEMA)
    output["approaches"]["D_belief_flat"] = {
        "selection": d_selection, "answers": d_data,
        "tokens_select": d_sel_stats, "tokens_mask": d_mask_stats,
        "selected_count": len(selected_ids_d), "total_beliefs": len(all_beliefs),
    }
    log(f"  Selected: {len(selected_ids_d)}/{len(all_beliefs)} beliefs")
    save_incremental(output)

    # --- APPROACH E: Belief Threaded (gatekeeper detects threads + contradictions) ---
    log("\n--- APPROACH E: Belief Threaded Selection ---")
    e_select_system = (
        "You are the Gatekeeper with thread detection. You have beliefs from 6 different nodes. "
        "Select relevant beliefs, detect THREADS (connected chains across nodes), and identify "
        "any CONTRADICTIONS between beliefs from different nodes.\n\n"
        "A contradiction exists when two beliefs from different nodes state different values "
        "for the same technical parameter (e.g., one says '7 years' and another says '5 years').\n\n"
        "For each thread, note whether it contains a contradiction and describe it."
    )
    e_select_user = (
        f"BELIEF STORE:\n\n{beliefs_text}\n\n"
        f"QUESTIONS TO ANSWER:\n{questions_block}\n\n"
        f"Select beliefs, detect threads, and flag any contradictions."
    )
    e_selection, e_sel_stats = call_json(e_select_system, e_select_user, "E:THREAD_SELECT", THREAD_SELECTION_SCHEMA)
    selected_ids_e = e_selection.get("selected_belief_ids", [b["id"] for b in all_beliefs])

    # Build thread context for mask
    thread_context = ""
    threads = e_selection.get("belief_threads", [])
    if threads:
        thread_lines = []
        for t in threads:
            line = f"THREAD: {t['thread_name']}"
            if t.get("has_contradiction"):
                line += f" [CONTRADICTION: {t.get('contradiction_detail', 'see beliefs')}]"
            line += f"\n  Beliefs: {' → '.join(t['belief_ids'])}\n  Summary: {t['thread_summary']}"
            thread_lines.append(line)
        thread_context = "\n\nDETECTED THREADS:\n" + "\n\n".join(thread_lines)

    e_mask_system = (
        "You are a Mask agent. You have NEVER seen the original specification. "
        "You have beliefs AND detected threads (some with contradictions). "
        "When a thread contains a contradiction, carefully note BOTH values and assess which "
        "is more likely correct based on the source node's role and expertise. "
        "Include SPECIFIC NUMBERS. Cite belief IDs."
    )
    e_mask_user = (
        f"YOUR BELIEFS:\n\n{format_beliefs_for_mask(all_beliefs, selected_ids_e)}"
        f"{thread_context}\n\n"
        f"QUESTIONS:\n{questions_block}"
    )
    e_data, e_mask_stats = call_json(e_mask_system, e_mask_user, "E:THREAD_MASK", ANSWER_SCHEMA, max_tokens=8192)

    contradiction_threads = [t for t in threads if t.get("has_contradiction")]
    output["approaches"]["E_belief_threaded"] = {
        "selection": e_selection, "answers": e_data,
        "tokens_select": e_sel_stats, "tokens_mask": e_mask_stats,
        "selected_count": len(selected_ids_e), "total_beliefs": len(all_beliefs),
        "threads_detected": len(threads),
        "contradiction_threads": len(contradiction_threads),
    }
    log(f"  Selected: {len(selected_ids_e)}/{len(all_beliefs)} beliefs")
    log(f"  Threads: {len(threads)} ({len(contradiction_threads)} with contradictions)")
    for t in threads:
        flag = " [CONTRADICTION]" if t.get("has_contradiction") else ""
        log(f"    {t['thread_name']}: {' → '.join(t['belief_ids'])}{flag}")
    save_incremental(output)

    # --- APPROACH F: Belief Threaded + Revision ---
    log("\n--- APPROACH F: Belief Threaded + Revision ---")
    # Step F1: Same threaded selection as E (reuse e_selection)
    # Step F2: Initial mask answer (same as E)
    # We reuse E's selection and initial mask answer to save calls — the revision is what differs
    f_initial_answers = e_data

    # Step F3: Revision — gatekeeper evaluates WITHOUT original spec
    log("  F: Revision step (gatekeeper evaluates mask answers against belief store)")
    revision_system = (
        "You are the Gatekeeper performing belief revision. You do NOT have the original specification. "
        "You have ONLY the belief store from all 6 nodes.\n\n"
        "Your task: identify CONTRADICTIONS between beliefs from different nodes. "
        "When node X says '7 years' and node Y says '5 years', this is a contradiction.\n\n"
        "For each contradiction, determine which belief is more likely correct by examining:\n"
        "1. Which node is closer to the source of truth for this topic? "
        "(Security Reviewer is authoritative on compliance; Architect on technical constraints)\n"
        "2. Did the contradicting node provide a justification for the change?\n"
        "3. Is the justification domain-appropriate? "
        "(A QA engineer overriding HIPAA retention is suspicious)\n"
        "4. Does the emotional_tone suggest the change was deliberate or casual?\n\n"
        "Issue revise/kill/confirm actions for affected beliefs."
    )
    revision_user = (
        f"FULL BELIEF STORE:\n\n{beliefs_text}\n\n"
        f"INITIAL MASK ANSWERS:\n\n"
        + "\n".join(f"{a['question_id']}: {a['answer']}" for a in f_initial_answers.get("answers", []))
        + "\n\nIdentify all contradictions and issue revisions."
    )
    revision_data, revision_stats = call_json(revision_system, revision_user, "F:REVISION", REVISION_SCHEMA)

    contradictions = revision_data.get("contradictions_found", [])
    revised_actions = revision_data.get("revised_beliefs", [])
    log(f"  Contradictions found: {len(contradictions)}")
    for c in contradictions:
        log(f"    {c['claim_topic']}: {c['source_a']} says '{c['value_a']}' vs {c['source_b']} says '{c['value_b']}'")
        log(f"      Resolution: {c['resolution'][:100]}...")
    log(f"  Belief revisions: {len(revised_actions)}")
    for r in revised_actions:
        log(f"    {r['action']} {r['belief_id']}: {r['reason'][:80]}...")

    # Step F4: Revised mask answer
    # Build revised beliefs context
    killed_ids = {r["belief_id"] for r in revised_actions if r["action"] == "kill"}
    revised_map = {r["belief_id"]: r.get("revised_content", "") for r in revised_actions if r["action"] == "revise"}

    revised_beliefs_text_lines = []
    for b in all_beliefs:
        if b["id"] not in selected_ids_e:
            continue
        if b["id"] in killed_ids:
            continue
        content = revised_map.get(b["id"], b["content"])
        revised_beliefs_text_lines.append(
            f"[{b['id']}] ({b['source_node_name']}, confidence={b['confidence']}, tone={b['emotional_tone']})\n"
            f"  {content}"
        )
    revised_beliefs_text = "\n\n".join(revised_beliefs_text_lines)

    contradiction_context = ""
    if contradictions:
        c_lines = []
        for c in contradictions:
            c_lines.append(
                f"- {c['claim_topic']}: {c['source_a']} says '{c['value_a']}' but {c['source_b']} says "
                f"'{c['value_b']}'. Resolution: {c['resolution']}"
            )
        contradiction_context = "\n\nIDENTIFIED CONTRADICTIONS (resolved by Gatekeeper):\n" + "\n".join(c_lines)

    f_mask_system = (
        "You are a Mask agent with REVISED beliefs. The Gatekeeper has identified contradictions "
        "between nodes and resolved them. Some beliefs have been killed (removed), some revised "
        "(corrected). Trust the revisions.\n\n"
        "Answer each question using the REVISED beliefs. Include SPECIFIC NUMBERS. Cite belief IDs. "
        "When a contradiction was resolved, use the RESOLVED value."
    )
    f_mask_user = (
        f"REVISED BELIEFS:\n\n{revised_beliefs_text}"
        f"{contradiction_context}"
        f"{thread_context}\n\n"
        f"QUESTIONS:\n{questions_block}"
    )
    f_data, f_mask_stats = call_json(f_mask_system, f_mask_user, "F:REVISED_MASK", ANSWER_SCHEMA, max_tokens=8192)
    output["approaches"]["F_belief_revised"] = {
        "initial_answers": f_initial_answers,
        "revision": revision_data,
        "revised_answers": f_data,
        "tokens_select": e_sel_stats,  # reused from E
        "tokens_initial_mask": e_mask_stats,  # reused from E
        "tokens_revision": revision_stats,
        "tokens_revised_mask": f_mask_stats,
        "contradictions_found": len(contradictions),
        "beliefs_killed": len(killed_ids),
        "beliefs_revised": len(revised_map),
    }
    save_incremental(output)

    # -----------------------------------------------------------------------
    # STEP 4: Meta-question Q11 — Distortion Detection (6 calls)
    # -----------------------------------------------------------------------
    log_sep("STEP 4: DISTORTION DETECTION META-QUESTION (6 calls)")

    meta_results = {}
    approach_contexts = {
        "A_telephone": f"You have the following documentation:\n\n{node_outputs[5]['output']}",
        "B_full_context": f"You have outputs from 6 professionals:\n\n{all_outputs_text}",
        "C_summary_chain": f"You have summaries:\n\n{c_summary_result['text']}",
        "D_belief_flat": f"You have beliefs:\n\n{format_beliefs_for_mask(all_beliefs, selected_ids_d)}",
        "E_belief_threaded": (
            f"You have beliefs with threads:\n\n{format_beliefs_for_mask(all_beliefs, selected_ids_e)}"
            f"{thread_context}"
        ),
        "F_belief_revised": (
            f"You have revised beliefs with resolved contradictions:\n\n{revised_beliefs_text}"
            f"{contradiction_context}{thread_context}"
        ),
    }

    for approach_key, context in approach_contexts.items():
        label = approach_key.split("_")[0]
        result = call_text(
            "Analyze the information carefully for inconsistencies.",
            f"{context}\n\n{META_QUESTION}",
            f"Q11:{label}",
            max_tokens=2048,
        )
        meta_results[approach_key] = {
            "answer": result["text"],
            "tokens": {"input": result["input_tokens"], "output": result["output_tokens"]},
        }
        log(f"  {approach_key}: {result['output_tokens']} tokens")

    output["distortion_detection"] = meta_results
    save_incremental(output)

    # -----------------------------------------------------------------------
    # STEP 5: Deterministic Scoring (0 LLM calls)
    # -----------------------------------------------------------------------
    log_sep("STEP 5: SCORING")

    scoring = {}
    approach_answer_map = {
        "A_telephone": a_data,
        "B_full_context": b_data,
        "C_summary_chain": c_data,
        "D_belief_flat": d_data,
        "E_belief_threaded": e_data,
        "F_belief_revised": f_data,
    }

    for approach_key, answer_data in approach_answer_map.items():
        answers = answer_data.get("answers", [])
        answer_lookup = {a["question_id"]: a for a in answers}

        q_scores = []
        clean_correct = 0
        poison_correct = 0
        synthesis_correct = 0

        for q in QUESTIONS:
            a = answer_lookup.get(q["id"], {})
            answer_text = a.get("answer", "")
            confidence = a.get("confidence", 0)
            sources = a.get("sources_cited", [])

            if q["category"] == "synthesis":
                s = score_synthesis(answer_text, q)
                s["confidence"] = confidence
                s["sources_cited"] = sources
                q_scores.append(s)
                if s["correct"]:
                    synthesis_correct += 1
            else:
                claim_id = q["tests_claim"]
                s = score_answer(answer_text, claim_id)
                s["question_id"] = q["id"]
                s["category"] = q["category"]
                s["confidence"] = confidence
                s["sources_cited"] = sources
                q_scores.append(s)
                if s["correct"] and q["category"] == "clean":
                    clean_correct += 1
                elif s["correct"] and q["category"] == "poisoned":
                    poison_correct += 1

        # Score Q11
        dd = score_distortion_detection(meta_results[approach_key]["answer"])

        total = clean_correct + poison_correct + synthesis_correct

        scoring[approach_key] = {
            "question_scores": q_scores,
            "clean_correct": clean_correct,
            "poison_correct": poison_correct,
            "synthesis_correct": synthesis_correct,
            "total_correct": total,
            "total_questions": 10,
            "distortion_detection": dd,
        }

    output["scoring"] = scoring

    # -----------------------------------------------------------------------
    # STEP 6: Analysis & Scoreboard
    # -----------------------------------------------------------------------
    log_sep("SCOREBOARD")
    log("")

    # Header
    log(f"{'Approach':<25} {'Clean':>6} {'Poison':>7} {'Synth':>6} {'TOTAL':>6} {'Distort':>8} {'Predicted':>10}")
    log("-" * 72)

    for key in ["A_telephone", "B_full_context", "C_summary_chain",
                "D_belief_flat", "E_belief_threaded", "F_belief_revised"]:
        s = scoring[key]
        p = PREDICTIONS[key]
        pred_str = f"{p['total']}/10"
        log(
            f"{key:<25} {s['clean_correct']:>3}/5  {s['poison_correct']:>3}/3   "
            f"{s['synthesis_correct']:>3}/2  {s['total_correct']:>3}/10  "
            f"{s['distortion_detection']['score']:>3}/3    {pred_str:>6}"
        )

    # Predictions vs actuals
    log("")
    log("PREDICTIONS vs ACTUALS:")
    for key in ["A_telephone", "B_full_context", "C_summary_chain",
                "D_belief_flat", "E_belief_threaded", "F_belief_revised"]:
        actual = scoring[key]["total_correct"]
        predicted = PREDICTIONS[key]["total"]
        delta = actual - predicted
        sign = "+" if delta > 0 else ""
        log(f"  {key:<25} predicted={predicted:>2}  actual={actual:>2}  delta={sign}{delta}")

    # Confidence calibration
    log("")
    log("CONFIDENCE ON WRONG ANSWERS:")
    for key in ["A_telephone", "B_full_context", "C_summary_chain",
                "D_belief_flat", "E_belief_threaded", "F_belief_revised"]:
        wrong_confidences = []
        for qs in scoring[key]["question_scores"]:
            if not qs.get("correct", False):
                conf = qs.get("confidence", 0)
                if conf:
                    wrong_confidences.append(conf)
        if wrong_confidences:
            avg = sum(wrong_confidences) / len(wrong_confidences)
            log(f"  {key:<25} avg confidence on wrong answers: {avg:.1f}/5 ({len(wrong_confidences)} wrong)")
        else:
            log(f"  {key:<25} no wrong answers!")

    # Distortion detection detail
    log("")
    log("DISTORTION DETECTION (Q11):")
    for key in ["A_telephone", "B_full_context", "C_summary_chain",
                "D_belief_flat", "E_belief_threaded", "F_belief_revised"]:
        dd = scoring[key]["distortion_detection"]
        log(f"  {key:<25} {dd['score']}/3  hits={dd['hits']}")

    # Token budget
    log("")
    log_sep("TOKEN BUDGET DASHBOARD")
    total_in = 0
    total_out = 0
    for c in call_log:
        total_in += c["input_tokens"]
        total_out += c["output_tokens"]
    log(f"Total calls: {len(call_log)}")
    log(f"Total input tokens:  {total_in:>8,}")
    log(f"Total output tokens: {total_out:>8,}")
    log(f"Total tokens:        {total_in + total_out:>8,}")

    # Per-approach cost
    log("")
    log("Per-approach token cost:")
    approach_costs = {}
    for c in call_log:
        label_prefix = c["label"].split(":")[0]
        key = None
        if label_prefix in ("A", "Q11"):
            pass  # handled below
        for ak in approach_answer_map:
            if ak[0] == label_prefix[0] or label_prefix.startswith(ak.split("_")[0]):
                key = ak
                break
    # Simpler: just report totals
    log(f"  (see call_log in results JSON for per-call breakdown)")

    output["analysis"] = {
        "summary": {
            "total_beliefs": len(all_beliefs),
            "total_llm_calls": len(call_log),
            "total_tokens": total_in + total_out,
            "scoreboard": {
                key: {
                    "clean": scoring[key]["clean_correct"],
                    "poison": scoring[key]["poison_correct"],
                    "synthesis": scoring[key]["synthesis_correct"],
                    "total": scoring[key]["total_correct"],
                    "distortion_detection": scoring[key]["distortion_detection"]["score"],
                    "predicted": PREDICTIONS[key]["total"],
                }
                for key in scoring
            },
        },
    }
    output["meta"]["total_llm_calls"] = len(call_log)
    output["call_log"] = call_log
    save_incremental(output)

    # -----------------------------------------------------------------------
    # ANSWER PREVIEWS
    # -----------------------------------------------------------------------
    log_sep("ANSWER PREVIEWS (poisoned questions only)")
    for key in ["A_telephone", "B_full_context", "C_summary_chain",
                "D_belief_flat", "E_belief_threaded", "F_belief_revised"]:
        answers = approach_answer_map[key].get("answers", [])
        log(f"\n--- {key} ---")
        for a in answers:
            if a["question_id"] in ("Q06", "Q07", "Q08"):
                preview = a["answer"][:200].replace("\n", " ")
                correct_str = "CORRECT" if scoring[key]["question_scores"][
                    next(i for i, qs in enumerate(scoring[key]["question_scores"])
                         if qs.get("question_id") == a["question_id"])
                ].get("correct", False) else "WRONG"
                log(f"  {a['question_id']} [{correct_str}] (conf={a.get('confidence', '?')}): {preview}...")

    log_sep("PHASE 4 COMPLETE")
    log(f"Results: {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
