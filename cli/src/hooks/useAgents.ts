import { useState, useEffect, useRef } from 'react';
import { api, getBaseUrl, getToken } from '../api/client.js';
import { WsClient } from '../api/ws-client.js';
import type { Agent, AgentGridState, AgentPoolSummary } from '../types/agents.js';
import type { AgentUpdateData, TaskUpdateData } from '../api/ws-types.js';
import type { AgentResponse } from '../api/types.js';

const emptyStats: AgentPoolSummary = {
  orchestrators: { total: 0, available: 0, max: 0 },
  workers: { total: 0, available: 0, max: 0 },
  utilities: { total: 0, available: 0, max: 0 },
};

function toAgent(r: AgentResponse): Agent {
  return {
    id: r.id,
    name: r.name,
    status: r.status,
    tier: r.tier,
    currentTask: r.current_task
      ? { id: r.current_task, title: '', status: 'in_progress', progress: 0 }
      : null,
  };
}

export function useAgents(): AgentGridState {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [stats, setStats] = useState<AgentPoolSummary>(emptyStats);
  const [connected, setConnected] = useState(false);
  const [loading, setLoading] = useState(true);
  const wsRef = useRef<WsClient | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function init() {
      try {
        const res = await api.agents.list();
        if (cancelled) return;
        setAgents(res.agents.map(toAgent));
        setStats(res.stats);
      } catch {
        // REST fetch failed; will rely on WS updates
      } finally {
        if (!cancelled) setLoading(false);
      }

      const ws = new WsClient(getBaseUrl(), getToken());
      wsRef.current = ws;

      ws.on('agent_update', (data) => {
        const update = data as AgentUpdateData;
        setAgents((prev) =>
          prev.map((a) =>
            a.id === update.id
              ? {
                  ...a,
                  status: update.status,
                  currentTask: update.current_task
                    ? a.currentTask?.id === update.current_task
                      ? a.currentTask
                      : { id: update.current_task, title: '', status: 'in_progress', progress: 0 }
                    : null,
                }
              : a,
          ),
        );
      });

      ws.on('task_update', (data) => {
        const update = data as TaskUpdateData;
        setAgents((prev) =>
          prev.map((a) =>
            a.currentTask?.id === update.id
              ? {
                  ...a,
                  currentTask: {
                    ...a.currentTask,
                    status: update.status,
                    progress: update.progress,
                  },
                }
              : a,
          ),
        );
      });

      try {
        await ws.connect();
        if (cancelled) {
          ws.disconnect();
          return;
        }
        ws.subscribe(['agents', 'tasks']);
        setConnected(true);
      } catch {
        // WS connection failed
      }
    }

    init();

    return () => {
      cancelled = true;
      wsRef.current?.disconnect();
      wsRef.current = null;
      setConnected(false);
    };
  }, []);

  return { agents, stats, connected, loading };
}
