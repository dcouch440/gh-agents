import type { EdgeTypes } from '@xyflow/react'
import { StepEdge } from './StepEdge'
import { ArtifactEdge } from './ArtifactEdge'

const edgeTypes: EdgeTypes = {
  stepEdge: StepEdge,
  artifactEdge: ArtifactEdge,
}

export { edgeTypes }
