#!/usr/bin/env python3
"""Generate the dispatch/runtime prompt SVGs for the README.

All prompt text is verbatim from workflow run 59456950
(workflow f76e002a "Example 3", 2026-08-25 20:16:56 -> 20:18:10).
"""

import textwrap

from _svg import (BG, CARD, CARD2, BORDER, BORDER_HI, FG, MUTED, DIM, TAG, GREEN,
                  BLUE, AMBER, PURPLE, MONO, SANS, CH, OUT,
                  Canvas, wrap, chars_for, block, xml_lines)


# ═══════════════════════════════════════════════════════════════════════════
# VERBATIM CONTENT  (workflow run 59456950)
# ═══════════════════════════════════════════════════════════════════════════

ROLE = """<role>
You are a system designer. You receive short, human-readable
descriptions of what a workflow step should accomplish — your job is
to figure out HOW: what files need to exist on disk, what expertise
produces each one, and how they connect. You design by writing
configuration files. When you call complete_system, the execution
engine reads your files and runs the agents you configured — in
containers with full shell access and web search.
</role>"""

RUNTIME_BLK = """<runtime>
All agents share a directory — one agent writes a file, the next
reads it. Agents execute in dependency order (topology.json). Same
level = parallel. Files and packages persist across agents.

Every agent gets run_command (full shell: python, node, curl, git,
etc.) plus web search. Do not tell agents HOW to use the shell —
they know. Tell them WHAT to produce.

The capabilities field is only for tools beyond the shell (API
integrations, database connectors). Most agents need none.
</runtime>"""

DISPATCH_INPUT = """Verify all research findings on multi-agent orchestration against multiple current sources.

<previous_step name="Multi-Agent Orchestration Research">
Current architectures, frameworks, and design patterns for building multi-agent orchestration systems, with source classification and recency flags.
</previous_step>

<previous_step name="Multi-Agent Orchestration Failures Research">
Structured research findings on production failures of multi-agent orchestration systems, including root causes, patterns, and detailed case studies from industry and academic sources.
</previous_step>

<previous_step name="Multi-Agent Orchestration Research">
Research findings on cost structures, latency profiles, and economic trade-offs of multi-agent orchestration systems.
</previous_step>"""

DISPATCH_OUTPUT = """{"summary":"One file: verified findings dataset. Single verifier
agent applies independent-corroboration methodology across
architectures, failures, and economic data from the three upstream
research steps.","sync":{"agents_created":["Verifier"],
"agents_removed":[],"agents_updated":[],"description_changed":true}}"""

PREAMBLE_FULL = """You are in a shared workspace. Files and installed packages from previous steps are available.
Save files with run_command — do not put file content in your response.
When saving non-code output files (reports, data, text), use specific descriptive names — never generic names like output.txt or result.json. If transforming an upstream file, save to a new name that reflects your contribution.
When previous steps mention files they saved, read those files before starting your work — do not assume their contents from the summary alone."""

