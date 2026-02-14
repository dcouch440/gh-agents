import { type ReactNode } from 'react'
import { Box } from '@mui/material'
import { MessageList } from './MessageList'
import { ChatInput } from './ChatInput'
import type { ChatMessageData } from './MessageList'

export type ChatPanelProps = {
  messages: ChatMessageData[]
  onSend: (message: string) => void
  streaming?: boolean
  disabled?: boolean
  className?: string
  emptyMessage?: string
  streamingContent?: ReactNode
}

function ChatPanel({ messages, onSend, streaming, disabled, emptyMessage, streamingContent }: ChatPanelProps) {
  return (
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        bgcolor: 'background.default',
      }}
    >
      <MessageList
        messages={messages}
        emptyMessage={emptyMessage}
        streamingContent={streamingContent}
        streaming={streaming}
      />
      <ChatInput onSend={onSend} disabled={disabled} />
    </Box>
  )
}

export { ChatPanel }
export type { ChatMessageData }
