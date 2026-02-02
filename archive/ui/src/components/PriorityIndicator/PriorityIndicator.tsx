interface PriorityIndicatorProps {
  priority: string;
}

const indicators: Record<string, { icon: string; color: string }> = {
  urgent: { icon: '◆', color: 'var(--color-status-error)' },
  high: { icon: '◇', color: 'var(--color-status-warning)' },
  normal: { icon: '○', color: 'var(--color-text-secondary)' },
  low: { icon: '·', color: 'var(--color-text-tertiary)' },
};

export function PriorityIndicator({ priority }: PriorityIndicatorProps) {
  const { icon, color } = indicators[priority] ?? indicators.normal;
  return <span style={{ color }} title={priority}>{icon}</span>;
}
