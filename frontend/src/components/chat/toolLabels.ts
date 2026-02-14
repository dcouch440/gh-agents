import type { ToolStatus } from '@/types'

const TOOL_LABELS: Record<string, (status: ToolStatus) => string> = {
  create_doc_def: (s) => (s === 'running' ? 'Creating document...' : 'Created document'),
  update_doc_def: (s) => (s === 'running' ? 'Updating document...' : 'Updated document'),
  delete_doc_def: (s) => (s === 'running' ? 'Removing document...' : 'Removed document'),
  update_prompt: (s) => (s === 'running' ? 'Updating prompt...' : 'Updated prompt'),
  read_context: (s) => (s === 'running' ? 'Reading context...' : 'Read context'),
  think: (s) => (s === 'running' ? 'Thinking...' : 'Thought'),
  render_panel: (s) => (s === 'running' ? 'Rendering panel...' : 'Rendered panel'),
}

const getToolLabel = (toolName: string, status: ToolStatus): string => {
  const labelFn = TOOL_LABELS[toolName]
  if (labelFn) return labelFn(status)
  const name = toolName.replace(/_/g, ' ')
  return status === 'running' ? `${name}...` : name
}

export { TOOL_LABELS, getToolLabel }
