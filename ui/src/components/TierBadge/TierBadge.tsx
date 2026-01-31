import styles from './TierBadge.module.css';

interface TierBadgeProps {
  tier: 'orchestrator' | 'worker' | 'utility';
}

const tierIcons: Record<string, string> = {
  orchestrator: '♛',
  worker: '⚒',
  utility: '⚙',
};

export function TierBadge({ tier }: TierBadgeProps) {
  return (
    <span className={`${styles.badge} ${styles[tier]}`}>
      {tierIcons[tier]} {tier}
    </span>
  );
}
