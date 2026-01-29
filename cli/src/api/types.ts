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
