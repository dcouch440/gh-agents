"""
Belief-Oriented Conversation Architecture — Phase 2

Proves two things Phase 1 left open:
1. Do beliefs beat a NAIVE SUMMARY? (the real comparison)
2. Does belief REVISION find things that single-pass misses?

Test: two connected files (resume.rs + single.rs), one hard cross-file
question, four approaches head-to-head:
  A) Full context (both files raw)
  B) Naive summary (summarize both files, then answer from summary)
  C) Belief pipeline single-pass (Phase 1 approach)
  D) Belief pipeline with revision (the new thing)
"""

import json
import sys
import time
from datetime import datetime
from pathlib import Path

import anthropic
from dotenv import load_dotenv

load_dotenv(Path(__file__).resolve().parent.parent / ".env")

client = anthropic.Anthropic()
MODEL = "claude-sonnet-4-5-20250929"

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------

LOG_FILE = Path(__file__).resolve().parent / "phase2.log"

def log(msg: str, level: str = "INFO"):
    """Write to both stdout and log file with timestamp."""
    ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]
    line = f"[{ts}] [{level}] {msg}"
    print(line, flush=True)
    with open(LOG_FILE, "a") as f:
        f.write(line + "\n")

def log_separator(title: str):
    sep = "=" * 70
    log(sep)
    log(title)
    log(sep)

# Clear log on start
LOG_FILE.write_text(f"# Phase 2 run started at {datetime.now().isoformat()}\n\n")

# ---------------------------------------------------------------------------
# Structured LLM calls via tool_use for JSON, plain text for prose
# ---------------------------------------------------------------------------

call_log: list[dict] = []


def call_text(system: str, user: str, label: str, max_tokens: int = 2048) -> dict:
    """LLM call expecting prose/text response."""
    log(f"CALL START: {label}", "API")
    log(f"  max_tokens={max_tokens}", "API")
    t0 = time.monotonic()
    resp = client.messages.create(
        model=MODEL,
        max_tokens=max_tokens,
        system=system,
        messages=[{"role": "user", "content": user}],
    )
    elapsed = int((time.monotonic() - t0) * 1000)
    text = resp.content[0].text
    usage = resp.usage
    result = {
        "label": label,
        "text": text,
        "input_tokens": usage.input_tokens,
        "output_tokens": usage.output_tokens,
        "ms": elapsed,
    }
    call_log.append(result)
    log(f"CALL DONE: {label}  in={usage.input_tokens:,}  out={usage.output_tokens:,}  {elapsed}ms", "API")
    log(f"  Preview: {text[:150].replace(chr(10), ' ')}...", "API")
    return result


def call_json(system: str, user: str, label: str, schema: dict, max_tokens: int = 4096) -> tuple[dict, dict]:
    """LLM call that returns structured JSON via tool_use. Returns (parsed_data, stats)."""
    tool_name = "structured_output"
    tool = {
        "name": tool_name,
        "description": "Return your structured analysis.",
        "input_schema": schema,
    }
    log(f"CALL START: {label} (structured)", "API")
    log(f"  schema keys: {list(schema.get('properties', {}).keys())}", "API")
    t0 = time.monotonic()
    resp = client.messages.create(
        model=MODEL,
        max_tokens=max_tokens,
        system=system + "\n\nYou MUST use the structured_output tool to return your response.",
        messages=[{"role": "user", "content": user}],
        tools=[tool],
        tool_choice={"type": "tool", "name": tool_name},
    )
    elapsed = int((time.monotonic() - t0) * 1000)
    usage = resp.usage

    # Extract tool use input
    data = None
    for block in resp.content:
        if block.type == "tool_use" and block.name == tool_name:
            data = block.input
            break

    if data is None:
        log(f"WARN: No tool_use block in response for {label}, falling back to text parse", "API")
        text = resp.content[0].text if resp.content else ""
        data = _extract_json_fallback(text)

    stats = {
        "label": label,
        "text": json.dumps(data, indent=2),
        "input_tokens": usage.input_tokens,
        "output_tokens": usage.output_tokens,
        "ms": elapsed,
    }
    call_log.append(stats)
    log(f"CALL DONE: {label}  in={usage.input_tokens:,}  out={usage.output_tokens:,}  {elapsed}ms", "API")
    return data, stats


