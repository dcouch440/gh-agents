import { FeedItem as FeedItemType } from '../../hooks/useFeed';
import { StatusDot } from '../StatusDot';
import styles from './FeedItem.module.css';

interface FeedItemProps {
  item: FeedItemType;
}

export function FeedItem({ item }: FeedItemProps) {
  const getAgentStyle = (agent: string): string => {
    if (agent.toLowerCase().includes('orchestrator')) return styles.agentOrchestrator;
    if (agent.toLowerCase().includes('worker')) return styles.agentWorker;
    return styles.agentDefault;
  };

  const getTypeStyle = (type: string): string => {
    switch (type) {
      case 'milestone':
        return styles.typeMilestone;
      case 'error':
        return styles.typeError;
      case 'warning':
        return styles.typeWarning;
      default:
        return styles.typeDefault;
    }
  };

  const getStatusType = (type: string): 'success' | 'error' | 'active' | 'idle' => {
    if (type === 'error') return 'error';
    if (type === 'milestone') return 'success';
    return 'active';
  };

  return (
    <div className={`${styles.item} ${getTypeStyle(item.type)}`}>
      <span className={styles.timestamp}>
        {new Date(item.timestamp).toLocaleTimeString()}
      </span>

      <StatusDot status={getStatusType(item.type)} />

      <span className={`${styles.agent} ${getAgentStyle(item.agent)}`}>
        {item.agent}
      </span>

      <span className={styles.content}>
        {item.content}
      </span>
    </div>
  );
}
