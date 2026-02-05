import WebSocket from 'ws';
import type { WsInbound, WsOutbound } from './ws-types.js';

type MessageHandler = (data: unknown) => void;

export class WsClient {
  private ws: WebSocket | null = null;
  private url: string;
  private token: string;
  private handlers: Map<string, Set<MessageHandler>> = new Map();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 10;
  private baseDelay = 1000;
  private subscriptions: Set<string> = new Set();
  private shouldReconnect = true;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(baseUrl: string, token: string) {
    this.url = baseUrl.replace(/^http/, 'ws') + '/ws';
    this.token = token;
  }

  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.shouldReconnect = true;

      this.ws = new WebSocket(this.url, {
        headers: { Authorization: `Bearer ${this.token}` },
      });

      this.ws.on('open', () => {
        this.reconnectAttempts = 0;
        if (this.subscriptions.size > 0) {
          this.send({ type: 'subscribe', channels: [...this.subscriptions] });
        }
        resolve();
      });

      this.ws.on('close', () => {
        this.attemptReconnect();
      });

      this.ws.on('error', (err) => {
        if (this.reconnectAttempts === 0 && this.ws?.readyState !== WebSocket.OPEN) {
          reject(err);
        }
      });

      this.ws.on('message', (raw: WebSocket.Data) => {
        try {
          const message: WsInbound = JSON.parse(raw.toString());
          this.dispatch(message);
        } catch {
          // ignore unparseable messages
        }
      });
    });
  }

  subscribe(channels: string[]): void {
    channels.forEach((c) => this.subscriptions.add(c));
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.send({ type: 'subscribe', channels });
    }
  }

  unsubscribe(channels: string[]): void {
    channels.forEach((c) => this.subscriptions.delete(c));
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.send({ type: 'unsubscribe', channels });
    }
  }

  on(type: string, handler: MessageHandler): void {
    if (!this.handlers.has(type)) {
      this.handlers.set(type, new Set());
    }
    this.handlers.get(type)!.add(handler);
  }

  off(type: string, handler: MessageHandler): void {
    this.handlers.get(type)?.delete(handler);
  }

  disconnect(): void {
    this.shouldReconnect = false;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.ws?.close();
    this.ws = null;
  }

  get connected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  private dispatch(message: WsInbound): void {
    const handlers = this.handlers.get(message.type);
    if (handlers) {
      const payload = 'data' in message ? message.data : message;
      handlers.forEach((handler) => handler(payload));
    }

    const allHandlers = this.handlers.get('*');
    if (allHandlers) {
      allHandlers.forEach((handler) => handler(message));
    }
  }

  private attemptReconnect(): void {
    if (!this.shouldReconnect) return;
    if (this.reconnectAttempts >= this.maxReconnectAttempts) return;

    this.reconnectAttempts++;
    const delay = this.baseDelay * Math.pow(2, this.reconnectAttempts - 1);
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connect().catch(() => {
        // reconnect failed, will retry via close handler
      });
    }, delay);
  }

  private send(message: WsOutbound): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }
}
