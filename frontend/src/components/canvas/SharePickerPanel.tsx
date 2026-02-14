import { useCallback, useMemo } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Checkbox from '@mui/material/Checkbox'
import type { SxProps, Theme } from '@mui/material/styles'
import { useStore, shareStore } from '@/stores'
import type { ShareableField } from '@/stores/shareStore'
import { Collections } from '@/utils/collections'
import { SECTION_LABEL_SX, COLOR_DOT_SX } from './constants'

// ── Styling ──────────────────────────────────────────────────────────────────

const ITEM_SX: SxProps<Theme> = {
  display: 'flex',
  alignItems: 'center',
  gap: 0.5,
  px: 1,
  py: 0.25,
  cursor: 'pointer',
  '&:hover': { backgroundColor: 'action.hover' },
}

const CATEGORY_ORDER = ['General', 'Documents', 'Agents', 'Members']

// ── Component ────────────────────────────────────────────────────────────────

type SharePickerPanelProps = {
  stepId: string
}

function SharePickerPanel({ stepId }: SharePickerPanelProps) {
  const availableFields = useStore(shareStore.store, shareStore.selectAvailableFields)
  const selectedKeys = useStore(shareStore.store, shareStore.selectSelectedKeys)

  const grouped = useMemo(() => {
    const groups = Collections.groupBy(availableFields, (f: ShareableField) => f.category)
    return Collections.filterMap(CATEGORY_ORDER, (c) => groups.has(c) ? { label: c, items: groups.get(c)! } : null)
  }, [availableFields])

  const handleToggle = useCallback((key: string) => {
    shareStore.toggleField(key)
  }, [])

  const selectedCount = selectedKeys.size
  const totalCount = availableFields.length

  void stepId // consumed by parent for identification, not needed here

  return (
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        overflow: 'hidden',
      }}
    >
      {/* Header */}
      <Box sx={{ px: 1.5, py: 1, borderBottom: 1, borderColor: 'divider', flexShrink: 0 }}>
        <Typography sx={{ fontSize: 11, fontWeight: 600, color: 'text.secondary', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
          Share context
        </Typography>
      </Box>

      {/* Scrollable field list */}
      <Box sx={{ flex: 1, minHeight: 0, overflowY: 'auto', py: 0.5 }}>
        {grouped.map((group, gi) => (
          <Box key={group.label}>
            {gi > 0 && <Box sx={{ mx: 1.5, my: 0.5, borderTop: 1, borderColor: 'divider' }} />}
            <Typography sx={SECTION_LABEL_SX}>{group.label}</Typography>
            {group.items.map((field) => (
              <Box
                key={field.key}
                onClick={() => {
                  handleToggle(field.key)
                }}
                sx={ITEM_SX}
              >
                <Checkbox
                  checked={selectedKeys.has(field.key)}
                  size="small"
                  tabIndex={-1}
                  disableRipple
                  sx={{
                    p: 0.25,
                    '& .MuiSvgIcon-root': { fontSize: 16 },
                    color: field.color,
                    '&.Mui-checked': { color: field.color },
                  }}
                />
                <Box sx={{ ...COLOR_DOT_SX, backgroundColor: field.color }} />
                <Typography
                  sx={{
                    fontSize: 12,
                    color: 'text.primary',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {field.label}
                </Typography>
              </Box>
            ))}
          </Box>
        ))}
      </Box>

      {/* Footer */}
      <Box
        sx={{
          px: 1.5,
          py: 0.75,
          borderTop: 1,
          borderColor: 'divider',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          flexShrink: 0,
        }}
      >
        <Typography sx={{ fontSize: 10, color: 'text.disabled' }}>
          {selectedCount} of {totalCount} selected
        </Typography>
        <Typography sx={{ fontSize: 10, color: 'text.disabled', fontStyle: 'italic' }}>
          Click a target node
        </Typography>
      </Box>
    </Box>
  )
}

export { SharePickerPanel }
export type { SharePickerPanelProps }
