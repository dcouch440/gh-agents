Protocol Layer — Issues & Gaps for Multi-Assignment Decomp

The Goal

The decomp protocol should allow the orchestrator LLM to assign multiple tasks to the same port/agent. For example, two tasks to "frontend" and one to "backend". At runtime, the "frontend" step should
automatically execute as a for_each over its assigned items.

---

Critical: Port Resolution Won't Work (json_path is positional)

expanders/decomp.rs:94 — json_path: format!("{}.content", i) resolves output by array index, not by port name. If the LLM outputs items in a different order than the ports were defined (which it will),
data routes to the wrong downstream step. With multi-assignment this completely breaks — there's no fixed index for "all frontend items".

Fix: Don't use positional json_path for decomp. Instead, leverage the existing label routing system (see below).

---

Critical: Bridge Protocols to Existing DAG Primitives

The protocol expansion generates its own types (StepDefinition, EdgeDefinition, OutputPortDefinition) but these don't map to the existing DAG executor primitives that already solve this problem. You
already have:

- for_each step type with for_each_label_field
- routing_mode="label" on steps
- StepRoutingRuleRow mapping label_value → agent_id

The decomp protocol should generate these existing primitives:

- The orchestrator step becomes a for_each with routing_mode="label", routing_field="port"
- Each port becomes a StepRoutingRuleRow mapping port_name → agent_id
- The DAG executor already knows how to iterate the array, match the label field, and route to the correct agent

This means no new resolution logic is needed. The "apply" layer just needs to create the right combination of existing DB rows (workflow_steps with for_each config + step_routing_rules + edges).

---

execution_mode Hardcoded to "single"

expanders/decomp.rs:67 — all downstream steps are created with execution_mode: "single". If using label routing, the orchestrator step itself needs to be the for_each, and downstream steps receive
individual items. But if you keep the current per-port-step architecture, those steps need execution_mode: "for_each" to handle multiple items per port.

---

Prompt Injection Gaps

Multi-assignment not communicated (prompt_gen.rs:34-41) — The example output only shows one item per port. The LLM needs to know it CAN assign multiple tasks to the same port. Something like: "You may
assign multiple tasks to the same port. Each will be executed independently."

Tools and descriptions missing — The prompt injection should include each agent's available tools and their descriptions so the orchestrator knows what each specialist is capable of. Right now it only
shows port name, agent name, and port description. The orchestrator can't make good routing decisions without knowing what tools each agent has access to.

---

Review Edges Point to Nonexistent Steps

expanders/review.rs:75-87 — Creates edges with target_port_name set to decision values ("approve", "reject", "revise") but steps is empty. The apply layer needs a mechanism for the user to configure which
existing step each decision routes to. This isn't defined yet — how does the user say "on approve, go to step X; on reject, go back to step Y"?

---

No Fan-In / Aggregation Protocol

Decomp fans out 1→N, but there's no protocol for collecting results back. In practice it's almost always decomp → specialist work → collect results. The DAG executor has ForEachAggregateEnvelope at
runtime, but there's no protocol that auto-wires an aggregation step after a decomp. Worth considering a "collect" or "aggregate" protocol type, or making it an option on decomp (config.auto_aggregate:
true).

---

Content Schema is Untyped

Both decomp and route schemas define content as bare "type": "object" with no properties. If a downstream agent expects specific input fields (e.g. task_description, files, context), the orchestrator LLM
is guessing at the shape. Could the downstream agent's input port schema feed back into the generated output schema so the orchestrator knows what structure to produce?

---

Port Name Validation

Port names are user-defined strings that the LLM must output verbatim in JSON. No validation that they're simple identifiers. A port named "front end stuff" or "C++/systems" will cause LLM parsing
failures. Enforce a slug pattern like [a-z][a-z0-9_]\* in validate().

---

Priority Order

1. Bridge to existing DAG primitives (label routing) — this is architectural, affects everything else
2. json_path / port resolution — currently routes data to wrong steps
3. Prompt injection (multi-assignment + tools/descriptions) — LLM can't do its job without this
4. Port name validation — easy win, prevents runtime failures
5. Review wiring mechanism — needed before review protocol is usable
6. Content schema typing — quality of life, improves LLM output accuracy
7. Fan-in protocol — can come later but will be needed for real pipelines
