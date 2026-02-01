import { useState, useEffect, useRef, useCallback } from 'react'
import { AgentActivityDemo } from '@/components/chat'
import {
  AgentPoolStatus,
  TaskQueueStatus,
  TokenUsageStatus,
  SystemHealthStatus,
  FeedStream,
  PipelineRenderer,
} from '@/components/dashboard'
import type { Agent, AgentPoolStats, Task, UsageSummary, Config, FeedItem, Pipeline, PipelineRun, StageExecution } from '@/types'

// ── Static seed data ──────────────────────────────

const AGENTS_SEED: Agent[] = [
  { id: 'a1', tier: 'orchestrator', persona_name: 'Atlas', persona_prompt: '', persona_style: 'technical', model_provider: 'anthropic', model_id: 'claude-opus-4-5-20251101', model_max_tokens: 16384, model_temperature: 0.7, status: 'idle' },
  { id: 'a2', tier: 'worker', persona_name: 'Forge', persona_prompt: '', persona_style: 'technical', model_provider: 'anthropic', model_id: 'claude-sonnet-4-20250514', model_max_tokens: 8192, model_temperature: 0.7, status: 'idle' },
  { id: 'a3', tier: 'worker', persona_name: 'Shield', persona_prompt: '', persona_style: 'technical', model_provider: 'anthropic', model_id: 'claude-sonnet-4-20250514', model_max_tokens: 8192, model_temperature: 0.3, status: 'idle' },
  { id: 'a4', tier: 'utility', persona_name: 'Lint', persona_prompt: '', persona_style: 'formal', model_provider: 'anthropic', model_id: 'claude-3-5-haiku-20241022', model_max_tokens: 4096, model_temperature: 0.3, status: 'idle' },
]

const CONFIG: Config = {
  verbosity: 'normal',
  models: {
    orchestrator: { provider: 'anthropic', model_id: 'opus', max_tokens: 16384, temperature: 0.7 },
    worker: { provider: 'anthropic', model_id: 'sonnet', max_tokens: 8192, temperature: 0.7 },
    utility: { provider: 'anthropic', model_id: 'haiku', max_tokens: 4096, temperature: 0.3 },
  },
  pool: { max_orchestrators: 2, max_workers: 4, max_utilities: 3 },
  autonomy: 'full',
  git_strategy: 'branch',
  sandbox_mode: 'strict',
}

const PIPELINE: Pipeline = {
  id: 'p1',
  name: 'deploy-auth',
  stages: [
    { stage_number: 1, agent_id: 'a1', cluster_id: null, role: 'planner', approval_required: false, fan_out: false, stage_name: 'plan', input_definitions: {}, output_description: 'execution plan', output_schema: null },
    { stage_number: 2, agent_id: 'a2', cluster_id: null, role: 'implementer', approval_required: false, fan_out: false, stage_name: 'implement', input_definitions: {}, output_description: 'code changes', output_schema: null },
    { stage_number: 3, agent_id: 'a3', cluster_id: null, role: 'reviewer', approval_required: true, fan_out: false, stage_name: 'review', input_definitions: {}, output_description: 'review result', output_schema: null },
    { stage_number: 4, agent_id: 'a4', cluster_id: null, role: 'formatter', approval_required: false, fan_out: false, stage_name: 'lint+test', input_definitions: {}, output_description: 'test results', output_schema: null },
  ],
}

const FEED_SEED: FeedItem[] = [
  { id: 'f0', agent_id: 'sys', content: 'System initialized', item_type: 'system_notice', verbosity_level: 'normal', timestamp: new Date(Date.now() - 30000).toISOString() },
]

// ── Script timeline ───────────────────────────────

type TimelineEvent = {
  at: number
  apply: (s: ShowcaseState) => ShowcaseState
}

type ShowcaseState = {
  agents: Agent[]
  stats: AgentPoolStats
  tasks: Task[]
  usage: UsageSummary[]
  feed: FeedItem[]
  run: PipelineRun | null
  stages: StageExecution[]
  wsConnected: boolean
}

const makeTask = (id: string, title: string, priority: 'low' | 'normal' | 'high' | 'urgent', status: 'pending' | 'in_progress' | 'review' | 'completed' | 'failed', agent: string | null, deps: string[] = []): Task => ({
  id, slice_id: null, title, description: '', assigned_tier: 'worker', assigned_agent: agent, status, priority, context_files: [], metadata: null, depends_on: deps, retry_count: 0, max_retries: 3, last_error: null, created_at: new Date().toISOString(), updated_at: new Date().toISOString(),
})

