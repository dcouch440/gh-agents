import { useParams } from 'react-router-dom'

function PipelineRunPage() {
  const { id, runId } = useParams()
  return <h1>Pipeline {id} — Run {runId}</h1>
}

export { PipelineRunPage }
