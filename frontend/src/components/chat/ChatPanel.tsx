import { useRef, useEffect } from 'react'
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
}

export function ChatPanel({ messages, onSend, streaming, disabled, emptyMessage }: ChatPanelProps) {
  const messagesRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const el = messagesRef.current
    if (!el) return

    const isNearBottom = el.scrollHeight - el.scrollTop - el.clientHeight <= 50
    if (isNearBottom) {
      el.scrollTop = el.scrollHeight
    }
  }, [messages])

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
          p: 2,
          display: 'flex',
          flexDirection: 'column',
          gap: 1,
        }}
      >
        {messages.length === 0 ? (
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
          messages.map((message, index) => {
            const isLastAssistant = message.role === 'assistant' && index === messages.length - 1
            return (
              <ChatMessage
                key={message.id}
                role={message.role}
                content={message.content}
                streaming={isLastAssistant ? streaming : undefined}
              />
            )
          })
        )}
      </Box>
      <ChatInput onSend={onSend} disabled={disabled} />
    </Box>
  )
}
