import { useMemo } from 'react'

type TransitionState = 'entering' | 'entered' | 'stable'

/**
 * Returns transition state for each node ID.
 * Currently marks all nodes as stable — entry animations are handled
 * via CSS transitions on the transform/opacity of TreeNodeGroup,
 * which naturally animate when new nodes appear at their position.
 */
const useNodeTransitions = (nodeIds: string[]): Map<string, TransitionState> => {
  const transitions = useMemo(() => {
    const map = new Map<string, TransitionState>()
    for (const id of nodeIds) {
      map.set(id, 'stable')
    }
    return map
  }, [nodeIds])

  return transitions
}

export { useNodeTransitions }
export type { TransitionState }
