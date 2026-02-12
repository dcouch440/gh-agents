"""
BOCA Phase 7: Multi-Workflow Meta-Convergence

Tests whether beliefs can serve as inter-workflow communication primitives.
Two workflows process the same healthcare system from different perspectives:
  - WF1: Technical specification (6 nodes, reused from Phase 4/6)
  - WF2: Operations runbook (4 new nodes, 1 poisoned)

Five improvements over Phase 6:
  1. Controlled tag vocabulary (taxonomy gatekeeper)
  2. Belief types (fact/policy/opinion/observation)
  3. Calibrated confidence (consensus_strength → answer confidence)
  4. Coverage gap declaration (full/partial/none)
  5. Hierarchical meta-convergence (per-workflow + cross-workflow merge)

Pipeline: ~15 LLM calls, ~$1.50
"""

import json
from datetime import datetime

from .client import (
    RESULTS_DIR, call_json, call_log, call_text, init_logging, log, log_sep,
    save_incremental,
)
from .claims import WF1_CLAIMS, WF2_CLAIMS, audit_claim_coverage, score_answer
from .formatters import (
    format_beliefs_for_convergence,
    format_converged_beliefs_for_mask,
    format_meta_beliefs_for_mask,
    format_node_outputs,
    format_questions_block,
)
from .prompts import (
    build_convergence_v2_system,
    build_convergence_v2_user,
    build_gatekeeper_v3_system,
    build_gatekeeper_v3_user,
    build_mask_v2_system,
    build_mask_v3_system,
    build_meta_convergence_system,
    build_meta_convergence_user,
    build_taxonomy_system,
    build_taxonomy_user,
)
from .questions import PHASE7_Q_CLAIM_MAP, PHASE7_QUESTIONS
from .schemas import (
    ANSWER_SCHEMA_V2,
    ANSWER_SCHEMA_V3,
    CONVERGENCE_SCHEMA_V2,
    BELIEF_SCHEMA_V3,
    META_CONVERGENCE_SCHEMA,
    TAXONOMY_SCHEMA,
)
from .sources import OPS_RUNBOOK_TEXT, SPEC_TEXT, WF2_NODES

OUTPUT_PATH = RESULTS_DIR / "phase7_results.json"
LOG_PATH = RESULTS_DIR / "phase7.log"
PHASE4_PATH = RESULTS_DIR / "phase4_results.json"
PHASE6_PATH = RESULTS_DIR / "phase6_results.json"


# ===========================================================================
# PRE-REGISTERED PREDICTIONS
# ===========================================================================

PREDICTIONS = {
    "meta_converged":  {"wf1": 5, "wf2": 4, "cross": 4, "adv": 4, "total": 17},
    "flat_converged":  {"wf1": 5, "wf2": 3, "cross": 3, "adv": 3, "total": 14},
    "full_context":    {"wf1": 5, "wf2": 5, "cross": 5, "adv": 3, "total": 18},
}


# ===========================================================================
# SCORING
# ===========================================================================

ALL_CLAIMS = {**WF1_CLAIMS, **WF2_CLAIMS}


def score_phase7_question(answer_text: str, question_id: str) -> dict:
    """Score a Phase 7 answer against its mapped claim(s)."""
    q_info = PHASE7_Q_CLAIM_MAP[question_id]
    category = q_info["category"]

    if "claims" in q_info:
        # Synthesis — all component claims must be correct
        claim_ids = q_info["claims"]
        results = [score_answer(answer_text, cid, ALL_CLAIMS) for cid in claim_ids]
        all_correct = all(r["correct"] for r in results)
        return {"question_id": question_id, "correct": all_correct,
                "category": category, "component_results": results}
    else:
        claim_id = q_info["claim"]
        r = score_answer(answer_text, claim_id, ALL_CLAIMS)
        return {**r, "question_id": question_id, "category": category}


