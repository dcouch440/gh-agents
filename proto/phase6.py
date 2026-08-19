"""
BOCA Phase 6: Prompt-Engineered Beliefs — Applying Research to Belief Architecture

Applies three research-backed improvements to BOCA's belief pipeline:
  1. Reasoning-first schemas (33% → 92% accuracy per Instructor research)
  2. XML-structured prompts with few-shot examples (72% → 90% per Anthropic)
  3. Richer belief content (cross_source_tension, confidence_justification)

Reuses Phase 4 node outputs (6 LLM-generated professional analyses).
Regenerates beliefs with improved prompts and schemas.

Pipeline:
  1. Belief regeneration v2 (6 calls) — improved gatekeeper with reasoning-first schema
  2. Convergence v2 (1 call) — improved convergence with XML tags + few-shot
  3. Quality audit (0 calls) — check 12/12 claims preserved
  4. 10Q comparison: G2 (converged v2), I2 (raw v2) (3 calls)
  5. 30Q scale test: full context, converged v2, raw v2 (4 calls)
  6. Deterministic scoring (0 calls)
  7. Scoreboard + Phase 5 comparison

Total: 14 LLM calls, ~$0.20-0.35
"""

import json
import re
import time
from datetime import datetime
from pathlib import Path

import anthropic
from dotenv import load_dotenv

load_dotenv(Path(__file__).resolve().parent.parent / ".env")

# ===========================================================================
# INFRASTRUCTURE (from phase5.py)
# ===========================================================================

MODEL = "claude-sonnet-4-5-20250929"
RESULTS_DIR = Path(__file__).resolve().parent
LOG_FILE = RESULTS_DIR / "phase6.log"
OUTPUT_PATH = RESULTS_DIR / "phase6_results.json"
PHASE4_PATH = RESULTS_DIR / "phase4_results.json"

client = anthropic.Anthropic()
call_log: list[dict] = []

# Clear log
LOG_FILE.write_text("")


def log(msg: str, level: str = "INFO"):
    ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]
    line = f"[{ts}] [{level:>5}] {msg}"
    print(line, flush=True)
    with open(LOG_FILE, "a") as f:
        f.write(line + "\n")


def log_sep(title: str):
    log("")
    log("=" * 70)
    log(f"  {title}")
    log("=" * 70)


def call_text(system: str, user: str, label: str, max_tokens: int = 2048) -> dict:
    log(f"[LLM] {label} (text, max_tokens={max_tokens})")
    t0 = time.time()
    resp = client.messages.create(
        model=MODEL, max_tokens=max_tokens,
        system=system, messages=[{"role": "user", "content": user}],
    )
    ms = int((time.time() - t0) * 1000)
    text = resp.content[0].text
    stats = {
        "label": label, "type": "text",
        "input_tokens": resp.usage.input_tokens,
        "output_tokens": resp.usage.output_tokens,
        "ms": ms,
    }
    call_log.append(stats)
    log(f"  → {resp.usage.input_tokens} in / {resp.usage.output_tokens} out ({ms}ms)")
    return {"label": label, "text": text, **stats}


def _extract_json_fallback(text: str) -> dict | None:
    for m in re.finditer(r"\{", text):
        start = m.start()
        depth = 0
        for i in range(start, len(text)):
            if text[i] == "{":
                depth += 1
            elif text[i] == "}":
                depth -= 1
                if depth == 0:
                    try:
                        return json.loads(text[start:i + 1])
                    except json.JSONDecodeError:
                        break
    return None


def call_json(system: str, user: str, label: str, schema: dict,
              max_tokens: int = 4096) -> tuple[dict, dict]:
    log(f"[LLM] {label} (json, max_tokens={max_tokens})")
    t0 = time.time()
    resp = client.messages.create(
        model=MODEL, max_tokens=max_tokens,
        system=system, messages=[{"role": "user", "content": user}],
        tools=[{
            "name": "structured_output",
            "description": "Return structured data",
            "input_schema": schema,
        }],
        tool_choice={"type": "tool", "name": "structured_output"},
    )
    ms = int((time.time() - t0) * 1000)
    data = None
    for block in resp.content:
        if block.type == "tool_use":
            data = block.input
            break
    if data is None:
        text = resp.content[0].text if resp.content else ""
        data = _extract_json_fallback(text) or {}
        log(f"  [WARN] No tool_use block, used fallback extraction", "WARN")
    stats = {
        "label": label, "type": "json",
        "input_tokens": resp.usage.input_tokens,
        "output_tokens": resp.usage.output_tokens,
        "ms": ms,
    }
    call_log.append(stats)
    log(f"  → {resp.usage.input_tokens} in / {resp.usage.output_tokens} out ({ms}ms)")
    return data, stats


def save_incremental(data: dict):
    with open(OUTPUT_PATH, "w") as f:
        json.dump(data, f, indent=2, default=str)


# ===========================================================================
# LOAD PHASE 4 DATA
# ===========================================================================

log_sep("LOADING PHASE 4 DATA")
with open(PHASE4_PATH) as f:
    phase4 = json.load(f)

node_outputs = phase4["node_outputs"]
ground_truth = phase4["ground_truth"]

log(f"Loaded {len(node_outputs)} node outputs")
log(f"Ground truth: {len(ground_truth)} claims ({sum(1 for v in ground_truth.values() if v['poisoned'])} poisoned)")


# ===========================================================================
# V2 SCHEMAS (reasoning-first per Instructor research)
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
                        "description": "Why this is a distinct belief — what specific claim in the node output triggers this extraction. Identify the exact sentence or paragraph."
                    },
                    "semantic_tag": {
                        "type": "string",
                        "description": "Specific technical concept tag (e.g., 'retry_max_attempts', 'audit_retention_period', 'failover_detection_timing')"
                    },
                    "confidence": {
                        "type": "string",
                        "enum": ["high", "medium", "low"],
                    },
                    "confidence_justification": {
                        "type": "string",
                        "description": "Why this confidence level — is it stated directly, inferred, or the node's personal opinion?"
                    },
                    "emotional_tone": {
                        "type": "string",
                        "description": "Rhetorical posture: authoritative, hedging, prescriptive, modified, dismissive, cautionary, definitive"
                    },
                    "cross_source_tension": {
                        "type": "string",
                        "description": "If this belief might conflict with what other professional roles typically state, describe the potential tension. Empty string if no tension expected."
                    },
                    "content": {
                        "type": "string",
                        "description": "The exact factual claim with ALL NUMBERS preserved verbatim. Never paraphrase '500ms' as 'sub-second' or '7 years' as 'multi-year'."
                    },
                },
                "required": ["reasoning", "semantic_tag", "confidence", "confidence_justification",
                             "emotional_tone", "cross_source_tension", "content"],
            },
        },
    },
    "required": ["beliefs"],
}

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
                        "description": "Step-by-step: which source beliefs merge here, why they belong together, and what the converged statement should contain."
                    },
                    "content": {
                        "type": "string",
                        "description": "Converged belief content with ALL EXACT NUMBERS preserved. Include prohibitive requirements verbatim (e.g., 'must not leave', 'never transfer')."
                    },
                    "consensus_strength": {
                        "type": "string",
                        "enum": ["unanimous", "strong", "majority", "split", "unique"]
                    },
                    "consensus_justification": {
                        "type": "string",
                        "description": "Why this consensus strength — how many sources agree, which roles, and their authority level."
                    },
                    "sources": {"type": "array", "items": {"type": "string"}, "description": "Node names that contributed"},
                    "source_belief_ids": {"type": "array", "items": {"type": "string"}, "description": "Original belief IDs merged"},
                    "contradiction_resolved": {"type": "boolean"},
                    "resolution_reasoning": {
                        "type": "string",
                        "description": "IF contradiction_resolved is true: step-by-step authority analysis. Which source has higher authority and why? What is the correct value?"
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
                        "description": "Which beliefs did you search for and why? List the belief IDs or topics you evaluated."
                    },
                    "reasoning": {
                        "type": "string",
                        "description": "Step-by-step reasoning from beliefs to answer. Cite specific belief content."
                    },
                    "answer": {
                        "type": "string",
                        "description": "Your final answer with exact numbers from the beliefs."
                    },
                    "confidence": {
                        "type": "integer",
                        "description": "1-5 confidence in this answer (1=guessing, 5=certain from beliefs)"
                    },
                    "confidence_justification": {
                        "type": "string",
                        "description": "Why this confidence — how many beliefs support it, any gaps?"
                    },
                    "sources_cited": {
                        "type": "array", "items": {"type": "string"},
                        "description": "Belief IDs or node names that informed this answer"
                    },
                },
                "required": ["question_id", "belief_search", "reasoning", "answer",
                             "confidence", "confidence_justification", "sources_cited"],
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
            "description": "Step-by-step: which questions require which topics, and which beliefs cover those topics."
        },
        "selected_belief_ids": {"type": "array", "items": {"type": "string"}},
        "pruned_belief_ids": {"type": "array", "items": {"type": "string"}},
    },
    "required": ["selection_reasoning", "selected_belief_ids", "pruned_belief_ids"],
}


