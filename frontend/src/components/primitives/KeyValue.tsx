import type { ReactNode } from 'react'

type KeyValueProps = {
  label: string
  children: ReactNode
}

function KeyValue({ label, children }: KeyValueProps) {
  return (
    <div className="kv">
      <dt className="kv__label">{label}</dt>
      <dd className="kv__value">{children}</dd>
    </div>
  )
}

export { KeyValue }
export type { KeyValueProps }
