import Select from '@mui/material/Select'
import MenuItem from '@mui/material/MenuItem'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'

type PropertySelectOption = {
  value: string
  label: string
  secondary?: string | null
}

type PropertySelectProps = {
  value: string | null
  options: PropertySelectOption[]
  onChange: (value: string | null) => void
  placeholder?: string
  allowNone?: boolean
  accentColor?: string
}

function PropertySelect({ value, options, onChange, placeholder = 'Select...', allowNone = false, accentColor }: PropertySelectProps) {
  const selected = options.find((o) => o.value === value)

  return (
    <Select
      value={value ?? ''}
      onChange={(e) => {
        const v = String(e.target.value)
        onChange(v === '' ? null : v)
      }}
      displayEmpty
      size="small"
      fullWidth
      renderValue={() => (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, minWidth: 0 }}>
          {accentColor !== undefined && selected !== undefined ? (
            <Box
              sx={{
                width: 6,
                height: 6,
                borderRadius: '50%',
                backgroundColor: accentColor,
                flexShrink: 0,
              }}
            />
          ) : null}
          <Typography
            sx={{
              fontSize: 11,
              fontWeight: selected !== undefined ? 500 : 400,
              color: selected !== undefined ? 'text.primary' : 'text.secondary',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {selected !== undefined ? selected.label : placeholder}
          </Typography>
        </Box>
      )}
      sx={{
        fontSize: 11,
        borderRadius: 0,
        transition: 'background-color 100ms ease',
        '& .MuiSelect-select': {
          py: '8px',
          px: '16px',
        },
        '& .MuiOutlinedInput-notchedOutline': {
          border: 'none',
        },
        '&:hover': {
          backgroundColor: (theme) => theme.palette.custom.hoverOverlay,
        },
        '&.Mui-focused': {
          backgroundColor: (theme) => theme.palette.custom.activeTint,
        },
      }}
      MenuProps={{
        PaperProps: {
          sx: {
            mt: 0.5,
            border: 1,
            borderColor: 'divider',
            '& .MuiMenuItem-root': {
              fontSize: 11,
              py: '6px',
              px: '12px',
              transition: 'background-color 100ms ease',
              '&:hover': { backgroundColor: (theme) => theme.palette.custom.hoverOverlay },
              '&.Mui-selected': { backgroundColor: (theme) => theme.palette.custom.activeTint },
              '&.Mui-selected:hover': { backgroundColor: (theme) => theme.palette.custom.activeTintStrong },
            },
          },
        },
      }}
    >
      {allowNone ? (
        <MenuItem value="">
          <Typography sx={{ fontSize: 11, color: 'text.secondary', fontStyle: 'italic' }}>None</Typography>
        </MenuItem>
      ) : null}
      {options.map((opt) => (
        <MenuItem key={opt.value} value={opt.value}>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: '1px', minWidth: 0 }}>
            <Typography
              sx={{
                fontSize: 11,
                fontWeight: 500,
                color: 'text.primary',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}
            >
              {opt.label}
            </Typography>
            {opt.secondary !== undefined && opt.secondary !== null ? (
              <Typography
                sx={{
                  fontSize: 10,
                  color: 'text.secondary',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}
              >
                {opt.secondary}
              </Typography>
            ) : null}
          </Box>
        </MenuItem>
      ))}
    </Select>
  )
}

export { PropertySelect }
export type { PropertySelectProps, PropertySelectOption }
