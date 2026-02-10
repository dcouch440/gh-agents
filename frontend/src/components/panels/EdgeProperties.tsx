import { useMemo } from 'react'
import { PropertySection, PropertyRow } from '@/components/primitives'
import { useCollapsible } from '@/hooks'
import type { WorkflowStepEdge, WorkflowStep } from '@/types/workflow'

type EdgePropertiesProps = {
  edge: WorkflowStepEdge
  steps: WorkflowStep[]
}

function EdgeProperties({ edge, steps }: EdgePropertiesProps) {
  const connection = useCollapsible(true)

  const fromStep = useMemo(() => steps.find((s) => s.id === edge.from_step_id), [steps, edge.from_step_id])

  const toStep = useMemo(() => steps.find((s) => s.id === edge.to_step_id), [steps, edge.to_step_id])

  return (
    <PropertySection title="Connection" {...connection}>
      <PropertyRow label="From" value={fromStep?.name ?? fromStep?.execution_mode ?? 'Unknown'} />
      <PropertyRow label="To" value={toStep?.name ?? toStep?.execution_mode ?? 'Unknown'} last />
    </PropertySection>
  )
}

export { EdgeProperties }
export type { EdgePropertiesProps }
