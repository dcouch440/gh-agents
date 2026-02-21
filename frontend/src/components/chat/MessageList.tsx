import { useRef, useEffect, type ReactNode } from 'react'
import { Box, Typography } from '@mui/material'
import { ChatMessage } from './ChatMessage'

export type ChatMessageData = {
  id: string
  role: 'user' | 'assistant' | 'system'
  content: string
  source_type?: string | null
}

export type MessageListProps = {
  messages: ChatMessageData[]
  emptyMessage?: string
  streamingContent?: ReactNode
  streaming?: boolean
  focusMode?: boolean
}

function MessageList({ messages, emptyMessage, streamingContent, streaming, focusMode }: MessageListProps) {
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
            return (
              <ChatMessage
                key={message.id}
                role={message.role}
                content={message.content}
                streaming={isLastAssistant ? streaming : undefined}
                sourceType={message.source_type}
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