def score_all_phase7(answers: list[dict]) -> dict:
    """Score all Phase 7 answers and aggregate by category."""
    lookup = {a["question_id"]: a for a in answers}

    wf1 = wf2 = cross = adv = 0
    wf1_total = wf2_total = cross_total = adv_total = 0
    details = {}

    for q in PHASE7_QUESTIONS:
        a = lookup.get(q["id"], {})
        text = a.get("answer", "")
        r = score_phase7_question(text, q["id"])
        r["confidence"] = a.get("confidence", 0)
        details[q["id"]] = r

        cat = q["category"]
        if cat == "wf1_only":
            wf1_total += 1
            if r["correct"]:
                wf1 += 1
        elif cat == "wf2_only":
            wf2_total += 1
            if r["correct"]:
                wf2 += 1
        elif cat == "cross_workflow":
            cross_total += 1
            if r["correct"]:
                cross += 1
        elif cat == "cross_workflow_adversarial":
            adv_total += 1
            if r["correct"]:
                adv += 1

    total = wf1 + wf2 + cross + adv
    return {
        "wf1": wf1, "wf1_total": wf1_total,
        "wf2": wf2, "wf2_total": wf2_total,
        "cross": cross, "cross_total": cross_total,
        "adv": adv, "adv_total": adv_total,
        "total": total, "total_possible": len(PHASE7_QUESTIONS),
        "details": details,
    }


# ===========================================================================
# PIPELINE
# ===========================================================================

