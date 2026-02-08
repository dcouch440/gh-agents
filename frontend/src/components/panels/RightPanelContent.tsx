import { PropertiesPanel } from './PropertiesPanel'
import { AgentsBrowserPanel } from './AgentsBrowserPanel'
import { PromptsBrowserPanel } from './PromptsBrowserPanel'
import { SchemasBrowserPanel } from './SchemasBrowserPanel'
import { ExecutionPanel } from './ExecutionPanel'

type RightPanelContentProps = {
  section: string | null
}

function RightPanelContent({ section }: RightPanelContentProps) {
  switch (section) {
    case 'properties':
      return <PropertiesPanel />
    case 'agents':
      return <AgentsBrowserPanel />
    case 'prompts':
      return <PromptsBrowserPanel />
    case 'schemas':
      return <SchemasBrowserPanel />
    case 'execution':
      return <ExecutionPanel />
    default:
      return null
  }
}

export { RightPanelContent }
export type { RightPanelContentProps }
