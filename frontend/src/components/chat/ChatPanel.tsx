import { useRef, type ReactNode } from 'react'
import { Box } from '@mui/material'
import { MessageList } from './MessageList'
import { ChatInput } from './ChatInput'
import { RichChatInput } from './RichChatInput'
import type { ChatMessageData } from './MessageList'

export type ChatPanelProps = {
  messages: ChatMessageData[]
  onSend: (message: string) => void
  streaming?: boolean
  disabled?: boolean
  className?: string
  emptyMessage?: string
  streamingContent?: ReactNode
  /** When provided, renders RichChatInput with mention chip support */
  stepId?: string
}

function ChatPanel({ messages, onSend, streaming, disabled, emptyMessage, streamingContent, stepId }: ChatPanelProps) {
  const inputRef = useRef<HTMLTextAreaElement>(null)

  return (
    <Box
      onClick={() => {
        if (!stepId) inputRef.current?.focus()
      }}
      sx={{
        display: 'flex',
        flexDirection: 'column',
        flex: 1,
        minHeight: 0,
        bgcolor: 'background.default',
        cursor: 'text',
      }}
    >
      <MessageList
        messages={messages}
        emptyMessage={emptyMessage}
        streamingContent={streamingContent}
        streaming={streaming}
      />
      {stepId ? (
        <RichChatInput onSend={onSend} stepId={stepId} disabled={disabled} />
      ) : (
        <ChatInput onSend={onSend} disabled={disabled} inputRef={inputRef} />
      )}
    </Box>
  )
}

export { ChatPanel }
export type { ChatMessageData }
