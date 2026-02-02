import type { Agent } from '@/types/agent'
import type { Task } from '@/types/task'
import type { Session, ChatMessage, Mode } from '@/types/session'
import type { Pipeline, PipelineRun, StageExecution } from '@/types/pipeline'
import type { Document } from '@/types/document'
import type { FeedItem } from '@/types/feed'
import type { Tool } from '@/types/tool'
import type { UsageSummary } from '@/types/stats'
import type { Config } from '@/types/config'
import type { CostResponse } from '@/types/cost'
import data from '@/mock-data.json'

const delay = (ms = 80): Promise<void> => new Promise((r) => setTimeout(r, ms))

const getAgents = async (): Promise<Agent[]> => {
  await delay()
  return data.agents as Agent[]
}

const getAgent = async (id: string): Promise<Agent | null> => {
  await delay()
  return (data.agents as Agent[]).find((a) => a.id === id) ?? null
}

const getTasks = async (): Promise<Task[]> => {
  await delay()
  return data.tasks as Task[]
}

const getTask = async (id: string): Promise<Task | null> => {
  await delay()
  return (data.tasks as Task[]).find((t) => t.id === id) ?? null
}

const getSessions = async (): Promise<Session[]> => {
  await delay()
  return data.sessions as Session[]
}

const getSession = async (id: string): Promise<Session | null> => {
  await delay()
  return (data.sessions as Session[]).find((s) => s.id === id) ?? null
}

const getChatHistory = async (sessionId: string): Promise<ChatMessage[]> => {
  await delay()
  const history = data.chat_history as Record<string, ChatMessage[]>
  return history[sessionId] ?? []
}

const getModes = async (): Promise<Mode[]> => {
  await delay()
  return data.modes as Mode[]
}

const getPipelines = async (): Promise<Pipeline[]> => {
  await delay()
  return data.pipelines as Pipeline[]
}

const getPipeline = async (id: string): Promise<Pipeline | null> => {
  await delay()
  return (data.pipelines as Pipeline[]).find((p) => p.id === id) ?? null
}

const getPipelineRuns = async (pipelineId?: string): Promise<PipelineRun[]> => {
  await delay()
  const runs = data.pipeline_runs as PipelineRun[]
  if (pipelineId) return runs.filter((r) => r.pipeline_id === pipelineId)
  return runs
}

const getPipelineRun = async (id: string): Promise<PipelineRun | null> => {
  await delay()
  return (data.pipeline_runs as PipelineRun[]).find((r) => r.id === id) ?? null
}

const getStageExecutions = async (runId: string): Promise<StageExecution[]> => {
  await delay()
  return (data.stage_executions as StageExecution[]).filter((se) => se.run_id === runId)
}

const getDocuments = async (): Promise<Document[]> => {
  await delay()
  return data.documents as Document[]
}

const getDocument = async (id: string): Promise<Document | null> => {
  await delay()
  return (data.documents as Document[]).find((d) => d.id === id) ?? null
}

const getFeed = async (): Promise<FeedItem[]> => {
  await delay()
  return data.feed as FeedItem[]
}

const getTools = async (): Promise<Tool[]> => {
  await delay()
  return data.tools as Tool[]
}

const getStats = async (): Promise<UsageSummary[]> => {
  await delay()
  return data.usage_stats as UsageSummary[]
}

const getCosts = async (): Promise<CostResponse> => {
  await delay()
  return { total_spend: 0, models: [] }
}

const getConfig = async (): Promise<Config> => {
  await delay()
  return data.config as unknown as Config
}

export const mock = {
  getAgents,
  getAgent,
  getTasks,
  getTask,
  getSessions,
  getSession,
  getChatHistory,
  getModes,
  getPipelines,
  getPipeline,
  getPipelineRuns,
  getPipelineRun,
  getStageExecutions,
  getDocuments,
  getDocument,
  getFeed,
  getTools,
  getStats,
  getCosts,
  getConfig,
}