def main():
    init_logging(LOG_PATH)

    output = {
        "meta": {
            "phase": 7,
            "description": "Multi-Workflow Meta-Convergence",
            "timestamp": datetime.now().isoformat(),
            "improvements": [
                "Controlled tag vocabulary",
                "Belief types (fact/policy/opinion/observation)",
                "Calibrated confidence",
                "Coverage gap declaration",
                "Hierarchical meta-convergence",
            ],
        },
        "predictions": PREDICTIONS,
        "taxonomy": {},
        "wf1": {},
        "wf2": {},
        "meta_convergence": {},
        "flat_convergence": {},
        "question_answering": {},
        "scoring": {},
    }

    log_sep("BOCA PHASE 7: MULTI-WORKFLOW META-CONVERGENCE")
    log("Testing beliefs as inter-workflow communication primitives")

    # ===================================================================
    # LOAD WF1 DATA (from Phase 4 + Phase 6 — zero regeneration cost)
    # ===================================================================
    log_sep("LOADING WF1 DATA (Phase 4/6)")

    with open(PHASE4_PATH) as f:
        phase4 = json.load(f)
    with open(PHASE6_PATH) as f:
        phase6 = json.load(f)

    wf1_node_outputs = phase4["node_outputs"]
    wf1_raw_beliefs = phase6["belief_generation"]["beliefs"]
    wf1_converged = phase6["convergence"]["converged_beliefs"]

    log(f"WF1 node outputs: {len(wf1_node_outputs)}")
    log(f"WF1 raw v2 beliefs: {len(wf1_raw_beliefs)}")
    log(f"WF1 converged v2 beliefs: {len(wf1_converged)}")

    output["wf1"] = {
        "node_count": len(wf1_node_outputs),
        "raw_belief_count": len(wf1_raw_beliefs),
        "converged_belief_count": len(wf1_converged),
        "source": "phase4_results.json + phase6_results.json",
    }
    save_incremental(output, OUTPUT_PATH)

    # ===================================================================
    # STEP 0: TAXONOMY GENERATION (1 LLM call)
    # ===================================================================
    log_sep("STEP 0: TAXONOMY GENERATION (1 call)")

    taxonomy_system = build_taxonomy_system()
    taxonomy_user = build_taxonomy_user(SPEC_TEXT, OPS_RUNBOOK_TEXT)

    taxonomy_data, taxonomy_stats = call_json(
        taxonomy_system, taxonomy_user, "TAXONOMY",
        TAXONOMY_SCHEMA, max_tokens=4096
    )

    tags = taxonomy_data.get("tags", [])
    tag_names = [t["tag"] for t in tags]
    log(f"Generated {len(tags)} taxonomy tags")
    for t in tags:
        log(f"  [{t['domain']:>12}] {t['tag']}: {t['description'][:60]}")

    output["taxonomy"] = {
        "tags": tags,
        "tag_count": len(tags),
        "reasoning": taxonomy_data.get("taxonomy_reasoning", ""),
        "tokens": taxonomy_stats,
    }
    save_incremental(output, OUTPUT_PATH)

    # ===================================================================
    # STEP 1: WF2 NODE GENERATION (4 LLM calls)
    # ===================================================================
    log_sep("STEP 1: WF2 NODE GENERATION (4 calls)")

    wf2_node_outputs: list[dict] = []

    for node in WF2_NODES:
        result = call_text(
            node["system"],
            f"Review and transform this operations runbook from your professional perspective:\n\n{OPS_RUNBOOK_TEXT}",
            f"WF2:Node_{node['id']}:{node['name']}",
            max_tokens=2048
        )
        wf2_node_outputs.append({
            "node_id": node["id"],
            "node_name": node["name"],
            "output": result["text"],
        })
        log(f"  Node {node['id']} ({node['name']}): {len(result['text'])} chars")

    output["wf2"]["node_outputs"] = wf2_node_outputs
    save_incremental(output, OUTPUT_PATH)

    # ===================================================================
    # STEP 2: WF2 BELIEF EXTRACTION V3 (4 LLM calls)
    # ===================================================================
    log_sep("STEP 2: WF2 BELIEF EXTRACTION V3 (4 calls)")

    wf2_all_beliefs: list[dict] = []
    belief_counter = 0

    for node_out in wf2_node_outputs:
        system = build_gatekeeper_v3_system(
            node_out["node_name"], node_out["node_id"], "MedAlert Operations"
        )
        user = build_gatekeeper_v3_user(node_out["output"], tag_names)

        data, stats = call_json(
            system, user,
            f"WF2:BELIEFS_V3:Node_{node_out['node_id']}",
            BELIEF_SCHEMA_V3, max_tokens=4096
        )

        node_beliefs = data.get("beliefs", [])
        for b in node_beliefs:
            belief_counter += 1
            b["id"] = f"w2b{belief_counter:02d}"
            b["source_node"] = node_out["node_id"]
            b["source_node_name"] = node_out["node_name"]
            b["workflow"] = "wf2"
            wf2_all_beliefs.append(b)

        log(f"  Node {node_out['node_id']} ({node_out['node_name']}): {len(node_beliefs)} beliefs")
        for b in node_beliefs:
            bt = f" [{b.get('belief_type', '?')}]" if b.get("belief_type") else ""
            log(f"    [{b['id']}]{bt} {', '.join(b.get('semantic_tags', []))} ({b['confidence']}) {b['content'][:70]}...")

    log(f"\nTotal WF2 v3 beliefs: {len(wf2_all_beliefs)}")

    # Belief type distribution
    type_dist = {}
    for b in wf2_all_beliefs:
        bt = b.get("belief_type", "unknown")
        type_dist[bt] = type_dist.get(bt, 0) + 1
    log(f"Belief type distribution: {type_dist}")

    # Tag coverage check
    used_tags = set()
    for b in wf2_all_beliefs:
        used_tags.update(b.get("semantic_tags", []))
    taxonomy_set = set(tag_names)
    out_of_vocab = used_tags - taxonomy_set
    if out_of_vocab:
        log(f"[WARN] Out-of-vocabulary tags: {out_of_vocab}", "WARN")

    output["wf2"]["beliefs"] = wf2_all_beliefs
    output["wf2"]["belief_count"] = len(wf2_all_beliefs)
    output["wf2"]["belief_type_distribution"] = type_dist
    output["wf2"]["tag_coverage"] = {
        "used_tags": sorted(used_tags),
        "taxonomy_tags": sorted(taxonomy_set),
        "out_of_vocab": sorted(out_of_vocab),
    }
    save_incremental(output, OUTPUT_PATH)

    # ===================================================================
    # STEP 3: WF2 CONVERGENCE (1 LLM call)
    # ===================================================================
    log_sep("STEP 3: WF2 CONVERGENCE (1 call)")

    wf2_beliefs_text = format_beliefs_for_convergence(wf2_all_beliefs)
    convergence_system = build_convergence_v2_system()
    convergence_user = build_convergence_v2_user(wf2_beliefs_text, len(wf2_all_beliefs))

    wf2_conv_data, wf2_conv_stats = call_json(
        convergence_system, convergence_user, "WF2:CONVERGENCE",
        CONVERGENCE_SCHEMA_V2, max_tokens=16384
    )

    wf2_converged = wf2_conv_data.get("converged_beliefs", [])
    wf2_comp_stats = wf2_conv_data.get("compression_stats", {})

    log(f"WF2 convergence: {len(wf2_all_beliefs)} → {len(wf2_converged)} beliefs")
    log(f"  Contradictions found: {wf2_comp_stats.get('contradictions_found', '?')}")
    log(f"  Contradictions resolved: {wf2_comp_stats.get('contradictions_resolved', '?')}")

    for cb in wf2_converged:
        flag = " [RESOLVED]" if cb.get("contradiction_resolved") else ""
        log(f"  {cb['id']}: {cb['topic']} ({cb['consensus_strength']}){flag}")

    output["wf2"]["convergence"] = {
        "converged_beliefs": wf2_converged,
        "compression_stats": wf2_comp_stats,
        "tokens": wf2_conv_stats,
    }
    save_incremental(output, OUTPUT_PATH)

    # ===================================================================
    # STEP 4: META-CONVERGENCE (1 LLM call)
    # ===================================================================
    log_sep("STEP 4: HIERARCHICAL META-CONVERGENCE (1 call)")

    wf1_conv_text = format_converged_beliefs_for_mask(wf1_converged)
    wf2_conv_text = format_converged_beliefs_for_mask(wf2_converged)

    meta_system = build_meta_convergence_system()
    meta_user = build_meta_convergence_user(
        wf1_conv_text, len(wf1_converged),
        wf2_conv_text, len(wf2_converged),
        tag_names
    )

    meta_data, meta_stats = call_json(
        meta_system, meta_user, "META_CONVERGENCE",
        META_CONVERGENCE_SCHEMA, max_tokens=16384
    )

    meta_beliefs = meta_data.get("meta_beliefs", [])
    cross_val_summary = meta_data.get("cross_validation_summary", {})
    meta_comp_stats = meta_data.get("compression_stats", {})

    log(f"\nMeta-convergence results:")
    log(f"  WF1 input: {len(wf1_converged)} converged beliefs")
    log(f"  WF2 input: {len(wf2_converged)} converged beliefs")
    log(f"  Output: {len(meta_beliefs)} meta-beliefs")
    log(f"  Cross-validated: {cross_val_summary.get('cross_validated', '?')}")
    log(f"  Cross-workflow splits: {cross_val_summary.get('cross_workflow_splits', '?')}")
    log(f"  WF1-only topics: {cross_val_summary.get('wf1_only_topics', '?')}")
    log(f"  WF2-only topics: {cross_val_summary.get('wf2_only_topics', '?')}")

    for mb in meta_beliefs:
        flag = " [RESOLVED]" if mb.get("contradiction_resolved") else ""
        cs = mb.get("consensus_strength", "?")
        log(f"  {mb['id']}: {mb['topic']} ({cs}){flag}")

    output["meta_convergence"] = {
        "meta_beliefs": meta_beliefs,
        "cross_validation_summary": cross_val_summary,
        "compression_stats": meta_comp_stats,
        "tokens": meta_stats,
    }
    save_incremental(output, OUTPUT_PATH)

    # ===================================================================
    # STEP 5: FLAT CONVERGENCE BASELINE (1 LLM call)
    # ===================================================================
    log_sep("STEP 5: FLAT CONVERGENCE BASELINE (1 call)")

    # Combine ALL raw beliefs from both workflows
    all_raw_beliefs = wf1_raw_beliefs + wf2_all_beliefs
    log(f"Total raw beliefs for flat convergence: {len(all_raw_beliefs)}")

    flat_beliefs_text = format_beliefs_for_convergence(all_raw_beliefs)
    flat_conv_system = build_convergence_v2_system()
    flat_conv_user = build_convergence_v2_user(flat_beliefs_text, len(all_raw_beliefs))

    flat_data, flat_stats = call_json(
        flat_conv_system, flat_conv_user, "FLAT_CONVERGENCE",
        CONVERGENCE_SCHEMA_V2, max_tokens=16384
    )

    flat_converged = flat_data.get("converged_beliefs", [])
    flat_comp_stats = flat_data.get("compression_stats", {})

    log(f"Flat convergence: {len(all_raw_beliefs)} → {len(flat_converged)} beliefs")
    log(f"  Contradictions found: {flat_comp_stats.get('contradictions_found', '?')}")
    log(f"  Contradictions resolved: {flat_comp_stats.get('contradictions_resolved', '?')}")

    output["flat_convergence"] = {
        "converged_beliefs": flat_converged,
        "compression_stats": flat_comp_stats,
        "tokens": flat_stats,
    }
    save_incremental(output, OUTPUT_PATH)

    # ===================================================================
    # STEP 6: QUALITY AUDIT (0 LLM calls)
    # ===================================================================
    log_sep("STEP 6: QUALITY AUDIT (0 calls)")

    # Audit WF2 convergence against WF2 claims
    wf2_audit = audit_claim_coverage(wf2_converged, WF2_CLAIMS)
    log(f"WF2 converged claim coverage: {wf2_audit['total_covered']}/{wf2_audit['total_claims']}")
    for cid, info in wf2_audit["claims"].items():
        status = "COVERED" if info["covered"] else "MISSING"
        log(f"  {cid} ({info['description']}): {status}")

    # Audit meta-convergence against ALL claims
    meta_audit_wf1 = audit_claim_coverage(meta_beliefs, WF1_CLAIMS)
    meta_audit_wf2 = audit_claim_coverage(meta_beliefs, WF2_CLAIMS)
    log(f"\nMeta-converged WF1 claim coverage: {meta_audit_wf1['total_covered']}/{meta_audit_wf1['total_claims']}")
    log(f"Meta-converged WF2 claim coverage: {meta_audit_wf2['total_covered']}/{meta_audit_wf2['total_claims']}")

    # Audit flat convergence
    flat_audit_wf1 = audit_claim_coverage(flat_converged, WF1_CLAIMS)
    flat_audit_wf2 = audit_claim_coverage(flat_converged, WF2_CLAIMS)
    log(f"\nFlat-converged WF1 claim coverage: {flat_audit_wf1['total_covered']}/{flat_audit_wf1['total_claims']}")
    log(f"Flat-converged WF2 claim coverage: {flat_audit_wf2['total_covered']}/{flat_audit_wf2['total_claims']}")

    output["audit"] = {
        "wf2_convergence": wf2_audit,
        "meta_wf1": meta_audit_wf1,
        "meta_wf2": meta_audit_wf2,
        "flat_wf1": flat_audit_wf1,
        "flat_wf2": flat_audit_wf2,
    }
    save_incremental(output, OUTPUT_PATH)

    # ===================================================================
    # STEP 7: QUESTION ANSWERING (3 LLM calls)
    # ===================================================================
    log_sep("STEP 7: QUESTION ANSWERING (3 calls)")

    questions_block = format_questions_block(PHASE7_QUESTIONS)

    # --- Approach A: Meta-converged beliefs → v3 mask ---
    log("\n--- APPROACH A: Meta-Converged Beliefs ---")
    meta_beliefs_text = format_meta_beliefs_for_mask(meta_beliefs)
    mask_v3_system = build_mask_v3_system()

    meta_user = (
        f"<beliefs>\n{meta_beliefs_text}\n</beliefs>\n\n"
        f"<questions>\n{questions_block}\n</questions>\n\n"
        f"Answer each question using ONLY the meta-converged beliefs above. "
        f"Calibrate confidence based on consensus_strength. "
        f"Declare coverage gaps honestly."
    )
    meta_answers, meta_ans_stats = call_json(
        mask_v3_system, meta_user, "META:ANSWERS_20Q",
        ANSWER_SCHEMA_V3, max_tokens=16384
    )
    output["question_answering"]["meta_converged"] = {
        "answers": meta_answers, "tokens": meta_ans_stats,
    }
    save_incremental(output, OUTPUT_PATH)

    # --- Approach B: Flat-converged beliefs → v2 mask ---
    log("\n--- APPROACH B: Flat-Converged Beliefs ---")
    flat_beliefs_text_mask = format_converged_beliefs_for_mask(flat_converged)
    mask_v2_system = build_mask_v2_system()

    flat_user = (
        f"<beliefs>\n{flat_beliefs_text_mask}\n</beliefs>\n\n"
        f"<questions>\n{questions_block}\n</questions>\n\n"
        f"Answer each question using ONLY the beliefs above."
    )
    flat_answers, flat_ans_stats = call_json(
        mask_v2_system, flat_user, "FLAT:ANSWERS_20Q",
        ANSWER_SCHEMA_V2, max_tokens=16384
    )
    output["question_answering"]["flat_converged"] = {
        "answers": flat_answers, "tokens": flat_ans_stats,
    }
    save_incremental(output, OUTPUT_PATH)

    # --- Approach C: Full context (all node outputs) → direct answering ---
    log("\n--- APPROACH C: Full Context (all node outputs) ---")
    all_node_outputs = wf1_node_outputs + wf2_node_outputs
    all_outputs_text = format_node_outputs(all_node_outputs)

    fc_system = (
        "You have outputs from 10 different professionals who reviewed a healthcare "
        "notification system from two perspectives (technical specification and operations runbook). "
        "Answer each question precisely. Include SPECIFIC NUMBERS in every answer."
    )
    fc_user = (
        f"<professional_outputs>\n{all_outputs_text}\n</professional_outputs>\n\n"
        f"<questions>\n{questions_block}\n</questions>\n\n"
        f"Answer each question using the professional outputs above."
    )
    fc_answers, fc_stats = call_json(
        fc_system, fc_user, "FULL_CONTEXT:ANSWERS_20Q",
        ANSWER_SCHEMA_V2, max_tokens=16384
    )
    output["question_answering"]["full_context"] = {
        "answers": fc_answers, "tokens": fc_stats,
    }
    save_incremental(output, OUTPUT_PATH)

    # ===================================================================
    # STEP 8: SCORING (0 LLM calls)
    # ===================================================================
    log_sep("STEP 8: SCORING")

    scoring = {}
    for approach_key in ["meta_converged", "flat_converged", "full_context"]:
        answers = output["question_answering"][approach_key]["answers"].get("answers", [])
        s = score_all_phase7(answers)
        scoring[approach_key] = s
        log(f"  {approach_key}: wf1={s['wf1']}/{s['wf1_total']} wf2={s['wf2']}/{s['wf2_total']} "
            f"cross={s['cross']}/{s['cross_total']} adv={s['adv']}/{s['adv_total']} "
            f"TOTAL={s['total']}/{s['total_possible']}")

    output["scoring"] = scoring
    save_incremental(output, OUTPUT_PATH)

    # ===================================================================
    # STEP 9: SCOREBOARD
    # ===================================================================
    log_sep("SCOREBOARD — PHASE 7")
    log("")
    log(f"{'Approach':<20} {'WF1':>5} {'WF2':>5} {'Cross':>6} {'Adv':>5} {'TOTAL':>7} {'Pred':>6}")
    log("-" * 60)

    for key in ["meta_converged", "flat_converged", "full_context"]:
        s = scoring[key]
        p = PREDICTIONS[key]
        log(f"{key:<20} {s['wf1']:>2}/{s['wf1_total']}  {s['wf2']:>2}/{s['wf2_total']}  "
            f"{s['cross']:>2}/{s['cross_total']}   {s['adv']:>2}/{s['adv_total']}  "
            f"{s['total']:>3}/{s['total_possible']}  {p['total']:>4}")

    # Hierarchical premium
    hp = scoring["meta_converged"]["total"] - scoring["flat_converged"]["total"]
    log(f"\nHierarchical premium: {'+' if hp >= 0 else ''}{hp} (predicted: +3)")

    # Cross-validation premium (meta vs flat on adversarial)
    cvp = scoring["meta_converged"]["adv"] - scoring["flat_converged"]["adv"]
    log(f"Cross-validation premium (adversarial): {'+' if cvp >= 0 else ''}{cvp}")

    # Predictions vs actuals
    log_sep("PREDICTIONS vs ACTUALS")
    for key in ["meta_converged", "flat_converged", "full_context"]:
        actual = scoring[key]["total"]
        predicted = PREDICTIONS[key]["total"]
        delta = actual - predicted
        sign = "+" if delta > 0 else ""
        log(f"  {key:<20} predicted={predicted:>2}  actual={actual:>2}  delta={sign}{delta}")

    # Per-question detail for adversarial
    log_sep("ADVERSARIAL QUESTION DETAIL")
    for qid in ["P7Q16", "P7Q17", "P7Q18", "P7Q19", "P7Q20"]:
        log(f"\n{qid} ({PHASE7_QUESTIONS[[q['id'] for q in PHASE7_QUESTIONS].index(qid)]['text'][:60]}...):")
        for key in ["meta_converged", "flat_converged", "full_context"]:
            d = scoring[key]["details"].get(qid, {})
            flags = []
            if d.get("has_correct"):
                flags.append("correct_val")
            if d.get("has_poison"):
                flags.append("poison_val")
            if d.get("detected_contradiction"):
                flags.append("CONTRADICTION")
            if d.get("recommends_correct"):
                flags.append("RESOLVED")
            # For synthesis questions
            if "component_results" in d:
                for cr in d["component_results"]:
                    if cr.get("has_poison"):
                        flags.append(f"poison:{cr['claim_id']}")
            status = "CORRECT" if d.get("correct") else "WRONG"
            log(f"  {key:<20} [{status:>7}] conf={d.get('confidence', '?')} {' | '.join(flags)}")

    # Confidence calibration check
    log_sep("CONFIDENCE CALIBRATION (meta-converged)")
    meta_ans = output["question_answering"]["meta_converged"]["answers"].get("answers", [])
    for a in meta_ans:
        cal = a.get("confidence_calibration", "N/A")[:80]
        cov = a.get("coverage_assessment", "?")
        gaps = a.get("coverage_gaps", "")[:60]
        gap_str = f" gaps={gaps}" if gaps else ""
        log(f"  {a['question_id']}: conf={a.get('confidence', '?')} cov={cov}{gap_str}")

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
        log(f"  {c['label']:<40} {c['input_tokens']:>6} in  {c['output_tokens']:>6} out  ({c['ms']}ms)")

    # Final
    output["meta"]["total_llm_calls"] = len(call_log)
    output["meta"]["total_tokens"] = total_in + total_out
    output["call_log"] = call_log
    save_incremental(output, OUTPUT_PATH)

    log_sep("PHASE 7 COMPLETE")
    log(f"Results: {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
