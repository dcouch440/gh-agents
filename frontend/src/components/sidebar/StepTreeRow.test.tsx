import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/render'
import { StepTreeRow } from './StepTreeRow'
import type { StepTreeRowProps } from './StepTreeRow'

const makeProps = (overrides: Partial<StepTreeRowProps> = {}): StepTreeRowProps => ({
  name: 'Research',
  stepId: 'step-1',
  executionMode: 'single',
  gutter: [],
  status: 'idle',
  output: null,
  error: null,
  isExpanded: false,
  isOutputExpanded: false,
  onToggle: () => {},
  onToggleOutputExpand: () => {},
  designStatus: null,
  designProgress: null,
  pinned: false,
  onTogglePin: () => {},
  ...overrides,
})

describe('StepTreeRow', () => {
  it('shows the live phase marker while a design is running', () => {
    render(<StepTreeRow {...makeProps({ designStatus: 'running', designProgress: 'designing agents' })} />)
    expect(screen.getByText('designing agents')).toBeInTheDocument()
  })

  it('shows why a design failed', () => {
    // A failed design used to colour the row red and say nothing about it.
    render(<StepTreeRow {...makeProps({
      status: 'error',
      designStatus: 'failed',
      designProgress: 'System node agent timed out after 120s',
    })} />)
    expect(screen.getByText('System node agent timed out after 120s')).toBeInTheDocument()
  })

  it('stays quiet for a design that has nothing to report', () => {
    render(<StepTreeRow {...makeProps({ designStatus: 'completed', designProgress: 'stale marker' })} />)
    expect(screen.queryByText('stale marker')).not.toBeInTheDocument()
  })
})
