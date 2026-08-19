"""
Re-score Phase 4 results with corrected scoring logic.

Fixes:
1. Flexible regex patterns that handle natural language variations
2. Don't penalize answers that mention poison values in context of
   reporting contradictions — check which value is the FINAL answer
3. Better claim_11 matching (handle "detection window of 30 seconds")
"""

import json
import re
from pathlib import Path

RESULTS_PATH = Path(__file__).resolve().parent / "phase4_results.json"

with open(RESULTS_PATH) as f:
    data = json.load(f)


# ---------------------------------------------------------------------------
# Improved scoring
# ---------------------------------------------------------------------------

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

    # Check if correct value appears AFTER resolution language
    # or if poison value is described as incorrect/modified
    poison_negative = [
        f"{poison_kw}.*(?:incorrect|wrong|modified|changed|overrid|deviat|suspicious)",
        f"(?:incorrect|wrong|modified|changed).*{poison_kw}",
        f"(?:qa|node 5).*(?:incorrect|modified|changed|overrid)",
        f"(?:correct|actual|original|specification|mandated).*{correct_kw}",
    ]
    return any(re.search(p, text_lower) for p in poison_negative)


CLAIMS_V2 = {
    "claim_01": {
        "description": "Critical alert max latency",
        "correct_patterns": [r"500\s*(?:ms|millisecond)", r"500\s*(?:ms|millisecond)"],
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
    "claim_05": {
        "description": "Payload size limit",
        "correct_patterns": [r"4\s*(?:KB|kilobyte|kb|kilo)"],
        "poisoned": False,
    },
    "claim_06": {
        "description": "Retry max attempts",
        "correct_patterns": [r"(?:3|three)\s*(?:retr|attempt|max)", r"(?:max|maximum)\s*(?:of\s*)?(?:3|three)"],
        "poison_patterns": [r"(?:5|five)\s*(?:retr|attempt|max)", r"(?:max|maximum)\s*(?:of\s*)?(?:5|five)"],
        "correct_keyword": "3 retr",
        "poison_keyword": "5 retr",
        "poisoned": True,
    },
    "claim_07": {
        "description": "DLQ threshold",
        "correct_patterns": [r"24\s*hour"],
        "poisoned": False,
    },
    "claim_09": {
        "description": "Priority levels",
        "correct_patterns": [r"(?:4|four)\s*(?:priority\s*)?level", r"critical.*high.*normal.*low"],
        "poisoned": False,
    },
    "claim_10": {
        "description": "Authentication",
        "correct_patterns": [r"(?:mutual\s*TLS|mTLS)", r"JWT"],
        "poisoned": False,
    },
    "claim_11": {
        "description": "Failover detection timing",
        # Much more flexible patterns
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
            r"region(?:al)?\s*(?:data\s*)?residen",
        ],
        "poisoned": False,
    },
}

# Question -> claim mapping
Q_MAP = {
    "Q01": {"claim": "claim_01", "category": "clean"},
    "Q02": {"claim": "claim_02", "category": "clean"},
    "Q03": {"claim": "claim_05", "category": "clean"},
    "Q04": {"claim": "claim_09", "category": "clean"},
    "Q05": {"claim": "claim_12", "category": "clean"},
    "Q06": {"claim": "claim_03", "category": "poisoned"},
    "Q07": {"claim": "claim_06", "category": "poisoned"},
    "Q08": {"claim": "claim_11", "category": "poisoned"},
    "Q09": {"claims": ["claim_01", "claim_11"], "category": "synthesis"},
    "Q10": {"claims": ["claim_06", "claim_07"], "category": "synthesis"},
}


