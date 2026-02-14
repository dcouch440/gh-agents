import { useCallback } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import CircularProgress from '@mui/material/CircularProgress'
import { useStore, workflowStore, contextMentionStore } from '@/stores'
import { useAssistantSession } from '@/hooks/useAssistantSession'
import { ChatPanel, StreamingMessage } from '@/components/chat'
import { ARCHETYPE_CONFIGS } from '../archetypes'
import type { Archetype } from '../archetypes'
import { ChatHeader } from './ChatHeader'
import { PanelOverlay } from './panel'

type ChatTabProps = {
  stepId: string
  archetype: Archetype
}

function ChatTab({ stepId, archetype }: ChatTabProps) {
  const workflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const { messages, streamingSegments, isLoading, error, streaming, activePanel, sendMessage, clearHistory, dismissPanel, submitPanelSelections } = useAssistantSession(workflowId, stepId)

  const handleClear = useCallback(() => {
    if (window.confirm('Clear chat history?')) {
      clearHistory()
    }
  }, [clearHistory])

  const handleSend = useCallback(
    (message: string) => {
      sendMessage(message)
      contextMentionStore.clearStep(stepId)
    },
    [sendMessage, stepId],
  )

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
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%', position: 'relative' }}>
      <ChatHeader stepId={stepId} onClear={handleClear} disabled={streaming || messages.length === 0} />
      <ChatPanel
        messages={messages}
        onSend={handleSend}
        streaming={streaming}
        disabled={streaming}
        streamingContent={streamingContent}
        emptyMessage={ARCHETYPE_CONFIGS[archetype].chatEmptyMessage}
        stepId={stepId}
      />
      {activePanel ? (
        <PanelOverlay
          content={activePanel.content}
          submitLabel={activePanel.submitLabel}
          onSubmit={submitPanelSelections}
          onDismiss={dismissPanel}
        />
      ) : null}
    </Box>
  )
}

export { ChatTab }
export type { ChatTabProps }
