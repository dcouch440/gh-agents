import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ExecutionPanel } from './ExecutionPanel'

describe('ExecutionPanel', () => {
  it('renders placeholder empty state', () => {
    render(<ExecutionPanel />)
    expect(screen.getByText('Execution view coming soon')).toBeInTheDocument()
  })
})