def _extract_json_fallback(text: str) -> dict:
    """Last-resort JSON extraction from text."""
    if "```json" in text:
        text = text.split("```json")[1].split("```")[0]
    elif "```" in text:
        text = text.split("```")[1].split("```")[0]
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        pass
    start = text.find("{")
    if start != -1:
        depth = 0
        for i in range(start, len(text)):
            if text[i] == "{":
                depth += 1
            elif text[i] == "}":
                depth -= 1
                if depth == 0:
                    try:
                        return json.loads(text[start : i + 1])
                    except json.JSONDecodeError:
                        break
    log(f"FATAL: Could not parse JSON from: {text[:200]}", "ERROR")
    sys.exit(1)


# ---------------------------------------------------------------------------
# JSON Schemas for structured calls
# ---------------------------------------------------------------------------

BELIEFS_SCHEMA = {
    "type": "object",
    "properties": {
        "beliefs": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "semantic_tag": {"type": "string"},
                    "confidence": {"type": "string", "enum": ["high", "medium", "low"]},
                    "emotional_tone": {"type": "string"},
                    "content": {"type": "string"},
                    "relevant_files": {"type": "array", "items": {"type": "string"}},
                    "cross_file_tension": {"type": ["string", "null"]},
                },
                "required": ["semantic_tag", "confidence", "emotional_tone", "content"],
            },
        }
    },
    "required": ["beliefs"],
}

ASSIGNMENT_SCHEMA = {
    "type": "object",
    "properties": {
        "selected_indices": {
            "type": "array",
            "items": {"type": "integer"},
        },
        "reasoning": {"type": "string"},
    },
    "required": ["selected_indices", "reasoning"],
}

EVALUATION_SCHEMA = {
    "type": "object",
    "properties": {
        "assessment": {"type": "string", "enum": ["accurate", "partial", "wrong"]},
        "gaps": {
            "type": "array",
            "items": {"type": "string"},
        },
        "belief_revisions": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["revise", "add", "kill"]},
                    "target_tag": {"type": ["string", "null"]},
                    "new_belief": {
                        "type": ["object", "null"],
                        "properties": {
                            "semantic_tag": {"type": "string"},
                            "confidence": {"type": "string"},
                            "emotional_tone": {"type": "string"},
                            "content": {"type": "string"},
                            "cross_file_tension": {"type": ["string", "null"]},
                        },
                    },
                },
                "required": ["action"],
            },
        },
    },
    "required": ["assessment", "gaps", "belief_revisions"],
}

# ---------------------------------------------------------------------------
# Source material
# ---------------------------------------------------------------------------

ROOT = Path(__file__).resolve().parent.parent
RESUME_RS = (ROOT / "src/server/hub/dag/resume.rs").read_text()
SINGLE_RS = (ROOT / "src/server/hub/dag/single.rs").read_text()
COMBINED = f"// === resume.rs ===\n{RESUME_RS}\n\n// === single.rs ===\n{SINGLE_RS}"

QUESTION = """\
When resume_workflow_via_engine calls execute_single_step for a resumed workflow, \
the pre-completed steps have synthetic envelopes with zeroed metadata (execution_time_ms: 0, \
no tokens, no cost, no model). Trace exactly what happens when a DOWNSTREAM step tries to \
resolve port inputs from one of these synthetic envelopes. Will it work correctly? \
What subtle data quality issues could emerge in the execution records, cost tracking, \
or observability? Identify at least one issue that would NOT be obvious from reading \
either file in isolation."""

log_separator("BELIEF-ORIENTED CONVERSATION ARCHITECTURE — Phase 2")
log(f"Files: resume.rs ({len(RESUME_RS.splitlines())} lines) + single.rs ({len(SINGLE_RS.splitlines())} lines)")
log(f"Combined: {len(COMBINED.splitlines())} lines")
log(f"Question: {QUESTION[:80]}...")

# ===================================================================
# APPROACH A: FULL CONTEXT
# ===================================================================

log_separator("APPROACH A: FULL CONTEXT")
log("Sending both files raw + question to model...")

a_result = call_text(
    "You are a code analyst. Answer the question precisely and thoroughly.",
    f"```rust\n{COMBINED}\n```\n\nQUESTION: {QUESTION}",
    "A:FULL_CONTEXT",
)
log("Approach A complete.")

# ===================================================================
# APPROACH B: NAIVE SUMMARY
# ===================================================================

log_separator("APPROACH B: NAIVE SUMMARY")
log("Step 1/2: Generating summary of both files...")

summary_result = call_text(
    "Summarize this code thoroughly. Cover all functions, data flow, error handling, and key design decisions. This summary will be used to answer questions later — include every detail that might matter.",
    f"```rust\n{COMBINED}\n```",
    "B:SUMMARIZE",
    max_tokens=3000,
)

