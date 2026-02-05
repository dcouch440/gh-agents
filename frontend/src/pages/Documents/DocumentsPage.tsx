import { FadeIn } from '@/components/animation'
import { PageHeader } from '@/components/primitives'

function DocumentsPage() {
  return (
    <FadeIn>
      <PageHeader title="Documents" description="Browse and manage documents" />
    </FadeIn>
  )
}

export { DocumentsPage }