AGENTS = [
    dict(
        step="research_architectures", agent="Researcher", ms="20:16:56.930", dur="29.1s",
        role="Multi-agent systems researcher. Focus on published architectures (hierarchical, peer-to-peer, supervisor-worker, graph-based, event-driven) and production frameworks (LangGraph, AutoGen, CrewAI, Semantic Kernel, LlamaIndex Workflows, Haystack, custom orchestration layers). Prioritize primary sources: framework documentation, GitHub repos, arXiv papers, conference talks from 2024-2026. Distinguish between research prototypes and production-ready systems. Record source type (official docs, academic paper, blog, benchmark study) and snapshot date for every claim.",
        assignment="Research current architectures and frameworks for multi-agent orchestration systems. Capture: (1) core architectural patterns with examples, (2) major frameworks with their orchestration model, strengths, limitations, and maturity level, (3) emerging trends (e.g., memory sharing, tool-use coordination, evaluation harnesses). Classify each data point as published (official source) or secondary. Flag any information older than 12 months. Save the structured research report.",
        expected="A saved research report organized by architecture patterns and frameworks. Each entry includes description, key references with URLs and dates, source classification, and recency flag. Downstream consumers need consistent structure for comparison or synthesis.",
        result="multi_agent_orchestration_research_report_aug2026.md",
        kind="file",
    ),
    dict(
        step="research_production_failures", agent="Researcher", ms="20:16:56.930", dur="28.1s",
        role="Distributed systems and AI reliability researcher. Focus on multi-agent orchestration: coordination protocols, state management, task decomposition, agent communication, failure detection/recovery. Common failure modes include race conditions in agent handoffs, inconsistent world models, cascading timeouts, resource contention, prompt injection propagation, observability blind spots. Prioritize primary sources: post-mortems, academic papers (arXiv, USENIX), industry reports (LangChain, AutoGen, CrewAI case studies), conference talks. Distinguish between lab/demo failures and production incidents. Record source URLs, dates, and confidence (anecdotal vs measured).",
        assignment="Research production failures of multi-agent orchestration systems. Identify recurring failure patterns and root causes. Collect and detail 4-6 key case studies from real deployments (include company/project names, what broke, impact, resolution if available). Structure findings by failure category with evidence. Use web search and page browsing to gather data. Save a comprehensive research report.",
        expected="A saved research report file containing: executive summary of top failure modes, categorized breakdown with root causes and evidence, 4-6 detailed case studies (context, incident, lessons), and a references section with URLs and dates. Downstream agents need structured, citable findings for analysis or recommendations.",
        result="returned 8,680 chars inline — no file written",
        kind="inline",
    ),
    dict(
        step="research_economics", agent="Researcher", ms="20:16:56.930", dur="25.8s",
        role="Multi-agent systems researcher specializing in orchestration economics. Draw from academic papers (arXiv, NeurIPS, ICML), industry benchmarks (LangChain, AutoGen, CrewAI reports), and cloud cost models (AWS/GCP/Azure agent hosting). Distinguish theoretical models from measured benchmarks. For costs: break down per-token LLM calls, coordination overhead, memory/state persistence, and retry logic. For latency: sequential vs parallel execution, message passing delays, tool invocation overhead, and scaling curves. For economics: identify break-even points, cost/latency trade-offs, and when single-agent or human-in-loop outperforms multi-agent. Flag assumptions in any model.",
        assignment="Research cost structures, latency profiles, and economics of multi-agent orchestration. Use web search and browse key sources for empirical data and models. Synthesize into structured findings covering: (1) cost components and scaling, (2) latency contributors and profiles, (3) economic trade-offs and decision frameworks. Include sources, data points, and caveats. Save the research report.",
        expected="A saved structured research report with sections on cost structures, latency profiles, and economics. Each section includes key data points, sources, models, and trade-offs. Downstream agents need: quantitative insights and references for analysis or decision-making.",
        result="returned 8,994 chars inline — no file written",
        kind="inline",
    ),
]

VERIFIER = dict(
    step="verify_research", agent="Verifier", dur="20.3s",
    role="Data verification specialist for AI systems research. Independent verification requires a corroborating source with its own primary observation — not re-publishing the same claim. Check: (1) official framework docs and changelogs for architecture claims, (2) production incident reports, postmortems, and academic papers for failure patterns, (3) benchmark studies and cost analyses for economic data. Flag contradictions between sources explicitly. Classify each data point as verified (multiple independent sources), partially verified, or unverified. Note source URLs and dates.",
    assignment="Read the three upstream research outputs on multi-agent orchestration architectures, failures, and economic trade-offs. For each key finding: attempt independent verification from sources not used in the original research. Annotate with verification status, corroborating source URLs, and notes on any contradictions or recency issues. Save the annotated dataset with confidence levels per finding.",
    expected="A saved verified findings file mirroring the structure of the upstream research but with each data point annotated: verification status, corroborating sources, contradictions, and confidence classification. Downstream agents need confidence-weighted findings for synthesis or decision-making.",
    result="verified_findings_multi_agent_orchestration_aug2026.md",
    kind="file",
)