const setAgentStatus = (agents: Agent[], id: string, status: Agent['status']): Agent[] =>
  agents.map((a) => (a.id === id ? { ...a, status } : a))

const computeStats = (agents: Agent[]): AgentPoolStats => {
  const count = (tier: Agent['tier']) => {
    const t = agents.filter((a) => a.tier === tier)
    const avail = t.filter((a) => a.status === 'idle').length
    const max = tier === 'orchestrator' ? 2 : tier === 'worker' ? 4 : 3
    return { total: t.length, available: avail, max }
  }
  return { orchestrators: count('orchestrator'), workers: count('worker'), utilities: count('utility') }
}

const addFeed = (feed: FeedItem[], content: string, itemType: FeedItem['item_type'], agentId: string): FeedItem[] => [
  ...feed,
  { id: `f${feed.length}`, agent_id: agentId, content, item_type: itemType, verbosity_level: 'normal', timestamp: new Date().toISOString() },
]

const makeExec = (stageNum: number, stageName: string, agentId: string, status: string, durationMs: number, inTok: number, outTok: number, completed: boolean): StageExecution => ({
  id: `se${stageNum}`, run_id: 'r1', stage_number: stageNum, stage_name: stageName, agent_id: agentId, status, rendered_prompt: null, output: null, structured_output: null, user_input: null, input_tokens: inTok, output_tokens: outTok, started_at: new Date().toISOString(), completed_at: completed ? new Date().toISOString() : null, duration_ms: durationMs,
})

const INITIAL_STATE: ShowcaseState = {
  agents: AGENTS_SEED,
  stats: computeStats(AGENTS_SEED),
  tasks: [
    makeTask('t1', 'Refactor auth middleware', 'high', 'pending', null),
    makeTask('t2', 'Add JWT refresh endpoint', 'normal', 'pending', null, ['t1']),
    makeTask('t3', 'Write auth integration tests', 'normal', 'pending', null, ['t1', 't2']),
    makeTask('t4', 'Update API docs', 'low', 'pending', null, ['t1']),
  ],
  usage: [
    { tier: 'orchestrator', model_id: 'opus', total_input: 0, total_output: 0, call_count: 0 },
    { tier: 'worker', model_id: 'sonnet', total_input: 0, total_output: 0, call_count: 0 },
    { tier: 'utility', model_id: 'haiku', total_input: 0, total_output: 0, call_count: 0 },
  ],
  feed: FEED_SEED,
  run: null,
  stages: [],
  wsConnected: true,
}

const addUsage = (usage: UsageSummary[], tier: string, inTok: number, outTok: number): UsageSummary[] =>
  usage.map((u) => (u.tier === tier ? { ...u, total_input: u.total_input + inTok, total_output: u.total_output + outTok, call_count: u.call_count + 1 } : u))

const setTaskStatus = (tasks: Task[], id: string, status: Task['status'], agent: string | null = null): Task[] =>
  tasks.map((t) => (t.id === id ? { ...t, status, assigned_agent: agent ?? t.assigned_agent } : t))

