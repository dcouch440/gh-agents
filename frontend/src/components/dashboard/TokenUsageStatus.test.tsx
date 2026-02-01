import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { TokenUsageStatus } from './TokenUsageStatus'
import type { UsageSummary } from '@/types'

const usage: UsageSummary[] = [
  { tier: 'orch', model_id: 'opus', total_input: 50000, total_output: 12000, call_count: 10 },
  { tier: 'work', model_id: 'sonnet', total_input: 120000, total_output: 30000, call_count: 45 },
]

describe('TokenUsageStatus', () => {
  it('renders tier rows', () => {
    render(<TokenUsageStatus usage={usage} />)
    expect(screen.getByText('orch')).toBeInTheDocument()
    expect(screen.getByText('work')).toBeInTheDocument()
  })

  it('formats tokens with k suffix', () => {
    render(<TokenUsageStatus usage={usage} />)
    expect(screen.getByText('50.0k')).toBeInTheDocument()
    expect(screen.getByText('12.0k')).toBeInTheDocument()
  })

  it('renders totals row', () => {
    render(<TokenUsageStatus usage={usage} />)
    expect(screen.getByText('TOTAL')).toBeInTheDocument()
    expect(screen.getByText('55')).toBeInTheDocument()
    expect(screen.getByText('170.0k')).toBeInTheDocument()
  })
})