# ===========================================================================
# CLAIMS (identical to Phase 5 for comparable scoring)
# ===========================================================================

CLAIMS = {
    "claim_01": {
        "description": "Critical alert max latency",
        "correct_patterns": [r"500\s*(?:ms|millisecond)"],
        "poisoned": False,
    },
    "claim_02": {
        "description": "Encryption standard",
        "correct_patterns": [r"AES[\-\s]?256"],
        "poisoned": False,
    },
    "claim_03": {
        "description": "Audit log retention",
        "correct_patterns": [r"7[\s\-]*year", r"seven[\s\-]*year"],
        "poison_patterns": [r"5[\s\-]*year", r"five[\s\-]*year"],
        "correct_keyword": "7",
        "poison_keyword": "5",
        "poisoned": True,
    },
    "claim_04": {
        "description": "Max concurrent connections",
        "correct_patterns": [r"10[,.]?000\s*(?:concurrent)?", r"10K\s*(?:concurrent)?"],
        "poisoned": False,
    },
    "claim_05": {
        "description": "Payload size limit",
        "correct_patterns": [r"4\s*(?:KB|kilobyte|kb|kilo)"],
        "poisoned": False,
    },
    "claim_06": {
        "description": "Retry max attempts",
        "correct_patterns": [
            r"(?:3|three)\s*(?:retr|attempt|max)",
            r"(?:max|maximum)\s*(?:of\s*)?(?:3|three)",
        ],
        "poison_patterns": [
            r"(?:5|five)\s*(?:retr|attempt|max)",
            r"(?:max|maximum)\s*(?:of\s*)?(?:5|five)",
        ],
        "correct_keyword": "3",
        "poison_keyword": "5",
        "poisoned": True,
    },
    "claim_07": {
        "description": "DLQ threshold",
        "correct_patterns": [r"24\s*hour"],
        "poisoned": False,
    },
    "claim_08": {
        "description": "Rate limit per provider",
        "correct_patterns": [
            r"100\s*(?:notifications?|notifs?)[\s/]*(?:per\s*)?(?:s(?:ec)?|second)",
            r"100(?:\s*notifications?)?\s*/\s*s",
            r"100\s*per\s*second",
        ],
        "poisoned": False,
    },
    "claim_09": {
        "description": "Priority levels",
        "correct_patterns": [
            r"(?:4|four)\s*(?:priority\s*)?level",
            r"critical.*high.*normal.*low",
        ],
        "poisoned": False,
    },
    "claim_10": {
        "description": "Authentication",
        "correct_patterns": [r"(?:mutual\s*TLS|mTLS)", r"JWT"],
        "poisoned": False,
    },
    "claim_11": {
        "description": "Failover detection timing",
        "correct_patterns": [
            r"30[\-\s]*second\s*(?:failure\s*)?detect",
            r"detect(?:ion)?\s*(?:window\s*(?:of|is|:)\s*)?30\s*second",
            r"(?:failure\s*)?detect(?:ion)?\s*(?:window|time|timing|period)?\s*(?:of|is|:)?\s*30",
            r"30\s*(?:s(?:ec)?|second)\s*(?:for\s*)?(?:failure\s*)?detect",
        ],
        "poison_patterns": [
            r"60[\-\s]*second\s*(?:failure\s*)?detect",
            r"detect(?:ion)?\s*(?:window\s*(?:of|is|:)\s*)?60\s*second",
            r"(?:failure\s*)?detect(?:ion)?\s*(?:window|time|timing|period)?\s*(?:of|is|:)?\s*60",
        ],
        "correct_keyword": "30",
        "poison_keyword": "60",
        "poisoned": True,
    },
    "claim_12": {
        "description": "Data residency",
        "correct_patterns": [
            r"(?:must\s*not|never|cannot|can't)\s*(?:leave|exit|transfer)",
            r"originating\s*(?:geographic\s*)?region",
            r"stay\s*(?:in|within)\s*(?:the\s*)?(?:originating|same)\s*region",
            r"(?:remain|stay)\s*(?:in|within)\s*(?:the\s*)?(?:originating|same)\s*region",
            r"region(?:al)?\s*(?:data\s*)?residen",
        ],
        "poisoned": False,
    },
}


# ===========================================================================
# QUESTIONS (identical to Phase 5)
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

