import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@/test/render'
import { PromptTab } from './PromptTab'

vi.mock('@/components/primitives/CodeEditor', () => ({
  CodeEditor: ({ value, onChange, placeholder }: { value: string; onChange: (v: string) => void; placeholder?: string }) => (
    <textarea data-testid="code-editor" value={value} onChange={(e) => onChange(e.target.value)} placeholder={placeholder} />
  ),
}))

describe('PromptTab', () => {
  const onChange = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders CodeEditor with the provided value', () => {
    render(<PromptTab value="Generate documentation" onChange={onChange} />)
    expect(screen.getByTestId('code-editor')).toHaveValue('Generate documentation')
  })

  it('calls onChange when editor content changes', () => {
    render(<PromptTab value="" onChange={onChange} />)
    fireEvent.change(screen.getByTestId('code-editor'), { target: { value: 'New prompt' } })
    expect(onChange).toHaveBeenCalledWith('New prompt')
  })

  it('shows placeholder when value is empty', () => {
    render(<PromptTab value="" onChange={onChange} />)
    expect(screen.getByPlaceholderText('Enter your prompt...')).toBeInTheDocument()
  })
})