BRIEF = [
    dict(
        step="write_executive_brief", agent="Analyst", order="agent_order 0", dur="10.6s",
        role="AI systems analyst specializing in multi-agent orchestration. Synthesize verified findings on architectures, failure modes, and economic trade-offs. Weight high-confidence data points more heavily. Identify patterns across sources, note contradictions explicitly, and map implications for different deployment scales. Distinguish data-backed observations from interpretive synthesis.",
        assignment="Read the verified research findings on multi-agent orchestration. Synthesize into: (1) overview of dominant architectures and their maturity, (2) categorized failure modes with frequency and impact where available, (3) economic trade-offs including cost, latency, and scalability metrics, (4) key trends and open questions. Tag each insight with confidence level from upstream verification. Save the structured synthesis.",
        expected="A saved synthesis file with sections on architectures, failures, trade-offs, and trends. Each claim annotated with confidence classification and source notes. Downstream agent needs: organized insights ready for executive narrative.",
        result="multi_agent_orchestration_synthesis_aug2026.md",
        kind="file",
    ),
    dict(
        step="write_executive_brief", agent="Writer", order="agent_order 1", dur="12.8s",
        role="Executive communications specialist for technology strategy. Lead with the most decision-relevant takeaways. Use tables or bullet summaries for data, short prose for context. Preserve confidence markers on claims. Keep the brief under 800 words, actionable for CTOs and engineering leaders. Distinguish verified facts from analyst judgment.",
        assignment="Read the structured synthesis of verified research. Produce an executive brief on the current state of multi-agent orchestration. Open with 3-5 key takeaways. Cover architectures, notable failures, economic considerations, and forward outlook. Include confidence indicators. Save the final executive brief.",
        expected="A saved executive brief document: key takeaways, architecture summary, failure analysis, economic trade-offs, outlook, with confidence annotations where applicable.",
        result="multi_agent_orchestration_executive_brief_aug2026.md",
        kind="file",
    ),
]



# ═══════════════════════════════════════════════════════════════════════════
# DIAGRAM 1 — DISPATCH
# ═══════════════════════════════════════════════════════════════════════════

