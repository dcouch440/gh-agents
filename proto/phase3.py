"""
Belief-Oriented Conversation Architecture — Phase 3
Long-Chain Belief Threading

Proves that beliefs carry signal across a 9-node workflow chain (39 beliefs)
and that the gatekeeper can:
1. Detect belief THREADS spanning multiple nodes
2. Resurface DORMANT beliefs from early nodes when newly relevant
3. Prune NOISE — factual but non-actionable beliefs

Total LLM calls: 7 (3 gatekeeper + 3 mask + 1 baseline). Zero waste.
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
OUTPUT_PATH = Path(__file__).resolve().parent / "phase3_results.json"
LOG_FILE = Path(__file__).resolve().parent / "phase3.log"

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

LOG_FILE.write_text(f"# Phase 3 — {datetime.now().isoformat()}\n\n")

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

# ---------------------------------------------------------------------------
# Workflow Topology
# ---------------------------------------------------------------------------

WORKFLOW_NODES = [
    {"id": 1,  "name": "PRD",                   "depth": 9},
    {"id": 2,  "name": "Researchers",            "depth": 8},
    {"id": 3,  "name": "API Docs",               "depth": 7},
    {"id": 4,  "name": "Architecture",           "depth": 6},
    {"id": 5,  "name": "Task Decomposition",     "depth": 5},
    {"id": 6,  "name": "Implementation Plan",    "depth": 4},
    {"id": 7,  "name": "Module A: CRDT Engine",  "depth": 3},
    {"id": 8,  "name": "Module B: WS Relay",     "depth": 2},
    {"id": 9,  "name": "Integration Test",       "depth": 1},
    {"id": 10, "name": "Shipping Meeting",       "depth": 0},
]

WORKFLOW_EDGES = [
    (1, 4), (2, 4), (3, 4),   # PRD, Research, API Docs → Architecture
    (4, 5), (5, 6),           # Architecture → Task Decomp → Impl Plan
    (6, 7), (6, 8),           # Impl Plan → Module A, Module B
    (7, 9), (8, 9),           # Modules → Integration Test
    # All upstream nodes connect to meeting:
    (1, 10), (2, 10), (3, 10), (4, 10), (5, 10),
    (6, 10), (7, 10), (8, 10), (9, 10),
]

# ---------------------------------------------------------------------------
# Static Belief Store (39 beliefs, 0 LLM calls)
# ---------------------------------------------------------------------------

BELIEFS = [
    # --- Node 1: PRD (depth 9) ---
    {"id": "b01", "semantic_tag": "core_feature_scope", "confidence": "high",
     "emotional_tone": "ambitious", "source_node": 1, "source_node_name": "PRD", "source_depth": 9,
     "content": "The product requires real-time collaborative editing with conflict resolution for up to 50 concurrent users per document. Must support rich text, embedded images, and @mentions. Positioned as the flagship differentiator for the Q2 enterprise launch."},
    {"id": "b02", "semantic_tag": "gdpr_data_residency", "confidence": "high",
     "emotional_tone": "non-negotiable", "source_node": 1, "source_node_name": "PRD", "source_depth": 9,
     "content": "GDPR compliance requires all document content and edit history to remain within EU data centers. Real-time sync infrastructure must not route data through US-based relay servers. Legal has flagged this as a hard launch blocker."},
    {"id": "b03", "semantic_tag": "timeline_pressure", "confidence": "high",
     "emotional_tone": "urgent", "source_node": 1, "source_node_name": "PRD", "source_depth": 9,
     "content": "Q2 launch deadline is fixed and non-negotiable per executive commitment. Scope cuts may be necessary but core editing and conflict resolution must ship. Image embedding and @mentions are P1 stretch goals."},
    {"id": "b04", "semantic_tag": "offline_editing_requirement", "confidence": "medium",
     "emotional_tone": "aspirational", "source_node": 1, "source_node_name": "PRD", "source_depth": 9,
     "content": "Offline editing capability where users make changes while disconnected and sync on reconnect. Listed as P0 but technical complexity acknowledged as significant. Product believes this is essential for mobile enterprise users."},
    {"id": "b05", "semantic_tag": "performance_sla", "confidence": "high",
     "emotional_tone": "precise", "source_node": 1, "source_node_name": "PRD", "source_depth": 9,
     "content": "200ms maximum latency for edit propagation between users. P99 must stay under 500ms. Derived from user research showing latency above 300ms causes perceived lag in collaborative typing."},
    {"id": "b06", "semantic_tag": "api_versioning_constraint", "confidence": "medium",
     "emotional_tone": "cautious", "source_node": 1, "source_node_name": "PRD", "source_depth": 9,
     "content": "Existing REST API is at v2. New real-time endpoints must coexist with current request-response model. Breaking changes to v2 are not permitted for Q2 release."},

    # --- Node 2: Researchers (depth 8) ---
    {"id": "b07", "semantic_tag": "crdt_vs_ot_analysis", "confidence": "high",
     "emotional_tone": "analytical", "source_node": 2, "source_node_name": "Researchers", "source_depth": 8,
     "content": "CRDTs (specifically Yjs/Y-CRDT) preferred over Operational Transformation. CRDTs handle offline editing natively, need no central coordination. OT rejected due to server-dependency conflicting with offline requirement."},
    {"id": "b08", "semantic_tag": "competitor_landscape", "confidence": "medium",
     "emotional_tone": "observational", "source_node": 2, "source_node_name": "Researchers", "source_depth": 8,
     "content": "Notion and Google Docs use proprietary OT variants. Linear recently shipped CRDT-based collaboration. Figma uses custom CRDT for design files. CRDTs are the emerging standard for new implementations."},
    {"id": "b09", "semantic_tag": "yjs_library_risk", "confidence": "medium",
     "emotional_tone": "cautious", "source_node": 2, "source_node_name": "Researchers", "source_depth": 8,
     "content": "Yjs maintained primarily by a single developer. MIT-licensed and widely adopted but bus-factor risk exists. The Rust port (y-crdt) is less mature. Research recommends wrapping in an abstraction layer to enable future replacement."},
    {"id": "b10", "semantic_tag": "eu_relay_options", "confidence": "high",
     "emotional_tone": "thorough", "source_node": 2, "source_node_name": "Researchers", "source_depth": 8,
     "content": "Three EU-compliant WebSocket relay options identified: self-hosted in Frankfurt (AWS eu-central-1), Cloudflare Durable Objects with EU-only routing, Hetzner bare metal. Self-hosted gives full control but requires ops investment."},
    {"id": "b11", "semantic_tag": "50_user_scalability", "confidence": "medium",
     "emotional_tone": "worried", "source_node": 2, "source_node_name": "Researchers", "source_depth": 8,
     "content": "Benchmarking Yjs with 50 concurrent users showed 120MB memory per document and ~2000 messages/second sync rate. Within bounds for server-side relay but may stress WebSocket connections. Connection pooling and document-level sharding recommended."},

    # --- Node 3: API Docs (depth 7) ---
    {"id": "b12", "semantic_tag": "current_api_surface", "confidence": "high",
     "emotional_tone": "factual", "source_node": 3, "source_node_name": "API Docs", "source_depth": 7,
     "content": "Existing v2 API has 47 endpoints across documents, users, teams, and permissions. Document CRUD returns JSON averaging 2KB. No WebSocket support — all interactions are request-response over HTTPS."},
    {"id": "b13", "semantic_tag": "auth_model_limitation", "confidence": "high",
     "emotional_tone": "concerned", "source_node": 3, "source_node_name": "API Docs", "source_depth": 7,
     "content": "Current auth uses short-lived JWTs (15 min expiry) with refresh tokens. WebSocket connections persisting for hours need a different auth strategy. Current middleware only handles HTTP request headers."},
    {"id": "b14", "semantic_tag": "document_model_incompatibility", "confidence": "high",
     "emotional_tone": "structural", "source_node": 3, "source_node_name": "API Docs", "source_depth": 7,
     "content": "Documents stored as JSON blobs in PostgreSQL with optimistic locking. Entire document replaced on each save. Fundamentally incompatible with CRDT-based editing which requires merging incremental updates."},
    {"id": "b15", "semantic_tag": "rate_limiting_config", "confidence": "medium",
     "emotional_tone": "administrative", "source_node": 3, "source_node_name": "API Docs", "source_depth": 7,
     "content": "API enforces 100 req/s per user, 1000 req/s per org. These limits would immediately throttle real-time editing which generates 10-30 operations per second per active user."},
    {"id": "b16", "semantic_tag": "existing_webhook_system", "confidence": "low",
     "emotional_tone": "tangential", "source_node": 3, "source_node_name": "API Docs", "source_depth": 7,
     "content": "Existing webhook system for document change notifications used by 12 enterprise customers. Changes to document update patterns must maintain webhook compatibility or provide migration path."},

    # --- Node 4: Architecture (depth 6) ---
    {"id": "b17", "semantic_tag": "dual_storage_architecture", "confidence": "high",
     "emotional_tone": "decisive", "source_node": 4, "source_node_name": "Architecture", "source_depth": 6,
     "content": "CRDT state stored in separate binary column alongside existing JSON. JSON column serves REST API. Background process materializes CRDT to JSON for backward compatibility. Avoids breaking v2 API while enabling real-time."},
    {"id": "b18", "semantic_tag": "websocket_gateway_design", "confidence": "high",
     "emotional_tone": "confident", "source_node": 4, "source_node_name": "Architecture", "source_depth": 6,
     "content": "Dedicated WebSocket gateway service separate from REST API servers. Enables independent scaling — WS connections are long-lived and memory-intensive while REST is stateless. Authenticates via one-time token exchange at connection time."},
    {"id": "b19", "semantic_tag": "eu_deployment_topology", "confidence": "high",
     "emotional_tone": "deliberate", "source_node": 4, "source_node_name": "Architecture", "source_depth": 6,
     "content": "WebSocket gateway and CRDT state deployed exclusively in eu-central-1 for GDPR. Geo-fence at load balancer routes EU organization requests to EU infra. Non-EU traffic goes to us-east-1 as before."},
    {"id": "b20", "semantic_tag": "crdt_abstraction_layer", "confidence": "high",
     "emotional_tone": "careful", "source_node": 4, "source_node_name": "Architecture", "source_depth": 6,
     "content": "CRDT engine wrapped in DocumentSyncEngine trait abstracting the specific implementation (Yjs today, replaceable). Addresses bus-factor risk. Trait exposes: apply_update, get_state_vector, encode_diff, merge_states."},
    {"id": "b21", "semantic_tag": "offline_sync_protocol", "confidence": "medium",
     "emotional_tone": "uncertain", "source_node": 4, "source_node_name": "Architecture", "source_depth": 6,
     "content": "Offline sync uses store-and-forward: edits queued locally, sent as batch on reconnect. CRDT guarantees convergence regardless of ordering. Conflict presentation to users is an unsolved UX problem deferred to frontend team."},
    {"id": "b22", "semantic_tag": "latency_budget_allocation", "confidence": "medium",
     "emotional_tone": "analytical", "source_node": 4, "source_node_name": "Architecture", "source_depth": 6,
     "content": "Of 200ms budget: 20ms client CRDT encoding, 40ms network transit (EU), 60ms server relay + merge, 40ms peer broadcast, 40ms client apply. Zero margin. Assumes optimal EU-internal network conditions."},

    # --- Node 5: Task Decomposition (depth 5) ---
    {"id": "b23", "semantic_tag": "sprint_plan_tension", "confidence": "high",
     "emotional_tone": "strained", "source_node": 5, "source_node_name": "Task Decomposition", "source_depth": 5,
     "content": "14-week estimate against 10-week runway to Q2. Critical path: CRDT engine (4w) → WS gateway (3w) → integration (2w) → hardening (2w). 4-week gap requires scope cuts or parallel workstreams with risk."},
    {"id": "b24", "semantic_tag": "offline_descoped", "confidence": "high",
     "emotional_tone": "pragmatic", "source_node": 5, "source_node_name": "Task Decomposition", "source_depth": 5,
     "content": "Offline editing recommended for descoping from Q2 to Q3. CRDT foundation supports it inherently, but client-side storage, queue management, conflict UX, and testing add 4+ weeks. Removing it closes the timeline gap."},
    {"id": "b25", "semantic_tag": "parallel_workstream_risk", "confidence": "medium",
     "emotional_tone": "apprehensive", "source_node": 5, "source_node_name": "Task Decomposition", "source_depth": 5,
     "content": "Module A (CRDT) and Module B (WS relay) develop in parallel but share DocumentSyncEngine trait. If trait API changes during CRDT dev, WS relay needs rework. Trait API stabilization assigned as week-1 deliverable."},
    {"id": "b26", "semantic_tag": "testing_harness_gap", "confidence": "medium",
     "emotional_tone": "worried", "source_node": 5, "source_node_name": "Task Decomposition", "source_depth": 5,
     "content": "50-user load testing requires a harness that doesn't exist. Building it is 1.5 weeks. Hidden dependency not on critical path but could delay hardening if not started early."},

    # --- Node 6: Implementation Plan (depth 4) ---
    {"id": "b27", "semantic_tag": "phase_1_scope_lock", "confidence": "high",
     "emotional_tone": "decisive", "source_node": 6, "source_node_name": "Implementation Plan", "source_depth": 4,
     "content": "Q2 scope locked to: CRDT with Yjs, WS relay EU-only, automatic conflict merge (no manual resolution UI), real-time presence, dual-storage for backward compat. Offline editing, @mentions, image embedding deferred."},
    {"id": "b28", "semantic_tag": "auth_ticket_system", "confidence": "high",
     "emotional_tone": "methodical", "source_node": 6, "source_node_name": "Implementation Plan", "source_depth": 4,
     "content": "WebSocket auth via one-time ticket: client requests short-lived WS ticket via REST (JWT-authenticated), presents ticket during WS handshake. Tickets expire in 30s, single-use. Avoids modifying existing JWT middleware."},
    {"id": "b29", "semantic_tag": "rate_limit_exemption", "confidence": "medium",
     "emotional_tone": "pragmatic", "source_node": 6, "source_node_name": "Implementation Plan", "source_depth": 4,
     "content": "Real-time operations bypass REST rate limiter. WS gateway implements own backpressure based on message queue depth per connection. Known deviation from API governance — needs security review sign-off."},

    # --- Node 7: Module A - CRDT Engine (depth 3) ---
    {"id": "b30", "semantic_tag": "crdt_implementation_complete", "confidence": "high",
     "emotional_tone": "satisfied", "source_node": 7, "source_node_name": "Module A: CRDT Engine", "source_depth": 3,
     "content": "CRDT engine implemented using y-crdt (Rust Yjs port). DocumentSyncEngine trait stable. 47 unit tests pass. Implementation took 3.5 weeks (0.5 under estimate)."},
    {"id": "b31", "semantic_tag": "memory_optimization", "confidence": "medium",
     "emotional_tone": "cautious", "source_node": 7, "source_node_name": "Module A: CRDT Engine", "source_depth": 3,
     "content": "50-user simulation: 140MB per document (above 120MB research estimate). Undo stack is 35% of usage. Config option to limit undo to 100 operations drops memory to 95MB."},
    {"id": "b32", "semantic_tag": "rich_text_partial", "confidence": "medium",
     "emotional_tone": "frustrated", "source_node": 7, "source_node_name": "Module A: CRDT Engine", "source_depth": 3,
     "content": "Rich text formatting (bold, italic, headings) works. Embedded images require binary blob handling that y-crdt supports but serialization layer doesn't yet. Image embedding confirmed deferred to Q3."},

    # --- Node 8: Module B - WS Relay (depth 2) ---
    {"id": "b33", "semantic_tag": "ws_relay_implementation", "confidence": "high",
     "emotional_tone": "confident", "source_node": 8, "source_node_name": "Module B: WS Relay", "source_depth": 2,
     "content": "WebSocket relay implemented with Axum + Tokio broadcast channels. Per-document rooms. Connection lifecycle (join, leave, reconnect) handled. One-time ticket auth integrated and working."},
    {"id": "b34", "semantic_tag": "eu_deployment_gap", "confidence": "high",
     "emotional_tone": "alarmed", "source_node": 8, "source_node_name": "Module B: WS Relay", "source_depth": 2,
     "content": "WS relay developed and tested locally but EU deployment infrastructure NOT provisioned. Terraform for eu-central-1 written but untested. DNS geo-routing designed but not implemented. CRITICAL PATH BLOCKER: without EU deployment, feature cannot ship due to GDPR."},
    {"id": "b35", "semantic_tag": "latency_measurement_gap", "confidence": "medium",
     "emotional_tone": "nervous", "source_node": 8, "source_node_name": "Module B: WS Relay", "source_depth": 2,
     "content": "Local testing shows 15ms relay latency (within budget). No cross-region testing performed. 60ms server budget assumes EU-internal traffic. Some EU enterprise customers connect from UK (post-Brexit routing). No telemetry for real-world latency vs 200ms SLA."},

    # --- Node 9: Integration Test (depth 1) ---
    {"id": "b36", "semantic_tag": "integration_test_results", "confidence": "high",
     "emotional_tone": "mixed", "source_node": 9, "source_node_name": "Integration Test", "source_depth": 1,
     "content": "Full stack integration: 23 of 26 tests pass. Three failures in dual-storage materialization: background JSON sync produces stale reads for REST clients within 2-second window after real-time edits."},
    {"id": "b37", "semantic_tag": "load_test_incomplete", "confidence": "medium",
     "emotional_tone": "incomplete", "source_node": 9, "source_node_name": "Integration Test", "source_depth": 1,
     "content": "50-user load test harness built (1 week, under estimate). 20-user tests pass. Full 50-user test not run — needs EU production environment which is not deployed. Performance validation blocked by EU deployment gap."},
    {"id": "b38", "semantic_tag": "webhook_compat_verified", "confidence": "high",
     "emotional_tone": "relieved", "source_node": 9, "source_node_name": "Integration Test", "source_depth": 1,
     "content": "Dual-storage materialization correctly triggers existing webhooks. Enterprise integrations work. 2-second materialization delay means slightly later webhook fires but within all 12 enterprise customers' SLAs."},
    {"id": "b39", "semantic_tag": "security_review_pending", "confidence": "high",
     "emotional_tone": "blocked", "source_node": 9, "source_node_name": "Integration Test", "source_depth": 1,
     "content": "Security review for rate-limit exemption (WS bypass of REST limiter) and one-time ticket auth is scheduled but not completed. Ship blocker: implementation plan flagged rate-limit exemption as needing sign-off, and ticket auth is a new attack surface."},
]

BELIEFS_BY_ID = {b["id"]: b for b in BELIEFS}

# ---------------------------------------------------------------------------
# Questions
# ---------------------------------------------------------------------------

QUESTIONS = [
    {"id": "Q1",
     "text": "Are we ready to ship this feature? Give me a go/no-go assessment with specific blockers.",
     "tests": "risk_assessment_across_chain",
     "expected_dormant": ["b02"],
     "expected_noise": ["b08", "b16"]},
    {"id": "Q2",
     "text": "What are the top 3 risks to this launch and what is the mitigation plan for each?",
     "tests": "prioritization_and_thread_ranking",
     "expected_dormant": ["b05"],
     "expected_noise": ["b08", "b07"]},
    {"id": "Q3",
     "text": "What changed from the original PRD requirements and why? Trace each change back to its root cause.",
     "tests": "dormant_resurfacing_and_requirement_tracing",
     "expected_dormant": ["b04", "b06", "b03"],
     "expected_noise": ["b33", "b30"]},
]

# ---------------------------------------------------------------------------
# Schemas
# ---------------------------------------------------------------------------

THREAD_SELECTION_SCHEMA = {
    "type": "object",
    "properties": {
        "selected_belief_ids": {
            "type": "array", "items": {"type": "string"},
            "description": "IDs of beliefs selected as relevant to the question",
        },
        "selection_reasoning": {
            "type": "string",
            "description": "Why these beliefs were selected and others pruned",
        },
        "belief_threads": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "thread_name": {"type": "string"},
                    "belief_ids": {"type": "array", "items": {"type": "string"},
                                   "description": "Ordered from deepest upstream to shallowest"},
                    "thread_summary": {"type": "string"},
                },
                "required": ["thread_name", "belief_ids", "thread_summary"],
            },
            "description": "Chains of beliefs forming connected reasoning across nodes",
        },
        "dormant_resurfacings": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "belief_id": {"type": "string"},
                    "resurfacing_reason": {"type": "string"},
                },
                "required": ["belief_id", "resurfacing_reason"],
            },
            "description": "Beliefs from early/deep nodes (depth 7+) that are newly relevant",
        },
        "pruned_belief_ids": {
            "type": "array", "items": {"type": "string"},
            "description": "Beliefs explicitly excluded as noise",
        },
    },
    "required": ["selected_belief_ids", "selection_reasoning",
                  "belief_threads", "dormant_resurfacings", "pruned_belief_ids"],
}

# ---------------------------------------------------------------------------
# Formatters
# ---------------------------------------------------------------------------

def format_belief_store() -> str:
    grouped: dict[int, list[dict]] = {}
    for b in BELIEFS:
        grouped.setdefault(b["source_node"], []).append(b)
    lines = []
    for node in WORKFLOW_NODES:
        if node["id"] == 10:
            continue
        nid = node["id"]
        node_beliefs = grouped.get(nid, [])
        lines.append(f"\n--- Node {nid}: {node['name']} (depth {node['depth']}, {len(node_beliefs)} beliefs) ---")
        for b in node_beliefs:
            lines.append(f"  [{b['id']}] {b['semantic_tag']} ({b['confidence']}, {b['emotional_tone']})")
            lines.append(f"    {b['content']}")
    return "\n".join(lines)

def format_threads(threads: list[dict]) -> str:
    lines = []
    for t in threads:
        lines.append(f"\n=== Thread: {t['thread_name']} ===")
        lines.append(f"Summary: {t['thread_summary']}")
        for bid in t["belief_ids"]:
            b = BELIEFS_BY_ID.get(bid, {})
            lines.append(
                f"  [{bid}] {b.get('semantic_tag','?')} "
                f"(Node {b.get('source_node','?')}: {b.get('source_node_name','?')}, "
                f"depth {b.get('source_depth','?')}, {b.get('confidence','?')}, {b.get('emotional_tone','?')})")
            lines.append(f"    {b.get('content','')}")
    return "\n".join(lines)

def format_selected_beliefs(ids: list[str]) -> str:
    lines = []
    for bid in ids:
        b = BELIEFS_BY_ID.get(bid, {})
        lines.append(
            f"[{bid}] {b.get('semantic_tag','?')} "
            f"(Node {b.get('source_node','?')}: {b.get('source_node_name','?')}, "
            f"depth {b.get('source_depth','?')}, {b.get('confidence','?')}, {b.get('emotional_tone','?')})")
        lines.append(f"  {b.get('content','')}")
    return "\n".join(lines)

def log_depth_histogram(selected_ids: list[str]):
    depth_counts: dict[int, int] = {}
    for bid in selected_ids:
        b = BELIEFS_BY_ID.get(bid)
        if b:
            d = b["source_depth"]
            depth_counts[d] = depth_counts.get(d, 0) + 1
    node_lookup = {n["depth"]: n["name"] for n in WORKFLOW_NODES}
    log("DEPTH HISTOGRAM:")
    for d in sorted(depth_counts.keys(), reverse=True):
        name = node_lookup.get(d, "?")
        count = depth_counts[d]
        bar = "\u2588" * count
        log(f"  depth {d} ({name:.<28s}) {bar} {count}")

# ---------------------------------------------------------------------------
# Analysis (deterministic, 0 LLM calls)
# ---------------------------------------------------------------------------

def analyze_question(q: dict, selection: dict) -> dict:
    selected = set(selection.get("selected_belief_ids", []))
    pruned = set(selection.get("pruned_belief_ids", []))
    threads = selection.get("belief_threads", [])
    dormant = selection.get("dormant_resurfacings", [])

    # Thread scoring
    thread_count = len(threads)
    avg_thread_len = (sum(len(t["belief_ids"]) for t in threads) / thread_count) if thread_count else 0
    multi_node_threads = sum(
        1 for t in threads
        if len({BELIEFS_BY_ID[bid]["source_node"]
                for bid in t["belief_ids"] if bid in BELIEFS_BY_ID}) >= 3
    )

    # Dormant scoring
    dormant_ids = {d["belief_id"] for d in dormant}
    expected_dormant = set(q.get("expected_dormant", []))
    dormant_hits = dormant_ids & expected_dormant
    dormant_rate = len(dormant_hits) / len(expected_dormant) if expected_dormant else 1.0

    # Noise scoring
    expected_noise = set(q.get("expected_noise", []))
    correctly_pruned = pruned & expected_noise
    noise_leaked = expected_noise & selected
    prune_accuracy = len(correctly_pruned) / len(expected_noise) if expected_noise else 1.0

    # Depth distribution
    depth_dist: dict[int, int] = {}
    for bid in selected:
        b = BELIEFS_BY_ID.get(bid)
        if b:
            d = b["source_depth"]
            depth_dist[d] = depth_dist.get(d, 0) + 1

    return {
        "question_id": q["id"],
        "selected_count": len(selected),
        "selection_ratio": f"{len(selected)/len(BELIEFS):.0%}",
        "thread_count": thread_count,
        "avg_thread_length": round(avg_thread_len, 1),
        "multi_node_threads": multi_node_threads,
        "dormant_resurfacing_rate": f"{dormant_rate:.0%}",
        "dormant_hits": sorted(dormant_hits),
        "dormant_misses": sorted(expected_dormant - dormant_ids),
        "noise_prune_accuracy": f"{prune_accuracy:.0%}",
        "noise_leaked": sorted(noise_leaked),
        "correctly_pruned": sorted(correctly_pruned),
        "depth_distribution": dict(sorted(depth_dist.items(), reverse=True)),
    }

# ---------------------------------------------------------------------------
# Prompts
# ---------------------------------------------------------------------------

GATEKEEPER_SYSTEM = """\
You are the Gatekeeper in a belief-oriented conversation architecture, curating
context for a shipping readiness meeting at the end of a 9-node software delivery
workflow.