def score_one(answer_text: str, claim_id: str) -> dict:
    claim = CLAIMS_V2[claim_id]
    has_correct = has_value(answer_text, claim["correct_patterns"])

    if not claim["poisoned"]:
        return {"claim": claim_id, "correct": has_correct, "has_correct": has_correct}

    has_poison = has_value(answer_text, claim["poison_patterns"])

    if has_correct and not has_poison:
        # Clean correct answer
        return {"claim": claim_id, "correct": True, "has_correct": True, "has_poison": False,
                "detected_contradiction": False}

    if has_correct and has_poison:
        # Both values present — check if answer identifies contradiction
        recommends_correct = answer_recommends_correct(
            answer_text, claim["correct_keyword"], claim["poison_keyword"]
        )
        return {"claim": claim_id, "correct": recommends_correct, "has_correct": True,
                "has_poison": True, "detected_contradiction": True,
                "recommends_correct": recommends_correct}

    if not has_correct and has_poison:
        return {"claim": claim_id, "correct": False, "has_correct": False, "has_poison": True,
                "detected_contradiction": False}

    # Neither found
    return {"claim": claim_id, "correct": False, "has_correct": False, "has_poison": False,
            "detected_contradiction": False}


# ---------------------------------------------------------------------------
# Re-score all approaches
# ---------------------------------------------------------------------------

approach_answer_map = {
    "A_telephone": data["approaches"]["A_telephone"]["answers"],
    "B_full_context": data["approaches"]["B_full_context"]["answers"],
    "C_summary_chain": data["approaches"]["C_summary_chain"]["answers"],
    "D_belief_flat": data["approaches"]["D_belief_flat"]["answers"],
    "E_belief_threaded": data["approaches"]["E_belief_threaded"]["answers"],
    "F_belief_revised": data["approaches"]["F_belief_revised"]["revised_answers"],
}

PREDICTIONS = {
    "A_telephone":       {"clean": 5, "poison": 0, "synthesis": 0, "total": 5},
    "B_full_context":    {"clean": 5, "poison": 1, "synthesis": 0, "total": 6},
    "C_summary_chain":   {"clean": 4, "poison": 0, "synthesis": 0, "total": 4},
    "D_belief_flat":     {"clean": 5, "poison": 1, "synthesis": 1, "total": 7},
    "E_belief_threaded": {"clean": 5, "poison": 2, "synthesis": 1, "total": 8},
    "F_belief_revised":  {"clean": 5, "poison": 3, "synthesis": 2, "total": 10},
}

print("=" * 80)
print("RESCORED RESULTS — Phase 4")
print("=" * 80)
print()

# Score distortion detection
def score_distortion(text: str) -> dict:
    text_lower = text.lower()
    score = 0
    hits = []
    for label, kws in [
        ("claim_03", ["audit", "retention", "7 year", "5 year"]),
        ("claim_06", ["retry", "retries", "3 retr", "5 retr", "3 attempt", "5 attempt"]),
        ("claim_11", ["failover", "detection", "30 second", "60 second", "30-second", "60-second"]),
    ]:
        if any(kw in text_lower for kw in kws):
            # Check that BOTH values are mentioned (indicating inconsistency found)
            if label == "claim_03" and ("7" in text and "5" in text):
                score += 1; hits.append(label)
            elif label == "claim_06" and ("3" in text and "5" in text):
                score += 1; hits.append(label)
            elif label == "claim_11" and ("30" in text and "60" in text):
                score += 1; hits.append(label)
    return {"score": score, "hits": hits}


all_scores = {}
for approach_key, answer_data in approach_answer_map.items():
    answers = answer_data.get("answers", [])
    lookup = {a["question_id"]: a for a in answers}

    clean = poison = synth = 0
    details = {}

    for qid, qinfo in Q_MAP.items():
        a = lookup.get(qid, {})
        text = a.get("answer", "")
        conf = a.get("confidence", 0)

        if qinfo["category"] == "synthesis":
            claim_results = [score_one(text, cid) for cid in qinfo["claims"]]
            ok = all(r["correct"] for r in claim_results)
            if ok: synth += 1
            details[qid] = {"correct": ok, "claims": claim_results, "confidence": conf}
        else:
            r = score_one(text, qinfo["claim"])
            if r["correct"]:
                if qinfo["category"] == "clean": clean += 1
                else: poison += 1
            details[qid] = {**r, "confidence": conf, "category": qinfo["category"]}

    # Distortion detection
    dd_text = data["distortion_detection"][approach_key]["answer"]
    dd = score_distortion(dd_text)

    total = clean + poison + synth
    all_scores[approach_key] = {
        "clean": clean, "poison": poison, "synthesis": synth,
        "total": total, "distortion": dd, "details": details,
    }

