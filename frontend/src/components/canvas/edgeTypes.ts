import type { EdgeTypes } from '@xyflow/react'
import { StepEdge } from './StepEdge'
import { DocumentEdge } from './DocumentEdge'
import { AgentEdge } from './AgentEdge'
import { NotesEdge } from './NotesEdge'

const edgeTypes: EdgeTypes = {
  stepEdge: StepEdge,
  documentEdge: DocumentEdge,
  agentEdge: AgentEdge,
  notesEdge: NotesEdge,
}

export { edgeTypes }
