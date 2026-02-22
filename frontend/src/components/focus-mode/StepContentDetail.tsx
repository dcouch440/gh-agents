import { useState, useCallback } from 'react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore } from '@/stores'
import { CodeEditor } from '@/components/primitives/CodeEditor'
import { TerminalBlock } from '@/components/primitives/terminal-renderer'
import { DetailShell } from './DetailShell'

type ContentViewMode = 'raw' | 'md'

type StepContentDetailProps = {
  stepId: string
  kind: 'input' | 'context'
  onClose: () => void
}

function StepContentDetail({ stepId, kind, onClose }: StepContentDetailProps) {
  const theme = useTheme()
  const step = useStore(workflowStore.store, workflowStore.selectStepById(stepId))
  const [viewMode, setViewMode] = useState<ContentViewMode>('raw')

  const accentColor = theme.palette.nodePalette[kind]
  const title = step?.name ?? (kind === 'input' ? 'Input' : 'Context')
  const content = step?.prompt_template ?? ''

  const handleChange = useCallback((value: string) => {
    workflowStore.patchStepLocal(stepId, { prompt_template: value })
  }, [stepId])

  return (
    <DetailShell title={title} accentColor={accentColor} onClose={onClose}>
      <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%', gap: 1 }}>
        {/* Raw / Md toggle */}
        <Box sx={{ display: 'flex', gap: 0.25, justifyContent: 'flex-end' }}>
          {(['raw', 'md'] as const).map((vm) => (
            <Box
              key={vm}
              onClick={() => {
                setViewMode(vm)
              }}
              sx={{
                px: 0.75,
                py: 0.25,
                borderRadius: '4px',
                fontSize: 10,
                fontWeight: 600,
                cursor: 'pointer',
                userSelect: 'none',
                color: viewMode === vm ? accentColor : 'text.disabled',
                backgroundColor: viewMode === vm ? `${accentColor}15` : 'transparent',
                transition: 'all 120ms ease',
                '&:hover': viewMode === vm ? {} : { color: 'text.secondary' },
              }}
            >
              {vm === 'raw' ? 'Raw' : 'Md'}
            </Box>
          ))}
        </Box>

        {/* Content area */}
        <Box sx={{ flex: 1, overflow: 'hidden' }}>
          {viewMode === 'raw' ? (
            <CodeEditor
              value={content}
              onChange={handleChange}
              placeholder={kind === 'input' ? 'Type your input here...' : 'Type your context here...'}
              height="100%"
            />
          ) : (
            <Box sx={{ overflow: 'auto', height: '100%' }}>
              <TerminalBlock content={content} />
            </Box>
          )}
        </Box>
      </Box>
    </DetailShell>
  )
}

export { StepContentDetail }
export type { StepContentDetailProps }
