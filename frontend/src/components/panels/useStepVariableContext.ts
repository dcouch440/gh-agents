import { useMemo, useEffect, useRef } from 'react'
import { workflowStore } from '@/stores'
import { buildVariableCompletions } from '@/utils/variableContext'
import { createVariableAutocomplete } from '@/utils/variableAutocomplete'
import type { Extension } from '@codemirror/state'
import type { VariableCompletion } from '@/utils/variableContext'
import type { WorkflowStep, OutputSchema } from '@/types'

type VariableContext = ReturnType<typeof buildVariableCompletions>

type UseStepVariableContextArgs = {
  upstreamIds: readonly string[]
  stepsById: ReadonlyMap<string, WorkflowStep>
  schemasMap: ReadonlyMap<string, OutputSchema>
  step: WorkflowStep
}

type UseStepVariableContextResult = {
  variableContext: VariableContext
  autocompleteExtension: Extension
}

const useStepVariableContext = ({ upstreamIds, stepsById, schemasMap, step }: UseStepVariableContextArgs): UseStepVariableContextResult => {
  // CodeMirror extensions must be stable (created once), but need access to
  // latest completions. We use a ref-based getter: the extension captures a
  // function that reads completionsRef.current lazily when autocomplete
  // triggers — never during render.
  const completionsRef = useRef<VariableCompletion[]>([])

  const variableContext = useMemo(
    () => buildVariableCompletions(upstreamIds, stepsById, schemasMap, step),
    [upstreamIds, stepsById, schemasMap, step],
  )

  useEffect(() => {
    completionsRef.current = variableContext.completions
  }, [variableContext])

  // Auto-set output_variable_name on upstream steps that don't have one,
  // so the backend can resolve variable references at execution time.
  // Uses patchStepSilent to avoid marking the form dirty for auto-derived values.
  useEffect(() => {
    for (const { stepId, derivedName } of variableContext.autoNamed) {
      workflowStore.patchStepSilent(stepId, { output_variable_name: derivedName })
    }
  }, [variableContext.autoNamed])

  // The getter reads completionsRef.current lazily at autocomplete-trigger time,
  // not during render. The useMemo just creates the stable extension wrapper.
  const autocompleteExtension = useMemo<Extension>(
    // eslint-disable-next-line react-hooks/refs -- lazy getter, ref read at autocomplete time only
    () => createVariableAutocomplete(() => completionsRef.current),
    [],
  )

  return { variableContext, autocompleteExtension }
}

export { useStepVariableContext }
export type { UseStepVariableContextArgs, UseStepVariableContextResult }