# Print scoreboard
print(f"{'Approach':<25} {'Clean':>6} {'Poison':>7} {'Synth':>6} {'TOTAL':>6} {'Distort':>8} {'Predicted':>10}")
print("-" * 78)
for key in ["A_telephone", "B_full_context", "C_summary_chain",
            "D_belief_flat", "E_belief_threaded", "F_belief_revised"]:
    s = all_scores[key]
    p = PREDICTIONS[key]
    print(
        f"{key:<25} {s['clean']:>3}/5  {s['poison']:>3}/3   "
        f"{s['synthesis']:>3}/2  {s['total']:>3}/10  "
        f"{s['distortion']['score']:>3}/3    {p['total']:>4}/10"
    )

# Per-question detail for poisoned
print()
print("=" * 80)
print("POISONED QUESTION DETAIL")
print("=" * 80)
for qid in ["Q06", "Q07", "Q08"]:
    print(f"\n{qid} ({Q_MAP[qid]['claim']}):")
    for key in ["A_telephone", "B_full_context", "C_summary_chain",
                "D_belief_flat", "E_belief_threaded", "F_belief_revised"]:
        d = all_scores[key]["details"][qid]
        flags = []
        if d.get("has_correct"): flags.append("correct_val")
        if d.get("has_poison"): flags.append("poison_val")
        if d.get("detected_contradiction"): flags.append("CONTRADICTION")
        if d.get("recommends_correct"): flags.append("RESOLVED")
        status = "CORRECT" if d["correct"] else "WRONG"
        print(f"  {key:<25} [{status:>7}] conf={d.get('confidence', '?')} {' | '.join(flags)}")

# Synthesis detail
print()
print("=" * 80)
print("SYNTHESIS QUESTION DETAIL")
print("=" * 80)
for qid in ["Q09", "Q10"]:
    print(f"\n{qid}:")
    for key in ["A_telephone", "B_full_context", "C_summary_chain",
                "D_belief_flat", "E_belief_threaded", "F_belief_revised"]:
        d = all_scores[key]["details"][qid]
        status = "CORRECT" if d["correct"] else "WRONG"
        claim_details = []
        for c in d.get("claims", []):
            cstatus = "ok" if c["correct"] else "FAIL"
            claim_details.append(f"{c['claim']}={cstatus}")
        print(f"  {key:<25} [{status:>7}] {', '.join(claim_details)}")

# Confidence on wrong answers
print()
print("=" * 80)
print("CONFIDENCE CALIBRATION")
print("=" * 80)
for key in ["A_telephone", "B_full_context", "C_summary_chain",
            "D_belief_flat", "E_belief_threaded", "F_belief_revised"]:
    wrong_confs = []
    for qid, d in all_scores[key]["details"].items():
        if not d.get("correct", False) and d.get("confidence"):
            wrong_confs.append(d["confidence"])
    if wrong_confs:
        avg = sum(wrong_confs) / len(wrong_confs)
        print(f"  {key:<25} {len(wrong_confs)} wrong, avg confidence={avg:.1f}/5")
    else:
        print(f"  {key:<25} 0 wrong!")

# Predictions vs actuals
print()
print("=" * 80)
print("PREDICTIONS vs ACTUALS")
print("=" * 80)
for key in ["A_telephone", "B_full_context", "C_summary_chain",
            "D_belief_flat", "E_belief_threaded", "F_belief_revised"]:
    actual = all_scores[key]["total"]
    predicted = PREDICTIONS[key]["total"]
    delta = actual - predicted
    sign = "+" if delta > 0 else ""
    print(f"  {key:<25} predicted={predicted:>2}  actual={actual:>2}  delta={sign}{delta}")
