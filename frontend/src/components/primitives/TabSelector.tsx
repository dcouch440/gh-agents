import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'

type TabOption = {
  value: string
  label: string
}

type TabSelectorProps = {
  options: TabOption[]
  value: string
  onChange: (value: string) => void
}

function TabSelector({ options, value, onChange }: TabSelectorProps) {
  return (
    <Box sx={{ display: 'flex' }}>
      {options.map((option) => {
        const isActive = option.value === value
        return (
          <Box
            key={option.value}
            onClick={() => { onChange(option.value) }}
            role="tab"
            tabIndex={0}
            aria-selected={isActive}
            onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') onChange(option.value) }}
            sx={{
              flex: 1,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              py: '8px',
              cursor: 'pointer',
              userSelect: 'none',
              borderBottom: '2px solid',
              borderColor: isActive ? 'primary.main' : 'transparent',
              backgroundColor: (theme) => isActive ? theme.palette.custom.activeTint : 'transparent',
              transition: 'all 120ms ease',
              '&:hover': isActive
                ? {}
                : {
                    backgroundColor: (theme) => theme.palette.custom.hoverOverlay,
                  },
            }}
          >
            <Typography
              sx={{
                fontSize: 10,
                fontWeight: 600,
                color: isActive ? 'primary.main' : 'text.secondary',
                letterSpacing: '0.02em',
                transition: 'color 120ms ease',
                ...(isActive ? {} : {
                  '&:hover': {
                    color: 'text.primary',
                  },
                }),
              }}
            >
              {option.label}
            </Typography>
          </Box>
        )
      })}
    </Box>
  )
}

export { TabSelector }
export type { TabOption, TabSelectorProps }
