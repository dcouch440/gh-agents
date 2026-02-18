import { describe, it, expect } from 'vitest'
import { buildProtocolsByStep } from './canvasContextMenuUtils'
import type { StepProtocolLink } from '@/stores'

describe('buildProtocolsByStep', () => {
  it('builds lookup from step protocol links', () => {
    const stepProtocols: Readonly<Record<string, StepProtocolLink>> = {
      'step-1': { protocolType: 'workforce', protocolName: 'WF', portNames: [] },
      'step-2': { protocolType: 'room', protocolName: 'RM', portNames: [] },
    }
    const result = buildProtocolsByStep(stepProtocols)

    expect(result.get('step-1')).toEqual({ protocol_type: 'workforce' })
    expect(result.get('step-2')).toEqual({ protocol_type: 'room' })
    expect(result.size).toBe(2)
  })

  it('returns empty map for empty input', () => {
    expect(buildProtocolsByStep({}).size).toBe(0)
  })
})

