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
    expect(container.querySelector('.tool-tile--completed')).toBeInTheDocument()
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

  it('renders circle element', () => {
    const { container } = render(
      <ToolActivityBox toolName="test" status="running" durationMs={null} />,
    )
    expect(container.querySelector('.tool-tile__circle')).toBeInTheDocument()
  })

  it('shows checkmark when completed', () => {
    const { container } = render(
      <ToolActivityBox toolName="test" status="completed" durationMs={1000} />,
    )
    expect(container.querySelector('.tool-tile__circle')?.textContent).toBe('\u2713')
  })

  it('shows X when error', () => {
    const { container } = render(
      <ToolActivityBox toolName="test" status="error" durationMs={1000} />,
    )
    expect(container.querySelector('.tool-tile__circle')?.textContent).toBe('\u2717')
  })
})
