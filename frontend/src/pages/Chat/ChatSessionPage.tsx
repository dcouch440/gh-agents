import { useParams } from 'react-router-dom'

function ChatSessionPage() {
  const { sessionId } = useParams()
  return <h1>Chat Session: {sessionId}</h1>
}

export { ChatSessionPage }
