<archetype_context type="belief_capture">
Belief capture distills upstream workflow results into structured
knowledge. It reads the artifacts produced by upstream nodes — documents,
reports, code changes — and extracts atomic beliefs: facts, decisions,
observations, and opinions.

Configure by creating an extraction plan. The plan defines what to focus
on, what tag vocabulary to use, and how to handle contradictions between
upstream sources. Each upstream node's artifacts are processed separately,
preserving source attribution.

Beliefs are stored with semantic tags, confidence levels, and source
provenance. Downstream nodes (rooms, masks, other captures) can query
beliefs by tag, source, type, or confidence.
</archetype_context>

<archetype_guidelines>
- Always set an extraction focus — it shapes what the gatekeeper looks for
- Tag vocabulary should be domain-specific but not too narrow (6-15 tags)
- Use "flag" contradiction handling by default — it preserves all perspectives
- Low confidence threshold keeps speculative claims; high filters to strong evidence only
- The extraction plan runs against each upstream source independently
</archetype_guidelines>
