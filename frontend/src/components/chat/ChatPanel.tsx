import { useRef, useEffect, type ReactNode } from 'react'
import { Box, Typography } from '@mui/material'
import { ChatMessage } from './ChatMessage'
import { ChatInput } from './ChatInput'

export type ChatMessageData = {
  id: string
  role: 'user' | 'assistant'
  content: string
}

export type ChatPanelProps = {
  messages: ChatMessageData[]
  onSend: (message: string) => void
  streaming?: boolean
  disabled?: boolean
  className?: string
  emptyMessage?: string
  streamingContent?: ReactNode
}

export function ChatPanel({ messages, onSend, streaming, disabled, emptyMessage, streamingContent }: ChatPanelProps) {
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
      sx={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        bgcolor: 'background.default',
      }}
    >
      <Box
        ref={messagesRef}
        sx={{
          flex: 1,
          overflowY: 'auto',
          px: 1.5,
          pt: 1.5,
          pb: 1,
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
                />
              )
            })}
            {streamingContent}
          </>
        )}
      </Box>
      <ChatInput onSend={onSend} disabled={disabled} />
    </Box>
  )
}