def build_dispatch(path):
    W = 1240
    M = 28
    IW = W - 2 * M            # inner width
    c = Canvas(W)

    y = 40
    c.text(M, y, "Dispatch — design time", size=21, fill=FG, family=SANS, weight="700")
    y += 22
    c.text(M, y, "Runs once per step, before any execution. A designer agent turns one "
                 "sentence drawn on the board into agent configs on disk.",
           size=12.5, fill=MUTED, family=SANS)
    y += 16
    c.text(M, y, "Verbatim from run 59456950 · step verify_research · "
                 "agent_executions.execution_type = 'dispatch'",
           size=11, fill=DIM, family=MONO)
    y += 26

    # ── 1. board node ──────────────────────────────────────────────────────
    cw = 68
    lines = [(l, FG) for l in wrap(
        "Verify all research findings on multi-agent orchestration against "
        "multiple current sources.", cw)]
    y = block(c, M, y, IW, lines, size=13, lh=18,
              title="1 · WHAT YOU DREW  →  workflow_steps.description",
              title_fill=BLUE, fill=CARD, stroke=BORDER_HI)

    arrow_y = y
    c.path(f"M {M+60} {arrow_y} L {M+60} {arrow_y+30}")
    y += 42

    # ── 2. designer system prompt (two columns) ────────────────────────────
    hdr = y
    c.text(M, y + 14, "2 · DESIGNER SYSTEM PROMPT", size=10, fill=PURPLE,
           family=MONO, weight="bold")
    c.text(M + 230, y + 14, "15,834 chars · 348 lines", size=10, fill=DIM, family=MONO)
    y += 26

    colw = (IW - 20) // 2
    cw2 = int((colw - 22) / (11.5 * CH))
    la, lb = xml_lines(ROLE, cw2), xml_lines(RUNTIME_BLK, cw2)
    n = max(len(la), len(lb))
    la += [("", FG)] * (n - len(la))
    lb += [("", FG)] * (n - len(lb))
    ry = block(c, M, y, colw, la, title="verbatim", title_fill=GREEN)
    ry2 = block(c, M + colw + 20, y, colw, lb, title="verbatim", title_fill=GREEN)
    y = max(ry, ry2) + 12

    # collapsed boilerplate strip
    sw = (IW - 24) // 3
    for i, (tag, n, what) in enumerate([
            ("<schema>", "33 lines", "config file shapes the designer must emit"),
            ("<guide>", "108 lines", "how to size a roster, name files, scope agents"),
            ("<examples>", "183 lines", "four worked <turn> examples")]):
        bx = M + i * (sw + 12)
        c.rect(bx, y, sw, 46, fill="#10151b", stroke=BORDER, dash="4 3")
        c.text(bx + 11, y + 19, tag, size=11.5, fill=TAG, family=MONO)
        c.text(bx + 11 + len(tag) * 11.5 * CH + 8, y + 19, n, size=10, fill=DIM, family=MONO)
        c.text(bx + 11, y + 35, what, size=9.5, fill=DIM, family=SANS)
    y += 46
    c.text(M, y + 15, "static template — identical for every step, omitted here",
           size=10, fill=DIM, family=SANS)
    y += 26

    c.path(f"M {M+60} {y} L {M+60} {y+30}")
    y += 42

    # ── 3. user input ──────────────────────────────────────────────────────
    cw3 = int((IW - 22) / (11.5 * CH))
    y = block(c, M, y, IW, xml_lines(DISPATCH_INPUT, cw3),
              title="3 · DESIGNER USER INPUT   ·   780 chars   ·   "
                    "upstream step summaries fan in as <previous_step> blocks",
              title_fill=BLUE)
    c.path(f"M {M+60} {y} L {M+60} {y+30}")
    y += 42

    # ── 4. designer writes config files ────────────────────────────────────
    real_cmds = [
        "mkdir -p agents",
        "cat > config.json << 'EOF'\n"
        '{\n'
        '  "name": "Multi-Agent Orchestration Verification",\n'
        '  "description": "Cross-verified research findings on multi-agent orchestration '
        'architectures, failures, and economic trade-offs, with confidence classifications, '
        'source audit trails, and contradiction notes."\n'
        '}\n'
        "EOF",
        "cat > topology.json << 'EOF'\n"
        '{\n'
        '  "agents": {\n'
        '    "verifier": { "depends_on": [] }\n'
        '  }\n'
        '}\n'
        "EOF",
        "cat > agents/verifier.json << 'EOF'\n"
        '{\n'
        '  "name": "Verifier",\n'
        '  "system_prompt": "Data verification specialist for AI systems research. '
        'Independent verification requires a corroborating source with its own primary '
        'observation \u2014 not re-publishing the same claim. Check: (1) official framework docs '
        'and changelogs for architecture claims, (2) production incident reports, postmortems, '
        'and academic papers for failure patterns, (3) benchmark studies and cost analyses for '
        'economic data. Flag contradictions between sources explicitly. Classify each data point '
        'as verified (multiple independent sources), partially verified, or unverified. Note '
        'source URLs and dates.",\n'
        '  "assignment": "Read the three upstream research outputs on multi-agent orchestration '
        'architectures, failures, and economic trade-offs. For each key finding: attempt '
        'independent verification from sources not used in the original research. Annotate with '
        'verification status, corroborating source URLs, and notes on any contradictions or '
        'recency issues. Save the annotated dataset with confidence levels per finding.",\n'
        '  "expected_output": "A saved verified findings file mirroring the structure of the '
        'upstream research but with each data point annotated: verification status, corroborating '
        'sources, contradictions, and confidence classification. Downstream agents need '
        'confidence-weighted findings for synthesis or decision-making.",\n'
        '  "capabilities": []\n'
        '}\n'
        "EOF",
    ]
    ncmd = chars_for(IW, 11.5)
    tool_lines = []
    for cmd in real_cmds:
        if tool_lines:
            tool_lines.append(("", FG))
        for i, raw in enumerate(cmd.split("\n")):
            prefix = "$ " if i == 0 else "  "
            col = GREEN if i == 0 else FG
            indent = len(raw) - len(raw.lstrip())
            wrapped = textwrap.wrap(raw, ncmd - 4, subsequent_indent=" " * (indent + 4)) or [""]
            for j, w in enumerate(wrapped):
                tool_lines.append((prefix + w if j == 0 else "    " + w, col))
    tool_lines.append(("", FG))
    tool_lines.append(("$ complete_system", AMBER))

    y = block(c, M, y, IW, tool_lines,
              title="4 · THE DESIGNER DESIGNS BY WRITING FILES   ·   run_command tool calls",
              title_fill=AMBER)
    c.path(f"M {M+60} {y} L {M+60} {y+30}")
    y += 42

    # ── 5. output ──────────────────────────────────────────────────────────
    check_lines = [
        ('{ "verify": {', FG),
        ('    "agents_complete": true,        "config_accurate": true,', GREEN),
        ('    "topology_complete": true,      "prompts_not_trivial": true,', GREEN),
        ('    "assignments_expanded": true,   "no_filenames_prescribed": true },', GREEN),
        ('  "summary": "One file: verified findings dataset. Single verifier agent applies', FG),
        ('     independent-corroboration methodology across architectures, failures, and', FG),
        ('     economic data from the three upstream research steps." }', FG),
    ]
    y = block(c, M, y, IW, check_lines,
              title="5 · complete_system   ·   the designer self-attests before handing off",
              title_fill=AMBER)
    y += 12
    y = block(c, M, y, IW, [(l, FG) for l in DISPATCH_OUTPUT.split("\n")],
              title="6 · PERSISTED   ·   314 chars   ·   agent_executions.output",
              title_fill=BLUE)

    y += 22
    c.text(M, y, "The execution engine now reads agents/*.json + topology.json and runs what the "
                 "designer configured. See the runtime diagram for what those agents received.",
           size=12, fill=MUTED, family=SANS)
    y += 20

    open(path, "w").write(c.render(y + 24))
    print(f"wrote {path}  ({W}x{y+24})")


