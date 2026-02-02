interface StatusDotProps {
  status: 'idle' | 'active' | 'success' | 'warning' | 'error';
  pulse?: boolean;
}

export function StatusDot({ status, pulse = false }: StatusDotProps) {
  const colors = {
    idle: 'bg-text-tertiary',
    active: 'bg-accent-secondary',
    success: 'bg-status-success',
    warning: 'bg-status-warning',
    error: 'bg-status-error',
  };

  return (
    <span
      className={`inline-block w-2 h-2 rounded-full ${colors[status]}
                  ${pulse && status === 'active' ? 'animate-pulse' : ''}`}
    />
  );
}
