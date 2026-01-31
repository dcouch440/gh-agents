import { useEffect, useRef } from 'react';
import { useAuthStore, useSessionStore } from '../store';
import { useAgentStore } from '../store/agentStore';
import { useTaskStore } from '../store/taskStore';

interface SessionUpdateMsg {
  type: 'session_update';
  data: {
    id: string;
    action: 'created' | 'updated' | 'deleted';
    title?: string;
    mode_id?: string;
  };
}

type ServerMessage = SessionUpdateMsg | { type: string };

/**
 * Connects to the WebSocket and subscribes to real-time channels.
 * Updates stores reactively when server pushes events.
 */
export function useWebSocket() {
  const token = useAuthStore((s) => s.token);
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    if (!isAuthenticated) return;

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const url = `${protocol}//${window.location.host}/ws${token ? `?token=${encodeURIComponent(token)}` : ''}`;

    const connect = () => {
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        // Subscribe to all channels
        ws.send(JSON.stringify({
          type: 'subscribe',
          channels: ['feed', 'tasks', 'agents', 'sessions'],
        }));
      };

      ws.onmessage = (event) => {
        try {
          const msg: ServerMessage = JSON.parse(event.data);
          handleMessage(msg);
        } catch {
          // ignore malformed messages
        }
      };

      ws.onclose = () => {
        wsRef.current = null;
        // Reconnect after 3 seconds
        setTimeout(connect, 3000);
      };

      ws.onerror = () => {
        ws.close();
      };
    };

    connect();

    return () => {
      const ws = wsRef.current;
      if (ws) {
        ws.onclose = null; // prevent reconnect on intentional close
        ws.close();
        wsRef.current = null;
      }
    };
  }, [token, isAuthenticated]);
}

function handleMessage(msg: ServerMessage) {
  if (msg.type === 'agent_update') {
    useAgentStore.getState().fetch();
    return;
  }

  if (msg.type === 'task_update') {
    useTaskStore.getState().fetch();
    return;
  }

  if (msg.type === 'session_update') {
    const { data } = msg as SessionUpdateMsg;
    const store = useSessionStore.getState();

    switch (data.action) {
      case 'created':
        // Refresh the full list to get complete session data
        store.refresh();
        break;
      case 'updated':
        if (data.title) {
          // Optimistic update — patch the title in place
          const existing = store.sessions.find((s) => s.id === data.id);
          if (existing) {
            store.updateSession(data.id, { ...existing, title: data.title });
          } else {
            store.refresh();
          }
        }
        break;
      case 'deleted':
        store.removeSession(data.id);
        break;
    }
  }
}
