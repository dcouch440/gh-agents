import type { AgentStatus, TaskStatus } from '../api/ws-types.js';

export type AgentTier = 'orchestrator' | 'worker' | 'utility';

export interface Agent {
  id: string;
  name: string;
  status: AgentStatus;
  tier: AgentTier;
  currentTask: AgentTask | null;
}

export interface AgentTask {
  id: string;
  title: string;
  status: TaskStatus;
  progress: number;
}

export interface PoolStats {
  total: number;
  available: number;
  max: number;
}

export interface AgentPoolSummary {
  orchestrators: PoolStats;
  workers: PoolStats;
  utilities: PoolStats;
}

export interface AgentGridState {
  agents: Agent[];
  stats: AgentPoolSummary;
  connected: boolean;
  loading: boolean;
}
