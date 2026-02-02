import { useState } from 'react';
import type { FeedItem as FeedItemType } from '../../hooks/useFeed';
import { StatusDot } from '../StatusDot';
import styles from './FeedItem.module.css';

interface FeedItemProps {
  item: FeedItemType;
}

export function FeedItem({ item }: FeedItemProps) {
  const [expanded, setExpanded] = useState(false);

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

  const isLong = item.content.length > 120;
  const displayContent = isLong && !expanded
    ? item.content.slice(0, 120) + '...'
    : item.content;

  return (
    <div
      className={`${styles.item} ${getTypeStyle(item.type)}`}
      onClick={isLong ? () => setExpanded(!expanded) : undefined}
    >
      <span className={styles.timestamp}>
        {new Date(item.timestamp).toLocaleTimeString()}
      </span>

      <StatusDot status={getStatusType(item.type)} />

      <span className={`${styles.agent} ${getAgentStyle(item.agent)}`}>
        {item.agent}
      </span>

      <span className={`${styles.content} ${isLong ? styles.contentClickable : ''}`}>
        {displayContent}
      </span>
    </div>
  );
}
