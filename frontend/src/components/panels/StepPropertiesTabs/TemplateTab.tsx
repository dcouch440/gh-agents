import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { PropertyRow, CodeEditor, PropertySelect, VariableChipStrip } from '@/components/primitives'
import { DESIGN } from '@/constants'
import { EDITOR_CONTAINER_SX } from './constants'
import type { Extension } from '@codemirror/state'
import type { VariableCompletion } from '@/utils/variableContext'
import type { WorkflowStep } from '@/types/workflow'
import type { PromptTemplate } from '@/types/template'
import type { PropertySelectOption } from '@/components/primitives'

type TemplateTabProps = {
  step: WorkflowStep
  readOnly: boolean
  templatesMap: Map<string, PromptTemplate>
  templateOptions: PropertySelectOption[]
  completions: VariableCompletion[]
  onCopyVariable: (label: string) => void
  onFieldChange: (field: 'name' | 'prompt_template' | 'system_prompt_suffix', value: string) => void
  onTemplateChange: (templateId: string | null) => void
  autocompleteExtension: Extension
}

function TemplateTab({
  step,
  readOnly,
  templatesMap,
  templateOptions,
  completions,
  onCopyVariable,
  onFieldChange,
  onTemplateChange,
  autocompleteExtension,
}: TemplateTabProps) {
  return (
    <Box
      sx={{
        flex: 1,
        display: 'flex',
        flexDirection: 'column',
        minHeight: 0,
      }}
    >
      {/* Template selector */}
      {readOnly ? (
        step.prompt_template_id ? (
          <Box sx={{ px: '16px', py: '8px' }}>
            <PropertyRow label="Template" value={templatesMap.get(step.prompt_template_id)?.name ?? 'Unknown'} />
          </Box>
        ) : null
      ) : (
        <Box sx={{ pb: '4px' }}>
          <Typography
            sx={{
              fontSize: 10,
              fontWeight: 500,
              color: 'text.secondary',
              textTransform: 'uppercase',
              letterSpacing: '0.04em',
              px: '16px',
              pt: '8px',
              pb: '2px',
            }}
          >
            Template
          </Typography>
          <PropertySelect
            value={step.prompt_template_id}
            options={templateOptions}
            onChange={onTemplateChange}
            placeholder="Select template..."
            allowNone
            accentColor={DESIGN.PORT_JSON}
          />
        </Box>
      )}

      {/* Available variables */}
      <VariableChipStrip completions={completions} onCopy={onCopyVariable} />

      {/* Editor */}
      <Box sx={EDITOR_CONTAINER_SX}>
        <CodeEditor
          key={`tpl-${step.id}`}
          value={step.prompt_template}
          onChange={(v: string) => {
            onFieldChange('prompt_template', v)
          }}
          language="markdown"
          placeholder="Enter prompt template..."
          height="100%"
          readOnly={readOnly}
          showLineNumbers
          extensions={[autocompleteExtension]}
        />
      </Box>
    </Box>
  )
}

export { TemplateTab }
export type { TemplateTabProps }
