import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { EdgeProperties } from './EdgeProperties'
import { mockWorkflowStep, mockWorkflowEdge } from '@/test/fixtures'
import type { WorkflowStep } from '@/types/workflow'

const step2: WorkflowStep = {
  ...mockWorkflowStep,
  id: 'step-002',
  name: 'Second Step',
}

const steps = [mockWorkflowStep, step2]

describe('EdgeProperties', () => {
  it('renders from and to step names', () => {
    render(<EdgeProperties edge={mockWorkflowEdge} steps={steps} />)
    expect(screen.getByText('First Step')).toBeInTheDocument()
    expect(screen.getByText('Second Step')).toBeInTheDocument()
  })

  it('shows "Always" when condition is null', () => {
    render(<EdgeProperties edge={mockWorkflowEdge} steps={steps} />)
    expect(screen.getByText('Always')).toBeInTheDocument()
  })

  it('shows condition when set', () => {
    const edgeWithCondition = { ...mockWorkflowEdge, condition: 'status == "ok"' }
    render(<EdgeProperties edge={edgeWithCondition} steps={steps} />)
    expect(screen.getByText('status == "ok"')).toBeInTheDocument()
  })

  it('shows "Unknown" for missing step references', () => {
    render(<EdgeProperties edge={mockWorkflowEdge} steps={[]} />)
    const unknowns = screen.getAllByText('Unknown')
    expect(unknowns).toHaveLength(2)
  })

  it('renders connection section title', () => {
    render(<EdgeProperties edge={mockWorkflowEdge} steps={steps} />)
    expect(screen.getByText('Connection')).toBeInTheDocument()
  })
})