log("Step 2/2: Answering question from summary only...")

b_result = call_text(
    "You are a code analyst. Answer the question using ONLY the summary provided. Do not assume details not in the summary.",
    f"CODE SUMMARY:\n{summary_result['text']}\n\nQUESTION: {QUESTION}",
    "B:ANSWER_FROM_SUMMARY",
)
log("Approach B complete.")

# ===================================================================
# APPROACH C: BELIEF PIPELINE (single pass)
# ===================================================================

log_separator("APPROACH C: BELIEF PIPELINE (single pass)")

GATEKEEPER_SYSTEM = """\
You are the Gatekeeper in a belief-oriented conversation architecture.

Decompose the source code into BELIEF SLICES. Each slice is a hypothesis
about what matters, tagged with semantic_tag, confidence, emotional_tone,
content (your understanding, NOT a code quote — dense enough that someone
who has never seen the source can reason about the system), relevant_files,
and cross_file_tension (tension/coupling between files, or null).

Produce 8-12 belief slices covering both files and their interaction."""

log("Step 1/3: Gatekeeper decomposing source into belief slices...")

c_beliefs_data, c_beliefs_stats = call_json(
    GATEKEEPER_SYSTEM,
    f"```rust\n{COMBINED}\n```",
    "C:GATEKEEPER_DECOMPOSE",
    BELIEFS_SCHEMA,
)
c_beliefs = c_beliefs_data["beliefs"]
log(f"Gatekeeper produced {len(c_beliefs)} beliefs:")
for i, b in enumerate(c_beliefs):
    log(f"  [{i}] ({b['confidence']}) ({b['emotional_tone']}) {b['semantic_tag']}")

log("Step 2/3: Gatekeeper assigning beliefs to question...")

c_assign_data, c_assign_stats = call_json(
    """\
You are the Gatekeeper designing a conversation. Given beliefs and a question,
select which beliefs are relevant. You know which paths will come up dry.
Select only what pushes toward truth.""",
    f"Beliefs:\n{json.dumps(c_beliefs, indent=2)}\n\nQuestion: {QUESTION}",
    "C:GATEKEEPER_ASSIGN",
    ASSIGNMENT_SCHEMA,
)
c_selected_indices = c_assign_data["selected_indices"]
c_selected = [c_beliefs[i] for i in c_selected_indices if i < len(c_beliefs)]
log(f"Selected {len(c_selected)}/{len(c_beliefs)} beliefs (indices: {c_selected_indices})")
log(f"Reasoning: {c_assign_data['reasoning'][:150]}...")

log("Step 3/3: Mask answering from belief slice...")

belief_text = "\n\n".join(
    f"[{b['semantic_tag']}] (confidence: {b['confidence']}, tone: {b['emotional_tone']})"
    + (f"\nCross-file tension: {b['cross_file_tension']}" if b.get("cross_file_tension") else "")
    + f"\n{b['content']}"
    for b in c_selected
)

c_result = call_text(
    """\
You are a Mask — a focused analytical perspective. You have NOT seen the
original source code. You have ONLY the belief slices provided by the
Gatekeeper. Answer using ONLY these beliefs. If they don't cover something,
say so explicitly.""",
    f"BELIEF CONTEXT:\n{belief_text}\n\nQUESTION: {QUESTION}",
    "C:MASK_ANSWER",
)
log("Approach C complete.")

# ===================================================================
# APPROACH D: BELIEF PIPELINE WITH REVISION
# ===================================================================

log_separator("APPROACH D: BELIEF PIPELINE WITH REVISION")
log("Reusing gatekeeper decomposition and initial mask answer from C.")
log("Step 1/2: Gatekeeper evaluating mask's answer against source...")

d_eval_data, d_eval_stats = call_json(
    """\
You are the Gatekeeper evaluating a Mask's answer. You have the FULL SOURCE
CODE and the mask's answer (which was produced from beliefs alone).

Identify what the mask GOT WRONG, what it MISSED, and what beliefs need to
be REVISED, ADDED, or KILLED. Be specific about cross-file interactions
the mask could not have seen.""",
    f"ORIGINAL SOURCE:\n```rust\n{COMBINED}\n```\n\nMASK'S ANSWER:\n{c_result['text']}\n\nQUESTION: {QUESTION}\n\nEvaluate the mask's answer. What did it miss? What beliefs need revision?",
    "D:GATEKEEPER_EVALUATE",
    EVALUATION_SCHEMA,
)