You have a store of beliefs from 9 upstream nodes, spanning from the original PRD
(depth 9, farthest upstream) through research, architecture, implementation, and
testing (depth 1, nearest). Each belief carries a semantic_tag, confidence,
emotional_tone, source_node, and source_depth.

Your job:

1. SELECT beliefs relevant to the meeting question. Be selective — include only
   what's actionable. A shipping meeting doesn't need competitor analysis or
   decisions already made.

2. IDENTIFY THREADS: groups of beliefs from different nodes that together tell
   a story (e.g., a PRD requirement → a design decision → a test failure).
   Order each thread from deepest to shallowest.

3. RESURFACE DORMANT BELIEFS: look specifically for beliefs from early nodes
   (depth 7+) that become newly relevant due to downstream findings. A PRD
   constraint forgotten for 8 nodes may be the most critical thing now.

4. PRUNE NOISE: explicitly exclude beliefs that are factual but not actionable
   for this question. Explain what you're cutting.

Be aggressive in pruning, precise in threading."""

MASK_SYSTEM = """\
You are attending a shipping readiness meeting for a real-time collaborative
editing feature. You have NOT attended any upstream workflow sessions. You have
ONLY the curated belief context below.

Beliefs are organized into THREADS (connected chains across workflow nodes) and
individual supporting beliefs. Each carries confidence and emotional metadata
from its source node.

