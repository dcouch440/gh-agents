import { useEffect } from 'react';
import { GothicPanel } from '../../components/GothicPanel';
import { TierBadge } from '../../components/TierBadge';
import { useAgentStore } from '../../store/agentStore';
import styles from './AgentsPage.module.css';

function TierGauge({ label, used, max }: { label: string; used: number; max: number }) {
  const pct = max > 0 ? (used / max) * 100 : 0;
  return (
    <GothicPanel>
      <div className={styles.gaugeLabel}>{label}</div>
      <div className={styles.gaugeBar}>
        <div className={styles.gaugeFill} style={{ width: `${pct}%` }} />
        <div className={styles.gaugeText}>{used} / {max}</div>
      </div>
    </GothicPanel>
  );
}

export function AgentsPage() {
  const { agents, stats, loading, fetch } = useAgentStore();

  useEffect(() => { fetch(); }, [fetch]);

  const tiers = ['orchestrator', 'worker', 'utility'] as const;
  const grouped = tiers.map((tier) => ({
    tier,
    agents: agents.filter((a) => a.tier === tier),
  }));

  return (
    <div className={styles.page}>
      <div>
        <div className={styles.header}>The War Room</div>
        <div className={styles.headerSub}>Agent pool overview</div>
      </div>

      <div className={styles.gaugeRow}>
        <TierGauge
          label="Orchestrators"
          used={stats?.orchestrators?.total ?? grouped[0].agents.length}
          max={stats?.orchestrators?.max ?? 1}
        />
        <TierGauge
          label="Workers"
          used={stats?.workers?.total ?? grouped[1].agents.length}
          max={stats?.workers?.max ?? 4}
        />
        <TierGauge
          label="Utilities"
          used={stats?.utilities?.total ?? grouped[2].agents.length}
          max={stats?.utilities?.max ?? 2}
        />
      </div>

      {agents.length === 0 && !loading ? (
        <div className={styles.empty}>No agents active — deploy from Agent Builder</div>
      ) : (
        grouped.map(({ tier, agents: tierAgents }) =>
          tierAgents.length > 0 ? (
            <GothicPanel key={tier} title={`${tier}s`}>
              <div className={styles.agentGrid}>
                {tierAgents.map((agent) => {
                  const isBusy = agent.status === 'busy';
                  const isOffline = agent.status === 'offline';
                  return (
                    <div
                      key={agent.id}
                      className={`${styles.agentCard} ${!isBusy && !isOffline ? styles.dimmed : ''} ${isBusy ? 'ember-glow-pulse' : ''}`}
                    >
                      <div className={styles.cardHeader}>
                        <TierBadge tier={tier} />
                        <span className={styles.agentName}>{agent.name ?? agent.id.slice(0, 8)}</span>
                      </div>
                      {agent.role && <div className={styles.agentRole}>{agent.role}</div>}
                      <div className={styles.statusRow}>
                        <span className={`${styles.statusDot} ${styles[agent.status] ?? styles.idle}`} />
                        <span style={{ color: 'var(--color-text-secondary)' }}>{agent.status}</span>
                      </div>
                      {agent.current_task && (
                        <div className={styles.taskLabel}>Task: {agent.current_task}</div>
                      )}
                    </div>
                  );
                })}
              </div>
            </GothicPanel>
          ) : null
        )
      )}
    </div>
  );
}
