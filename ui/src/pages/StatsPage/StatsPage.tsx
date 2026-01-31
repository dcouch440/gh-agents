import { useEffect, useState, useCallback } from 'react';
import { GothicPanel } from '../../components/GothicPanel';
import { useAgentStore } from '../../store/agentStore';
import { api } from '../../api/client';
import type { StatsResponse, IndexingStatus, UsageSummaryRow } from '../../api/client';
import styles from './StatsPage.module.css';

function BarChart({ data, formatValue }: { data: Record<string, number>; formatValue?: (v: number) => string }) {
  const entries = Object.entries(data);
  const max = Math.max(...entries.map(([, v]) => v), 1);
  const fmt = formatValue ?? ((v: number) => v.toLocaleString());

  if (entries.length === 0) return <div className={styles.empty}>No data yet</div>;

  return (
    <div>
      {entries.map(([label, value]) => (
        <div key={label} className={styles.barRow}>
          <span className={styles.barLabel}>{label}</span>
          <div className={styles.barTrack}>
            <div className={styles.barFill} style={{ width: `${(value / max) * 100}%` }} />
          </div>
          <span className={styles.barValue}>{fmt(value)}</span>
        </div>
      ))}
    </div>
  );
}

function transformStats(rows: UsageSummaryRow[]): StatsResponse {
  const token_usage: Record<string, number> = {};
  const call_counts: Record<string, number> = {};
  let total_tokens = 0;

  for (const row of rows) {
    const key = `${row.tier} / ${row.model_id}`;
    const tokens = row.total_input + row.total_output;
    token_usage[key] = (token_usage[key] ?? 0) + tokens;
    call_counts[key] = (call_counts[key] ?? 0) + row.call_count;
    total_tokens += tokens;
  }

  return { token_usage, total_tokens, call_counts };
}

const statusColors: Record<string, string> = {
  idle: 'var(--color-text-tertiary)',
  running: 'var(--color-gold)',
  complete: 'var(--color-status-success)',
  failed: 'var(--color-status-error)',
};

export function StatsPage() {
  const { agents, stats, fetch: fetchAgents } = useAgentStore();
  const [statsData, setStatsData] = useState<StatsResponse | null>(null);
  const [statsError, setStatsError] = useState<string | null>(null);
  const [indexing, setIndexing] = useState<IndexingStatus | null>(null);

  const pollIndexing = useCallback(() => {
    api.indexing.status().then(setIndexing).catch(() => {});
  }, []);

  useEffect(() => {
    fetchAgents();
    api.stats.get().then((rows) => {
      console.log('[StatsPage] raw stats response:', rows);
      setStatsData(transformStats(rows));
    }).catch((err) => {
      console.error('[StatsPage] stats fetch failed:', err);
      setStatsError(`${err.status ?? '?'}: ${err.message ?? err}`);
    });
    pollIndexing();
  }, [fetchAgents, pollIndexing]);

  // Poll while running
  useEffect(() => {
    if (indexing?.state !== 'running') return;
    const interval = setInterval(pollIndexing, 2000);
    return () => clearInterval(interval);
  }, [indexing?.state, pollIndexing]);

  const handleStartIndexing = async () => {
    await api.indexing.start().catch(() => {});
    pollIndexing();
  };

  const handleStopIndexing = async () => {
    await api.indexing.stop().catch(() => {});
    pollIndexing();
  };

  const activeAgents = agents.filter((a) => a.status === 'busy').length;
  const totalTokens = statsData?.total_tokens ?? 0;
  const totalCalls = statsData ? Object.values(statsData.call_counts).reduce((a, b) => a + b, 0) : 0;
  const isRunning = indexing?.state === 'running';
  const indexPct = indexing && indexing.files_total > 0
    ? (indexing.files_indexed / indexing.files_total) * 100
    : 0;

  return (
    <div className={styles.page}>
      <div>
        <div className={styles.header}>The Treasury</div>
        <div className={styles.headerSub}>Usage and performance metrics</div>
      </div>

      <GothicPanel title="Repo Index" variant={isRunning ? 'highlight' : 'default'}>
        <div className={styles.indexRow}>
          <div className={styles.indexStatus}>
            <span
              className={styles.indexDot}
              style={{ background: statusColors[indexing?.state ?? 'idle'] }}
            />
            <span style={{ color: statusColors[indexing?.state ?? 'idle'] }}>
              {indexing?.state ?? 'idle'}
            </span>
            {indexing?.state === 'complete' && indexing.files_indexed > 0 && (
              <span className={styles.indexCount}>{indexing.files_indexed} files</span>
            )}
            {indexing?.last_completed && (
              <span className={styles.indexTime}>
                Last: {new Date(indexing.last_completed).toLocaleTimeString()}
              </span>
            )}
          </div>
          <div className={styles.indexActions}>
            {isRunning ? (
              <button className={styles.btnStop} onClick={handleStopIndexing}>Stop</button>
            ) : (
              <button className={styles.btnStart} onClick={handleStartIndexing}>Start Indexing</button>
            )}
          </div>
        </div>
        {isRunning && (
          <div className={styles.indexProgress}>
            <div className={styles.barTrack}>
              <div
                className={`${styles.barFill} ember-glow-pulse`}
                style={{ width: `${indexPct}%` }}
              />
            </div>
            <span className={styles.indexProgressText}>
              {indexing?.files_indexed ?? 0} / {indexing?.files_total ?? 0}
            </span>
          </div>
        )}
        {indexing?.error && (
          <div style={{ color: 'var(--color-status-error)', fontSize: '0.75rem', marginTop: '0.5rem' }}>
            {indexing.error}
          </div>
        )}
      </GothicPanel>

      {statsError && (
        <GothicPanel variant="danger">
          <div style={{ color: 'var(--color-status-error)', fontSize: '0.8125rem' }}>
            Stats fetch failed: {statsError}
          </div>
        </GothicPanel>
      )}

      <div className={styles.summaryRow}>
        <GothicPanel>
          <div className={styles.bigNumber}>{totalTokens.toLocaleString()}</div>
          <div className={styles.bigLabel}>Total Tokens</div>
        </GothicPanel>
        <GothicPanel>
          <div className={styles.bigNumber}>{totalCalls.toLocaleString()}</div>
          <div className={styles.bigLabel}>API Calls (24h)</div>
        </GothicPanel>
        <GothicPanel>
          <div className={styles.bigNumber}>{activeAgents}</div>
          <div className={styles.bigLabel}>Active Agents</div>
        </GothicPanel>
      </div>

      <GothicPanel title="Token Usage by Model">
        <BarChart data={statsData?.token_usage ?? {}} />
      </GothicPanel>

      <GothicPanel title="API Calls by Model">
        <BarChart data={statsData?.call_counts ?? {}} />
      </GothicPanel>

      <GothicPanel title="Agent Pool">
        <table className={styles.table}>
          <thead>
            <tr>
              <th>Tier</th>
              <th>Active</th>
              <th>Available</th>
              <th>Max</th>
            </tr>
          </thead>
          <tbody>
            {(['orchestrators', 'workers', 'utilities'] as const).map((tier) => {
              const s = stats?.[tier];
              return (
                <tr key={tier}>
                  <td>{tier}</td>
                  <td>{s?.total ?? 0}</td>
                  <td>{s?.available ?? 0}</td>
                  <td>{s?.max ?? '—'}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </GothicPanel>
    </div>
  );
}
