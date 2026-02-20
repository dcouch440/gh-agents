import type { Components, Theme } from '@mui/material/styles'
import type { CustomTokens } from './customTokens'

const getComponents = (mode: 'light' | 'dark', custom: CustomTokens): Components<Theme> => {
  const isDark = mode === 'dark'

  return {
    MuiCssBaseline: {
      styleOverrides: {
        body: {
          '& ::selection': {
            backgroundColor: isDark ? 'rgba(59, 130, 246, 0.3)' : 'rgba(90, 138, 110, 0.25)',
            color: 'inherit',
          },
          scrollbarColor: isDark ? `#3a3f4b ${custom.screenBg}` : `#BFB3A3 ${custom.screenBg}`,
          '&::-webkit-scrollbar, & *::-webkit-scrollbar': {
            width: 6,
            height: 6,
          },
          '&::-webkit-scrollbar-thumb, & *::-webkit-scrollbar-thumb': {
            borderRadius: 6,
            backgroundColor: isDark ? '#3a3f4b' : '#BFB3A3',
          },
          '&::-webkit-scrollbar-track, & *::-webkit-scrollbar-track': {
            backgroundColor: 'transparent',
          },
        },
      },
    },
    MuiButton: {
      defaultProps: {
        disableElevation: true,
        variant: 'contained',
      },
      styleOverrides: {
        root: {
          borderRadius: 8,
          padding: '6px 14px',
          fontSize: '0.8125rem',
          fontWeight: 500,
          transition: 'all 150ms ease',
        },
        sizeSmall: {
          padding: '4px 10px',
          fontSize: '0.75rem',
        },
        sizeLarge: {
          padding: '8px 18px',
          fontSize: '0.875rem',
        },
        contained: {
          boxShadow: 'none',
          '&:hover': {
            boxShadow: 'none',
          },
        },
      },
    },
    MuiCard: {
      defaultProps: {
        elevation: 0,
      },
      styleOverrides: {
        root: ({ theme }) => ({
          backgroundImage: 'none',
          backgroundColor: custom.surfaceBg,
          border: `1px solid ${theme.palette.divider}`,
          borderRadius: 12,
        }),
      },
    },
    MuiPaper: {
      defaultProps: {
        elevation: 0,
      },
      styleOverrides: {
        root: ({ theme }) => ({
          backgroundImage: 'none',
          backgroundColor: custom.surfaceBg,
          border: `1px solid ${theme.palette.divider}`,
          borderRadius: 12,
        }),
      },
    },
    MuiTableCell: {
      styleOverrides: {
        root: ({ theme }) => ({
          borderBottom: `1px solid ${theme.palette.divider}`,
          padding: '10px 16px',
          fontSize: '0.8125rem',
        }),
        head: {
          fontWeight: 600,
          fontSize: '0.75rem',
          textTransform: 'uppercase' as const,
          letterSpacing: '0.04em',
          color: isDark ? '#7d8590' : '#6B5742',
          backgroundColor: isDark ? 'rgba(255, 255, 255, 0.02)' : '#F4F0EA',
        },
      },
    },
    MuiTextField: {
      defaultProps: {
        variant: 'outlined',
        size: 'small',
      },
    },
    MuiOutlinedInput: {
      styleOverrides: {
        root: ({ theme }) => ({
          borderRadius: 8,
          '& .MuiOutlinedInput-notchedOutline': {
            borderColor: theme.palette.divider,
          },
          '&:hover .MuiOutlinedInput-notchedOutline': {
            borderColor: isDark ? 'rgba(240, 246, 252, 0.15)' : 'rgba(45, 27, 14, 0.18)',
          },
          '&.Mui-focused .MuiOutlinedInput-notchedOutline': {
            borderColor: theme.palette.primary.main,
            borderWidth: 1,
          },
        }),
        input: {
          padding: '8px 12px',
          fontSize: '0.8125rem',
        },
      },
    },
    MuiChip: {
      styleOverrides: {
        root: {
          borderRadius: 6,
          height: 24,
          fontSize: '0.75rem',
        },
      },
    },
    MuiDialog: {
      styleOverrides: {
        paper: ({ theme }) => ({
          backgroundImage: 'none',
          backgroundColor: custom.elevatedBg,
          border: `1px solid ${theme.palette.divider}`,
          borderRadius: 16,
        }),
      },
    },
    MuiAppBar: {
      defaultProps: {
        elevation: 0,
      },
      styleOverrides: {
        root: ({ theme }) => ({
          backgroundColor: custom.appBarBg,
          backdropFilter: 'blur(12px)',
          borderBottom: `1px solid ${theme.palette.divider}`,
        }),
      },
    },
    MuiDrawer: {
      styleOverrides: {
        paper: ({ theme }) => ({
          backgroundColor: custom.appBarBg,
          borderRight: `1px solid ${theme.palette.divider}`,
        }),
      },
    },
    MuiTooltip: {
      defaultProps: {
        arrow: false,
        enterDelay: 200,
        enterNextDelay: 100,
      },
      styleOverrides: {
        tooltip: {
          fontSize: '0.6875rem',
          fontWeight: 500,
          letterSpacing: '0.01em',
          borderRadius: 8,
          padding: '6px 12px',
          backgroundColor: isDark ? custom.elevatedBg : '#2D1B0E',
          border: `1px solid ${isDark ? 'rgba(240, 246, 252, 0.08)' : 'rgba(45, 27, 14, 0.12)'}`,
          backdropFilter: 'blur(8px)',
          boxShadow: isDark ? '0 4px 12px rgba(0, 0, 0, 0.4)' : '0 4px 12px rgba(0, 0, 0, 0.08)',
        },
      },
    },
    MuiMenu: {
      styleOverrides: {
        paper: ({ theme }) => ({
          backgroundColor: custom.elevatedBg,
          border: `1px solid ${theme.palette.divider}`,
          borderRadius: 10,
          boxShadow: isDark ? '0 8px 24px rgba(0, 0, 0, 0.4)' : '0 8px 24px rgba(0, 0, 0, 0.08)',
        }),
      },
    },
    MuiMenuItem: {
      styleOverrides: {
        root: {
          borderRadius: 6,
          margin: '2px 4px',
          padding: '6px 12px',
          fontSize: '0.8125rem',
        },
      },
    },
    MuiDivider: {
      styleOverrides: {
        root: {
          borderColor: isDark ? 'rgba(240, 246, 252, 0.06)' : 'rgba(45, 27, 14, 0.10)',
        },
      },
    },
    MuiIconButton: {
      styleOverrides: {
        root: {
          borderRadius: 8,
        },
      },
    },
  }
}

export { getComponents }
