import { Component, type ReactNode } from 'react';
import { AlertTriangle, RotateCcw } from 'lucide-react';

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  handleRetry = () => {
    this.setState({ hasError: false, error: null });
  };

  render() {
    if (this.state.hasError) {
      return (
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          minHeight: '50vh',
          gap: '1rem',
          color: 'var(--color-text-secondary)',
          padding: '2rem',
        }}>
          <AlertTriangle size={40} style={{ color: 'var(--color-status-warning)' }} />
          <h2 style={{ fontSize: '1.125rem', fontWeight: 600, color: 'var(--color-text-primary)' }}>
            Something went wrong
          </h2>
          <p style={{ fontSize: '0.875rem', textAlign: 'center', maxWidth: '24rem' }}>
            An unexpected error occurred. Try refreshing the page or click retry below.
          </p>
          {this.state.error && (
            <code style={{
              fontSize: '0.75rem',
              padding: '0.5rem 0.75rem',
              backgroundColor: 'var(--color-bg-tertiary)',
              borderRadius: '0.375rem',
              maxWidth: '32rem',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              color: 'var(--color-status-error)',
            }}>
              {this.state.error.message}
            </code>
          )}
          <button
            onClick={this.handleRetry}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '0.5rem',
              padding: '0.5rem 1rem',
              backgroundColor: 'var(--color-accent)',
              color: 'white',
              border: 'none',
              borderRadius: '0.375rem',
              cursor: 'pointer',
              fontSize: '0.8125rem',
              fontWeight: 500,
              marginTop: '0.5rem',
            }}
          >
            <RotateCcw size={14} />
            Retry
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
