import { useRef, useEffect } from 'react'
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
}

export function ChatPanel({ messages, onSend, streaming, disabled, className }: ChatPanelProps) {
  const messagesRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const el = messagesRef.current
    if (!el) return

    const isNearBottom = el.scrollHeight - el.scrollTop - el.clientHeight <= 50
    if (isNearBottom) {
      el.scrollTop = el.scrollHeight
    }
  }, [messages])

  const panelClassName = ['chat-panel', className].filter(Boolean).join(' ')

  return (
    <div className={panelClassName}>
      <div className="chat-panel__messages" ref={messagesRef}>
        {messages.length === 0 ? (
          <div className="chat-panel__empty">No messages yet</div>
        ) : (
          messages.map((message, index) => {
            const isLastAssistant =
              message.role === 'assistant' && index === messages.length - 1
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
      </div>
      <ChatInput onSend={onSend} disabled={disabled} />
    </div>
  )
}
