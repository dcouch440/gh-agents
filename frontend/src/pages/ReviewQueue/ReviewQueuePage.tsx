import { useState, useCallback } from 'react'
import { Box, Typography } from '@mui/material'
import { FadeIn } from '@/components/animation'
import { PageHeader } from '@/components/primitives/PageHeader'
import { LoadingSpinner } from '@/components/primitives/LoadingSpinner'
import { ErrorMessage } from '@/components/primitives/ErrorMessage'
import { EmptyState } from '@/components/primitives/EmptyState'
import { ChatPanel } from '@/components/chat/ChatPanel'
import { MarkdownPreview } from '@/components/primitives/MarkdownPreview'
import { ReviewCard, ApproveButton, CollapsibleSection } from '@/components/review'
import { useReviewQueue } from '@/hooks/useReviewQueue'
import { useInteractiveChat } from '@/hooks/useInteractiveChat'
import type { ChatMessageData } from '@/components/chat/ChatPanel'

function ReviewQueuePage() {
  const { executions, loading, error, reload } = useReviewQueue()

  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [inputOpen, setInputOpen] = useState(false)
  const [outputOpen, setOutputOpen] = useState(true)

  const chat = useInteractiveChat(selectedId ?? '')
  const selectedExecution = executions.find((e) => e.id === selectedId) ?? null

  const chatMessages: ChatMessageData[] = chat.messages
    .filter((m) => m.role === 'user' || m.role === 'assistant')
    .map((m) => ({
      id: m.id,
      role: m.role as 'user' | 'assistant',
      content: m.content,
    }))

  const handleSend = useCallback(
    (message: string) => {
      void chat.sendMessage(message)
    },
    [chat],
  )

  const handleApprove = useCallback(() => {
    void chat.approve().then(() => {
      setSelectedId(null)
      reload()
    })
  }, [chat, reload])

  const handleSelect = useCallback((id: string) => {
    setSelectedId(id)
    setInputOpen(false)
    setOutputOpen(true)
  }, [])

  if (loading && executions.length === 0) {
    return (
      <FadeIn>
        <Box sx={{ p: 3 }}>
          <PageHeader title="Review Queue" />
          <LoadingSpinner centered />
        </Box>
      </FadeIn>
    )
  }

  if (error) {
    return (
      <FadeIn>
        <Box sx={{ p: 3 }}>
          <PageHeader title="Review Queue" />
          <ErrorMessage message={error} onRetry={reload} />
        </Box>
      </FadeIn>
    )
  }

  if (executions.length === 0) {
    return (
      <FadeIn>
        <Box sx={{ p: 3 }}>
          <PageHeader title="Review Queue" />
          <EmptyState message="No executions awaiting review" />
        </Box>
      </FadeIn>
    )
  }

  return (
    <FadeIn>
      <Box sx={{ p: 3, height: 'calc(100vh - 64px)', display: 'flex', flexDirection: 'column' }}>
        <PageHeader title="Review Queue" />
        <Box sx={{ display: 'flex', gap: 2, flex: 1, minHeight: 0 }}>
          {/* Left panel — execution list */}
          <Box
            sx={{
              width: 340,
              minWidth: 340,
              overflowY: 'auto',
              display: 'flex',
              flexDirection: 'column',
              gap: 1,
            }}
          >
            {executions.map((execution) => (
              <ReviewCard
                key={execution.id}
                execution={execution}
                selected={execution.id === selectedId}
                onSelect={handleSelect}
              />
            ))}
          </Box>

          {/* Right panel — detail + chat */}
          <Box
            sx={{
              flex: 1,
              display: 'flex',
              flexDirection: 'column',
              border: 1,
              borderColor: 'divider',
              borderRadius: 2,
              overflow: 'hidden',
              bgcolor: 'background.paper',
            }}
          >
            {selectedExecution ? (
              <>
                {/* Collapsible sections */}
                <Box sx={{ p: 2, borderBottom: 1, borderColor: 'divider', overflowY: 'auto', maxHeight: '40%' }}>
                  <CollapsibleSection
                    title="Input"
                    open={inputOpen}
                    onToggle={() => setInputOpen((v) => !v)}
                  >
                    <Typography
                      variant="body2"
                      sx={{ fontFamily: 'monospace', fontSize: '0.8125rem', whiteSpace: 'pre-wrap' }}
                    >
                      {selectedExecution.input}
                    </Typography>
                  </CollapsibleSection>

                  <CollapsibleSection
                    title="Output"
                    open={outputOpen}
                    onToggle={() => setOutputOpen((v) => !v)}
                  >
                    {selectedExecution.output ? (
                      <MarkdownPreview content={selectedExecution.output} />
                    ) : (
                      <Typography variant="body2" color="text.secondary">
                        No output yet
                      </Typography>
                    )}
                  </CollapsibleSection>
                </Box>

                {/* Chat area */}
                <Box sx={{ flex: 1, minHeight: 0 }}>
                  <ChatPanel
                    messages={chatMessages}
                    onSend={handleSend}
                    streaming={chat.streaming}
                    disabled={chat.sending}
                  />
                </Box>

                {/* Approve bar */}
                <Box
                  sx={{
                    p: 1.5,
                    borderTop: 1,
                    borderColor: 'divider',
                    display: 'flex',
                    justifyContent: 'flex-end',
                    alignItems: 'center',
                    gap: 2,
                  }}
                >
                  {chat.error ? (
                    <Typography variant="caption" color="error" sx={{ flex: 1 }}>
                      {chat.error}
                    </Typography>
                  ) : null}
                  <ApproveButton
                    onApprove={handleApprove}
                    loading={chat.sending}
                    disabled={chat.streaming}
                  />
                </Box>
              </>
            ) : (
              <Box
                sx={{
                  flex: 1,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                }}
              >
                <Typography variant="body2" color="text.secondary">
                  Select an execution to review
                </Typography>
              </Box>
            )}
          </Box>
        </Box>
      </Box>
    </FadeIn>
  )
}

export { ReviewQueuePage }