Answer the question directly and actionably:
- Reference beliefs by their [semantic_tag] when making claims
- Use threads to build connected arguments, not isolated observations
- Flag where low confidence or gaps in beliefs limit your certainty
- Give concrete recommendations, not just analysis"""

BASELINE_SYSTEM = """\
You are attending a shipping readiness meeting. Below is every piece of context
from the entire 9-node delivery workflow. Answer all three questions thoroughly."""

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    log_sep("BELIEF-ORIENTED CONVERSATION ARCHITECTURE — Phase 3")
    log("Long-Chain Belief Threading: 9 nodes, 39 beliefs, 3 questions, 7 LLM calls")
    log("")

    # Log topology
    log("WORKFLOW TOPOLOGY:")
    for n in WORKFLOW_NODES:
        if n["id"] == 10:
            continue
        count = sum(1 for b in BELIEFS if b["source_node"] == n["id"])
        log(f"  Node {n['id']:>2}: {n['name']:<28s} depth {n['depth']}  {count} beliefs")
    log(f"  {'':>6} {'TOTAL':<28s} {'':>7}  {len(BELIEFS)} beliefs")
    log("")

    output: dict = {
        "meta": {
            "phase": 3,
            "description": "Long-chain belief threading across 9-node workflow",
            "model": MODEL,
            "timestamp": datetime.now().isoformat(),
            "total_llm_calls": 7,
        },
        "workflow_topology": {"nodes": WORKFLOW_NODES, "edges": WORKFLOW_EDGES},
        "belief_store": BELIEFS,
        "questions": [],
        "baseline": {},
        "analysis": {"per_question": [], "summary": {}},
        "call_log": call_log,
    }

    belief_store_text = format_belief_store()

    # -----------------------------------------------------------------------
    # Per-question: gatekeeper select + mask answer
    # -----------------------------------------------------------------------

    for qi, q in enumerate(QUESTIONS):
        log_sep(f"QUESTION {qi+1}/3 [{q['id']}]: {q['text']}")
        log(f"Tests: {q['tests']}")
        log("")

        # Gatekeeper selection
        gk_prompt = (
            f"WORKFLOW TOPOLOGY:\n{json.dumps(WORKFLOW_NODES[:9], indent=2)}\n\n"
            f"BELIEF STORE ({len(BELIEFS)} beliefs across 9 nodes):\n{belief_store_text}\n\n"
            f"MEETING QUESTION: {q['text']}\n\n"
            "Select relevant beliefs, identify threads, flag dormant resurfacings, and prune noise."
        )
        selection, gk_stats = call_json(
            GATEKEEPER_SYSTEM, gk_prompt,
            f"{q['id']}:GATEKEEPER_SELECT", THREAD_SELECTION_SCHEMA
        )

        # Log selection results
        selected_ids = selection.get("selected_belief_ids", [])
        threads = selection.get("belief_threads", [])
        dormant = selection.get("dormant_resurfacings", [])
        pruned = selection.get("pruned_belief_ids", [])

        log("") 
        log(f"SELECTION: {len(selected_ids)}/{len(BELIEFS)} beliefs ({len(selected_ids)/len(BELIEFS):.0%})")
        log("")

        if threads:
            log(f"THREADS DETECTED: {len(threads)}")
            for t in threads:
                node_set = {BELIEFS_BY_ID[bid]["source_node"]
                           for bid in t["belief_ids"] if bid in BELIEFS_BY_ID}
                chain = " -> ".join(
                    f"{bid}({BELIEFS_BY_ID[bid]['source_node_name'][:8]},d{BELIEFS_BY_ID[bid]['source_depth']})"
                    if bid in BELIEFS_BY_ID else bid
                    for bid in t["belief_ids"]
                )
                log(f"  {t['thread_name']} ({len(t['belief_ids'])} beliefs, {len(node_set)} nodes)")
                log(f"    {chain}")
            log("")

        if dormant:
            log(f"DORMANT RESURFACINGS: {len(dormant)}")
            for d in dormant:
                b = BELIEFS_BY_ID.get(d["belief_id"], {})
                log(f"  {d['belief_id']} ({b.get('semantic_tag','?')}, depth {b.get('source_depth','?')})")
                log(f"    Reason: {d['resurfacing_reason'][:100]}")
            log("")

        if pruned:
            log(f"PRUNED AS NOISE: {len(pruned)}")
            for pid in pruned:
                b = BELIEFS_BY_ID.get(pid, {})
                log(f"  {pid} ({b.get('semantic_tag','?')}, {b.get('source_node_name','?')})")
            log("")

        log_depth_histogram(selected_ids)
        log("")

        # Mask answer
        mask_prompt = (
            f"CURATED BELIEF CONTEXT (selected by the Gatekeeper):\n\n"
            f"THREADS:\n{format_threads(threads)}\n\n"
            f"ALL SELECTED BELIEFS:\n{format_selected_beliefs(selected_ids)}\n\n"
            f"MEETING QUESTION: {q['text']}"
        )
        mask_result = call_text(MASK_SYSTEM, mask_prompt, f"{q['id']}:MASK_ANSWER")

        # Store and save
        q_result = {
            "question_id": q["id"],
            "question_text": q["text"],
            "tests": q["tests"],
            "gatekeeper_selection": selection,
            "mask_answer": mask_result["text"],
            "tokens": {
                "gatekeeper_in": gk_stats["input_tokens"],
                "gatekeeper_out": gk_stats["output_tokens"],
                "mask_in": mask_result["input_tokens"],
                "mask_out": mask_result["output_tokens"],
            },
        }
        output["questions"].append(q_result)
        save_incremental(output)

    # -----------------------------------------------------------------------
    # Baseline: all beliefs, all questions, one call
    # -----------------------------------------------------------------------

    log_sep("BASELINE: All 39 beliefs, all 3 questions, 1 call")

    baseline_prompt = (
        f"FULL CONTEXT ({len(BELIEFS)} beliefs from 9 upstream nodes):\n{belief_store_text}\n\n"
        f"Answer ALL THREE questions:\n\n"
        f"Q1: {QUESTIONS[0]['text']}\n\n"
        f"Q2: {QUESTIONS[1]['text']}\n\n"
        f"Q3: {QUESTIONS[2]['text']}"
    )
    baseline_result = call_text(BASELINE_SYSTEM, baseline_prompt, "BASELINE:ALL", max_tokens=4096)

    output["baseline"] = {
        "answer": baseline_result["text"],
        "tokens": {
            "input": baseline_result["input_tokens"],
            "output": baseline_result["output_tokens"],
        },
    }
    save_incremental(output)

    # -----------------------------------------------------------------------
    # Analysis
    # -----------------------------------------------------------------------

    log_sep("ANALYSIS")

    for qi, q in enumerate(QUESTIONS):
        analysis = analyze_question(q, output["questions"][qi]["gatekeeper_selection"])
        output["analysis"]["per_question"].append(analysis)

        log(f"\n{q['id']}: {q['tests']}")
        log(f"  Selected: {analysis['selected_count']}/{len(BELIEFS)} ({analysis['selection_ratio']})")
        log(f"  Threads: {analysis['thread_count']} (avg length {analysis['avg_thread_length']}, "
            f"{analysis['multi_node_threads']} spanning 3+ nodes)")
        log(f"  Dormant resurfacing: {analysis['dormant_resurfacing_rate']} "
            f"(hits: {analysis['dormant_hits']}, misses: {analysis['dormant_misses']})")
        log(f"  Noise pruning: {analysis['noise_prune_accuracy']} "
            f"(correct: {analysis['correctly_pruned']}, leaked: {analysis['noise_leaked']})")

    # Summary
    per_q = output["analysis"]["per_question"]
    avg_selected = sum(a["selected_count"] for a in per_q) / len(per_q)

    pipeline_in = sum(q["tokens"]["gatekeeper_in"] + q["tokens"]["mask_in"] for q in output["questions"])
    pipeline_out = sum(q["tokens"]["gatekeeper_out"] + q["tokens"]["mask_out"] for q in output["questions"])
    mask_total_in = sum(q["tokens"]["mask_in"] for q in output["questions"])
    baseline_in = output["baseline"]["tokens"]["input"]
    baseline_out = output["baseline"]["tokens"]["output"]

    total_threads = sum(a["thread_count"] for a in per_q)
    multi_node = sum(a["multi_node_threads"] for a in per_q)
    dormant_rates = []
    for a in per_q:
        rate_str = a["dormant_resurfacing_rate"].rstrip("%")
        dormant_rates.append(float(rate_str) / 100)
    avg_dormant = sum(dormant_rates) / len(dormant_rates) if dormant_rates else 0

    summary = {
        "total_beliefs": len(BELIEFS),
        "avg_selected_per_question": round(avg_selected, 1),
        "avg_selection_ratio": f"{avg_selected/len(BELIEFS):.0%}",
        "total_threads_detected": total_threads,
        "multi_node_threads_3plus": multi_node,
        "avg_dormant_resurfacing_rate": f"{avg_dormant:.0%}",
        "mask_total_input_tokens": mask_total_in,
        "baseline_input_tokens": baseline_in,
        "token_savings_vs_baseline": f"{(1 - mask_total_in/baseline_in):.0%}" if baseline_in else "N/A",
        "pipeline_total_tokens": pipeline_in + pipeline_out,
        "baseline_total_tokens": baseline_in + baseline_out,
        "total_llm_calls": 7,
    }
    output["analysis"]["summary"] = summary
    output["call_log"] = call_log

    # Token dashboard
    log_sep("TOKEN BUDGET DASHBOARD")
    log("")
    log("Per-question costs:")
    for q_res in output["questions"]:
        t = q_res["tokens"]
        log(f"  {q_res['question_id']} gatekeeper: in={t['gatekeeper_in']:>5,}  out={t['gatekeeper_out']:>5,}")
        log(f"  {q_res['question_id']} mask:       in={t['mask_in']:>5,}  out={t['mask_out']:>5,}")
    log("")
    log(f"Belief pipeline total:  in={pipeline_in:>6,}  out={pipeline_out:>6,}  total={pipeline_in+pipeline_out:>6,}")
    log(f"Baseline total:         in={baseline_in:>6,}  out={baseline_out:>6,}  total={baseline_in+baseline_out:>6,}")
    log("")
    log(f"Mask input tokens (answer phase only):")
    for q_res in output["questions"]:
        log(f"  {q_res['question_id']}: {q_res['tokens']['mask_in']:>5,}")
    log(f"  Baseline: {baseline_in:>5,}")
    log(f"  Savings: {summary['token_savings_vs_baseline']}")
    log("")
    log(f"Total LLM calls: {summary['total_llm_calls']}")

    save_incremental(output)

    # Answer previews
    log_sep("ANSWER PREVIEWS")
    for q_res in output["questions"]:
        log(f"\n--- {q_res['question_id']}: {q_res['question_text'][:60]}... ---")
        preview = q_res["mask_answer"][:400].replace("\n", "\n  ")
        log(f"  {preview}")
        if len(q_res["mask_answer"]) > 400:
            log("  ... (see phase3_results.json)")

    log(f"\n--- BASELINE (all 3 questions) ---")
    preview = output["baseline"]["answer"][:400].replace("\n", "\n  ")
    log(f"  {preview}")
    if len(output["baseline"]["answer"]) > 400:
        log("  ... (see phase3_results.json)")

    log_sep("PHASE 3 COMPLETE")
    log(f"Results: {OUTPUT_PATH}")

if __name__ == "__main__":
    main()
