"""Text formatters for beliefs, questions, and node outputs."""


def format_beliefs_for_convergence(beliefs: list[dict]) -> str:
    """Format raw beliefs for the convergence gatekeeper."""
    lines = []
    for b in beliefs:
        tension = f", tension={b['cross_source_tension']}" if b.get("cross_source_tension") else ""
        tag_field = b.get("semantic_tags") or [b.get("semantic_tag", "unknown")]
        tags = ", ".join(tag_field) if isinstance(tag_field, list) else tag_field
        belief_type = f", type={b['belief_type']}" if b.get("belief_type") else ""
        lines.append(
            f"[{b['id']}] Node {b['source_node']}: {b['source_node_name']} "
            f"(confidence={b['confidence']}, tone={b['emotional_tone']}, tags={tags}{belief_type}{tension})\n"
            f"  Reasoning: {b.get('reasoning', 'N/A')}\n"
            f"  Content: {b['content']}"
        )
    return "\n\n".join(lines)


def format_converged_beliefs_for_mask(converged: list[dict]) -> str:
    """Format converged beliefs for mask agent."""
    lines = []
    for cb in converged:
        header = f"[{cb['id']}] {cb['topic']} (consensus={cb['consensus_strength']}, sources={', '.join(cb['sources'])})"
        body = f"  {cb['content']}"
        if cb.get("contradiction_resolved") and cb.get("resolution_detail"):
            body += f"\n  [RESOLUTION: {cb['resolution_detail']}]"
        lines.append(f"{header}\n{body}")
    return "\n\n".join(lines)


def format_meta_beliefs_for_mask(meta_beliefs: list[dict]) -> str:
    """Format meta-converged beliefs for mask agent."""
    lines = []
    for mb in meta_beliefs:
        tags = ", ".join(mb.get("semantic_tags", []))
        wf_sources = []
        for ws in mb.get("workflow_sources", []):
            wf_sources.append(f"{ws['workflow']}:{','.join(ws['belief_ids'])}")
        wf_str = " | ".join(wf_sources)

        header = (
            f"[{mb['id']}] {mb['topic']} "
            f"(consensus={mb['consensus_strength']}, type={mb.get('belief_type', '?')}, "
            f"tags=[{tags}])"
        )
        body = f"  {mb['content']}"
        body += f"\n  [WORKFLOWS: {wf_str}]"
        if mb.get("contradiction_resolved") and mb.get("resolution_detail"):
            body += f"\n  [RESOLUTION: {mb['resolution_detail']}]"
        lines.append(f"{header}\n{body}")
    return "\n\n".join(lines)


def format_raw_beliefs_for_mask(beliefs: list[dict], selected_ids: list[str] | None = None) -> str:
    """Format raw beliefs for mask, optionally filtering by selected IDs."""
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
