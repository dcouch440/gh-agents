"""
Belief-Oriented Conversation Architecture — Phase 1 Prototype

Proves the core claim: a gatekeeper that decomposes source material into
authored belief slices can match full-context quality at a fraction of
the token cost.

Test subject: resume.rs (444 lines of DAG orchestration logic)
Two questions asked to masks with only their belief slice vs baseline
with the full source.
"""

import json
import time
from pathlib import Path

import anthropic
from dotenv import load_dotenv

# Load .env from the parent repo root
load_dotenv(Path(__file__).resolve().parent.parent / ".env")

client = anthropic.Anthropic()
MODEL = "claude-sonnet-4-5-20250929"
GATEKEEPER_MAX_TOKENS = 4096
DEFAULT_MAX_TOKENS = 1024

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def call(system: str, user: str, label: str, max_tokens: int = DEFAULT_MAX_TOKENS) -> dict:
    """Make one LLM call and return {text, input_tokens, output_tokens, ms}."""
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
    print(f"\n{'='*60}")
    print(f"[{label}]  tokens in={usage.input_tokens}  out={usage.output_tokens}  {elapsed}ms")
    print(f"{'='*60}")
    print(text[:500])
    if len(text) > 500:
        print("... (truncated)")
    return result


# ---------------------------------------------------------------------------
# Source material
# ---------------------------------------------------------------------------

SOURCE_FILE = Path(__file__).resolve().parent.parent / "src/server/hub/dag/resume.rs"
SOURCE = SOURCE_FILE.read_text()

# Two questions that probe different aspects of the code
QUESTIONS = [
    "What are the failure modes in this code? List every way an error can be returned or a step can be silently skipped, and explain the implications of each.",
    "Trace the data flow for the `var_outputs` HashMap from construction through to its final use. What role does it play in inter-step communication, and what would break if it were removed?",
]

# ---------------------------------------------------------------------------
# Step 1: GATEKEEPER — decompose source into tagged belief slices
# ---------------------------------------------------------------------------

GATEKEEPER_SYSTEM = """\
You are the Gatekeeper in a belief-oriented conversation architecture.

Your job: decompose source code into BELIEF SLICES. Each slice is NOT a
summary — it is a hypothesis about what matters, tagged with:
- semantic_tag: what domain concept this slice covers
- confidence: high / medium / low
- emotional_tone: the "feel" of this code area (e.g. "defensive", "fragile",
  "confident", "rushed", "careful")
- content: the actual belief — what you understand about this piece, written
  as a statement of understanding, not a quote of the code
- relevant_lines: approximate line range for reference

Output valid JSON: { "beliefs": [ { ... }, ... ] }
Produce 5-8 belief slices that cover the full file. Each belief should be
dense enough that someone who has NEVER seen the source code could reason
about the system from your beliefs alone.
"""

GATEKEEPER_USER = f"""\
Decompose this source file into belief slices.

```rust
{SOURCE}
```
"""


def run_gatekeeper() -> tuple[list[dict], dict]:
    """Run the gatekeeper and return (beliefs, call_stats)."""
    stats = call(GATEKEEPER_SYSTEM, GATEKEEPER_USER, "GATEKEEPER", max_tokens=GATEKEEPER_MAX_TOKENS)
    text = stats["text"]
    # Extract JSON from possible markdown fencing
    if "```json" in text:
        text = text.split("```json")[1].split("```")[0]
    elif "```" in text:
        text = text.split("```")[1].split("```")[0]
    beliefs = json.loads(text)["beliefs"]
    print(f"\nGatekeeper produced {len(beliefs)} belief slices")
    for b in beliefs:
        print(f"  [{b['confidence']}] ({b['emotional_tone']}) {b['semantic_tag']}")
    return beliefs, stats


# ---------------------------------------------------------------------------
# Step 2: GATEKEEPER assigns beliefs to questions (conversation design)
# ---------------------------------------------------------------------------

ASSIGN_SYSTEM = """\
You are the Gatekeeper designing conversations. Given a set of belief slices
and a question, select which beliefs are RELEVANT to answering that question.

You are the smartest person in the room. You know which beliefs matter and
which will come up dry. Only select beliefs that push the conversation
toward truth.

Output valid JSON: { "selected_indices": [0, 2, 5] }
Use zero-based indices into the beliefs array.
"""


def assign_beliefs(beliefs: list[dict], question: str) -> tuple[list[dict], dict]:
    """Gatekeeper selects which beliefs are relevant for a question."""
    user = f"""\
Beliefs:
{json.dumps(beliefs, indent=2)}

Question: {question}

Select the relevant belief indices.
"""
    stats = call(ASSIGN_SYSTEM, user, f"ASSIGN: {question[:50]}...")
    text = stats["text"]
    if "```json" in text:
        text = text.split("```json")[1].split("```")[0]
    elif "```" in text:
        text = text.split("```")[1].split("```")[0]
    indices = json.loads(text)["selected_indices"]
    selected = [beliefs[i] for i in indices if i < len(beliefs)]
    print(f"  Selected {len(selected)}/{len(beliefs)} beliefs")
    return selected, stats


