import { useCallback } from 'react'
import IconButton from '@mui/material/IconButton'
import DeleteOutlined from '@mui/icons-material/DeleteOutlined'
import { useStore, workflowStore } from '@/stores'
import { assistantSessionStore } from '@/stores/assistantSessionStore'

type ChatClearButtonProps = {
  stepId: string
}

function ChatClearButton({ stepId }: ChatClearButtonProps) {
  const workflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const messages = useStore(assistantSessionStore.store, assistantSessionStore.selectMessages(stepId))
  const streaming = useStore(assistantSessionStore.store, assistantSessionStore.selectStreaming(stepId))

  const handleClear = useCallback(() => {
    if (!workflowId) return
    if (window.confirm('Clear chat history?')) {
      void assistantSessionStore.clearMessages(workflowId, stepId)
    }
  }, [workflowId, stepId])

  return (
    <IconButton
      className="nodrag"
      onClick={handleClear}
      disabled={streaming || messages.length === 0}
      size="small"
      sx={{
        p: 0.25,
        border: 'none',
        background: 'none',
        opacity: 0.4,
        '&:hover': { opacity: 1, background: 'none' },
      }}
    >
      <DeleteOutlined sx={{ fontSize: 18 }} />
    </IconButton>
  )
}

export { ChatClearButton }
