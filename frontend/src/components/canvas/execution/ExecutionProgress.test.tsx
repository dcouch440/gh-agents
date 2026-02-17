import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'

import { ExecutionProgress } from './ExecutionProgress'

describe('ExecutionProgress', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders progress label', () => {
    render(<ExecutionProgress completed={3} total={10} label="Items" />)
    expect(screen.getByText('3/10 Items')).toBeInTheDocument()
  })

  it('shows count without label when label omitted', () => {
    render(<ExecutionProgress completed={5} total={8} />)
    expect(screen.getByText('5/8')).toBeInTheDocument()
  })

  it('handles zero total', () => {
    render(<ExecutionProgress completed={0} total={0} />)
    expect(screen.getByText('0/0')).toBeInTheDocument()
  })
})
