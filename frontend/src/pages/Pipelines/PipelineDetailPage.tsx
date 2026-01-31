import { useParams } from 'react-router-dom'

function PipelineDetailPage() {
  const { id } = useParams()
  return <h1>Pipeline: {id}</h1>
}

export { PipelineDetailPage }
