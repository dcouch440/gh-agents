import type { ReactNode } from 'react'
import { Button as MuiButton, CircularProgress } from '@mui/material'
import { ANIMATION } from '@/constants'

type ButtonVariant = 'primary' | 'secondary' | 'danger'
type ButtonSize = 'small' | 'medium'

type ButtonProps = {
  onClick?: () => void
  children: ReactNode
  variant?: ButtonVariant
  size?: ButtonSize
  disabled?: boolean
  loading?: boolean
  icon?: ReactNode
  type?: 'button' | 'submit' | 'reset'
}

const VARIANT_MAP = {
  primary: 'contained',
  secondary: 'outlined',
  danger: 'contained',
} as const

function Button({ onClick, children, variant = 'primary', size = 'small', disabled, loading, icon, type = 'button' }: ButtonProps) {
  return (
    <MuiButton
      onClick={onClick}
      variant={VARIANT_MAP[variant]}
      color={variant === 'danger' ? 'error' : 'primary'}
      size={size}
      disabled={disabled ?? loading}
      type={type}
      startIcon={loading ? <CircularProgress size={16} color="inherit" /> : icon}
      sx={{
        transition: `all ${ANIMATION.FAST}ms ease`,
        ...(variant === 'primary' && !disabled && !loading
          ? {
              '&:hover': {
                transform: 'scale(1.02)',
              },
              '&:active': {
                transform: 'scale(0.98)',
              },
            }
          : {}),
      }}
    >
      {children}
    </MuiButton>
  )
}

export { Button }
export type { ButtonProps, ButtonVariant, ButtonSize }
