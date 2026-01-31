import { Check } from 'lucide-react';
import type { ToolExecution } from '../../hooks/useChat';
import styles from './ToolActivity.module.css';

interface ToolActivityProps {
  executions: ToolExecution[];
}

export function ToolActivity({ executions }: ToolActivityProps) {
  if (executions.length === 0) return null;

  return (
    <div className={styles.container}>
      {executions.map((exec, i) => (
        <div
          key={exec.id}
          className={`${styles.card} ${exec.status === 'done' ? styles.cardDone : ''}`}
          style={{ animationDelay: `${i * 80}ms` }}
        >
          <span className={styles.indicator}>
            {exec.status === 'done' ? (
              <Check size={12} className={styles.checkIcon} />
            ) : (
              <span className={styles.dot} />
            )}
          </span>
          <span className={styles.toolName}>{exec.name}</span>
          {exec.status === 'running' && (
            <span className={styles.dots}>
              <span className={styles.dot1}>·</span>
              <span className={styles.dot2}>·</span>
              <span className={styles.dot3}>·</span>
            </span>
          )}
        </div>
      ))}
    </div>
  );
}
