import { useMemo } from 'react'
import { PropertySection, PropertyRow } from '@/components/primitives'
import { useCollapsible } from '@/hooks'
import { Collections } from '@/utils/collections'
import type { WorkflowStepEdge, WorkflowStep } from '@/types/workflow'

type EdgePropertiesProps = {
  edge: WorkflowStepEdge
  steps: WorkflowStep[]
}

function EdgeProperties({ edge, steps }: EdgePropertiesProps) {
  const connection = useCollapsible(true)

  const stepsById = useMemo(() => Collections.indexById(steps), [steps])
  const fromStep = stepsById.get(edge.from_step_id)
  const toStep = stepsById.get(edge.to_step_id)

  return (
    <PropertySection title="Connection" {...connection}>
      <PropertyRow label="From" value={fromStep?.name ?? fromStep?.execution_mode ?? 'Unknown'} />
      <PropertyRow label="To" value={toStep?.name ?? toStep?.execution_mode ?? 'Unknown'} last />
    </PropertySection>
  )
}

export { EdgeProperties }
export type { EdgePropertiesProps }
