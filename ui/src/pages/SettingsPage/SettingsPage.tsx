import { useEffect, useState } from 'react';
import { GothicPanel } from '../../components/GothicPanel';
import { useConfigStore } from '../../store/configStore';
import { useToastStore } from '../../store';
import type { Config } from '../../api/client';
import styles from './SettingsPage.module.css';

export function SettingsPage() {
  const { config, loading, fetch, update } = useConfigStore();
  const addToast = useToastStore((s) => s.addToast);
  const [form, setForm] = useState<Partial<Config>>({});

  useEffect(() => { fetch(); }, [fetch]);
  useEffect(() => { if (config) setForm(config); }, [config]);

  const setModel = (tier: 'orchestrator' | 'worker' | 'utility', field: string, value: string | number) => {
    setForm((prev) => ({
      ...prev,
      models: {
        ...prev.models!,
        [tier]: { ...prev.models?.[tier], [field]: value },
      },
    }));
  };

  const setPool = (key: string, value: number) => {
    setForm((prev) => ({ ...prev, pool: { ...prev.pool!, [key]: value } }));
  };

  const set = (key: string, value: string) => {
    setForm((prev) => ({ ...prev, [key]: value }));
  };

  const handleSave = async () => {
    await update(form);
    addToast('Configuration saved', 'success');
  };

  const handleReset = () => {
    if (config) setForm(config);
  };

  if (loading && !config) {
    return <div className={styles.page}><span style={{ color: 'var(--color-text-secondary)' }}>Loading configuration...</span></div>;
  }

  return (
    <div className={styles.page}>
      <div>
        <div className={styles.header}>The Forge</div>
        <div className={styles.headerSub}>System configuration</div>
      </div>

      {(['orchestrator', 'worker', 'utility'] as const).map((tier) => (
        <GothicPanel key={tier} title={`${tier.charAt(0).toUpperCase()}${tier.slice(1)} Model`}>
          <div className={styles.fieldGroup}>
            <div className={styles.field}>
              <label className={styles.label}>Model ID</label>
              <input
                className={styles.input}
                value={form.models?.[tier]?.model_id ?? ''}
                onChange={(e) => setModel(tier, 'model_id', e.target.value)}
                placeholder="e.g. claude-sonnet-4-20250514"
              />
            </div>
            <div className={styles.fieldRow}>
              <div className={styles.field}>
                <label className={styles.label}>Max Tokens</label>
                <input
                  type="number"
                  className={styles.input}
                  value={form.models?.[tier]?.max_tokens ?? ''}
                  onChange={(e) => setModel(tier, 'max_tokens', Number(e.target.value))}
                  min={1}
                  max={100000}
                />
              </div>
              <div className={styles.field}>
                <label className={styles.label}>Temperature</label>
                <input
                  type="number"
                  className={styles.input}
                  value={form.models?.[tier]?.temperature ?? ''}
                  onChange={(e) => setModel(tier, 'temperature', Number(e.target.value))}
                  min={0}
                  max={1}
                  step={0.1}
                />
              </div>
            </div>
          </div>
        </GothicPanel>
      ))}

      <GothicPanel title="Agent Pool">
        <div className={styles.fieldGroup}>
          {[
            { key: 'max_orchestrators', label: 'Max Orchestrators' },
            { key: 'max_workers', label: 'Max Workers' },
            { key: 'max_utilities', label: 'Max Utilities' },
          ].map(({ key, label }) => (
            <div key={key} className={styles.field}>
              <label className={styles.label}>{label}</label>
              <input
                type="number"
                className={styles.input}
                value={(form.pool as Record<string, number> | undefined)?.[key] ?? ''}
                onChange={(e) => setPool(key, Number(e.target.value))}
                min={0}
                max={20}
              />
            </div>
          ))}
        </div>
      </GothicPanel>

      <GothicPanel title="Behavior">
        <div className={styles.fieldGroup}>
          <div className={styles.field}>
            <label className={styles.label}>Autonomy</label>
            <select className={styles.select} value={form.autonomy ?? ''} onChange={(e) => set('autonomy', e.target.value)}>
              <option value="">--</option>
              <option value="full_auto">Full Auto</option>
              <option value="approval_gates">Approval Gates</option>
              <option value="supervised">Supervised</option>
            </select>
          </div>
          <div className={styles.field}>
            <label className={styles.label}>Git Strategy</label>
            <select className={styles.select} value={form.git_strategy ?? ''} onChange={(e) => set('git_strategy', e.target.value)}>
              <option value="">--</option>
              <option value="branch_per_slice">Branch Per Slice</option>
              <option value="branch_per_ticket">Branch Per Ticket</option>
            </select>
          </div>
          <div className={styles.field}>
            <label className={styles.label}>Sandbox Mode</label>
            <select className={styles.select} value={form.sandbox_mode ?? ''} onChange={(e) => set('sandbox_mode', e.target.value)}>
              <option value="">--</option>
              <option value="docker">Docker</option>
              <option value="local_restricted">Local Restricted</option>
              <option value="none">None</option>
            </select>
          </div>
        </div>
      </GothicPanel>

      <GothicPanel title="Display">
        <div className={styles.fieldGroup}>
          <div className={styles.field}>
            <label className={styles.label}>Verbosity</label>
            <select className={styles.select} value={form.verbosity ?? ''} onChange={(e) => set('verbosity', e.target.value)}>
              <option value="quiet">Quiet</option>
              <option value="normal">Normal</option>
              <option value="verbose">Verbose</option>
            </select>
          </div>
        </div>
      </GothicPanel>

      <div className={styles.actions}>
        <button className={styles.btnSave} onClick={handleSave}>Save</button>
        <button className={styles.btnReset} onClick={handleReset}>Reset</button>
      </div>
    </div>
  );
}