const TIMELINE: TimelineEvent[] = [
  // 0s: Pipeline starts, Atlas plans
  { at: 0, apply: (s) => {
    const agents = setAgentStatus(s.agents, 'a1', 'working')
    const run: PipelineRun = { id: 'r1', pipeline_id: 'p1', user_id: 'u1', status: 'running', initial_task: 'deploy auth refactor', stage_outputs: {}, current_stage: 1, started_at: new Date().toISOString(), completed_at: null, total_input_tokens: 0, total_output_tokens: 0 }
    return { ...s, agents, stats: computeStats(agents), run, feed: addFeed(s.feed, 'Pipeline deploy-auth started', 'system_notice', 'sys'), tasks: setTaskStatus(s.tasks, 't1', 'in_progress', 'Atlas') }
  }},
  // 1s: Atlas planning, feed update
  { at: 1000, apply: (s) => ({
    ...s, feed: addFeed(s.feed, 'Atlas: analyzing auth module structure', 'agent_report', 'a1'), usage: addUsage(s.usage, 'orchestrator', 4200, 1800),
    stages: [makeExec(1, 'plan', 'a1', 'running', 1000, 4200, 1800, false)],
    run: s.run ? { ...s.run, total_input_tokens: 4200, total_output_tokens: 1800 } : null,
  })},
  // 2.5s: Stage 1 done, Forge starts implement
  { at: 2500, apply: (s) => {
    const agents = setAgentStatus(setAgentStatus(s.agents, 'a1', 'idle'), 'a2', 'working')
    return {
      ...s, agents, stats: computeStats(agents),
      feed: addFeed(s.feed, 'Atlas: plan complete — 3 files to modify', 'task_completed', 'a1'),
      stages: [makeExec(1, 'plan', 'a1', 'completed', 2500, 4200, 1800, true), makeExec(2, 'implement', 'a2', 'running', 0, 0, 0, false)],
      run: s.run ? { ...s.run, current_stage: 2 } : null,
      tasks: setTaskStatus(s.tasks, 't1', 'in_progress', 'Forge'),
    }
  }},
  // 3.5s: Forge working
  { at: 3500, apply: (s) => ({
    ...s,
    feed: addFeed(s.feed, 'Forge: modifying src/auth/middleware.rs', 'agent_report', 'a2'),
    usage: addUsage(s.usage, 'worker', 6100, 3200),
    stages: [s.stages[0]!, makeExec(2, 'implement', 'a2', 'running', 1000, 6100, 3200, false)],
    run: s.run ? { ...s.run, total_input_tokens: 10300, total_output_tokens: 5000 } : null,
  })},
  // 5s: Forge done, Shield reviews, task t1 review
  { at: 5000, apply: (s) => {
    const agents = setAgentStatus(setAgentStatus(s.agents, 'a2', 'idle'), 'a3', 'working')
    return {
      ...s, agents, stats: computeStats(agents),
      feed: addFeed(s.feed, 'Forge: implementation complete — 3 files changed', 'task_completed', 'a2'),
      usage: addUsage(s.usage, 'worker', 2400, 800),
      stages: [s.stages[0]!, makeExec(2, 'implement', 'a2', 'completed', 2500, 8500, 4000, true), makeExec(3, 'review', 'a3', 'running', 0, 0, 0, false)],
      run: s.run ? { ...s.run, current_stage: 3, total_input_tokens: 12700, total_output_tokens: 5800 } : null,
      tasks: setTaskStatus(setTaskStatus(s.tasks, 't1', 'review', 'Shield'), 't2', 'in_progress', 'Forge'),
    }
  }},
  // 6.5s: Shield reviewing, approval needed
  { at: 6500, apply: (s) => ({
    ...s,
    feed: addFeed(s.feed, 'Shield: reviewing code changes...', 'agent_report', 'a3'),
    usage: addUsage(s.usage, 'worker', 5200, 2100),
    stages: [s.stages[0]!, s.stages[1]!, makeExec(3, 'review', 'a3', 'running', 1500, 5200, 2100, false)],
  })},
  // 8s: Review done, waiting for approval
  { at: 8000, apply: (s) => {
    const agents = setAgentStatus(s.agents, 'a3', 'waiting_for_approval')
    return {
      ...s, agents, stats: computeStats(agents),
      feed: addFeed(s.feed, 'Shield: review passed — awaiting approval', 'milestone', 'a3'),
      stages: [s.stages[0]!, s.stages[1]!, makeExec(3, 'review', 'a3', 'completed', 3000, 5200, 2100, true)],
      run: s.run ? { ...s.run, status: 'waiting_for_approval', total_input_tokens: 17900, total_output_tokens: 7900 } : null,
    }
  }},
  // 9.5s: Auto-approve, Lint starts
  { at: 9500, apply: (s) => {
    const agents = setAgentStatus(setAgentStatus(s.agents, 'a3', 'idle'), 'a4', 'working')
    return {
      ...s, agents, stats: computeStats(agents),
      feed: addFeed(s.feed, 'Approval granted — continuing pipeline', 'system_notice', 'sys'),
      stages: [...s.stages.slice(0, 3), makeExec(4, 'lint+test', 'a4', 'running', 0, 0, 0, false)],
      run: s.run ? { ...s.run, status: 'running', current_stage: 4 } : null,
      tasks: setTaskStatus(s.tasks, 't1', 'completed', 'Shield'),
    }
  }},
  // 10.5s: Lint working
  { at: 10500, apply: (s) => ({
    ...s,
    feed: addFeed(s.feed, 'Lint: running clippy + cargo test', 'agent_report', 'a4'),
    usage: addUsage(s.usage, 'utility', 1800, 600),
    stages: [...s.stages.slice(0, 3), makeExec(4, 'lint+test', 'a4', 'running', 1000, 1800, 600, false)],
    run: s.run ? { ...s.run, total_input_tokens: 19700, total_output_tokens: 8500 } : null,
  })},
  // 12s: Pipeline complete
  { at: 12000, apply: (s) => {
    const agents = setAgentStatus(s.agents, 'a4', 'idle')
    return {
      ...s, agents, stats: computeStats(agents),
      feed: addFeed(addFeed(s.feed, 'Lint: all checks passed', 'task_completed', 'a4'), 'Pipeline deploy-auth completed', 'milestone', 'sys'),
      usage: addUsage(s.usage, 'utility', 800, 400),
      stages: [...s.stages.slice(0, 3), makeExec(4, 'lint+test', 'a4', 'completed', 2500, 2600, 1000, true)],
      run: s.run ? { ...s.run, status: 'completed', current_stage: 4, completed_at: new Date().toISOString(), total_input_tokens: 20500, total_output_tokens: 8900 } : null,
      tasks: setTaskStatus(setTaskStatus(s.tasks, 't2', 'completed', 'Forge'), 't3', 'in_progress', 'Shield'),
    }
  }},
  // 13s: WS flicker
  { at: 13000, apply: (s) => ({ ...s, wsConnected: false, feed: addFeed(s.feed, 'WebSocket disconnected', 'error', 'sys') }) },
  { at: 13800, apply: (s) => ({ ...s, wsConnected: true, feed: addFeed(s.feed, 'WebSocket reconnected', 'system_notice', 'sys') }) },
]

