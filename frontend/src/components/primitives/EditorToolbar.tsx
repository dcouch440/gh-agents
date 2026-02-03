import type { ReactNode } from 'react'

export type EditorToolbarProps = {
  children: ReactNode
  className?: string
}

export function EditorToolbar({ children, className }: EditorToolbarProps) {
  const toolbarClassName = ['editor-toolbar', className].filter(Boolean).join(' ')

  return <div className={toolbarClassName}>{children}</div>
}
