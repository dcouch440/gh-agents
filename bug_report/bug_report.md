# Bug Report: Workforce Execution — Designer, Web Search, and File Write Failures

**Date:** 2026-02-18
**Workflow:** AI Investment Research Team (`2fba9a3b-587f-4f4e-bf1e-dcda9168826a`)
**Execution:** `6cc43606-a3c1-46eb-b770-f4ac98625c3d`
**Status:** Completed (with issues)

---

## Bug 1: Designer Phase Returns Invalid JSON

**Severity:** High
**Phase:** `designer`
**Error:** `Agent Designer JSON does not match expected schema: missing field 'agents'`

The Designer LLM returned JSON that didn't include the required `agents` field. The workforce recovered by falling back to the pre-configured roster agents, but the designer phase is recorded as `failed`. This also happened on a prior run (`82525d94`) on a different workflow with the same error.

**Impact:** Designer output is lost. Any dynamic agent configuration or task assignment from the designer is skipped. The workforce runs with the static roster configuration only.

**To investigate:**
- Check the designer system prompt and expected JSON schema in `src/server/hub/dag/workforce/mod.rs`
- Look at the designer input builder in `src/server/hub/dag/designer_input/workforce.rs`
- Determine if the schema changed or the LLM is consistently failing to produce valid JSON

---

## Bug 2: Web Search Tool Not Functional for Workforce Agents

**Severity:** High
**Agents affected:** AI Market Researcher (`agent_0`), Financial Analyst (`agent_1`)
**Capability configured:** `web_search`

Both agents with `web_search` capability reported "persistent technical issues with the web research tool" and fell back to generating content from training data. The agents completed successfully but their outputs contain stale information rather than live web results.

**Agent output evidence:**
- AI Market Researcher: *"I'm encountering persistent technical issues with the web research tool."*
- Financial Analyst: *"I'm experiencing technical issues with the web research tool."*

**To investigate:**
- Check how workforce agent capabilities map to actual tools in `src/server/tools/workforce/mod.rs`
- Verify the Grok web search provider is configured and the API key is valid
- Check if `web_search` capability is properly wired to the tool registry for workforce agent executions
- Note: No `execution_messages` were stored for these agents (0 rows) — message persistence may also be broken for workforce agent phases

---

## Bug 3: File Write Tool Not Available to Investment Writer

**Severity:** Medium
**Agent affected:** Investment Writer (`agent_3`)
**Capability configured:** `file_write`

The Investment Writer agent reported "the file writing tool isn't available in this context" and output the full report as inline text. The deliverable document (`AI Investment Report`) was never created — `document_id` remains NULL on the `protocol_document_defs` row.

**To investigate:**
- Check how `file_write` capability maps to tools for workforce agents
- Verify the deliverable document creation flow — the writer should create/update the document via `add_deliverable` or similar tool
- Check if the workforce agent execution context provides document-writing tools

---

## Bug 4: No Execution Messages Stored

**Severity:** Medium

All 5 protocol executions (designer + 4 agents) have 0 associated `execution_messages` rows. The `agent_executions` table also has 0 rows for this workflow execution. This means there's no LLM conversation history available for debugging.

**To investigate:**
- Check if workforce agent phases store messages via `execution_messages` or a different mechanism
- The outputs are stored in `protocol_executions.output_content`, but the full conversation (tool calls, intermediate responses) is lost

---

## Execution Timeline

| Phase | Agent | Status | Duration | Tokens (in/out) | Issue |
|-------|-------|--------|----------|-----------------|-------|
| designer | — | **FAILED** | ~10 min | 0/0 | Invalid JSON schema |
| agent_0 | AI Market Researcher | complete | ~1 min | 17.5k/2.5k | Web search broken |
| agent_1 | Financial Analyst | complete | ~51s | 15k/2.4k | Web search broken |
| agent_2 | Strategic Analyst | complete | ~36s | 8k/1.6k | OK |
| agent_3 | Investment Writer | complete | ~55s | 13.5k/3.6k | File write unavailable, no doc created |

**Total cost:** ~$0.31
