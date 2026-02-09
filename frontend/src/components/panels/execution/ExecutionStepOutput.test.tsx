import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ExecutionStepOutput } from './ExecutionStepOutput'

vi.mock('@/components/primitives', () => ({
  MarkdownPreview: ({ content }: { content: string }) => <div data-testid="markdown">{content}</div>,
}))

describe('ExecutionStepOutput', () => {
  it('renders error in styled box', () => {
    render(<ExecutionStepOutput output={null} error="Connection refused" />)
    expect(screen.getByText('Connection refused')).toBeInTheDocument()
  })

  it('renders markdown output', () => {
    render(<ExecutionStepOutput output="# Hello World" error={null} />)
    expect(screen.getByTestId('markdown')).toHaveTextContent('# Hello World')
  })

  it('renders placeholder when no output or error', () => {
    render(<ExecutionStepOutput output={null} error={null} />)
    expect(screen.getByText('No output yet')).toBeInTheDocument()
  })

  it('prioritizes error over output when both present', () => {
    render(<ExecutionStepOutput output="some output" error="some error" />)
    expect(screen.getByText('some error')).toBeInTheDocument()
    expect(screen.queryByTestId('markdown')).not.toBeInTheDocument()
  })
})