build_dispatch(OUT / "dispatch-prompts.svg")


# ═══════════════════════════════════════════════════════════════════════════
# DIAGRAM 2 — RUNTIME
# ═══════════════════════════════════════════════════════════════════════════

def agent_card(c, x, y, w, spec, size=10.8, lh=14.2, show_prev=None):
    """One agent: header, designer-written role, <assignment>/<expected_output>, result."""
    n = chars_for(w, size)
    # header
    c.rect(x, y, w, 40, fill="#1b2129", stroke=BORDER_HI)
    c.text(x + 11, y + 17, spec["agent"], size=12.5, fill=FG, family=SANS, weight="700")
    meta = spec.get("order") or spec.get("ms", "")
    if meta:
        c.text(x + w - 11, y + 17, meta, size=9.5, fill=DIM, family=MONO, anchor="end")
    c.text(x + 11, y + 32, spec["step"], size=9.5, fill=BLUE, family=MONO)
    c.text(x + w - 11, y + 32, spec["dur"], size=9.5, fill=DIM, family=MONO, anchor="end")
    y += 40

    if show_prev:
        c.rect(x, y, w, 34, fill="#1a1712", stroke="#4a3c1a")
        c.text(x + 11, y + 15, show_prev[0], size=10, fill=AMBER, family=MONO)
        c.text(x + 11, y + 28, show_prev[1], size=9.5, fill=MUTED, family=SANS)
        y += 38

    y = block(c, x, y, w, [(l, FG) for l in wrap(spec["role"], n)],
              size=size, lh=lh,
              title="SYSTEM PROMPT · designer-written role",
              title_fill=PURPLE, title_size=9)
    y += 8

    inp = f'<assignment>\n{spec["assignment"]}\n</assignment>\n\n' \
          f'<expected_output>\n{spec["expected"]}\n</expected_output>'
    y = block(c, x, y, w, xml_lines(inp, n), size=size, lh=lh,
              title="USER INPUT", title_fill=BLUE, title_size=9)
    y += 8

    # result chip
    isfile = spec["kind"] == "file"
    col = GREEN if isfile else AMBER
    bg  = "#111a13" if isfile else "#1a1712"
    br  = "#1f4d2b" if isfile else "#4a3c1a"
    rl = wrap(spec["result"], chars_for(w, 10) - 3)
    h = 12 + len(rl) * 13 + 8
    c.rect(x, y, w, h, fill=bg, stroke=br)
    ty = y + 18
    for i, l in enumerate(rl):
        c.text(x + 11, ty, ("→ " if i == 0 else "  ") + l, size=10, fill=col, family=MONO)
        ty += 13
    return y + h


