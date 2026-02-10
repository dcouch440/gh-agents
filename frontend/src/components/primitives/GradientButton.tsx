import type { ReactNode } from 'react'
import MuiButton from '@mui/material/Button'
import CircularProgress from '@mui/material/CircularProgress'
import { useTheme } from '@mui/material/styles'

type GradientButtonColor = 'primary' | 'success' | 'error'

type GradientButtonProps = {
  children: ReactNode
  onClick: () => void
  icon?: ReactNode
  color?: GradientButtonColor
  disabled?: boolean
  loading?: boolean
  minWidth?: number
}

function GradientButton({ children, onClick, icon, color = 'primary', disabled, loading, minWidth = 80 }: GradientButtonProps) {
  const theme = useTheme()
  const palette = theme.palette[color]
  const isDark = theme.palette.mode === 'dark'

  const bgGradient = isDark
    ? `linear-gradient(135deg, ${palette.main} 0%, ${palette.dark} 100%)`
    : `linear-gradient(135deg, ${palette.light} 0%, ${palette.main} 100%)`
  const hoverGradient = isDark
    ? `linear-gradient(135deg, ${palette.dark} 0%, ${palette.dark} 100%)`
    : `linear-gradient(135deg, ${palette.main} 0%, ${palette.main} 100%)`
  const shadow = isDark ? `0 2px 8px ${palette.main}66` : `0 2px 8px ${palette.main}33`
  const hoverShadow = isDark ? `0 4px 14px ${palette.main}80` : `0 4px 14px ${palette.main}4d`

  return (
    <MuiButton
      size="small"
      variant="contained"
      startIcon={loading ? <CircularProgress size={14} thickness={5} color="inherit" /> : icon}
      onClick={onClick}
      disabled={disabled ?? loading}
      sx={{
        fontSize: 13,
        fontWeight: 600,
        textTransform: 'none',
        px: 2.5,
        py: 0.75,
        minWidth,
        background: bgGradient,
        boxShadow: shadow,
        transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
        '&:hover': {
          background: hoverGradient,
          boxShadow: hoverShadow,
          transform: 'translateY(-1px)',
          '& .MuiSvgIcon-root': {
            transform: 'scale(1.1)',
          },
        },
        '&:active': {
          transform: 'translateY(0) scale(0.98)',
          boxShadow: shadow,
        },
        '&.Mui-disabled': {
          background: `${palette.main}4d`,
          color: 'rgba(255, 255, 255, 0.5)',
        },
      }}
    >
      {children}
    </MuiButton>
  )
}

export { GradientButton }
export type { GradientButtonProps, GradientButtonColor }
