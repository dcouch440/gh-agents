import { FadeIn } from '@/components/animation'
import { PageHeader } from '@/components/primitives'

function DashboardPage() {
  return (
    <FadeIn>
      <PageHeader title="Dashboard" description="Overview of your agents and tasks" />
    </FadeIn>
  )
}

export { DashboardPage }
