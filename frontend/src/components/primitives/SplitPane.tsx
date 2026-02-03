import type { ReactNode } from 'react'

type SplitPaneProps = {
  left: ReactNode
  right: ReactNode
  splitPercent: number
  onMouseDown: (e: React.MouseEvent) => void
  className?: string
}

function SplitPane({ left, right, splitPercent, onMouseDown, className }: SplitPaneProps) {
  const containerClass = className ? `split-pane ${className}` : 'split-pane'

  return (
    <div className={containerClass} style={{ display: 'flex', height: '100%' }}>
      <div className="split-pane__left" style={{ width: `${splitPercent}%` }}>
        {left}
      </div>
      <div className="split-pane__handle" onMouseDown={onMouseDown} />
      <div className="split-pane__right" style={{ flex: 1 }}>
        {right}
      </div>
    </div>
  )
}

export { SplitPane }
export type { SplitPaneProps }
