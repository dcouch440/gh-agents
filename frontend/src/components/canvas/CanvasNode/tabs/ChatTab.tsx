import { useCallback, useEffect } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import CircularProgress from '@mui/material/CircularProgress'
import { useStore, workflowStore, contextMentionStore } from '@/stores'
import { useAssistantSession } from '@/hooks/useAssistantSession'
import { ChatPanel, StreamingMessage, ThinkingIndicator } from '@/components/chat'
import { ARCHETYPE_CONFIGS } from '../registry'
import type { Archetype } from '../registry'
import { PanelOverlay } from './panel'

type ChatTabProps = {
  stepId: string
  archetype: Archetype
  focusMode?: boolean
}

function ChatTab({ stepId, archetype, focusMode }: ChatTabProps) {
  const workflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const { messages, streamingSegments, isLoading, error, streaming, activePanel, sendMessage, cancelGeneration, dismissPanel, submitPanelSelections } = useAssistantSession(workflowId, stepId)

  const handleSend = useCallback(
    (message: string) => {
      sendMessage(message)
      contextMentionStore.clearStep(stepId)
    },
    [sendMessage, stepId],
  )

  // Document-level Escape to cancel when input doesn't have focus
  useEffect(() => {
    if (!streaming) return
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault()
        cancelGeneration()
      }
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [streaming, cancelGeneration])

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

  const streamingContent = streaming ? (
    streamingSegments.length > 0 ? (
      <StreamingMessage segments={streamingSegments} streaming />
    ) : (
      <ThinkingIndicator />
    )
  ) : undefined

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%', position: 'relative' }}>
      <ChatPanel
        messages={messages}
        onSend={handleSend}
        onCancel={streaming ? cancelGeneration : undefined}
        streaming={streaming}
        disabled={streaming}
        streamingContent={streamingContent}
        emptyMessage={ARCHETYPE_CONFIGS[archetype].chatEmptyMessage}
        stepId={stepId}
        focusMode={focusMode}
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
