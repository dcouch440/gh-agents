import type { ReactNode } from 'react'

type PageHeaderProps = {
  title: string
  children?: ReactNode
}

function PageHeader({ title, children }: PageHeaderProps) {
  return (
    <div className="page-header">
      <h1 className="page-header__title">{title}</h1>
      {children ? <div className="page-header__actions">{children}</div> : null}
    </div>
  )
}

export { PageHeader }
export type { PageHeaderProps }
