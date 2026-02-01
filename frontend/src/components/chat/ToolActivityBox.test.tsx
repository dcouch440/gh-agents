import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ToolActivityBox } from './ToolActivityBox'

describe('ToolActivityBox', () => {
  it('renders tool name', () => {
    render(<ToolActivityBox toolName="search_files" status="running" durationMs={null} />)
    expect(screen.getByText('search_files')).toBeInTheDocument()
  })

  it('applies status class', () => {
    const { container } = render(
      <ToolActivityBox toolName="read_file" status="completed" durationMs={1200} />,
    )
    expect(container.querySelector('.tool-box--completed')).toBeInTheDocument()
  })

  it('shows duration when provided', () => {
    render(<ToolActivityBox toolName="test" status="completed" durationMs={1200} />)
    expect(screen.getByText('1.2s')).toBeInTheDocument()
  })

  it('shows milliseconds for short durations', () => {
    render(<ToolActivityBox toolName="test" status="running" durationMs={450} />)
    expect(screen.getByText('450ms')).toBeInTheDocument()
  })

  it('shows detail text when running', () => {
    render(
      <ToolActivityBox toolName="test" status="running" durationMs={null} detail="searching..." />,
    )
    expect(screen.getByText('searching...')).toBeInTheDocument()
  })

  it('hides detail when completed', () => {
    render(
      <ToolActivityBox toolName="test" status="completed" durationMs={500} detail="done" />,
    )
    expect(screen.queryByText('done')).not.toBeInTheDocument()
  })

  it('renders progress bar when not completed', () => {
    const { container } = render(
      <ToolActivityBox toolName="test" status="running" durationMs={null} progress={50} />,
    )
    const bar = container.querySelector('.tool-box__progress-bar')
    expect(bar).toBeInTheDocument()
    expect(bar).toHaveStyle({ width: '50%' })
  })

  it('hides progress bar when completed', () => {
    const { container } = render(
      <ToolActivityBox toolName="test" status="completed" durationMs={1000} />,
    )
    expect(container.querySelector('.tool-box__progress')).not.toBeInTheDocument()
  })

  it('shows correct prefix per status', () => {
    const { container: running } = render(
      <ToolActivityBox toolName="t" status="running" durationMs={null} />,
    )
    expect(running.querySelector('.tool-box__prefix')?.textContent).toBe('>')

    const { container: completed } = render(
      <ToolActivityBox toolName="t" status="completed" durationMs={100} />,
    )
    expect(completed.querySelector('.tool-box__prefix')?.textContent).toBe('\u2713')
  })
})
