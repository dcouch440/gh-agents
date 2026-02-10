import type { EdgeTypes } from '@xyflow/react'
import { StepEdge } from './StepEdge'
import { DocumentEdge } from './DocumentEdge'

const edgeTypes: EdgeTypes = {
  stepEdge: StepEdge,
  documentEdge: DocumentEdge,
}

export { edgeTypes }
