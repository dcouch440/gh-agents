"""
BOCA Phase 5: Belief Convergence — Compressed Knowledge from Distributed Sources

Loads Phase 4 results (70 beliefs, 6 node outputs, 12 claims) and tests whether
belief convergence improves answer quality by pre-resolving contradictions.

Pipeline:
  1. Convergence: 70 raw beliefs → ~18 converged beliefs (1 call)
  2. Quality audit: check 12/12 claims preserved (0 calls)
  3. 10-question comparison: G (converged), H (converged+resolutions), I (raw flat) (4 calls)
  4. 30-question scale test: full context, converged, raw flat (4 calls)
  5. Deterministic scoring (0 calls)
  6. Scoreboard + analysis

Total: 9 LLM calls, ~$0.15-0.25
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
# INFRASTRUCTURE (from phase4.py)
# ===========================================================================

MODEL = "claude-sonnet-4-5-20250929"
RESULTS_DIR = Path(__file__).resolve().parent
LOG_FILE = RESULTS_DIR / "phase5.log"
OUTPUT_PATH = RESULTS_DIR / "phase5_results.json"
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

all_beliefs = phase4["belief_store"]
node_outputs = phase4["node_outputs"]
ground_truth = phase4["ground_truth"]

log(f"Loaded {len(all_beliefs)} beliefs from {len(node_outputs)} nodes")
log(f"Ground truth: {len(ground_truth)} claims ({sum(1 for v in ground_truth.values() if v['poisoned'])} poisoned)")


# ===========================================================================
# CLAIMS (with relaxed patterns from rescore.py)
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
# QUESTIONS
# ===========================================================================

# Phase 4's 10 original questions
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

# 20 new questions for 30-question scale test
NEW_QUESTIONS = [
    # 5 clean (cover claims 4, 7, 8, 10 not well tested in Phase 4)
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

    # 5 cross-cutting (combine 3+ claims)
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

    # 5 hypothetical (require reasoning from correct baseline)
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

    # 5 adversarial (directly probe poison vs correct values)
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
# SCHEMAS
# ===========================================================================

CONVERGENCE_SCHEMA = {
    "type": "object",
    "properties": {
        "converged_beliefs": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id": {"type": "string", "description": "Converged belief ID (cb01, cb02, ...)"},
                    "topic": {"type": "string", "description": "Semantic topic tag"},
                    "content": {"type": "string", "description": "Converged belief content with EXACT numbers. If a contradiction was resolved, include the resolution."},
                    "consensus_strength": {"type": "string", "enum": ["unanimous", "strong", "majority", "split", "unique"]},
                    "sources": {"type": "array", "items": {"type": "string"}, "description": "Node names that contributed"},
                    "source_belief_ids": {"type": "array", "items": {"type": "string"}, "description": "Original belief IDs merged into this"},
                    "contradiction_resolved": {"type": "boolean"},
                    "resolution_detail": {"type": "string", "description": "If contradiction_resolved, explain the resolution"},
                },
                "required": ["id", "topic", "content", "consensus_strength", "sources", "source_belief_ids", "contradiction_resolved"],
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
                "contradictions_resolved": {"type": "integer"},
                "redundancies_removed": {"type": "integer"},
                "unique_insights_preserved": {"type": "integer"},
            },
            "required": ["input_beliefs", "output_beliefs", "contradictions_resolved",
                         "redundancies_removed", "unique_insights_preserved"],
        },
    },
    "required": ["converged_beliefs", "pruned_beliefs", "compression_stats"],
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

FLAT_SELECT_SCHEMA = {
    "type": "object",
    "properties": {
        "selected_belief_ids": {"type": "array", "items": {"type": "string"}},
        "pruned_belief_ids": {"type": "array", "items": {"type": "string"}},
        "selection_reasoning": {"type": "string"},
    },
    "required": ["selected_belief_ids", "pruned_belief_ids", "selection_reasoning"],
}


# ===========================================================================
# PRE-REGISTERED PREDICTIONS
# ===========================================================================

PREDICTIONS_10Q = {
    "G_converged":          {"clean": 5, "poison": 3, "synthesis": 2, "total": 10},
    "H_converged_resolved": {"clean": 5, "poison": 3, "synthesis": 2, "total": 10},
    "I_raw_flat":           {"clean": 4, "poison": 2, "synthesis": 1, "total": 7},
}

PREDICTIONS_30Q = {
    "full_context": {"total": 22},
    "converged":    {"total": 27},
    "raw_flat":     {"total": 20},
}


# ===========================================================================
# HELPER FUNCTIONS
# ===========================================================================

def format_beliefs_for_convergence(beliefs: list[dict]) -> str:
    """Format all beliefs for the convergence gatekeeper."""
    lines = []
    for b in beliefs:
        lines.append(
            f"[{b['id']}] Node {b['source_node']}: {b['source_node_name']} "
            f"(confidence={b['confidence']}, tone={b['emotional_tone']}, tag={b['semantic_tag']})\n"
            f"  {b['content']}"
        )
    return "\n\n".join(lines)


def format_converged_beliefs_for_mask(converged: list[dict], include_resolutions: bool = False) -> str:
    """Format converged beliefs for the mask agent."""
    lines = []
    for cb in converged:
        header = f"[{cb['id']}] {cb['topic']} (consensus={cb['consensus_strength']}, sources={', '.join(cb['sources'])})"
        body = f"  {cb['content']}"
        if include_resolutions and cb.get("contradiction_resolved") and cb.get("resolution_detail"):
            body += f"\n  [RESOLUTION: {cb['resolution_detail']}]"
        lines.append(f"{header}\n{body}")
    return "\n\n".join(lines)


def format_raw_beliefs_for_mask(beliefs: list[dict], selected_ids: list[str] | None = None) -> str:
    """Format raw beliefs for mask (same as Phase 4)."""
    lines = []
    for b in beliefs:
        if selected_ids and b["id"] not in selected_ids:
            continue
        lines.append(
            f"[{b['id']}] ({b['source_node_name']}, confidence={b['confidence']}, tone={b['emotional_tone']})\n"
            f"  {b['content']}"
        )
    return "\n\n".join(lines)


def format_questions_block(questions: list[dict]) -> str:
    return "\n".join(f"{q['id']}: {q['text']}" for q in questions)


def format_node_outputs(outputs: list[dict]) -> str:
    return "\n\n---\n\n".join(
        f"## Node {n['node_id']}: {n['node_name']}\n\n{n['output']}" for n in outputs
    )


# ===========================================================================
# SCORING (improved from rescore.py)
# ===========================================================================

def has_value(text: str, patterns: list[str]) -> bool:
    return any(re.search(p, text, re.IGNORECASE) for p in patterns)


def answer_recommends_correct(text: str, correct_kw: str, poison_kw: str) -> bool:
    """When both values present, check if the answer identifies the correct one as authoritative."""
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
    """Score a single answer against ground truth."""
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
    """Score a synthesis/cross-cutting question."""
    results = [score_answer(answer_text, cid) for cid in claim_ids]
    all_correct = all(r["correct"] for r in results)
    return {"correct": all_correct, "component_results": results}


def score_question(answer_text: str, question: dict) -> dict:
    """Score any question type."""
    if question["category"] in ("synthesis", "cross_cutting"):
        claim_ids = question["tests_claims"]
        r = score_synthesis(answer_text, claim_ids)
        return {**r, "question_id": question["id"], "category": question["category"]}
    else:
        claim_id = question["tests_claim"]
        r = score_answer(answer_text, claim_id)
        return {**r, "question_id": question["id"], "category": question["category"]}


def score_all_questions(answers: list[dict], questions: list[dict]) -> dict:
    """Score all answers against questions. Returns summary + details."""
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


def audit_claim_coverage(converged_beliefs: list[dict]) -> dict:
    """Check that converged beliefs preserve all 12 claims."""
    all_text = " ".join(cb["content"] for cb in converged_beliefs)
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
            "phase": 5,
            "description": "Belief Convergence — Compressed Knowledge from Distributed Sources",
            "model": MODEL,
            "timestamp": datetime.now().isoformat(),
            "total_llm_calls": 0,
        },
        "predictions": {
            "10q": PREDICTIONS_10Q,
            "30q": PREDICTIONS_30Q,
        },
        "convergence": {},
        "audit": {},
        "comparison_10q": {},
        "scale_30q": {},
        "scoring": {},
        "analysis": {},
    }

    log_sep("BOCA PHASE 5: BELIEF CONVERGENCE")
    log("Compressed Knowledge from Distributed Sources")
    log(f"Model: {MODEL}")
    log(f"Input: {len(all_beliefs)} beliefs from Phase 4")
    log(f"Budget: 9 LLM calls")
    log("")

    # ===================================================================
    # STEP 1: CONVERGENCE (1 LLM call)
    # ===================================================================
    log_sep("STEP 1: BELIEF CONVERGENCE (1 call)")

    beliefs_text = format_beliefs_for_convergence(all_beliefs)

    convergence_system = (
        "You are the Convergence Gatekeeper in a belief-oriented conversation architecture.\n\n"
        "You receive raw beliefs extracted from 6 different professional perspectives on the same "
        "technical specification. Your task is to CONVERGE these into a minimal, authoritative set "
        "of beliefs by:\n\n"
        "1. CLUSTERING beliefs by semantic topic (e.g., retry_max_attempts, audit_retention)\n"
        "2. MERGING concordant beliefs into single converged beliefs with cumulative citations\n"
        "3. RESOLVING contradictions using authority hierarchy:\n"
        "   - Regulatory/compliance claims: Security Reviewer + regulatory citations > all others\n"
        "   - Product requirements: PM (specification owner) + Architect > QA (tester)\n"
        "   - When 5/6 sources agree and 1 disagrees, the 5-source consensus wins\n"
        "   - A tester recommending different thresholds does NOT override the specification\n"
        "4. PRESERVING unique insights that only one source provides (gaps, risks, tradeoffs)\n"
        "5. PRUNING pure redundancies and non-actionable observations\n\n"
        "CRITICAL RULES:\n"
        "- Every converged belief MUST include EXACT NUMBERS. Never paraphrase '500ms' as 'sub-second'.\n"
        "- When resolving a contradiction, the converged belief states the CORRECT value and notes "
        "the resolution. The WRONG value is NOT presented as a valid alternative.\n"
        "- consensus_strength: 'unanimous' (all sources agree), 'strong' (5-6 agree), "
        "'majority' (3-4 agree), 'split' (equal disagreement), 'unique' (single source)\n"
        "- Target: 15-20 converged beliefs from the input. If you produce fewer than 12 or more "
        "than 25, reconsider your clustering.\n"
        "- Every measurable technical parameter from the input beliefs MUST appear in at least "
        "one converged belief. Do not drop information."
    )

    convergence_user = (
        f"RAW BELIEF STORE ({len(all_beliefs)} beliefs from 6 nodes):\n\n"
        f"{beliefs_text}\n\n"
        f"Converge these beliefs into a minimal, authoritative set. "
        f"Resolve all contradictions. Preserve all technical parameters."
    )

    convergence_data, convergence_stats = call_json(
        convergence_system, convergence_user, "CONVERGENCE",
        CONVERGENCE_SCHEMA, max_tokens=8192
    )

    converged_beliefs = convergence_data.get("converged_beliefs", [])
    pruned = convergence_data.get("pruned_beliefs", [])
    comp_stats = convergence_data.get("compression_stats", {})

    log(f"\nConvergence results:")
    log(f"  Input beliefs:  {comp_stats.get('input_beliefs', len(all_beliefs))}")
    log(f"  Output beliefs: {len(converged_beliefs)}")
    log(f"  Compression:    {len(all_beliefs) / max(len(converged_beliefs), 1):.1f}x")
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
    # STEP 2: QUALITY AUDIT (0 LLM calls)
    # ===================================================================
    log_sep("STEP 2: CONVERGENCE QUALITY AUDIT (0 calls)")

    audit = audit_claim_coverage(converged_beliefs)
    log(f"Claim coverage: {audit['total_covered']}/{audit['total_claims']}")
    for claim_id, info in audit["claims"].items():
        status = "COVERED" if info["covered"] else "MISSING"
        log(f"  {claim_id} ({info['description']}): {status}")

    if not audit["all_covered"]:
        log("[WARN] Not all claims covered! Convergence lost information.", "WARN")

    # Check contradiction resolutions
    resolved = [cb for cb in converged_beliefs if cb.get("contradiction_resolved")]
    log(f"\nContradiction resolutions: {len(resolved)}")
    for cb in resolved:
        log(f"  {cb['id']} ({cb['topic']}): {cb.get('resolution_detail', 'no detail')[:100]}...")

    output["audit"] = audit
    save_incremental(output)

    # ===================================================================
    # STEP 3: 10-QUESTION COMPARISON (4 LLM calls)
    # ===================================================================
    log_sep("STEP 3: 10-QUESTION COMPARISON (4 calls)")

    questions_10_block = format_questions_block(PHASE4_QUESTIONS)
    answer_system = (
        "Answer each question precisely. Include SPECIFIC NUMBERS in every answer. "
        "For each answer, rate your confidence 1-5 and cite which sources informed your answer."
    )

    # --- G: Converged beliefs → mask ---
    log("\n--- APPROACH G: Converged Beliefs ---")
    g_mask_system = (
        "You are a Mask agent. You have NEVER seen the original specification. "
        "You have ONLY converged beliefs — pre-resolved, authoritative facts. "
        "Answer each question using ONLY these beliefs. Include SPECIFIC NUMBERS. "
        "Do not speculate beyond what the beliefs state."
    )
    g_beliefs_text = format_converged_beliefs_for_mask(converged_beliefs, include_resolutions=False)
    g_user = (
        f"CONVERGED BELIEFS:\n\n{g_beliefs_text}\n\n"
        f"QUESTIONS:\n{questions_10_block}"
    )
    g_data, g_stats = call_json(g_mask_system, g_user, "G:CONVERGED_10Q", ANSWER_SCHEMA, max_tokens=4096)
    output["comparison_10q"]["G_converged"] = {"answers": g_data, "tokens": g_stats}
    save_incremental(output)

    # --- H: Converged beliefs + resolution context → mask ---
    log("\n--- APPROACH H: Converged + Resolutions ---")
    h_beliefs_text = format_converged_beliefs_for_mask(converged_beliefs, include_resolutions=True)
    h_user = (
        f"CONVERGED BELIEFS (with resolution details):\n\n{h_beliefs_text}\n\n"
        f"QUESTIONS:\n{questions_10_block}"
    )
    h_data, h_stats = call_json(g_mask_system, h_user, "H:RESOLVED_10Q", ANSWER_SCHEMA, max_tokens=4096)
    output["comparison_10q"]["H_converged_resolved"] = {"answers": h_data, "tokens": h_stats}
    save_incremental(output)

    # --- I: Raw flat (gatekeeper selects from 70 → mask answers) ---
    log("\n--- APPROACH I: Raw Flat Baseline ---")
    raw_beliefs_text = format_raw_beliefs_for_mask(all_beliefs)

    i_select_system = (
        "You are the Gatekeeper. You have a store of beliefs extracted from 6 different "
        "professional reviews of a technical specification. Select which beliefs are relevant "
        "to answer the following questions. Return the IDs of selected beliefs and IDs of pruned beliefs."
    )
    i_select_user = (
        f"BELIEF STORE:\n\n{raw_beliefs_text}\n\n"
        f"QUESTIONS TO ANSWER:\n{questions_10_block}\n\n"
        f"Select the beliefs needed to answer ALL these questions accurately."
    )
    i_selection, i_sel_stats = call_json(
        i_select_system, i_select_user, "I:RAW_SELECT_10Q", FLAT_SELECT_SCHEMA
    )
    selected_ids_i = i_selection.get("selected_belief_ids", [b["id"] for b in all_beliefs])

    i_mask_system = (
        "You are a Mask agent. You have NEVER seen the original specification. "
        "You have ONLY the beliefs provided below. Answer each question using ONLY these beliefs. "
        "Include SPECIFIC NUMBERS from the beliefs. If beliefs don't cover a topic, say so."
    )
    i_mask_user = (
        f"YOUR BELIEFS:\n\n{format_raw_beliefs_for_mask(all_beliefs, selected_ids_i)}\n\n"
        f"QUESTIONS:\n{questions_10_block}"
    )
    i_data, i_mask_stats = call_json(i_mask_system, i_mask_user, "I:RAW_MASK_10Q", ANSWER_SCHEMA, max_tokens=4096)
    output["comparison_10q"]["I_raw_flat"] = {
        "selection": i_selection, "answers": i_data,
        "tokens_select": i_sel_stats, "tokens_mask": i_mask_stats,
        "selected_count": len(selected_ids_i),
    }
    save_incremental(output)

    # ===================================================================
    # STEP 4: 30-QUESTION SCALE TEST (4 LLM calls)
    # ===================================================================
    log_sep("STEP 4: 30-QUESTION SCALE TEST (4 calls)")

    questions_30_block = format_questions_block(ALL_QUESTIONS)

    # --- Full Context: all 6 node outputs → 30 answers ---
    log("\n--- SCALE: Full Context (all 6 node outputs) ---")
    all_outputs_text = format_node_outputs(node_outputs)
    fc_user = (
        f"You have outputs from 6 different professionals who reviewed the same specification:\n\n"
        f"{all_outputs_text}\n\n"
        f"Answer these questions:\n{questions_30_block}"
    )
    fc_data, fc_stats = call_json(answer_system, fc_user, "SCALE:FULL_CONTEXT_30Q", ANSWER_SCHEMA, max_tokens=16384)
    output["scale_30q"]["full_context"] = {"answers": fc_data, "tokens": fc_stats}
    save_incremental(output)

    # --- Converged: converged beliefs → 30 answers ---
    log("\n--- SCALE: Converged Beliefs ---")
    conv_user = (
        f"CONVERGED BELIEFS (with resolution details):\n\n{h_beliefs_text}\n\n"
        f"QUESTIONS:\n{questions_30_block}"
    )
    conv_data, conv_stats = call_json(g_mask_system, conv_user, "SCALE:CONVERGED_30Q", ANSWER_SCHEMA, max_tokens=16384)
    output["scale_30q"]["converged"] = {"answers": conv_data, "tokens": conv_stats}
    save_incremental(output)

    # --- Raw Flat: selection from 70 + mask → 30 answers ---
    log("\n--- SCALE: Raw Flat ---")
    rf_select_user = (
        f"BELIEF STORE:\n\n{raw_beliefs_text}\n\n"
        f"QUESTIONS TO ANSWER:\n{questions_30_block}\n\n"
        f"Select the beliefs needed to answer ALL these questions accurately."
    )
    rf_selection, rf_sel_stats = call_json(
        i_select_system, rf_select_user, "SCALE:RAW_SELECT_30Q", FLAT_SELECT_SCHEMA
    )
    selected_ids_rf = rf_selection.get("selected_belief_ids", [b["id"] for b in all_beliefs])

    rf_mask_user = (
        f"YOUR BELIEFS:\n\n{format_raw_beliefs_for_mask(all_beliefs, selected_ids_rf)}\n\n"
        f"QUESTIONS:\n{questions_30_block}"
    )
    rf_data, rf_mask_stats = call_json(
        i_mask_system, rf_mask_user, "SCALE:RAW_MASK_30Q", ANSWER_SCHEMA, max_tokens=16384
    )
    output["scale_30q"]["raw_flat"] = {
        "selection": rf_selection, "answers": rf_data,
        "tokens_select": rf_sel_stats, "tokens_mask": rf_mask_stats,
        "selected_count": len(selected_ids_rf),
    }
    save_incremental(output)

    # ===================================================================
    # STEP 5: SCORING (0 LLM calls)
    # ===================================================================
    log_sep("STEP 5: SCORING")

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
    # STEP 6: SCOREBOARD + ANALYSIS
    # ===================================================================
    log_sep("SCOREBOARD — 10-QUESTION COMPARISON")
    log("")
    log(f"{'Approach':<25} {'Clean':>6} {'Poison':>7} {'Synth':>6} {'TOTAL':>6} {'Predicted':>10}")
    log("-" * 68)

    for key in ["G_converged", "H_converged_resolved", "I_raw_flat"]:
        s = scoring_10q[key]
        p = PREDICTIONS_10Q[key]
        log(f"{key:<25} {s['clean']:>3}/{s['clean_total']}  {s['poison']:>3}/{s['poison_total']}   "
            f"{s['synthesis']:>3}/{s['synthesis_total']}  {s['total']:>3}/{s['total_possible']}  "
            f"{p['total']:>6}/10")

    log_sep("SCOREBOARD — 30-QUESTION SCALE TEST")
    log("")
    log(f"{'Approach':<18} {'Clean':>6} {'Poison':>7} {'Synth':>6} {'Cross':>6} {'Hypo':>6} {'Adv':>6} {'TOTAL':>7} {'Pred':>6}")
    log("-" * 80)

    for key in ["full_context", "converged", "raw_flat"]:
        s = scoring_30q[key]
        p = PREDICTIONS_30Q[key]
        log(f"{key:<18} {s['clean']:>3}/{s['clean_total']}  {s['poison']:>3}/{s['poison_total']}   "
            f"{s['synthesis']:>3}/{s['synthesis_total']}  {s['cross_cutting']:>3}/{s['cross_cutting_total']}  "
            f"{s['hypothetical']:>3}/{s['hypothetical_total']}  {s['adversarial']:>3}/{s['adversarial_total']}  "
            f"{s['total']:>3}/{s['total_possible']}  {p['total']:>4}")

    # Predictions vs actuals
    log("")
    log_sep("PREDICTIONS vs ACTUALS")

    log("\n10-question:")
    for key in ["G_converged", "H_converged_resolved", "I_raw_flat"]:
        actual = scoring_10q[key]["total"]
        predicted = PREDICTIONS_10Q[key]["total"]
        delta = actual - predicted
        sign = "+" if delta > 0 else ""
        log(f"  {key:<25} predicted={predicted:>2}  actual={actual:>2}  delta={sign}{delta}")

    log("\n30-question:")
    for key in ["full_context", "converged", "raw_flat"]:
        actual = scoring_30q[key]["total"]
        predicted = PREDICTIONS_30Q[key]["total"]
        delta = actual - predicted
        sign = "+" if delta > 0 else ""
        log(f"  {key:<18} predicted={predicted:>2}  actual={actual:>2}  delta={sign}{delta}")

    # Poisoned question detail
    log_sep("POISONED QUESTION DETAIL")
    for qid in ["Q06", "Q07", "Q08"]:
        log(f"\n{qid}:")
        for key in ["G_converged", "H_converged_resolved", "I_raw_flat"]:
            d = scoring_10q[key]["details"].get(qid, {})
            flags = []
            if d.get("has_correct"): flags.append("correct_val")
            if d.get("has_poison"): flags.append("poison_val")
            if d.get("detected_contradiction"): flags.append("CONTRADICTION")
            if d.get("recommends_correct"): flags.append("RESOLVED")
            status = "CORRECT" if d.get("correct") else "WRONG"
            log(f"  {key:<25} [{status:>7}] conf={d.get('confidence', '?')} {' | '.join(flags)}")

    # Adversarial question detail (30Q only)
    log_sep("ADVERSARIAL QUESTION DETAIL (30Q)")
    for qid in ["Q26", "Q27", "Q28", "Q29", "Q30"]:
        log(f"\n{qid}:")
        for key in ["full_context", "converged", "raw_flat"]:
            d = scoring_30q[key]["details"].get(qid, {})
            flags = []
            if d.get("has_correct"): flags.append("correct_val")
            if d.get("has_poison"): flags.append("poison_val")
            status = "CORRECT" if d.get("correct") else "WRONG"
            log(f"  {key:<18} [{status:>7}] conf={d.get('confidence', '?')} {' | '.join(flags)}")

    # Confound check: do G/H answers contain poison values?
    log_sep("CONFOUND ELIMINATION CHECK")
    log("Do converged approaches mention poison values in Q06-Q08?")
    for key in ["G_converged", "H_converged_resolved"]:
        answers = output["comparison_10q"][key]["answers"].get("answers", [])
        lookup = {a["question_id"]: a for a in answers}
        for qid, claim_id in [("Q06", "claim_03"), ("Q07", "claim_06"), ("Q08", "claim_11")]:
            a = lookup.get(qid, {})
            text = a.get("answer", "")
            claim = CLAIMS[claim_id]
            has_poison = has_value(text, claim.get("poison_patterns", []))
            status = "POISON PRESENT" if has_poison else "CLEAN"
            log(f"  {key:<25} {qid}: {status}")

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
        log(f"  {c['label']:<30} {c['input_tokens']:>6} in  {c['output_tokens']:>6} out  ({c['ms']}ms)")

    # Cost-per-correct-answer (30Q)
    log_sep("COST EFFICIENCY (30Q)")
    for key in ["full_context", "converged", "raw_flat"]:
        s = scoring_30q[key]
        # Sum tokens for this approach
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
    log(f"Compression: {len(all_beliefs)} → {len(converged_beliefs)} beliefs ({len(all_beliefs) / max(len(converged_beliefs), 1):.1f}x)")
    log(f"Claim coverage: {audit['total_covered']}/{audit['total_claims']}")
    log(f"Contradictions resolved: {comp_stats.get('contradictions_resolved', '?')}")

    # Final analysis
    output["analysis"] = {
        "convergence": {
            "input_beliefs": len(all_beliefs),
            "output_beliefs": len(converged_beliefs),
            "compression_ratio": round(len(all_beliefs) / max(len(converged_beliefs), 1), 1),
            "claim_coverage": f"{audit['total_covered']}/{audit['total_claims']}",
            "all_claims_covered": audit["all_covered"],
            "contradictions_resolved": comp_stats.get("contradictions_resolved", 0),
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
        "total_llm_calls": len(call_log),
        "total_tokens": total_in + total_out,
    }
    output["meta"]["total_llm_calls"] = len(call_log)
    output["call_log"] = call_log
    save_incremental(output)

    log_sep("PHASE 5 COMPLETE")
    log(f"Results: {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
