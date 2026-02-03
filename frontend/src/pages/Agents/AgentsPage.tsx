import { Link } from 'react-router-dom'
import { PageHeader } from '@/components/primitives'
import { ROUTES } from '@/constants'

function AgentsPage() {
  return (
    <div>
      <PageHeader title="Agents">
        <Link to={ROUTES.AGENT_WORKSHOP} className="btn btn--primary">Workshop</Link>
      </PageHeader>
    </div>
  )
}

export { AgentsPage }
