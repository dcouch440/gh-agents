import { useEffect, useCallback } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Paper from '@mui/material/Paper'
import Chip from '@mui/material/Chip'
import Fade from '@mui/material/Fade'
import { useStore, workflowStore, contextPickerStore } from '@/stores'
import { useAssistantSession } from '@/hooks/useAssistantSession'
import { Button } from '@/components/primitives'
import { formatEntityContext } from '@/utils/formatEntityContext'
import type { PickableEntity } from '@/stores/contextPickerStore'

const ENTITY_LABELS: Record<string, string> = {
  'agent': 'Agent',
  'prompt-template': 'Prompt Template',
  'output-schema': 'Output Schema',
  'workflow-step': 'Workflow Step',
  'document': 'Document',
  'context-node': 'Context Node',
}

const buildPreview = (entity: PickableEntity) => {
  const lines = formatEntityContext(entity).split('\n')
  // Show first 3 lines max for the compact preview
  const preview = lines.slice(0, 3).join('\n')
  return lines.length > 3 ? `${preview}\n...` : preview
}

function ContextPickerOverlay() {
  const isActive = useStore(contextPickerStore.store, contextPickerStore.selectActive)
  const pendingEntity = useStore(contextPickerStore.store, contextPickerStore.selectPendingEntity)
  const targetStepId = useStore(contextPickerStore.store, contextPickerStore.selectTargetStepId)
  const workflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)

  const { sendMessage } = useAssistantSession(workflowId, targetStepId ?? '')

  // ESC key exits picking mode
  useEffect(() => {
    if (!isActive) return
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        contextPickerStore.deactivate()
      }
    }
    document.addEventListener('keydown', handler)
    return () => {
      document.removeEventListener('keydown', handler)
    }
  }, [isActive])

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      contextPickerStore.reset()
    }
  }, [])

  const handleConfirm = useCallback(() => {
    if (!pendingEntity) return
    const formatted = formatEntityContext(pendingEntity)
    sendMessage(formatted)
    contextPickerStore.dismissPending()
  }, [pendingEntity, sendMessage])

  const handleCancel = useCallback(() => {
    contextPickerStore.dismissPending()
  }, [])

  return (
    <Fade in={pendingEntity !== null} unmountOnExit>
      <Paper
        elevation={8}
        sx={{
          position: 'fixed',
          bottom: 24,
          left: '50%',
          transform: 'translateX(-50%)',
          zIndex: 1400,
          width: 360,
          borderRadius: 2,
          overflow: 'hidden',
          border: 1,
          borderColor: 'divider',
        }}
      >
        {pendingEntity ? (
          <Box sx={{ p: 2 }}>
            {/* Header: kind chip + name */}
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
              <Chip
                label={ENTITY_LABELS[pendingEntity.kind] ?? pendingEntity.kind}
                size="small"
                variant="outlined"
                sx={{ fontSize: 10, height: 20 }}
              />
              <Typography sx={{ fontSize: 13, fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                {pendingEntity.name}
              </Typography>
            </Box>

            {/* Compact preview */}
            <Box
              sx={{
                p: 1,
                mb: 1.5,
                borderRadius: 1,
                bgcolor: 'action.hover',
                fontFamily: 'monospace',
                fontSize: 10,
                lineHeight: 1.4,
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-word',
                maxHeight: 72,
                overflow: 'hidden',
                color: 'text.secondary',
              }}
            >
              {buildPreview(pendingEntity)}
            </Box>

            {/* Actions */}
            <Box sx={{ display: 'flex', gap: 1, justifyContent: 'flex-end' }}>
              <Button variant="secondary" onClick={handleCancel} size="small">
                Skip
              </Button>
              <Button variant="primary" onClick={handleConfirm} size="small">
                Send to Assistant
              </Button>
            </Box>
          </Box>
        ) : null}
      </Paper>
    </Fade>
  )
}

export { ContextPickerOverlay }