def build_runtime(path):
    W = 1240
    M = 28
    IW = W - 2 * M
    c = Canvas(W)

    y = 40
    c.text(M, y, "Runtime — execution", size=21, fill=FG, family=SANS, weight="700")
    y += 22
    c.text(M, y, "The same run, executing. Five steps, six agents, 74 seconds. Every prompt "
                 "below was written by the designer, not by a human.",
           size=12.5, fill=MUTED, family=SANS)
    y += 16
    c.text(M, y, "Verbatim from run 59456950 · agent_executions.execution_type = "
                 "'pipeline_agent'", size=11, fill=DIM, family=MONO)
    y += 28

    # ── shared preamble band ───────────────────────────────────────────────
    c.rect(M, y, IW, 52, fill="#181e26", stroke=BORDER_HI, dash="5 3")
    c.text(M + 14, y + 20, "SHARED PREAMBLE", size=10, fill=GREEN, family=MONO, weight="bold")
    c.text(M + 150, y + 20, "appended to all six system prompts — byte-identical (md5 eaa2b894…)",
           size=10, fill=DIM, family=MONO)
    c.text(M + 14, y + 39,
           "shared workspace  ·  save files with run_command, never inline  ·  "
           "descriptive filenames  ·  read upstream files before starting",
           size=11, fill=MUTED, family=MONO)
    y += 52

    # bus from preamble down to the three columns
    colw = (IW - 2 * 16) // 3
    xs = [M + i * (colw + 16) for i in range(3)]
    bus = y + 22
    c.path(f"M {M+60} {y} L {M+60} {bus}", marker=False, dash="5 3")
    c.path(f"M {xs[0]+colw//2} {bus} L {xs[2]+colw//2} {bus}", marker=False, dash="5 3")
    for x in xs:
        c.path(f"M {x+colw//2} {bus} L {x+colw//2} {bus+22}", dash="5 3")
    y = bus + 30

    c.text(M, y, "PARALLEL — all three dispatched in the same millisecond, 20:16:56.930",
           size=10.5, fill=GREEN, family=MONO, weight="bold")
    y += 14

    bottoms = [agent_card(c, xs[i], y, colw, AGENTS[i]) for i in range(3)]
    ymax = max(bottoms)

    # ── fan-in ─────────────────────────────────────────────────────────────
    conv = ymax + 34
    for i, x in enumerate(xs):
        cx = x + colw // 2
        c.path(f"M {cx} {bottoms[i]} L {cx} {conv}", marker=False)
    c.path(f"M {xs[0]+colw//2} {conv} L {xs[2]+colw//2} {conv}", marker=False)
    mid = W // 2
    c.path(f"M {mid} {conv} L {mid} {conv+30}")
    c.text(mid + 14, conv + 22, "9,021 chars fan in as <previous_step> blocks",
           size=10, fill=DIM, family=MONO)
    y = conv + 42

    # ── verifier ───────────────────────────────────────────────────────────
    vw = int(IW * 0.62)
    vx = (W - vw) // 2
    vb = agent_card(c, vx, y, vw, VERIFIER,
                    show_prev=("<previous_step> × 3",
                               "the three reports above, concatenated"))

    c.path(f"M {mid} {vb} L {mid} {vb+34}")
    y = vb + 46

    # ── brief step: two sequential agents ──────────────────────────────────
    c.text(M, y, "SEQUENTIAL — one step, two agents, agent_order 0 → 1",
           size=10.5, fill=PURPLE, family=MONO, weight="bold")
    y += 14
    bw = (IW - 90) // 2
    b1 = agent_card(c, M, y, bw, BRIEF[0])
    b2 = agent_card(c, M + bw + 90, y, bw, BRIEF[1])
    # horizontal arrow between them, bending around the gap
    ay = y + 100
    c.path(f"M {M+bw} {ay} L {M+bw+90} {ay}")
    c.text(M + bw + 45, ay - 10, "file", size=9.5, fill=GREEN, family=MONO, anchor="middle")
    y = max(b1, b2) + 24

    c.text(M, y, "Two of the three researchers ignored the preamble and returned their "
                 "reports inline instead of writing files — the shared-workspace contract "
                 "held for four of six agents.",
           size=11.5, fill=AMBER, family=SANS)
    y += 18

    open(path, "w").write(c.render(y + 20))
    print(f"wrote {path}  ({W}x{y+20})")


build_runtime(OUT / "runtime-prompts.svg")
