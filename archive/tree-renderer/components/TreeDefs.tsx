import type { TreeTheme } from '../types'

type TreeDefsProps = {
  theme: TreeTheme
}

function TreeDefs({ theme }: TreeDefsProps) {
  return (
    <defs>
      {/* Arrow marker for edge endpoints */}
      <marker
        id="tree-arrow"
        viewBox="0 0 10 7"
        refX="10"
        refY="3.5"
        markerWidth="8"
        markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 3.5 L 0 7 Z" fill={theme.colorEdge} />
      </marker>
      <marker
        id="tree-arrow-active"
        viewBox="0 0 10 7"
        refX="10"
        refY="3.5"
        markerWidth="8"
        markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 3.5 L 0 7 Z" fill={theme.colorEdgeActive} />
      </marker>

      {/* Glow filter for running nodes */}
      {theme.glowEnabled ? (
        <filter id="tree-glow" x="-50%" y="-50%" width="200%" height="200%">
          <feGaussianBlur in="SourceAlpha" stdDeviation="3" result="blur" />
          <feFlood floodColor={theme.colorRunning} floodOpacity="0.4" result="color" />
          <feComposite in="color" in2="blur" operator="in" result="glow" />
          <feMerge>
            <feMergeNode in="glow" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      ) : null}
    </defs>
  )
}

export { TreeDefs }
