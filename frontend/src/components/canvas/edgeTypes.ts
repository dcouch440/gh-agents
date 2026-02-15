import type { EdgeTypes } from '@xyflow/react'
import { StepEdge } from './StepEdge'
import { DocumentEdge } from './DocumentEdge'
import { NotesEdge } from './NotesEdge'

const edgeTypes: EdgeTypes = {
  stepEdge: StepEdge,
  documentEdge: DocumentEdge,
  notesEdge: NotesEdge,
}

export { edgeTypes }
