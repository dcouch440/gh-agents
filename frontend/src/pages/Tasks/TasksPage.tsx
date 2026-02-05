import { FadeIn } from '@/components/animation'
import { PageHeader } from '@/components/primitives'

function TasksPage() {
  return (
    <FadeIn>
      <PageHeader title="Tasks" description="Manage and track your tasks" />
    </FadeIn>
  )
}

export { TasksPage }
