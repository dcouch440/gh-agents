// --- Outbound (client → server) ---
export interface WsSubscribe {
  type: 'subscribe';
  channels: string[];
}

export interface WsUnsubscribe {
  type: 'unsubscribe';
  channels: string[];
}

export type WsOutbound = WsSubscribe | WsUnsubscribe;

// --- Inbound (server → client) ---
export type AgentStatus = 'busy' | 'idle' | 'offline';
export type TaskStatus = 'pending' | 'in_progress' | 'completed' | 'failed';

export interface AgentUpdateData {
  id: string;
  status: AgentStatus;
  current_task: string | null;
}

export interface TaskUpdateData {
  id: string;
  status: TaskStatus;
  progress: number; // 0.0 - 1.0
  assigned_agent: string | null;
}

export interface WsAgentUpdate {
  type: 'agent_update';
  data: AgentUpdateData;
}

export interface WsTaskUpdate {
  type: 'task_update';
  data: TaskUpdateData;
}

export interface WsSubscribed {
  type: 'subscribed';
  channels: string[];
}

export interface WsError {
  type: 'error';
  message: string;
}

export type WsInbound = WsAgentUpdate | WsTaskUpdate | WsSubscribed | WsError;
