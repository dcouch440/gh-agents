import { boardStore } from '@/stores/boardStore'
import { workflowStore } from '@/stores/workflowStore'
import { dispatchStore } from '@/stores/dispatchStore'
import { Collections } from '@/utils/collections'
import type { DispatchTraceEvent } from '@/stores/dispatchStore'

type ToolExport = {
  name: string
  input: Record<string, unknown>
  result: unknown
}

type PhaseDetail = {
  system_prompt: string
  user_message: string
  response: string
  tools: ToolExport[]
}

type PhaseExport = { builder: PhaseDetail; designer: PhaseDetail }

const EMPTY_PHASE: PhaseDetail = { system_prompt: '', user_message: '', response: '', tools: [] }

const structureTrace = (events: readonly DispatchTraceEvent[]): PhaseDetail => {
  let systemPrompt = ''
  let userMessage = ''
  const responseParts: string[] = []
  const tools: ToolExport[] = []
  const pendingTools = new Map<string, ToolExport>()

  for (const e of events) {
    switch (e.type) {
      case 'system_prompt':
        if (systemPrompt.length === 0) systemPrompt = e.content
        break
      case 'user_message':
        if (userMessage.length === 0) userMessage = e.content
        break
      case 'token':
        responseParts.push(e.content)
        break
      case 'tool_start': {
        const tool: ToolExport = { name: e.toolName, input: e.input, result: null }
        pendingTools.set(e.toolId, tool)
        tools.push(tool)
        break
      }
      case 'tool_end': {
        const pending = pendingTools.get(e.toolId)
        if (pending) pending.result = e.result
        break
      }
      case 'error':
      case 'phase_marker':
        break
    }
  }

  return {
    system_prompt: systemPrompt,
    user_message: userMessage,
    response: responseParts.join('').trim(),
    tools,
  }
}

const splitByPhase = (trace: readonly DispatchTraceEvent[]): PhaseExport => {
  // Try phase_marker first (WebSocket live data)
  const phaseIdx = trace.findIndex((e) => e.type === 'phase_marker')
  if (phaseIdx !== -1) {
    return {
      builder: structureTrace(trace.slice(0, phaseIdx)),
      designer: structureTrace(trace.slice(phaseIdx + 1)),
    }
  }

  // Fallback: split at the second system_prompt (designer's prompt).
  // REST-hydrated traces don't include phase_marker events.
  let systemPromptCount = 0
  const designerIdx = trace.findIndex((e) => {
    if (e.type === 'system_prompt') systemPromptCount++
    return systemPromptCount === 2
  })

  if (designerIdx === -1) {
    return { builder: structureTrace(trace), designer: { ...EMPTY_PHASE } }
  }

  return {
    builder: structureTrace(trace.slice(0, designerIdx)),
    designer: structureTrace(trace.slice(designerIdx)),
  }
}

const extractNodeName = (trace: readonly DispatchTraceEvent[]): string | null => {
  for (const e of trace) {
    if (e.type === 'tool_start' && e.toolName === 'set_node_name') {
      const name = e.input['name']
      if (typeof name === 'string') return name
    }
  }
  return null
}

const buildDispatchExport = (): Record<string, PhaseExport> => {
  const dispatches = boardStore.store.getState().lastResponse?.dispatches ?? []
  const steps = workflowStore.store.getState().steps
  const stepNameMap = Collections.toLookupMap(steps, (s) => s.id, (s) => s.name ?? null)
  const dispatchState = dispatchStore.store.getState()

  const result: Record<string, PhaseExport> = {}

  for (const d of dispatches) {
    const entry = dispatchState.byStep[d.step_id] ?? null
    if (entry === null) continue
    const name = extractNodeName(entry.trace)
      ?? stepNameMap.get(d.step_id)
      ?? d.step_id.slice(0, 8)
    result[name] = splitByPhase(entry.trace)
  }

  return result
}

// ── Run export ──────────────────────────────────────────────────────────────

import { agentTraceStore } from '@/stores/agentTraceStore'
import type { AgentTrace } from '@/stores/agentTraceStore'

type AgentExport = {
  agent: string
  system_prompt: string
  messages: string[]
  tools: ToolExport[]
}

const structureAgentTrace = (trace: AgentTrace): AgentExport => {
  const messages: string[] = []
  const tools: ToolExport[] = []
  const pendingTools = new Map<string, ToolExport>()
  let systemPrompt = ''

  for (const e of trace.events) {
    switch (e.type) {
      case 'system_prompt':
        if (systemPrompt.length === 0) systemPrompt = e.content
        break
      case 'user_message':
        messages.push(e.content)
        break
      case 'assistant_message':
        messages.push(e.content)
        break
      case 'tool_call': {
        const tool: ToolExport = { name: e.toolName, input: e.input, result: null }
        pendingTools.set(e.toolId, tool)
        tools.push(tool)
        break
      }
      case 'tool_result': {
        const pending = pendingTools.get(e.toolId)
        if (pending) pending.result = e.result
        break
      }
    }
  }

  return {
    agent: trace.agentName ?? 'unknown',
    system_prompt: systemPrompt,
    messages,
    tools,
  }
}

const buildRunExport = (): Record<string, AgentExport[]> => {
  const { traces, order } = agentTraceStore.store.getState()
  const steps = workflowStore.store.getState().steps
  const stepNameMap = Collections.toLookupMap(steps, (s) => s.id, (s) => s.name ?? null)

  // Build a step→name map from dispatch traces (set_node_name tool calls)
  const dispatchState = dispatchStore.store.getState()
  const dispatchNameMap = new Map<string, string>()
  for (const entry of Object.values(dispatchState.byStep)) {
    const name = extractNodeName(entry.trace)
    if (name !== null) dispatchNameMap.set(entry.stepId, name)
  }

  const resolveStepName = (stepId: string): string =>
    dispatchNameMap.get(stepId)
    ?? stepNameMap.get(stepId)
    ?? stepId.slice(0, 8)

  const result: Record<string, AgentExport[]> = {}

  for (const id of order) {
    const trace = traces[id]
    if (trace === undefined) continue
    const stepName = resolveStepName(trace.stepId)
    const existing = result[stepName]
    const exported = structureAgentTrace(trace)
    if (existing !== undefined) {
      existing.push(exported)
    } else {
      result[stepName] = [exported]
    }
  }

  return result
}

export { buildDispatchExport, buildRunExport }
