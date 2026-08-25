#!/usr/bin/env python3
"""Generate the architecture overview SVG for the README.

Every claim is traced to source. Key references:
  dispatch arms      src/server/hub/dag/orchestration/dispatch.rs:31-43
  strategies         src/server/hub/execution/strategies/
  L1-L4 levels       src/server/hub/board/state/types.rs:19-23
                     src/config/protocols.rs:222-236
  L4 writes configs  src/server/executors/dispatch/system_node.rs:137
  board ingest       src/server/hub/board/serializer/filter.rs:36,180
  transport          src/server/mod.rs:207 (WS) / src/server/api/chat/mod.rs:172 (SSE)
  workspace gate     src/server/state/mod.rs:200-205  workspace().is_some()
  provider           src/constants.rs:24  ACTIVE_PROVIDER = "xai"
  protocol registry  src/server/hub/protocols/compilers/mod.rs  (empty)
"""

from _svg import (BORDER, BORDER_HI, FG, MUTED, DIM, GREEN, BLUE, AMBER,
                  PURPLE, MONO, SANS, OUT, Canvas)

W, M = 1300, 28
IW = W - 2 * M


def panel(c, x, y, w, title, subtitle, rows, accent=BLUE, fill="#161b22",
          stroke=BORDER, dash=None, pad=13, note=None):
    h = pad * 2 + 17 + (13 if subtitle else 0) + len(rows) * 15 + (15 if note else 0)
    c.rect(x, y, w, h, fill=fill, stroke=stroke, dash=dash)
    ty = y + pad + 11
    c.text(x + pad, ty, title, size=10.5, fill=accent, family=MONO, weight="bold")
    ty += 15
    if subtitle:
        c.text(x + pad, ty, subtitle, size=9.5, fill=DIM, family=SANS)
        ty += 13
    for r, col in rows:
        c.text(x + pad, ty, r, size=11, fill=col, family=MONO)
        ty += 15
    if note:
        c.text(x + pad, ty + 2, note, size=9.5, fill=DIM, family=SANS)
    return y + h


def level(c, x, y, w, tag, name, strategy, mode, bullets, accent):
    h = 28 + len(bullets) * 14 + 14
    c.rect(x, y, w, h, fill="#12171e", stroke=BORDER)
    c.add(f'<rect x="{x}" y="{y}" width="4" height="{h}" fill="{accent}"/>')
    c.text(x + 16, y + 19, tag, size=11, fill=accent, family=MONO, weight="bold")
    c.text(x + 48, y + 19, name, size=11.5, fill=FG, family=SANS, weight="700")
    c.text(x + w - 14, y + 19, strategy, size=10, fill=PURPLE, family=MONO, anchor="end")
    if mode:
        c.text(x + 232, y + 19, mode, size=9.5, fill=DIM, family=MONO)
    ty = y + 38
    for b in bullets:
        c.text(x + 48, ty, b, size=10, fill=MUTED, family=MONO)
        ty += 14
    return y + h


