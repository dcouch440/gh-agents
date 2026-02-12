import { useCallback } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import CircularProgress from '@mui/material/CircularProgress'
import { useStore, workflowStore } from '@/stores'
import { useAssistantSession } from '@/hooks/useAssistantSession'
import { ChatPanel } from '@/components/chat'
import { AssistantHeader } from './AssistantHeader'

type AssistantTabProps = {
  stepId: string
}

function AssistantTab({ stepId }: AssistantTabProps) {
  const workflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const { messages, isLoading, error, streaming, sendMessage, clearHistory } = useAssistantSession(workflowId, stepId)

  const handleClear = useCallback(() => {
    if (window.confirm('Clear assistant chat history?')) {
      clearHistory()
    }
  }, [clearHistory])

  if (!workflowId) return null

  if (isLoading) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
        <CircularProgress size={20} />
      </Box>
    )
  }

  if (error) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', p: 2 }}>
        <Typography variant="body2" color="error">
          {error}
        </Typography>
      </Box>
    )
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <AssistantHeader onClear={handleClear} disabled={streaming || messages.length === 0} />
      <ChatPanel
        messages={messages}
        onSend={sendMessage}
        streaming={streaming}
        disabled={isLoading}
        emptyMessage="Ask me to help set up documents for this step."
      />
    </Box>
  )
}

export { AssistantTab }
export type { AssistantTabProps }
