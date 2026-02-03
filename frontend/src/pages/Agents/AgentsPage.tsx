import { Link } from 'react-router-dom'
import { PageHeader } from '@/components/primitives'
import { ROUTES } from '@/constants'

function AgentsPage() {
  return (
    <div>
      <PageHeader title="Agents">
        <Link to={ROUTES.AGENT_CREATE} className="btn btn--primary">Create Agent</Link>
      </PageHeader>
    </div>
  )
}

export { AgentsPage }
