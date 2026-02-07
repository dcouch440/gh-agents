// ============================================================================
// bridge/edgeTypes — Custom React Flow edge component registry
// ============================================================================

import { DataFlowEdge } from './DataFlowEdge'
import { ConditionalEdge } from './ConditionalEdge'

const edgeTypes = {
  dataFlow: DataFlowEdge,
  conditional: ConditionalEdge,
}

export { edgeTypes }
