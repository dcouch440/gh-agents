import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { CodeEditor, VariableChipStrip } from '@/components/primitives'
import { EDITOR_CONTAINER_SX, MUTED_EDITOR_CONTAINER_SX, SECTION_LABEL_SX } from './constants'
import type { Extension } from '@codemirror/state'
import type { VariableCompletion } from '@/utils/variableContext'
import type { WorkflowStep } from '@/types/workflow'
import type { Agent } from '@/types/agent'

type SystemTabProps = {
  step: WorkflowStep
  agent: Agent | undefined
  readOnly: boolean
  completions: VariableCompletion[]
  onCopyVariable: (label: string) => void
  onFieldChange: (field: 'name' | 'prompt_template' | 'system_prompt_suffix', value: string) => void
  autocompleteExtension: Extension
}

function SystemTab({ step, agent, readOnly, completions, onCopyVariable, onFieldChange, autocompleteExtension }: SystemTabProps) {
  return (
    <Box
      sx={{
        flex: 1,
        display: 'flex',
        flexDirection: 'column',
        minHeight: 0,
        overflow: 'auto',
      }}
    >
      {/* Agent base system prompt (read-only, muted) */}
      <Typography sx={SECTION_LABEL_SX}>Agent System Prompt</Typography>
      {agent ? (
        <Box
          sx={{
            ...MUTED_EDITOR_CONTAINER_SX,
            flex: 'none',
            minHeight: 120,
            maxHeight: 240,
          }}
        >
          <CodeEditor
            key={`sys-base-${step.id}-${agent.id}`}
            value={agent.system_prompt}
            onChange={() => {}}
            language="markdown"
            placeholder="No system prompt defined on agent"
            height="100%"
            readOnly
          />
        </Box>
      ) : (
        <Box sx={{ px: '16px', py: '12px' }}>
          <Typography
            sx={{
              fontSize: 11,
              color: 'text.disabled',
              fontStyle: 'italic',
            }}
          >
            Select an agent to view its system prompt.
          </Typography>
        </Box>
      )}

      {/* Divider */}
      <Box sx={{ borderTop: 1, borderColor: 'divider' }} />

      {/* Available variables */}
      <VariableChipStrip completions={completions} onCopy={onCopyVariable} />

      {/* Step-level extension (editable) */}
      <Typography sx={SECTION_LABEL_SX}>Step Extension</Typography>
      <Box sx={{ ...EDITOR_CONTAINER_SX, flex: 1, minHeight: 120 }}>
        <CodeEditor
          key={`sys-ext-${step.id}`}
          value={step.system_prompt_suffix ?? ''}
          onChange={(v: string) => {
            onFieldChange('system_prompt_suffix', v)
          }}
          language="markdown"
          placeholder="Enter system prompt extension..."
          height="100%"
          readOnly={readOnly}
          extensions={[autocompleteExtension]}
        />
      </Box>
    </Box>
  )
}

export { SystemTab }
export type { SystemTabProps }