# ---------------------------------------------------------------------------
# Step 3: MASKS — answer questions with only their curated belief slice
# ---------------------------------------------------------------------------

MASK_SYSTEM = """\
You are a Mask — a focused analytical perspective. You have NOT seen the
original source code. You have ONLY the belief slices provided to you by
the Gatekeeper. These beliefs represent authored understanding of the code.

Answer the question using ONLY the beliefs provided. Be specific and
analytical. If the beliefs don't cover something, say so — do not fabricate.
"""


def run_mask(beliefs: list[dict], question: str, label: str) -> dict:
    """Run a mask with curated beliefs to answer a question."""
    belief_text = "\n\n".join(
        f"[{b['semantic_tag']}] (confidence: {b['confidence']}, tone: {b['emotional_tone']})\n{b['content']}"
        for b in beliefs
    )
    user = f"""\
BELIEF CONTEXT:
{belief_text}

QUESTION:
{question}
"""
    return call(MASK_SYSTEM, user, label)


# ---------------------------------------------------------------------------
# Step 4: BASELINE — full-context answers for comparison
# ---------------------------------------------------------------------------

BASELINE_SYSTEM = """\
You are a code analyst. Answer the question about the provided source code.
Be specific and analytical.
"""


def run_baseline(question: str, label: str) -> dict:
    """Answer a question with the full source as context."""
    user = f"""\
```rust
{SOURCE}
```

QUESTION:
{question}
"""
    return call(BASELINE_SYSTEM, user, label)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    print("=" * 60)
    print("BELIEF-ORIENTED CONVERSATION ARCHITECTURE — Phase 1")
    print(f"Source: {SOURCE_FILE.name} ({len(SOURCE.splitlines())} lines)")
    print("=" * 60)

    # 1. Gatekeeper decomposes
    beliefs, gk_stats = run_gatekeeper()

    all_stats = [gk_stats]

    for i, question in enumerate(QUESTIONS):
        print(f"\n{'#'*60}")
        print(f"QUESTION {i+1}: {question}")
        print(f"{'#'*60}")

        # 2. Gatekeeper assigns beliefs
        selected, assign_stats = assign_beliefs(beliefs, question)
        all_stats.append(assign_stats)

        # 3. Mask answers with belief slice
        mask_stats = run_mask(selected, question, f"MASK Q{i+1}")
        all_stats.append(mask_stats)

        # 4. Baseline answers with full source
        baseline_stats = run_baseline(question, f"BASELINE Q{i+1}")
        all_stats.append(baseline_stats)

    # ---------------------------------------------------------------------------
    # Results
    # ---------------------------------------------------------------------------
    print("\n\n" + "=" * 60)
    print("RESULTS COMPARISON")
    print("=" * 60)

    belief_total_in = sum(s["input_tokens"] for s in all_stats if s["label"].startswith(("GATEKEEPER", "ASSIGN", "MASK")))
    belief_total_out = sum(s["output_tokens"] for s in all_stats if s["label"].startswith(("GATEKEEPER", "ASSIGN", "MASK")))
    baseline_total_in = sum(s["input_tokens"] for s in all_stats if s["label"].startswith("BASELINE"))
    baseline_total_out = sum(s["output_tokens"] for s in all_stats if s["label"].startswith("BASELINE"))

    print(f"\nBelief pipeline:   {belief_total_in:,} input + {belief_total_out:,} output = {belief_total_in + belief_total_out:,} total tokens")
    print(f"Baseline (full):   {baseline_total_in:,} input + {baseline_total_out:,} output = {baseline_total_in + baseline_total_out:,} total tokens")

    if baseline_total_in > 0:
        mask_in = sum(s["input_tokens"] for s in all_stats if s["label"].startswith("MASK"))
        print(f"\nMask input tokens vs baseline input: {mask_in:,} vs {baseline_total_in:,} ({mask_in / baseline_total_in * 100:.1f}%)")

    print("\nPer-call breakdown:")
    for s in all_stats:
        print(f"  {s['label']:30s}  in={s['input_tokens']:>6,}  out={s['output_tokens']:>5,}  {s['ms']:>5}ms")

    # Write full results for review
    output_path = Path(__file__).resolve().parent / "results.json"
    with open(output_path, "w") as f:
        json.dump(all_stats, f, indent=2)
    print(f"\nFull results written to {output_path}")


if __name__ == "__main__":
    main()