const CYCLE_DURATION = 16000

// ── Showcase Page ─────────────────────────────────

function ShowcasePage() {
  const [state, setState] = useState<ShowcaseState>(INITIAL_STATE)
  const timersRef = useRef<ReturnType<typeof setTimeout>[]>([])
  const cycleRef = useRef(0)

  const runCycleRef = useRef<() => void>(() => {})

  const clearTimers = useCallback(() => {
    for (const t of timersRef.current) clearTimeout(t)
    timersRef.current = []
  }, [])

  const runCycle = useCallback(() => {
    setState(INITIAL_STATE)
    clearTimers()

    for (const event of TIMELINE) {
      const timer = setTimeout(() => {
        setState((prev) => event.apply(prev))
      }, event.at)
      timersRef.current.push(timer)
    }

    const restart = setTimeout(() => {
      cycleRef.current++
      runCycleRef.current()
    }, CYCLE_DURATION)
    timersRef.current.push(restart)
  }, [clearTimers])

  useEffect(() => {
    runCycleRef.current = runCycle
  }, [runCycle])

  useEffect(() => {
    const kickoff = setTimeout(() => {
      runCycleRef.current()
    }, 0)
    return () => {
      clearTimeout(kickoff)
      clearTimers()
    }
  }, [clearTimers])

  return (
    <div className="showcase">
      <div className="showcase__header">COMPONENT SHOWCASE</div>

      <div className="showcase__grid">
        <div className="showcase__section">
          <div className="showcase__title">AGENT POOL</div>
          <AgentPoolStatus agents={state.agents} stats={state.stats} />
        </div>

        <div className="showcase__section">
          <div className="showcase__title">SYSTEM HEALTH</div>
          <SystemHealthStatus config={CONFIG} agentStats={state.stats} wsConnected={state.wsConnected} />
        </div>

        <div className="showcase__section">
          <div className="showcase__title">TASK QUEUE</div>
          <TaskQueueStatus tasks={state.tasks} />
        </div>

        <div className="showcase__section">
          <div className="showcase__title">TOKEN USAGE (24h)</div>
          <TokenUsageStatus usage={state.usage} />
        </div>

        <div className="showcase__section showcase__section--wide">
          <div className="showcase__title">PIPELINE</div>
          <PipelineRenderer pipeline={PIPELINE} run={state.run} stages={state.stages} />
        </div>

        <div className="showcase__section showcase__section--wide">
          <div className="showcase__title">ACTIVITY FEED</div>
          <FeedStream items={state.feed} maxVisible={8} />
        </div>

        <div className="showcase__section showcase__section--wide">
          <div className="showcase__title">CHAT ACTIVITY</div>
          <AgentActivityDemo />
        </div>
      </div>
    </div>
  )
}

export { ShowcasePage }
