import { useRef, useEffect, type ReactNode } from 'react'
import { Box, Typography } from '@mui/material'
import { ChatMessage } from './ChatMessage'

export type PanelMessageMetadata = {
  submitLabel: string
  submitted: boolean
}

export type ChatMessageData = {
  id: string
  role: 'user' | 'assistant' | 'system' | 'tool'
  content: string
  source_type?: string | null
  panelMeta?: PanelMessageMetadata | null
  toolName?: string
  toolResult?: string
}

export type MessageListProps = {
  messages: ChatMessageData[]
  emptyMessage?: string
  streamingContent?: ReactNode
  streaming?: boolean
  focusMode?: boolean
  onPanelSubmit?: (messageId: string, selections: string) => void
}

function MessageList({ messages, emptyMessage, streamingContent, streaming, focusMode, onPanelSubmit }: MessageListProps) {
  const messagesRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const el = messagesRef.current
    if (!el) return

    const isNearBottom = el.scrollHeight - el.scrollTop - el.clientHeight <= 50
    if (isNearBottom) {
      el.scrollTop = el.scrollHeight
    }
  }, [messages, streamingContent])

  return (
    <Box
      ref={messagesRef}
      sx={{
        flex: 1,
        minHeight: 0,
        overflowY: 'auto',
        scrollbarWidth: 'none',
        '&::-webkit-scrollbar': { display: 'none' },
        px: focusMode ? 3 : 1.5,
        pt: focusMode ? 2.5 : 1.5,
        pb: focusMode ? 1.5 : 1,
        display: 'flex',
        flexDirection: 'column',
        gap: 0,
        maskImage:
          'linear-gradient(to bottom, transparent 0%, black 12px, black calc(100% - 8px), transparent 100%)',
        WebkitMaskImage:
          'linear-gradient(to bottom, transparent 0%, black 12px, black calc(100% - 8px), transparent 100%)',
      }}
    >
      {messages.length === 0 && !streamingContent ? (
        <Box
          sx={{
            flex: 1,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          <Typography variant="body2" color="text.secondary">
            {emptyMessage ?? 'No messages yet'}
          </Typography>
        </Box>
      ) : (
        <>
          {messages.map((message, index) => {
            const isLastAssistant = message.role === 'assistant' && index === messages.length - 1
            if (isLastAssistant && streamingContent) return null
            if (message.role === 'tool') {
              return (
                <Box
                  key={message.id}
                  component="details"
                  sx={{
                    mx: 1,
                    my: 0.5,
                    px: 1.5,
                    py: 0.5,
                    borderRadius: 1,
                    backgroundColor: 'action.hover',
                    fontSize: 12,
                    fontFamily: 'monospace',
                    '& summary': { cursor: 'pointer', color: 'text.secondary', userSelect: 'none' },
                    '& pre': { whiteSpace: 'pre-wrap', wordBreak: 'break-all', m: 0, mt: 0.5, fontSize: 11 },
                  }}
                >
                  <summary>{message.toolName ?? 'tool'}: {message.content.slice(0, 80)}{message.content.length > 80 ? '…' : ''}</summary>
                  <pre>{message.content}</pre>
                  {message.toolResult !== undefined && message.toolResult !== '' && (
                    <pre style={{ color: '#888', marginTop: 4 }}>{message.toolResult}</pre>
                  )}
                </Box>
              )
            }
            return (
              <ChatMessage
                key={message.id}
                role={message.role}
                content={message.content}
                streaming={isLastAssistant ? streaming : undefined}
                sourceType={message.source_type}
                panelMeta={message.panelMeta}
                onPanelSubmit={onPanelSubmit}
                messageId={message.id}
              />
            )
          })}
          {streamingContent}
        </>
      )}
    </Box>
  )
}

export { MessageList }
export type { MessageListProps as MessageListBaseProps }
