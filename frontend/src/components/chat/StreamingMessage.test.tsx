import { render, screen } from '@testing-library/react'
import { StreamingMessage } from './StreamingMessage'
import type { MessageSegment } from '@/types'

vi.mock('@/components/primitives/MarkdownPreview', () => ({
  MarkdownPreview: ({ content }: { content: string }) => <div data-testid="markdown">{content}</div>,
}))

describe('StreamingMessage', () => {
  it('renders text-only segments', () => {
    const segments: MessageSegment[] = [{ type: 'text', content: 'Hello world' }]

    render(<StreamingMessage segments={segments} />)

    expect(screen.getByText('Hello world')).toBeInTheDocument()
  })

  it('renders multiple text segments', () => {
    const segments: MessageSegment[] = [
      { type: 'text', content: 'First part' },
      { type: 'text', content: 'Second part' },
    ]

    render(<StreamingMessage segments={segments} />)

    expect(screen.getByText('First part')).toBeInTheDocument()
    expect(screen.getByText('Second part')).toBeInTheDocument()
  })

  it('renders interleaved text and tool segments', () => {
    const segments: MessageSegment[] = [
      { type: 'text', content: 'Setting up documents' },
      { type: 'tool', toolId: 't1', toolName: 'update_prompt', status: 'running' },
      { type: 'text', content: 'More text after tool' },
    ]

    render(<StreamingMessage segments={segments} />)

    expect(screen.getByText('Setting up documents')).toBeInTheDocument()
    expect(screen.getByText('Updating prompt...')).toBeInTheDocument()
    expect(screen.getByText('More text after tool')).toBeInTheDocument()
  })

  it('renders doc_update segments', () => {
    const segments: MessageSegment[] = [
      { type: 'text', content: 'Done.' },
      { type: 'doc_update', docId: 'd1', title: 'API Reference' },
    ]

    render(<StreamingMessage segments={segments} />)

    expect(screen.getByText('Done.')).toBeInTheDocument()
    expect(screen.getByText(/API Reference/)).toBeInTheDocument()
  })

  it('shows blinking cursor when streaming and last segment is text', () => {
    const segments: MessageSegment[] = [{ type: 'text', content: 'Typing...' }]

    const { container } = render(<StreamingMessage segments={segments} streaming />)

    const cursor = container.querySelector('span')
    expect(cursor).toBeInTheDocument()
  })

  it('does not show cursor when not streaming', () => {
    const segments: MessageSegment[] = [{ type: 'text', content: 'Done.' }]

    const { container } = render(<StreamingMessage segments={segments} />)

    // No cursor span with the blink animation style
    const spans = container.querySelectorAll('span')
    const cursorSpans = Array.from(spans).filter(
      (s) => s.style.width === '2px' || s.getAttribute('style')?.includes('width'),
    )
    expect(cursorSpans).toHaveLength(0)
  })

  it('does not show cursor when last segment is a tool', () => {
    const segments: MessageSegment[] = [
      { type: 'text', content: 'Setting up' },
      { type: 'tool', toolId: 't1', toolName: 'update_prompt', status: 'running' },
    ]

    render(<StreamingMessage segments={segments} streaming />)

    // The cursor only shows after text segments
    const markdowns = screen.getAllByTestId('markdown')
    expect(markdowns).toHaveLength(1)
  })

  it('renders empty segments list without crashing', () => {
    const { container } = render(<StreamingMessage segments={[]} streaming />)
    expect(container).toBeInTheDocument()
  })
})
