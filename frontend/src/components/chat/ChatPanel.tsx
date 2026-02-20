import { useRef, type ReactNode } from 'react'
import { Box } from '@mui/material'
import { MessageList } from './MessageList'
import { ChatInput } from './ChatInput'
import { RichChatInput } from './RichChatInput'
import type { ChatMessageData } from './MessageList'

export type ChatPanelProps = {
  messages: ChatMessageData[]
  onSend: (message: string) => void
  onCancel?: () => void
  streaming?: boolean
  disabled?: boolean
  className?: string
  emptyMessage?: string
  streamingContent?: ReactNode
  /** When provided, renders RichChatInput with mention chip support */
  stepId?: string
  /** When true, uses more generous spacing for fullscreen focus mode */
  focusMode?: boolean
}

function ChatPanel({ messages, onSend, onCancel, streaming, disabled, emptyMessage, streamingContent, stepId, focusMode }: ChatPanelProps) {
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
        cursor: 'text',
      }}
    >
      <MessageList
        messages={messages}
        emptyMessage={emptyMessage}
        streamingContent={streamingContent}
        streaming={streaming}
        focusMode={focusMode}
      />
      {stepId ? (
        <RichChatInput onSend={onSend} onCancel={onCancel} stepId={stepId} disabled={disabled} focusMode={focusMode} />
      ) : (
        <ChatInput onSend={onSend} disabled={disabled} inputRef={inputRef} />
      )}
    </Box>
  )
}

export { ChatPanel }
export type { ChatMessageData }
