export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: string;
}

export interface HealthResponse {
  status: string;
  version: string;
  db_connected: boolean;
}

export interface LoginResponse {
  token: string;
  expires_in: number;
}

export interface ChatSendResponse {
  message_id: string;
  status: string;
}

export interface AuthMeResponse {
  user: string;
  authenticated: boolean;
  token_expires: number;
}

export interface AgentResponse {
  id: string;
  name: string;
  status: 'busy' | 'idle' | 'offline';
  tier: 'orchestrator' | 'worker' | 'utility';
  current_task: string | null;
}

export interface AgentsListResponse {
  agents: AgentResponse[];
  stats: {
    orchestrators: { total: number; available: number; max: number };
    workers: { total: number; available: number; max: number };
    utilities: { total: number; available: number; max: number };
  };
}
