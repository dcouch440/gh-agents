import { useParams } from 'react-router-dom'

function AgentDetailPage() {
  const { id } = useParams()
  return <h1>Agent: {id}</h1>
}

export { AgentDetailPage }