log(f"Assessment: {d_eval_data['assessment']}")
log(f"Gaps found: {len(d_eval_data['gaps'])}")
for i, gap in enumerate(d_eval_data["gaps"]):
    log(f"  Gap {i+1}: {gap[:120]}...")
log(f"Belief revisions: {len(d_eval_data['belief_revisions'])}")
for rev in d_eval_data["belief_revisions"]:
    tag = rev.get("target_tag", "new")
    log(f"  {rev['action'].upper()}: {tag}")

# Apply revisions
d_beliefs = list(c_selected)
for rev in d_eval_data["belief_revisions"]:
    if rev["action"] == "kill":
        d_beliefs = [b for b in d_beliefs if b.get("semantic_tag") != rev.get("target_tag")]
    elif rev["action"] == "revise":
        d_beliefs = [b for b in d_beliefs if b.get("semantic_tag") != rev.get("target_tag")]
        if rev.get("new_belief"):
            d_beliefs.append(rev["new_belief"])
    elif rev["action"] == "add":
        if rev.get("new_belief"):
            d_beliefs.append(rev["new_belief"])

log(f"Revised belief set: {len(d_beliefs)} beliefs (was {len(c_selected)})")

log("Step 2/2: Mask answering from REVISED beliefs...")

revised_belief_text = "\n\n".join(
    f"[{b.get('semantic_tag', 'unknown')}] (confidence: {b.get('confidence', 'high')}, tone: {b.get('emotional_tone', 'analytical')})"
    + (f"\nCross-file tension: {b['cross_file_tension']}" if b.get("cross_file_tension") else "")
    + f"\n{b.get('content', '')}"
    for b in d_beliefs
)

d_result = call_text(
    """\
You are a Mask — a focused analytical perspective. You have NOT seen the
original source code. You have ONLY the REVISED belief slices provided by
the Gatekeeper after evaluating a previous attempt. These beliefs have been
refined — some added, some revised, some removed.

Answer using ONLY these beliefs. Be precise about what you know vs what
you're inferring.""",
    f"REVISED BELIEF CONTEXT:\n{revised_belief_text}\n\nQUESTION: {QUESTION}",
    "D:MASK_REVISED_ANSWER",
)
log("Approach D complete.")

# ===================================================================
# RESULTS
# ===================================================================

log_separator("RESULTS")

approaches = {
    "A:FULL_CONTEXT": [a_result],
    "B:NAIVE_SUMMARY": [summary_result, b_result],
    "C:BELIEF_SINGLE": [c_beliefs_stats, c_assign_stats, c_result],
    "D:BELIEF_REVISED": [c_beliefs_stats, c_assign_stats, c_result, d_eval_stats, d_result],
}

header = f"{'Approach':<25} {'Input Tok':>10} {'Output Tok':>11} {'Total Tok':>10} {'Calls':>6} {'Time':>8}"
log(header)
log("-" * 70)
for name, calls in approaches.items():
    total_in = sum(c["input_tokens"] for c in calls)
    total_out = sum(c["output_tokens"] for c in calls)
    total_ms = sum(c["ms"] for c in calls)
    log(f"{name:<25} {total_in:>10,} {total_out:>11,} {total_in + total_out:>10,} {len(calls):>6} {total_ms:>7,}ms")

log("")
log("ANSWER-PHASE INPUT TOKENS (what the answering model received):")
log(f"  A (full context):     {a_result['input_tokens']:>6,}")
log(f"  B (from summary):     {b_result['input_tokens']:>6,}")
log(f"  C (from beliefs):     {c_result['input_tokens']:>6,}")
log(f"  D (revised beliefs):  {d_result['input_tokens']:>6,}")

# Write full outputs
output = {
    "question": QUESTION,
    "answers": {
        "A_full_context": a_result["text"],
        "B_naive_summary": b_result["text"],
        "C_belief_single": c_result["text"],
        "D_belief_revised": d_result["text"],
    },
    "beliefs_initial": c_beliefs,
    "beliefs_selected": c_selected,
    "beliefs_revised": d_beliefs,
    "gatekeeper_evaluation": d_eval_data,
    "call_log": call_log,
}

output_path = Path(__file__).resolve().parent / "phase2_results.json"
with open(output_path, "w") as f:
    json.dump(output, f, indent=2)
log(f"Full results written to {output_path}")

# Print answer previews
for label, text in output["answers"].items():
    log_separator(label)
    # Print first ~600 chars
    preview = text[:600]
    for line in preview.split("\n"):
        log(line)
    if len(text) > 600:
        log("... (see phase2_results.json for full answer)")
