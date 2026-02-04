import type { ReactNode } from 'react'
import { Button as MuiButton } from '@mui/material'

type ButtonVariant = 'primary' | 'secondary' | 'danger'
type ButtonSize = 'small' | 'medium'

type ButtonProps = {
  onClick: () => void
  children: ReactNode
  variant?: ButtonVariant
  size?: ButtonSize
  disabled?: boolean
  type?: 'button' | 'submit' | 'reset'
}

const VARIANT_MAP = {
  primary: 'contained',
  secondary: 'outlined',
  danger: 'contained',
} as const

function Button({
  onClick,
  children,
  variant = 'primary',
  size = 'small',
  disabled,
  type = 'button',
}: ButtonProps) {
  return (
    <MuiButton
      onClick={onClick}
      variant={VARIANT_MAP[variant]}
      color={variant === 'danger' ? 'error' : 'primary'}
      size={size}
      disabled={disabled}
      type={type}
    >
      {children}
    </MuiButton>
  )
}

export { Button }
export type { ButtonProps, ButtonVariant, ButtonSize }
