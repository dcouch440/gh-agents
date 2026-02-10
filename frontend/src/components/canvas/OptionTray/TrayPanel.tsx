import type { ReactNode } from 'react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { AnimatePresence, motion } from 'framer-motion'
import { useReducedMotion } from '@/hooks/useReducedMotion'
import { OPTION_TRAY } from './constants'

type TrayPanelProps = {
  visible: boolean
  dirty: boolean
  children: ReactNode
}

function TrayPanel({ visible, dirty, children }: TrayPanelProps) {
  const theme = useTheme()
  const isDark = theme.palette.mode === 'dark'
  const reducedMotion = useReducedMotion()
  const duration = reducedMotion ? 0 : OPTION_TRAY.ANIMATION_DURATION

  return (
    <Box
      sx={{
        position: 'absolute',
        bottom: OPTION_TRAY.PANEL_BOTTOM,
        left: 0,
        right: 0,
        display: 'flex',
        justifyContent: 'center',
        zIndex: 10,
        pointerEvents: 'none',
      }}
    >
      <AnimatePresence>
        {visible && (
          <motion.div
            key="option-tray-panel"
            initial={{ y: 20, opacity: 0 }}
            animate={{ y: 0, opacity: 1 }}
            exit={{ y: 20, opacity: 0 }}
            transition={{
              duration,
              ease: OPTION_TRAY.EASING,
            }}
            style={{ pointerEvents: 'auto' }}
          >
            <Box
              data-testid={dirty ? 'save-discard-bar' : undefined}
              sx={{
                display: 'flex',
                alignItems: 'center',
                gap: 1.5,
                px: 2,
                py: 1.25,
                borderRadius: `${OPTION_TRAY.PANEL_BORDER_RADIUS}px`,
                backgroundColor: theme.palette.custom.floatingPanelBg,
                backdropFilter: 'blur(16px)',
                border: '1px solid',
                borderColor: theme.palette.custom.floatingPanelBorder,
                boxShadow: isDark
                  ? '0 8px 32px rgba(0, 0, 0, 0.5), 0 1px 2px rgba(0, 0, 0, 0.3), inset 0 1px 0 rgba(255, 255, 255, 0.05)'
                  : '0 8px 32px rgba(45, 27, 14, 0.12), 0 1px 2px rgba(45, 27, 14, 0.06)',
                transition: 'border-color 0.2s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                '&:hover': {
                  borderColor: theme.palette.custom.borderHover,
                  boxShadow: isDark
                    ? '0 12px 40px rgba(0, 0, 0, 0.6), 0 2px 4px rgba(0, 0, 0, 0.3), inset 0 1px 0 rgba(255, 255, 255, 0.08)'
                    : '0 12px 40px rgba(45, 27, 14, 0.16), 0 2px 4px rgba(45, 27, 14, 0.08)',
                },
              }}
            >
              {children}
            </Box>
          </motion.div>
        )}
      </AnimatePresence>
    </Box>
  )
}

export { TrayPanel }
export type { TrayPanelProps }
