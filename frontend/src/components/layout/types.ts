import { type ReactNode } from 'react'

type NavBarItem = {
  key: string
  icon: ReactNode
  label: string
  isActive: boolean
  badge?: number
  onClick: () => void
}

export type { NavBarItem }