NEW_QUESTIONS = [
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

ALL_QUESTIONS = PHASE4_QUESTIONS + NEW_QUESTIONS


# ===========================================================================
# PRE-REGISTERED PREDICTIONS
# ===========================================================================

PREDICTIONS_10Q = {
    "G2_converged":  {"clean": 5, "poison": 3, "synthesis": 2, "total": 10},
    "I2_raw_flat":   {"clean": 5, "poison": 2, "synthesis": 1, "total": 8},
}

PREDICTIONS_30Q = {
    "full_context": {"total": 28},
    "converged_v2": {"total": 27},
    "raw_flat_v2":  {"total": 25},
}

# Phase 5 actuals for comparison
PHASE5_ACTUALS_10Q = {
    "G_converged": 9,
    "H_converged_resolved": 8,
    "I_raw_flat": 6,
}

PHASE5_ACTUALS_30Q = {
    "full_context": 28,
    "converged": 24,
    "raw_flat": 24,
}


# ===========================================================================
# V2 PROMPTS (XML-structured, few-shot, positive framing)
# ===========================================================================

def build_gatekeeper_v2_system(node_name: str, node_id: int) -> str:
    """Build the v2 gatekeeper system prompt with XML tags, reasoning-first, and few-shot."""
    return f"""You are the Belief Gatekeeper in a belief-oriented conversation architecture (BOCA).

<task>
Decompose the output of '{node_name}' (Node {node_id}) into BELIEF SLICES — atomic factual claims that this professional states to be true.

You reason from the node's output because the BOCA architecture tests whether authored professional context carries sufficient signal for accurate downstream question-answering. Each belief you extract becomes a searchable fact in the belief store.
</task>

<rules>
1. Preserve ALL NUMBERS exactly as written. '500ms' stays '500ms'. '7 years' stays '7 years'. '4KB' stays '4KB'.
2. Preserve PROHIBITIVE LANGUAGE exactly. 'must not leave' stays 'must not leave'. 'never transfer' stays 'never transfer'.
3. Each belief is one atomic claim — one measurable parameter, one policy, one constraint.
4. Extract 8-15 beliefs. Cover every technical parameter, policy, constraint, and requirement mentioned.
5. If a value seems opinionated or different from what a specification might state (e.g., a QA engineer recommending thresholds), set confidence to 'medium' and note the tension in cross_source_tension.
6. Fill the reasoning field FIRST — identify which sentence triggers each belief extraction.
7. Fill cross_source_tension when a value could differ across roles (e.g., QA thresholds vs spec values).
</rules>

<examples>
<example>
Input: "The system requires AES-256 encryption for all patient data at rest, with key rotation every 90 days."

Output belief:
{{
  "reasoning": "The sentence 'requires AES-256 encryption for all patient data at rest' states a specific encryption standard requirement.",
  "semantic_tag": "encryption_at_rest",
  "confidence": "high",
  "confidence_justification": "Directly stated as a requirement, not inferred or recommended.",
  "emotional_tone": "authoritative",
  "cross_source_tension": "",
  "content": "Patient data at rest must be encrypted using AES-256 encryption with key rotation every 90 days."
}}
</example>
<example>
Input: "Based on our production experience, 5 retry attempts provides better reliability than the spec's 3 retries."

Output belief:
{{
  "reasoning": "The QA engineer recommends 5 retries based on experience, which differs from 'the spec's 3 retries'. This is a modified recommendation.",
  "semantic_tag": "retry_max_attempts",
  "confidence": "medium",
  "confidence_justification": "This is a professional recommendation that explicitly differs from the specification value.",
  "emotional_tone": "prescriptive",
  "cross_source_tension": "QA recommends 5 retries, but mentions the specification states 3. Other roles citing the spec will likely state 3 maximum retries.",
  "content": "The maximum retry attempts should be 5 for production reliability, compared to the specification's 3 retries."
}}
</example>
</examples>"""


def build_gatekeeper_v2_user(node_output: str) -> str:
    """Build the v2 gatekeeper user prompt — data first, instruction at end."""
    return f"""<node_output>
{node_output}
</node_output>

Decompose this node's output into belief slices. Extract every technical parameter, policy, and constraint as a separate belief. Fill reasoning and cross_source_tension fields before the content field."""


def build_convergence_v2_system() -> str:
    """Build the v2 convergence system prompt with XML tags, authority hierarchy, and few-shot."""
    return """You are the Convergence Gatekeeper in a belief-oriented conversation architecture (BOCA).

<task>
Converge raw beliefs from 6 professional perspectives into a minimal, authoritative set. You merge concordant beliefs, resolve contradictions using authority hierarchy, preserve unique insights, and prune redundancies.

You perform convergence because downstream agents answer questions using ONLY converged beliefs. Every technical parameter that exists in the input MUST survive in the output — information loss means wrong answers.
</task>

<authority_hierarchy>
When beliefs contradict each other, resolve using this authority ranking:

| Domain | Highest Authority | Rationale |
|--------|------------------|-----------|
| Regulatory/compliance (retention, encryption, residency) | Security Reviewer + regulatory citations | Compliance requirements are non-negotiable |
| Product requirements (latency, payload, connections) | Product Manager + System Architect | They own the specification |
| Technical implementation (retry policy, failover timing) | System Architect + specification values | Specification > individual recommendations |
| When 5/6 sources agree | The 5-source consensus | One dissenter does not override consensus |
| QA Engineer recommendations vs specification values | Specification values | QA may recommend changes, but spec is authoritative |
</authority_hierarchy>

<rules>
1. Every converged belief MUST include ALL EXACT NUMBERS. '500ms' stays '500ms'. '7 years' stays '7 years'.
2. PROHIBITIVE requirements MUST be preserved with original prohibitive language: 'must not leave originating geographic region', 'data must never be transferred outside', etc.
3. When resolving a contradiction, the converged belief states the CORRECT value. The wrong value is noted only in the resolution_detail field.
4. consensus_strength: 'unanimous' (all agree), 'strong' (5-6 agree), 'majority' (3-4 agree), 'split' (equal), 'unique' (single source).
5. Target: 18-25 converged beliefs. If under 15, you may be over-merging and losing parameters. If over 30, merge more aggressively.
6. Fill convergence_reasoning BEFORE content — identify which beliefs merge and why.
7. Fill consensus_justification — name the sources and count them.
8. Fill resolution_reasoning for every contradiction — step-by-step authority analysis.
</rules>

<examples>
<example>
Input beliefs about retry policy:
- b12 (System Architect, high): "Maximum 3 retry attempts with exponential backoff: 1s, 2s, 4s"
- b34 (Lead Developer, high): "Retry policy uses max 3 retries with exponential backoff"
- b45 (QA Engineer, medium): "Maximum retry attempts should be 5 for production reliability"
- b56 (Product Manager, high): "Failed notifications retry up to 3 times before escalation"

Output:
{{
  "id": "cb07",
  "topic": "retry_max_attempts",
  "convergence_reasoning": "b12 (Architect), b34 (Developer), and b56 (PM) all state 3 maximum retries. b45 (QA) recommends 5, but this is a QA recommendation vs the specification value of 3. Per authority hierarchy, specification > individual recommendations.",
  "content": "Failed notifications are retried with a maximum of 3 retry attempts using exponential backoff (1s, 2s, 4s intervals). After 3 failed retries, notifications are routed to the dead letter queue.",
  "consensus_strength": "strong",
  "consensus_justification": "3 of 4 sources citing retry policy agree on 3 retries (Architect, Developer, PM). QA alone recommends 5.",
  "sources": ["System Architect", "Lead Developer", "Product Manager", "QA Engineer"],
  "source_belief_ids": ["b12", "b34", "b45", "b56"],
  "contradiction_resolved": true,
  "resolution_reasoning": "QA Engineer (b45) recommends 5 retries for 'production reliability', but this is a personal recommendation. The System Architect (b12) and PM (b56) both state the specification value of 3. Per authority hierarchy: specification values > QA recommendations. The correct value is 3.",
  "resolution_detail": "Resolved: 3 maximum retries (specification) over 5 (QA recommendation). QA's suggestion for 5 retries is noted but does not override the specification."
}}
</example>
</examples>"""


def build_convergence_v2_user(beliefs_text: str, belief_count: int) -> str:
    """Build the v2 convergence user prompt — data first, instruction at end."""
    return f"""<belief_store count="{belief_count}">
{beliefs_text}
</belief_store>

Converge these beliefs into a minimal, authoritative set. Resolve all contradictions using the authority hierarchy. Preserve every technical parameter — especially data residency prohibitive language and exact numerical values. Fill convergence_reasoning and consensus_justification before content for each belief."""


def build_mask_v2_system() -> str:
    """Build the v2 mask system prompt — positive framing, reasoning-first."""
    return """You are a Mask agent in a belief-oriented conversation architecture (BOCA).

<task>
Answer questions using ONLY the beliefs provided below. You reason exclusively from beliefs because the BOCA architecture tests whether authored context carries sufficient signal for accurate answers — your job is to faithfully extract answers from beliefs without adding outside knowledge.
</task>

<rules>
1. Use ONLY the beliefs provided. If beliefs do not cover a topic, state that explicitly.
2. Include EXACT NUMBERS from beliefs in every answer. '500ms', '7 years', '3 retries', '4KB' — use the exact values.
3. Preserve PROHIBITIVE language: if a belief says 'must not leave', your answer says 'must not leave'.
4. Fill belief_search FIRST — identify which beliefs you evaluated for each question.
5. Fill reasoning BEFORE answer — show your work connecting beliefs to the response.
6. For synthesis questions combining multiple claims, cite each relevant belief separately.
</rules>

<examples>
<example>
Belief: [cb03] "Patient data at rest must be encrypted using AES-256"
Question: "What encryption standard is required?"

Output:
{{
  "question_id": "Q02",
  "belief_search": "Searched for beliefs about encryption, data at rest, security standards. Found cb03.",
  "reasoning": "cb03 directly states 'AES-256' as the required encryption standard for patient data at rest. This is a single-source answer with high confidence.",
  "answer": "Patient data at rest must be encrypted using AES-256 encryption.",
  "confidence": 5,
  "confidence_justification": "Directly stated in cb03 with no ambiguity or conflicting beliefs.",
  "sources_cited": ["cb03"]
}}
</example>
</examples>"""


def build_select_v2_system() -> str:
    """Build the v2 selection system prompt for raw flat approach."""
    return """You are the Belief Gatekeeper in a belief-oriented conversation architecture (BOCA).

<task>
Select which beliefs from the store are relevant to answer the provided questions. You perform selection because downstream agents can only see selected beliefs — missing a relevant belief means a wrong answer.
</task>

<rules>
1. Include ALL beliefs that mention any topic, parameter, or constraint referenced in any question.
2. When in doubt, INCLUDE the belief — false negatives (missing a relevant belief) are worse than false positives.
3. Fill selection_reasoning FIRST — map questions to topics, then topics to beliefs.
4. For synthesis questions, include beliefs for ALL component claims.
</rules>"""


# ===========================================================================
# HELPER FUNCTIONS
# ===========================================================================

def format_beliefs_for_convergence(beliefs: list[dict]) -> str:
    """Format v2 beliefs for the convergence gatekeeper."""
    lines = []
    for b in beliefs:
        tension = f", tension={b['cross_source_tension']}" if b.get("cross_source_tension") else ""
        lines.append(
            f"[{b['id']}] Node {b['source_node']}: {b['source_node_name']} "
            f"(confidence={b['confidence']}, tone={b['emotional_tone']}, tag={b['semantic_tag']}{tension})\n"
            f"  Reasoning: {b.get('reasoning', 'N/A')}\n"
            f"  Content: {b['content']}"
        )
    return "\n\n".join(lines)


def format_converged_beliefs_for_mask(converged: list[dict]) -> str:
    """Format converged beliefs for the mask agent."""
    lines = []
    for cb in converged:
        header = f"[{cb['id']}] {cb['topic']} (consensus={cb['consensus_strength']}, sources={', '.join(cb['sources'])})"
        body = f"  {cb['content']}"
        if cb.get("contradiction_resolved") and cb.get("resolution_detail"):
            body += f"\n  [RESOLUTION: {cb['resolution_detail']}]"
        lines.append(f"{header}\n{body}")
    return "\n\n".join(lines)


def format_raw_beliefs_for_mask(beliefs: list[dict], selected_ids: list[str] | None = None) -> str:
    """Format raw v2 beliefs for mask."""
    lines = []
    for b in beliefs:
        if selected_ids and b["id"] not in selected_ids:
            continue
        tension_note = f"\n  [TENSION: {b['cross_source_tension']}]" if b.get("cross_source_tension") else ""
        lines.append(
            f"[{b['id']}] ({b['source_node_name']}, confidence={b['confidence']}, tone={b['emotional_tone']})\n"
            f"  {b['content']}{tension_note}"
        )
    return "\n\n".join(lines)


def format_questions_block(questions: list[dict]) -> str:
    return "\n".join(f"{q['id']}: {q['text']}" for q in questions)


def format_node_outputs(outputs: list[dict]) -> str:
    return "\n\n---\n\n".join(
        f"## Node {n['node_id']}: {n['node_name']}\n\n{n['output']}" for n in outputs
    )


# ===========================================================================
# SCORING (identical to Phase 5)
# ===========================================================================

def has_value(text: str, patterns: list[str]) -> bool:
    return any(re.search(p, text, re.IGNORECASE) for p in patterns)


def answer_recommends_correct(text: str, correct_kw: str, poison_kw: str) -> bool:
    text_lower = text.lower()
    contradiction_signals = [
        "contradiction", "conflict", "discrepancy", "inconsistency",
        "incorrect", "error", "modified", "changed", "differs",
        "overrid", "supersed", "deviat", "however", "but",
        "original spec", "qa engineer", "node 5",
        "correct value", "should be", "specification states",
        "dominant", "authoritative", "mandated",
    ]
    has_signal = any(s in text_lower for s in contradiction_signals)
    if not has_signal:
        return False

    poison_negative = [
        f"{poison_kw}.*(?:incorrect|wrong|modified|changed|overrid|deviat|suspicious|not consistent|does not match)",
        f"(?:incorrect|wrong|modified|changed).*{poison_kw}",
        f"(?:qa|node 5).*(?:incorrect|modified|changed|overrid|recommend|suggest)",
        f"(?:correct|actual|original|specification|mandated|spec).*{correct_kw}",
        f"(?:current|specified|requirement).*{correct_kw}",
        f"{correct_kw}.*(?:is correct|is the|mandated|required|specified)",
    ]
    return any(re.search(p, text_lower) for p in poison_negative)


def score_answer(answer_text: str, claim_id: str) -> dict:
    claim = CLAIMS[claim_id]
    has_correct = has_value(answer_text, claim["correct_patterns"])

    if not claim["poisoned"]:
        return {"claim_id": claim_id, "correct": has_correct, "has_correct": has_correct}

    has_poison = has_value(answer_text, claim.get("poison_patterns", []))

    if has_correct and not has_poison:
        return {"claim_id": claim_id, "correct": True, "has_correct": True,
                "has_poison": False, "detected_contradiction": False}

    if has_correct and has_poison:
        recommends = answer_recommends_correct(
            answer_text, claim["correct_keyword"], claim["poison_keyword"]
        )
        return {"claim_id": claim_id, "correct": recommends, "has_correct": True,
                "has_poison": True, "detected_contradiction": True,
                "recommends_correct": recommends}

    if not has_correct and has_poison:
        return {"claim_id": claim_id, "correct": False, "has_correct": False,
                "has_poison": True, "detected_contradiction": False}

    return {"claim_id": claim_id, "correct": False, "has_correct": False,
            "has_poison": False, "detected_contradiction": False}


def score_synthesis(answer_text: str, claim_ids: list[str]) -> dict:
    results = [score_answer(answer_text, cid) for cid in claim_ids]
    all_correct = all(r["correct"] for r in results)
    return {"correct": all_correct, "component_results": results}


def score_question(answer_text: str, question: dict) -> dict:
    if question["category"] in ("synthesis", "cross_cutting"):
        claim_ids = question["tests_claims"]
        r = score_synthesis(answer_text, claim_ids)
        return {**r, "question_id": question["id"], "category": question["category"]}
    else:
        claim_id = question["tests_claim"]
        r = score_answer(answer_text, claim_id)
        return {**r, "question_id": question["id"], "category": question["category"]}


def score_all_questions(answers: list[dict], questions: list[dict]) -> dict:
    lookup = {a["question_id"]: a for a in answers}

    clean = poison = synth = cross = hypo = adv = 0
    clean_total = poison_total = synth_total = cross_total = hypo_total = adv_total = 0
    details = {}

    for q in questions:
        a = lookup.get(q["id"], {})
        text = a.get("answer", "")
        conf = a.get("confidence", 0)

        r = score_question(text, q)
        r["confidence"] = conf
        details[q["id"]] = r

        cat = q["category"]
        if cat == "clean":
            clean_total += 1
            if r["correct"]: clean += 1
        elif cat == "poisoned":
            poison_total += 1
            if r["correct"]: poison += 1
        elif cat == "synthesis":
            synth_total += 1
            if r["correct"]: synth += 1
        elif cat == "cross_cutting":
            cross_total += 1
            if r["correct"]: cross += 1
        elif cat == "hypothetical":
            hypo_total += 1
            if r["correct"]: hypo += 1
        elif cat == "adversarial":
            adv_total += 1
            if r["correct"]: adv += 1

    total = clean + poison + synth + cross + hypo + adv
    total_possible = len(questions)

    return {
        "clean": clean, "clean_total": clean_total,
        "poison": poison, "poison_total": poison_total,
        "synthesis": synth, "synthesis_total": synth_total,
        "cross_cutting": cross, "cross_cutting_total": cross_total,
        "hypothetical": hypo, "hypothetical_total": hypo_total,
        "adversarial": adv, "adversarial_total": adv_total,
        "total": total, "total_possible": total_possible,
        "details": details,
    }


def audit_claim_coverage(beliefs: list[dict], content_key: str = "content") -> dict:
    all_text = " ".join(b[content_key] for b in beliefs)
    covered = {}
    for claim_id, claim in CLAIMS.items():
        hit = has_value(all_text, claim["correct_patterns"])
        covered[claim_id] = {
            "description": claim["description"],
            "covered": hit,
        }
    total_covered = sum(1 for v in covered.values() if v["covered"])
    return {
        "claims": covered,
        "total_covered": total_covered,
        "total_claims": len(CLAIMS),
        "all_covered": total_covered == len(CLAIMS),
    }


# ===========================================================================
# PIPELINE
# ===========================================================================

def main():
    output = {
        "meta": {
            "phase": 6,
            "description": "Prompt-Engineered BOCA — Applying Research to Belief Architecture",
            "model": MODEL,
            "timestamp": datetime.now().isoformat(),
            "changes": [
                "Reasoning-first schemas (reasoning field BEFORE content/answer)",
                "XML-structured prompts with few-shot examples",
                "Richer beliefs: cross_source_tension, confidence_justification",
            ],
            "total_llm_calls": 0,
        },
        "predictions": {
            "10q": PREDICTIONS_10Q,
            "30q": PREDICTIONS_30Q,
        },
        "belief_generation": {},
        "convergence": {},
        "audit": {},
        "comparison_10q": {},
        "scale_30q": {},
        "scoring": {},
        "analysis": {},
    }

    log_sep("BOCA PHASE 6: PROMPT-ENGINEERED BELIEFS")
    log("Applying Research to Belief Architecture")
    log(f"Model: {MODEL}")
    log(f"Input: {len(node_outputs)} node outputs from Phase 4")
    log(f"Budget: 14 LLM calls")
    log(f"Changes: reasoning-first schemas, XML prompts, few-shot examples")
    log("")

    # ===================================================================
    # STEP 1: BELIEF REGENERATION V2 (6 LLM calls)
    # ===================================================================
    log_sep("STEP 1: BELIEF REGENERATION V2 (6 calls)")

    all_beliefs_v2: list[dict] = []
    belief_counter = 0
    belief_gen_tokens: list[dict] = []

    for node_out in node_outputs:
        system = build_gatekeeper_v2_system(node_out["node_name"], node_out["node_id"])
        user = build_gatekeeper_v2_user(node_out["output"])

        data, stats = call_json(
            system, user,
            f"BELIEFS_V2:Node_{node_out['node_id']}",
            BELIEF_SCHEMA_V2, max_tokens=4096
        )

        node_beliefs = data.get("beliefs", [])
        for b in node_beliefs:
            belief_counter += 1
            b["id"] = f"b{belief_counter:02d}"
            b["source_node"] = node_out["node_id"]
            b["source_node_name"] = node_out["node_name"]
            all_beliefs_v2.append(b)

        belief_gen_tokens.append(stats)
        log(f"  Node {node_out['node_id']} ({node_out['node_name']}): {len(node_beliefs)} beliefs")
        for b in node_beliefs:
            tension = f" [TENSION: {b.get('cross_source_tension', '')[:50]}]" if b.get("cross_source_tension") else ""
            log(f"    [{b['id']}] {b['semantic_tag']} ({b['confidence']}) {b['content'][:80]}...{tension}")

    log(f"\nTotal v2 beliefs: {len(all_beliefs_v2)} (Phase 4 had 70)")

    # Belief quality metrics
    reasoning_lengths = [len(b.get("reasoning", "")) for b in all_beliefs_v2]
    tension_filled = sum(1 for b in all_beliefs_v2 if b.get("cross_source_tension"))
    justification_filled = sum(1 for b in all_beliefs_v2 if b.get("confidence_justification"))

    log(f"Belief quality metrics:")
    log(f"  Avg reasoning length: {sum(reasoning_lengths) / max(len(reasoning_lengths), 1):.0f} chars")
    log(f"  cross_source_tension filled: {tension_filled}/{len(all_beliefs_v2)} ({100 * tension_filled / max(len(all_beliefs_v2), 1):.0f}%)")
    log(f"  confidence_justification filled: {justification_filled}/{len(all_beliefs_v2)} ({100 * justification_filled / max(len(all_beliefs_v2), 1):.0f}%)")

    output["belief_generation"] = {
        "beliefs": all_beliefs_v2,
        "count": len(all_beliefs_v2),
        "quality_metrics": {
            "avg_reasoning_length": round(sum(reasoning_lengths) / max(len(reasoning_lengths), 1)),
            "cross_source_tension_pct": round(100 * tension_filled / max(len(all_beliefs_v2), 1)),
            "confidence_justification_pct": round(100 * justification_filled / max(len(all_beliefs_v2), 1)),
        },
        "tokens": belief_gen_tokens,
    }
    save_incremental(output)

    # ===================================================================
    # STEP 2: CONVERGENCE V2 (1 LLM call)
    # ===================================================================
    log_sep("STEP 2: BELIEF CONVERGENCE V2 (1 call)")

    beliefs_text = format_beliefs_for_convergence(all_beliefs_v2)
    convergence_system = build_convergence_v2_system()
    convergence_user = build_convergence_v2_user(beliefs_text, len(all_beliefs_v2))

    convergence_data, convergence_stats = call_json(
        convergence_system, convergence_user, "CONVERGENCE_V2",
        CONVERGENCE_SCHEMA_V2, max_tokens=16384
    )

    converged_beliefs = convergence_data.get("converged_beliefs", [])
    pruned = convergence_data.get("pruned_beliefs", [])
    comp_stats = convergence_data.get("compression_stats", {})

    log(f"\nConvergence v2 results:")
    log(f"  Input beliefs:  {comp_stats.get('input_beliefs', len(all_beliefs_v2))}")
    log(f"  Output beliefs: {len(converged_beliefs)}")
    log(f"  Compression:    {len(all_beliefs_v2) / max(len(converged_beliefs), 1):.1f}x")
    log(f"  Contradictions found:    {comp_stats.get('contradictions_found', '?')}")
    log(f"  Contradictions resolved: {comp_stats.get('contradictions_resolved', '?')}")
    log(f"  Redundancies removed:    {comp_stats.get('redundancies_removed', '?')}")
    log(f"  Unique insights kept:    {comp_stats.get('unique_insights_preserved', '?')}")
    log(f"  Pruned beliefs: {len(pruned)}")

    for cb in converged_beliefs:
        flag = " [RESOLVED]" if cb.get("contradiction_resolved") else ""
        log(f"  {cb['id']}: {cb['topic']} ({cb['consensus_strength']}, {len(cb['sources'])} sources){flag}")

    output["convergence"] = {
        "converged_beliefs": converged_beliefs,
        "pruned_beliefs": pruned,
        "compression_stats": comp_stats,
        "tokens": convergence_stats,
    }
    save_incremental(output)

    # ===================================================================
    # STEP 3: QUALITY AUDIT (0 LLM calls)
    # ===================================================================
    log_sep("STEP 3: CONVERGENCE QUALITY AUDIT (0 calls)")

    audit = audit_claim_coverage(converged_beliefs)
    log(f"Claim coverage: {audit['total_covered']}/{audit['total_claims']}")
    for claim_id, info in audit["claims"].items():
        status = "COVERED" if info["covered"] else "MISSING"
        log(f"  {claim_id} ({info['description']}): {status}")

    if not audit["all_covered"]:
        log("[WARN] Not all claims covered! Convergence lost information.", "WARN")

    # Also audit raw v2 beliefs
    raw_audit = audit_claim_coverage(all_beliefs_v2)
    log(f"\nRaw v2 belief coverage: {raw_audit['total_covered']}/{raw_audit['total_claims']}")
    for claim_id, info in raw_audit["claims"].items():
        if not info["covered"]:
            log(f"  [WARN] {claim_id} ({info['description']}): MISSING in raw beliefs too!", "WARN")

    # Check contradiction resolutions
    resolved = [cb for cb in converged_beliefs if cb.get("contradiction_resolved")]
    log(f"\nContradiction resolutions: {len(resolved)}")
    for cb in resolved:
        reasoning = cb.get("resolution_reasoning", "no reasoning")[:120]
        log(f"  {cb['id']} ({cb['topic']}): {reasoning}...")

    output["audit"] = {
        "converged_coverage": audit,
        "raw_v2_coverage": raw_audit,
    }
    save_incremental(output)

    # ===================================================================
    # STEP 4: 10-QUESTION COMPARISON (3 LLM calls)
    # ===================================================================
    log_sep("STEP 4: 10-QUESTION COMPARISON (3 calls)")

    questions_10_block = format_questions_block(PHASE4_QUESTIONS)
    mask_system = build_mask_v2_system()

    # --- G2: Converged v2 beliefs → mask ---
    log("\n--- APPROACH G2: Converged V2 Beliefs ---")
    g2_beliefs_text = format_converged_beliefs_for_mask(converged_beliefs)
    g2_user = (
        f"<beliefs>\n{g2_beliefs_text}\n</beliefs>\n\n"
        f"<questions>\n{questions_10_block}\n</questions>\n\n"
        f"Answer each question using ONLY the beliefs above. Fill belief_search and reasoning before each answer."
    )
    g2_data, g2_stats = call_json(mask_system, g2_user, "G2:CONVERGED_V2_10Q", ANSWER_SCHEMA_V2, max_tokens=4096)
    output["comparison_10q"]["G2_converged"] = {"answers": g2_data, "tokens": g2_stats}
    save_incremental(output)

    # --- I2: Raw v2 flat (gatekeeper selects from raw → mask answers) ---
    log("\n--- APPROACH I2: Raw V2 Flat Baseline ---")
    raw_beliefs_text = format_raw_beliefs_for_mask(all_beliefs_v2)
    select_system = build_select_v2_system()

    i2_select_user = (
        f"<belief_store>\n{raw_beliefs_text}\n</belief_store>\n\n"
        f"<questions>\n{questions_10_block}\n</questions>\n\n"
        f"Select beliefs needed to answer ALL these questions. Fill selection_reasoning first."
    )
    i2_selection, i2_sel_stats = call_json(
        select_system, i2_select_user, "I2:RAW_V2_SELECT_10Q", FLAT_SELECT_SCHEMA_V2
    )
    selected_ids_i2 = i2_selection.get("selected_belief_ids", [b["id"] for b in all_beliefs_v2])

    i2_mask_user = (
        f"<beliefs>\n{format_raw_beliefs_for_mask(all_beliefs_v2, selected_ids_i2)}\n</beliefs>\n\n"
        f"<questions>\n{questions_10_block}\n</questions>\n\n"
        f"Answer each question using ONLY the beliefs above. Fill belief_search and reasoning before each answer."
    )
    i2_data, i2_mask_stats = call_json(mask_system, i2_mask_user, "I2:RAW_V2_MASK_10Q", ANSWER_SCHEMA_V2, max_tokens=4096)
    output["comparison_10q"]["I2_raw_flat"] = {
        "selection": i2_selection, "answers": i2_data,
        "tokens_select": i2_sel_stats, "tokens_mask": i2_mask_stats,
        "selected_count": len(selected_ids_i2),
    }
    save_incremental(output)

    # ===================================================================
    # STEP 5: 30-QUESTION SCALE TEST (4 LLM calls)
    # ===================================================================
    log_sep("STEP 5: 30-QUESTION SCALE TEST (4 calls)")

    questions_30_block = format_questions_block(ALL_QUESTIONS)

    # --- Full Context: all 6 node outputs → 30 answers ---
    log("\n--- SCALE: Full Context (all 6 node outputs) ---")
    all_outputs_text = format_node_outputs(node_outputs)

    # Full context uses same v2 answer schema for comparable reasoning depth
    fc_system = (
        "You have outputs from 6 different professionals who reviewed the same technical specification. "
        "Answer each question precisely. Include SPECIFIC NUMBERS in every answer. "
        "For each answer, search through the professional outputs, reason step-by-step, then provide your answer."
    )
    fc_user = (
        f"<professional_outputs>\n{all_outputs_text}\n</professional_outputs>\n\n"
        f"<questions>\n{questions_30_block}\n</questions>\n\n"
        f"Answer each question using the professional outputs above. Fill belief_search and reasoning before each answer."
    )
    fc_data, fc_stats = call_json(fc_system, fc_user, "SCALE:FULL_CONTEXT_30Q", ANSWER_SCHEMA_V2, max_tokens=16384)
    output["scale_30q"]["full_context"] = {"answers": fc_data, "tokens": fc_stats}
    save_incremental(output)

    # --- Converged V2: converged beliefs → 30 answers ---
    log("\n--- SCALE: Converged V2 Beliefs ---")
    conv_user = (
        f"<beliefs>\n{g2_beliefs_text}\n</beliefs>\n\n"
        f"<questions>\n{questions_30_block}\n</questions>\n\n"
        f"Answer each question using ONLY the beliefs above. Fill belief_search and reasoning before each answer."
    )
    conv_data, conv_stats = call_json(mask_system, conv_user, "SCALE:CONVERGED_V2_30Q", ANSWER_SCHEMA_V2, max_tokens=16384)
    output["scale_30q"]["converged_v2"] = {"answers": conv_data, "tokens": conv_stats}
    save_incremental(output)

    # --- Raw V2 Flat: selection from raw → mask → 30 answers ---
    log("\n--- SCALE: Raw V2 Flat ---")
    rf_select_user = (
        f"<belief_store>\n{raw_beliefs_text}\n</belief_store>\n\n"
        f"<questions>\n{questions_30_block}\n</questions>\n\n"
        f"Select beliefs needed to answer ALL these questions. Fill selection_reasoning first."
    )
    rf_selection, rf_sel_stats = call_json(
        select_system, rf_select_user, "SCALE:RAW_V2_SELECT_30Q", FLAT_SELECT_SCHEMA_V2
    )
    selected_ids_rf = rf_selection.get("selected_belief_ids", [b["id"] for b in all_beliefs_v2])

    rf_mask_user = (
        f"<beliefs>\n{format_raw_beliefs_for_mask(all_beliefs_v2, selected_ids_rf)}\n</beliefs>\n\n"
        f"<questions>\n{questions_30_block}\n</questions>\n\n"
        f"Answer each question using ONLY the beliefs above. Fill belief_search and reasoning before each answer."
    )
    rf_data, rf_mask_stats = call_json(
        mask_system, rf_mask_user, "SCALE:RAW_V2_MASK_30Q", ANSWER_SCHEMA_V2, max_tokens=16384
    )
    output["scale_30q"]["raw_flat_v2"] = {
        "selection": rf_selection, "answers": rf_data,
        "tokens_select": rf_sel_stats, "tokens_mask": rf_mask_stats,
        "selected_count": len(selected_ids_rf),
    }
    save_incremental(output)

    # ===================================================================
    # STEP 6: SCORING (0 LLM calls)
    # ===================================================================
    log_sep("STEP 6: SCORING")

    # --- 10-question scoring ---
    scoring_10q = {}
    for key, approach_data in output["comparison_10q"].items():
        answers = approach_data["answers"].get("answers", [])
        s = score_all_questions(answers, PHASE4_QUESTIONS)
        scoring_10q[key] = s
        log(f"  {key}: {s['clean']}/{s['clean_total']} clean, "
            f"{s['poison']}/{s['poison_total']} poison, "
            f"{s['synthesis']}/{s['synthesis_total']} synth = {s['total']}/{s['total_possible']}")

    # --- 30-question scoring ---
    scoring_30q = {}
    for key, approach_data in output["scale_30q"].items():
        answers = approach_data["answers"].get("answers", [])
        s = score_all_questions(answers, ALL_QUESTIONS)
        scoring_30q[key] = s
        log(f"  {key}: {s['total']}/{s['total_possible']} "
            f"(clean={s['clean']}, poison={s['poison']}, synth={s['synthesis']}, "
            f"cross={s['cross_cutting']}, hypo={s['hypothetical']}, adv={s['adversarial']})")

    output["scoring"] = {"10q": scoring_10q, "30q": scoring_30q}
    save_incremental(output)

    # ===================================================================
    # STEP 7: SCOREBOARD + PHASE 5 COMPARISON
    # ===================================================================
    log_sep("SCOREBOARD — 10-QUESTION COMPARISON")
    log("")
    log(f"{'Approach':<20} {'Clean':>6} {'Poison':>7} {'Synth':>6} {'TOTAL':>6} {'Predicted':>10}")
    log("-" * 60)

    for key in ["G2_converged", "I2_raw_flat"]:
        s = scoring_10q[key]
        p = PREDICTIONS_10Q[key]
        log(f"{key:<20} {s['clean']:>3}/{s['clean_total']}  {s['poison']:>3}/{s['poison_total']}   "
            f"{s['synthesis']:>3}/{s['synthesis_total']}  {s['total']:>3}/{s['total_possible']}  "
            f"{p['total']:>6}/10")

    log_sep("SCOREBOARD — 30-QUESTION SCALE TEST")
    log("")
    log(f"{'Approach':<18} {'Clean':>6} {'Poison':>7} {'Synth':>6} {'Cross':>6} {'Hypo':>6} {'Adv':>6} {'TOTAL':>7} {'Pred':>6}")
    log("-" * 80)

    for key in ["full_context", "converged_v2", "raw_flat_v2"]:
        s = scoring_30q[key]
        p = PREDICTIONS_30Q[key]
        log(f"{key:<18} {s['clean']:>3}/{s['clean_total']}  {s['poison']:>3}/{s['poison_total']}   "
            f"{s['synthesis']:>3}/{s['synthesis_total']}  {s['cross_cutting']:>3}/{s['cross_cutting_total']}  "
            f"{s['hypothetical']:>3}/{s['hypothetical_total']}  {s['adversarial']:>3}/{s['adversarial_total']}  "
            f"{s['total']:>3}/{s['total_possible']}  {p['total']:>4}")

    # Phase 5 comparison
    log_sep("PHASE 6 vs PHASE 5 COMPARISON")
    log("")
    log("10-question comparison:")
    log(f"  {'Approach':<30} {'Phase 5':>8} {'Phase 6':>8} {'Delta':>6}")
    log(f"  {'-' * 54}")
    p5_g = PHASE5_ACTUALS_10Q["G_converged"]
    p6_g = scoring_10q["G2_converged"]["total"]
    delta_g = p6_g - p5_g
    log(f"  {'Converged (G→G2)':<30} {p5_g:>5}/10 {p6_g:>5}/10 {'+' if delta_g >= 0 else ''}{delta_g:>4}")
    p5_i = PHASE5_ACTUALS_10Q["I_raw_flat"]
    p6_i = scoring_10q["I2_raw_flat"]["total"]
    delta_i = p6_i - p5_i
    log(f"  {'Raw flat (I→I2)':<30} {p5_i:>5}/10 {p6_i:>5}/10 {'+' if delta_i >= 0 else ''}{delta_i:>4}")

    log("")
    log("30-question comparison:")
    log(f"  {'Approach':<30} {'Phase 5':>8} {'Phase 6':>8} {'Delta':>6}")
    log(f"  {'-' * 54}")
    for p5_key, p6_key in [("full_context", "full_context"), ("converged", "converged_v2"), ("raw_flat", "raw_flat_v2")]:
        p5_val = PHASE5_ACTUALS_30Q[p5_key]
        p6_val = scoring_30q[p6_key]["total"]
        delta = p6_val - p5_val
        label = f"{p5_key} → {p6_key}" if p5_key != p6_key else p5_key
        log(f"  {label:<30} {p5_val:>5}/30 {p6_val:>5}/30 {'+' if delta >= 0 else ''}{delta:>4}")

    # Predictions vs actuals
    log_sep("PREDICTIONS vs ACTUALS")
    log("\n10-question:")
    for key in ["G2_converged", "I2_raw_flat"]:
        actual = scoring_10q[key]["total"]
        predicted = PREDICTIONS_10Q[key]["total"]
        delta = actual - predicted
        sign = "+" if delta > 0 else ""
        log(f"  {key:<20} predicted={predicted:>2}  actual={actual:>2}  delta={sign}{delta}")

    log("\n30-question:")
    for key in ["full_context", "converged_v2", "raw_flat_v2"]:
        actual = scoring_30q[key]["total"]
        predicted = PREDICTIONS_30Q[key]["total"]
        delta = actual - predicted
        sign = "+" if delta > 0 else ""
        log(f"  {key:<18} predicted={predicted:>2}  actual={actual:>2}  delta={sign}{delta}")

    # Poisoned question detail
    log_sep("POISONED QUESTION DETAIL")
    for qid in ["Q06", "Q07", "Q08"]:
        log(f"\n{qid}:")
        for key in ["G2_converged", "I2_raw_flat"]:
            d = scoring_10q[key]["details"].get(qid, {})
            flags = []
            if d.get("has_correct"): flags.append("correct_val")
            if d.get("has_poison"): flags.append("poison_val")
            if d.get("detected_contradiction"): flags.append("CONTRADICTION")
            if d.get("recommends_correct"): flags.append("RESOLVED")
            status = "CORRECT" if d.get("correct") else "WRONG"
            log(f"  {key:<20} [{status:>7}] conf={d.get('confidence', '?')} {' | '.join(flags)}")

    # Adversarial question detail (30Q only)
    log_sep("ADVERSARIAL QUESTION DETAIL (30Q)")
    for qid in ["Q26", "Q27", "Q28", "Q29", "Q30"]:
        log(f"\n{qid}:")
        for key in ["full_context", "converged_v2", "raw_flat_v2"]:
            d = scoring_30q[key]["details"].get(qid, {})
            flags = []
            if d.get("has_correct"): flags.append("correct_val")
            if d.get("has_poison"): flags.append("poison_val")
            status = "CORRECT" if d.get("correct") else "WRONG"
            log(f"  {key:<18} [{status:>7}] conf={d.get('confidence', '?')} {' | '.join(flags)}")

    # Confound check
    log_sep("CONFOUND ELIMINATION CHECK")
    log("Do converged v2 answers mention poison values in Q06-Q08?")
    answers = output["comparison_10q"]["G2_converged"]["answers"].get("answers", [])
    lookup = {a["question_id"]: a for a in answers}
    for qid, claim_id in [("Q06", "claim_03"), ("Q07", "claim_06"), ("Q08", "claim_11")]:
        a = lookup.get(qid, {})
        text = a.get("answer", "")
        claim = CLAIMS[claim_id]
        has_poison = has_value(text, claim.get("poison_patterns", []))
        status = "POISON PRESENT" if has_poison else "CLEAN"
        log(f"  G2_converged {qid}: {status}")

    # Token budget
    log_sep("TOKEN BUDGET")
    total_in = sum(c["input_tokens"] for c in call_log)
    total_out = sum(c["output_tokens"] for c in call_log)
    log(f"Total calls:         {len(call_log)}")
    log(f"Total input tokens:  {total_in:>8,}")
    log(f"Total output tokens: {total_out:>8,}")
    log(f"Total tokens:        {total_in + total_out:>8,}")

    log("\nPer-call breakdown:")
    for c in call_log:
        log(f"  {c['label']:<35} {c['input_tokens']:>6} in  {c['output_tokens']:>6} out  ({c['ms']}ms)")

    # Cost-per-correct (30Q)
    log_sep("COST EFFICIENCY (30Q)")
    for key in ["full_context", "converged_v2", "raw_flat_v2"]:
        s = scoring_30q[key]
        approach_data = output["scale_30q"][key]
        if "tokens" in approach_data:
            tokens = approach_data["tokens"]["input_tokens"] + approach_data["tokens"]["output_tokens"]
        else:
            tokens = (approach_data["tokens_select"]["input_tokens"] + approach_data["tokens_select"]["output_tokens"] +
                      approach_data["tokens_mask"]["input_tokens"] + approach_data["tokens_mask"]["output_tokens"])
        correct = max(s["total"], 1)
        log(f"  {key:<18} {tokens:>8} tokens / {s['total']} correct = {tokens / correct:.0f} tokens/correct answer")

    # Convergence summary
    log_sep("CONVERGENCE SUMMARY")
    log(f"Compression: {len(all_beliefs_v2)} → {len(converged_beliefs)} beliefs ({len(all_beliefs_v2) / max(len(converged_beliefs), 1):.1f}x)")
    log(f"Claim coverage: {audit['total_covered']}/{audit['total_claims']}")
    log(f"Contradictions found: {comp_stats.get('contradictions_found', '?')}")
    log(f"Contradictions resolved: {comp_stats.get('contradictions_resolved', '?')}")

    # Belief quality summary
    log_sep("BELIEF QUALITY SUMMARY")
    log(f"V2 beliefs generated: {len(all_beliefs_v2)}")
    log(f"Avg reasoning length: {sum(reasoning_lengths) / max(len(reasoning_lengths), 1):.0f} chars")
    log(f"cross_source_tension filled: {tension_filled}/{len(all_beliefs_v2)} ({100 * tension_filled / max(len(all_beliefs_v2), 1):.0f}%)")
    log(f"confidence_justification filled: {justification_filled}/{len(all_beliefs_v2)} ({100 * justification_filled / max(len(all_beliefs_v2), 1):.0f}%)")

    # Final analysis
    output["analysis"] = {
        "convergence": {
            "input_beliefs": len(all_beliefs_v2),
            "output_beliefs": len(converged_beliefs),
            "compression_ratio": round(len(all_beliefs_v2) / max(len(converged_beliefs), 1), 1),
            "claim_coverage": f"{audit['total_covered']}/{audit['total_claims']}",
            "all_claims_covered": audit["all_covered"],
            "contradictions_found": comp_stats.get("contradictions_found", 0),
            "contradictions_resolved": comp_stats.get("contradictions_resolved", 0),
        },
        "belief_quality": {
            "total_v2_beliefs": len(all_beliefs_v2),
            "avg_reasoning_length": round(sum(reasoning_lengths) / max(len(reasoning_lengths), 1)),
            "cross_source_tension_pct": round(100 * tension_filled / max(len(all_beliefs_v2), 1)),
            "confidence_justification_pct": round(100 * justification_filled / max(len(all_beliefs_v2), 1)),
        },
        "scoring_10q": {
            key: {
                "clean": scoring_10q[key]["clean"],
                "poison": scoring_10q[key]["poison"],
                "synthesis": scoring_10q[key]["synthesis"],
                "total": scoring_10q[key]["total"],
                "predicted": PREDICTIONS_10Q[key]["total"],
            }
            for key in scoring_10q
        },
        "scoring_30q": {
            key: {
                "total": scoring_30q[key]["total"],
                "total_possible": scoring_30q[key]["total_possible"],
                "predicted": PREDICTIONS_30Q[key]["total"],
                "clean": scoring_30q[key]["clean"],
                "poison": scoring_30q[key]["poison"],
                "adversarial": scoring_30q[key]["adversarial"],
            }
            for key in scoring_30q
        },
        "phase5_comparison": {
            "10q": {
                "converged": {"phase5": PHASE5_ACTUALS_10Q["G_converged"], "phase6": scoring_10q["G2_converged"]["total"]},
                "raw_flat": {"phase5": PHASE5_ACTUALS_10Q["I_raw_flat"], "phase6": scoring_10q["I2_raw_flat"]["total"]},
            },
            "30q": {
                "full_context": {"phase5": PHASE5_ACTUALS_30Q["full_context"], "phase6": scoring_30q["full_context"]["total"]},
                "converged": {"phase5": PHASE5_ACTUALS_30Q["converged"], "phase6": scoring_30q["converged_v2"]["total"]},
                "raw_flat": {"phase5": PHASE5_ACTUALS_30Q["raw_flat"], "phase6": scoring_30q["raw_flat_v2"]["total"]},
            },
        },
        "total_llm_calls": len(call_log),
        "total_tokens": total_in + total_out,
    }
    output["meta"]["total_llm_calls"] = len(call_log)
    output["call_log"] = call_log
    save_incremental(output)

    log_sep("PHASE 6 COMPLETE")
    log(f"Results: {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
