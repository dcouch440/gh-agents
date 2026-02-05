import { FadeIn } from '@/components/animation'
import { PageHeader } from '@/components/primitives'

function ChatPage() {
  return (
    <FadeIn>
      <PageHeader title="Chat" description="Start a conversation with an agent" />
    </FadeIn>
  )
}

export { ChatPage }
