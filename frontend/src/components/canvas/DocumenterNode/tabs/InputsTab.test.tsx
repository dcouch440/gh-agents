import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/render'
import { InputsTab } from './InputsTab'

describe('InputsTab', () => {
  it('renders empty state when no upstream connections exist', () => {
    render(<InputsTab upstreamStepNames={[]} />)
    expect(screen.getByText('No upstream connections')).toBeInTheDocument()
  })

  it('renders upstream step name chips', () => {
    render(<InputsTab upstreamStepNames={['Parse Input', 'Fetch Data']} />)
    expect(screen.getByText('Upstream Inputs')).toBeInTheDocument()
    expect(screen.getByText('Parse Input')).toBeInTheDocument()
    expect(screen.getByText('Fetch Data')).toBeInTheDocument()
  })

  it('renders a single upstream step name', () => {
    render(<InputsTab upstreamStepNames={['Only Step']} />)
    expect(screen.getByText('Upstream Inputs')).toBeInTheDocument()
    expect(screen.getByText('Only Step')).toBeInTheDocument()
  })
})
