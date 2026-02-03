import { MarkdownPreview } from '@/components/primitives/MarkdownPreview'

type ChatMessageProps = {
  role: 'user' | 'assistant'
  content: string
  streaming?: boolean
}

function ChatMessage({ role, content, streaming }: ChatMessageProps) {
  if (role === 'user') {
    return (
      <div className="chat-message chat-message--user">
        <span style={{ whiteSpace: 'pre-wrap' }}>{content}</span>
      </div>
    )
  }

  return (
    <div className="chat-message chat-message--assistant">
      <MarkdownPreview content={content} />
      {streaming && content ? <span className="chat-message__cursor" /> : null}
    </div>
  )
}

export { ChatMessage }
export type { ChatMessageProps }
