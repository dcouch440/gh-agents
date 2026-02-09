import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ExecutionRunHeader } from './ExecutionRunHeader'

describe('ExecutionRunHeader', () => {
  it('shows running state with progress', () => {
    render(
      <ExecutionRunHeader
        isRunning={true}
        completedSteps={1}
        totalSteps={3}
        durationMs={null}
        error={null}
        startedAt="2025-01-01T00:00:00Z"
        completedAt={null}
      />,
    )
    expect(screen.getByText('Running...')).toBeInTheDocument()
    expect(screen.getByText('1 / 3 steps')).toBeInTheDocument()
  })

  it('shows completed state with duration', () => {
    render(
      <ExecutionRunHeader
        isRunning={false}
        completedSteps={3}
        totalSteps={3}
        durationMs={2500}
        error={null}
        startedAt="2025-01-01T00:00:00Z"
        completedAt="2025-01-01T00:00:02Z"
      />,
    )
    expect(screen.getByText('Completed')).toBeInTheDocument()
    expect(screen.getByText('3 / 3 steps')).toBeInTheDocument()
    expect(screen.getByText('2.5s')).toBeInTheDocument()
  })

  it('shows failed state with error', () => {
    render(
      <ExecutionRunHeader
        isRunning={false}
        completedSteps={1}
        totalSteps={3}
        durationMs={null}
        error="LLM timeout"
        startedAt="2025-01-01T00:00:00Z"
        completedAt="2025-01-01T00:00:05Z"
      />,
    )
    expect(screen.getByText('Failed')).toBeInTheDocument()
    expect(screen.getByText('LLM timeout')).toBeInTheDocument()
  })

  it('shows idle state when not running and no completion', () => {
    render(
      <ExecutionRunHeader
        isRunning={false}
        completedSteps={0}
        totalSteps={0}
        durationMs={null}
        error={null}
        startedAt={null}
        completedAt={null}
      />,
    )
    expect(screen.getByText('Idle')).toBeInTheDocument()
  })

  it('formats long duration with minutes', () => {
    render(
      <ExecutionRunHeader
        isRunning={false}
        completedSteps={5}
        totalSteps={5}
        durationMs={125000}
        error={null}
        startedAt="2025-01-01T00:00:00Z"
        completedAt="2025-01-01T00:02:05Z"
      />,
    )
    expect(screen.getByText('2m 5s')).toBeInTheDocument()
  })
})