def build(path):
    c = Canvas(W)
    y = 40
    c.text(M, y, "Architecture", size=21, fill=FG, family=SANS, weight="700")
    y += 22
    c.text(M, y, "Two planes. A design plane of chat and background builders writes agent "
                 "config files to disk; a run plane reads them back.",
           size=12.5, fill=MUTED, family=SANS)
    y += 30

    y = panel(c, M, y, IW, "BROWSER", None, [
        ("Canvas  .  Sidebar  .  Activity panel  .  Chat", FG),
    ], accent=FG, fill="#1b2129", stroke=BORDER_HI)

    lx, rx = M + IW * 0.22, M + IW * 0.70
    c.path(f"M {lx} {y} L {lx} {y+50}")
    c.path(f"M {rx} {y} L {rx} {y+50}")
    c.text(lx + 12, y + 21, "WebSocket  /ws", size=10.5, fill=GREEN, family=MONO)
    c.text(lx + 12, y + 35, "all run / step / board events, run-scoped", size=9.5,
           fill=DIM, family=SANS)
    c.text(rx + 12, y + 21, "SSE  api/chat", size=10.5, fill=GREEN, family=MONO)
    c.text(rx + 12, y + 35, "chat tokens only - disjoint bus", size=9.5, fill=DIM, family=SANS)
    y += 60

    # ── design plane ───────────────────────────────────────────────────────
    dp_top = y
    c.text(M + 16, y + 22, "DESIGN PLANE", size=11, fill=PURPLE, family=MONO, weight="bold")
    c.text(M + 150, y + 22, "one pattern at two scales - the agent edits files, the files "
                            "sync back to the DB", size=10, fill=DIM, family=SANS)
    y += 36
    dx, dw = M + 16, IW - 32

    y = level(c, dx, y, dw, "1", "Workflow Agent", "WorkflowAgentStrategy",
              "scope: the whole board",
              ["the chat panel of the workflow editor - this is where you type",
               "DB -> repo  ->  agent edits files  ->  repo -> DB",
               "writes  board/topology.json   board/nodes/<slug>.md"], GREEN)
    c.path(f"M {dx+90} {y} L {dx+90} {y+30}")
    c.text(dx + 102, y + 20, "one dispatch per node - on board submit, or the Generate button",
           size=9.5, fill=DIM, family=MONO)
    y += 34

    y = level(c, dx, y, dw, "2", "Node Builder", "SystemNodeStrategy",
              "scope: one node",
              ["background, in a container - recorded as execution_type = 'dispatch'",
               "same projection one level down: agents within a single node",
               "writes  system_node/<step-id>/{config,topology}.json   agents/*.json"], GREEN)
    y += 14

    c.rect(dx, y, dw, 46, fill="#12171e", stroke=BORDER, dash="4 3")
    c.text(dx + 14, y + 19, "legacy, superseded by the Workflow Agent:",
           size=10, fill=DIM, family=SANS)
    c.text(dx + 14, y + 35, 'L1 Manager Assistant + L2 Manager Builder (ManagerDispatchStrategy), '
                            'execution_mode == "manager"  -  0 rows, 0 sessions',
           size=10, fill=DIM, family=MONO)
    y += 46
    c.parts.insert(0, f'<rect x="{M}" y="{dp_top}" width="{IW}" height="{y-dp_top}" '
                      f'rx="10" fill="#0f1319" stroke="{PURPLE}" stroke-width="1.2" '
                      f'opacity="0.85"/>')
    y += 16

    c.path(f"M {W//2} {y} L {W//2} {y+34}")
    c.text(W // 2 + 16, y + 22, "config files on the shared filesystem",
           size=10.5, fill=GREEN, family=MONO)
    y += 44

    y = panel(c, M, y, IW, "SHARED FILESYSTEM   .   JuiceFS at /mnt/jfs",
              "live when state.workspace().is_some() - not gated on container_enabled", [
        ("workflows/<workflow-id>/", GREEN),
        ("  board/topology.json   board/nodes/<slug>.md        <- workflow agent wrote", FG),
        ("  system_node/<step-id>/{config,topology}.json agents/*.json  <- node builder", FG),
        ("  runs/<run-id>/*.md                                      <- agents produced", FG),
    ], accent=GREEN, note="board element id = step id = system_node directory name")
    y += 14
    c.path(f"M {W//2} {y} L {W//2} {y+30}")
    y += 40

    # ── run plane ──────────────────────────────────────────────────────────
    rp_top = y
    c.text(M + 16, y + 22, "RUN PLANE", size=11, fill=BLUE, family=MONO, weight="bold")
    c.text(M + 130, y + 22, "POST /workflows/:id/run  ->  spawned, returns 202 immediately",
           size=10, fill=DIM, family=MONO)
    y += 36
    rx2, rw = M + 16, IW - 32

    y = panel(c, rx2, y, rw, "1 . DAG ORCHESTRATOR   -   across steps",
              "topological_sort_levels()  .  steps in one level run in a JoinSet", [
        ("guards first:  cancellation . pinned replay . dead-path . conditional edges", FG),
    ], accent=GREEN, fill="#12171e")
    y += 12
    c.path(f"M {W//2} {y-12} L {W//2} {y+16}")
    y += 24

    y = panel(c, rx2, y, rw, "2 . dispatch_step()   -   exactly three arms",
              "execution_mode decides ONLY the first arm; the rest routes on a column", [
        ('"context" | "input"           ->  passthrough, no LLM call', FG),
        ("child_workflow_id.is_some()   ->  WorkforceAgentStrategy   reads agents/*.json", GREEN),
        ("_  (requires agent_id)        ->  DagStepStrategy", GREEN),
        ("", FG),
        ('"workforce" / "single" / "manager" are never matched here -', AMBER),
        ("only two of the six strategies are reachable from a DAG step", AMBER),
    ], accent=GREEN, fill="#12171e")
    y += 12
    c.path(f"M {W//2} {y-12} L {W//2} {y+16}")
    c.text(W // 2 + 14, y + 6, "workforce arm only", size=9.5, fill=DIM, family=MONO)
    y += 24

    y = panel(c, rx2, y, rw, "3 . WORKFORCE PIPELINE   -   across agents inside one step",
              "compute_execution_levels()  .  the second topological sort", [
        ("agents declare depends_on  ->  levels  ->  same level runs concurrently", FG),
    ], accent=GREEN, fill="#12171e")
    y += 12
    c.path(f"M {W//2} {y-12} L {W//2} {y+16}")
    y += 24

    y = panel(c, rx2, y, rw, "4 . EXECUTION ENGINE   -   one loop for every LLM call",
              "ExecutionEngine::execute()  .  parameterized, never branched", [
        ("on_start  ->  LLM  ->  on_response  ->  tool dispatch  ->  on_output", GREEN),
        ("          |__ repeats until strategy.should_stop() __|", DIM),
        ("", FG),
        ("7 filters compose behaviour:  agent_guidance . few_shot . reasoning_trace", FG),
        ("schema_enhancement . schema_validation_retry . partial_json_recovery", FG),
        ("debate_verification", FG),
    ], accent=GREEN, fill="#12171e")
    y += 12

    y = panel(c, rx2, y, rw, "PARALLEL MERGE",
              "steps in one level write through OverlayFS; diffs merge before the next", [
        ("classify -> diff3 -> LLM conflict resolve -> verify -> persist", FG),
        ("verify: non-empty . plausible length . imports kept . still parses", GREEN),
    ], accent=AMBER, fill="#12171e")
    y += 14
    c.parts.insert(0, f'<rect x="{M}" y="{rp_top}" width="{IW}" height="{y-rp_top}" '
                      f'rx="10" fill="#0f1319" stroke="{BLUE}" stroke-width="1.2" '
                      f'opacity="0.85"/>')
    y += 20

    y = panel(c, M, y, IW, "BOARD INGEST   .   POST /workflows/:id/board/submit",
              "every canvas edit is filtered before it can cost an LLM call", [
        ("diff.rs    snapshot diff by element id  ->  changeset", FG),
        ("filter.rs  1 pan  2 whitespace  3 oscillation  4 reorder  5 scoring  6 topo sort", FG),
        ("structural edits (delete, rewire)  ->  written straight to the DB, always", FG),
        ("aggregate_score >= dispatch_threshold  ->  wake an agent; below it, dropped", GREEN),
    ], accent=AMBER)
    y += 22

    hw = (IW - 24) // 2
    a = panel(c, M, y, hw, "POSTGRESQL", None, [
        ("workflows . steps . edges . executions . traces", FG),
        ("protocol_executions   per-agent cost ledger", FG),
        ("protocols  0 rows - compiler registry is empty", DIM),
    ], accent=FG, fill="#1b2129", stroke=BORDER_HI)
    b = panel(c, M + hw + 24, y, hw, "LLM PROVIDER", None, [
        ('ACTIVE_PROVIDER = "xai"   grok-4-1-fast-reasoning', FG),
        ("anthropic . ollama . noop clients exist, unused", DIM),
        ("registry . retry . rate-limit middleware", FG),
    ], accent=FG, fill="#1b2129", stroke=BORDER_HI)
    y = max(a, b) + 24

    c.text(M, y, "Dormant, and drawn nowhere above: the protocol compiler (hub/protocols) "
                 "has an empty registry and no way to fill it - register() needs &mut on an "
                 "Arc-wrapped engine.", size=10.5, fill=DIM, family=SANS)
    y += 16

    open(path, "w").write(c.render(y + 18))
    print(f"wrote {path}  ({W}x{y+18})")


build(OUT / "architecture.svg")
