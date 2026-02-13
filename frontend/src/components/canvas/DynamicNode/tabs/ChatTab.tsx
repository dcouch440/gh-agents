import { useCallback } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import CircularProgress from '@mui/material/CircularProgress'
import { useStore, workflowStore } from '@/stores'
import { useAssistantSession } from '@/hooks/useAssistantSession'
import { ChatPanel, StreamingMessage } from '@/components/chat'
import { ARCHETYPE_CONFIGS } from '../archetypes'
import type { Archetype } from '../archetypes'
import { ChatHeader } from './ChatHeader'

type ChatTabProps = {
  stepId: string
  archetype: Archetype
}

function ChatTab({ stepId, archetype }: ChatTabProps) {
  const workflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const { messages, streamingSegments, isLoading, error, streaming, sendMessage, clearHistory } = useAssistantSession(workflowId, stepId)

  const handleClear = useCallback(() => {
    if (window.confirm('Clear chat history?')) {
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

  if (error && !streaming && messages.length === 0) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', p: 2 }}>
        <Typography variant="body2" color="error">
          {error}
        </Typography>
      </Box>
    )
  }

  const streamingContent =
    streaming && streamingSegments.length > 0 ? (
      <StreamingMessage segments={streamingSegments} streaming />
    ) : undefined

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <ChatHeader onClear={handleClear} disabled={streaming || messages.length === 0} />
      <ChatPanel
        messages={messages}
        onSend={sendMessage}
        streaming={streaming}
        disabled={streaming}
        streamingContent={streamingContent}
        emptyMessage={ARCHETYPE_CONFIGS[archetype].chatEmptyMessage}
      />
    </Box>
  )
}

export { ChatTab }
export type { ChatTabProps }
