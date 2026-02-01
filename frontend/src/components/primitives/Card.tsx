import type { ReactNode } from 'react'

type CardProps = {
  title?: string
  children: ReactNode
}

function Card({ title, children }: CardProps) {
  return (
    <div className="card">
      {title ? <h3 className="card__title">{title}</h3> : null}
      {children}
    </div>
  )
}

export { Card }
export type { CardProps }
