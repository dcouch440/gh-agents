import { useEffect } from 'react';
import { GothicPanel } from '../../components/GothicPanel';
import { PriorityIndicator } from '../../components/PriorityIndicator';
import { useTaskStore } from '../../store/taskStore';
import type { Task } from '../../api/client';
import styles from './TasksPage.module.css';

function timeAgo(dateStr: string): string {
  const diff = Date.now() - new Date(dateStr).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  return `${Math.floor(hrs / 24)}d ago`;
}

const columns = [
  { status: 'pending', label: 'Pending' },
  { status: 'in_progress', label: 'In Progress' },
  { status: 'completed', label: 'Completed' },
  { status: 'failed', label: 'Failed' },
] as const;

function TaskCard({ task }: { task: Task }) {
  return (
    <div className={styles.taskCard}>
      <div className={styles.taskTitle}>
        <PriorityIndicator priority={task.priority} />
        <span>{task.title}</span>
      </div>
      <div className={styles.taskMeta}>
        <span>{task.assigned_agent ? `Agent: ${task.assigned_agent.slice(0, 8)}` : 'Unassigned'}</span>
        <span>{timeAgo(task.created_at)}</span>
      </div>
    </div>
  );
}

export function TasksPage() {
  const { tasks, loading, filter, fetch, setFilter } = useTaskStore();

  useEffect(() => { fetch(); }, [fetch]);

  const filtered = tasks.filter((t) => {
    if (filter.search) {
      const q = filter.search.toLowerCase();
      if (!t.title.toLowerCase().includes(q) && !t.description.toLowerCase().includes(q)) return false;
    }
    return true;
  });

  const byStatus = (status: string) => filtered.filter((t) => t.status === status);

  return (
    <div className={styles.page}>
      <div>
        <div className={styles.header}>The Quest Board</div>
        <div className={styles.headerSub}>Task tracker</div>
      </div>

      <div className={styles.filterBar}>
        <input
          className={styles.searchInput}
          placeholder="Search quests..."
          value={filter.search ?? ''}
          onChange={(e) => setFilter({ search: e.target.value })}
        />
        {columns.map(({ status, label }) => (
          <button
            key={status}
            className={`${styles.filterChip} ${filter.status === status ? styles.filterChipActive : ''}`}
            onClick={() => setFilter({ status: filter.status === status ? undefined : status })}
          >
            {label}
          </button>
        ))}
      </div>

      <div className={styles.board}>
        {columns.map(({ status, label }) => {
          const colTasks = filter.status && filter.status !== status ? [] : byStatus(status);
          const borderClass = styles[`${status}Border` as keyof typeof styles] ?? '';
          return (
            <div key={status} className={styles.column}>
              <GothicPanel
                title={`${label}`}
                className={borderClass}
              >
                <span className={styles.countBadge}>{colTasks.length}</span>
                <div className={styles.columnCards}>
                  {colTasks.length === 0 ? (
                    <div className={styles.emptyCol}>{loading ? 'Loading...' : 'No quests'}</div>
                  ) : (
                    colTasks.map((task) => <TaskCard key={task.id} task={task} />)
                  )}
                </div>
              </GothicPanel>
            </div>
          );
        })}
      </div>
    </div>
  );
}
