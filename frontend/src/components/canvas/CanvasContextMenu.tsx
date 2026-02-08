import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { workflowStore } from '@/stores'
import { STEP_TYPE_COLORS, DEFAULT_STEP_TYPE_COLOR } from './constants'

type MenuPosition = {
  x: number
  y: number
  flowX: number
  flowY: number
} | null

type CanvasContextMenuProps = {
  position: MenuPosition
  onClose: () => void
}

const STEP_TYPES = [
  { key: 'llm', label: 'LLM Step' },
  { key: 'for_each', label: 'For-Each Step' },
  { key: 'router', label: 'Router Step' },
  { key: 'human', label: 'Human Review' },
  { key: 'tool', label: 'Tool Step' },
] as const

function CanvasContextMenu({ position, onClose }: CanvasContextMenuProps) {
  if (!position) return null

  const handleAdd = (stepType: string) => {
    void workflowStore.createStep({
      name: `New ${stepType} step`,
      step_type: stepType,
      position_x: Math.round(position.flowX),
      position_y: Math.round(position.flowY),
    })
    onClose()
  }

  return (
    <Box
      sx={{
        position: 'fixed',
        left: position.x,
        top: position.y,
        zIndex: 1000,
        backgroundColor: 'background.paper',
        border: 1,
        borderColor: 'divider',
        borderRadius: '8px',
        boxShadow: '0 4px 24px rgba(0, 0, 0, 0.4)',
        minWidth: 160,
        py: 0.5,
      }}
    >
      <Typography
        sx={{
          px: 1.5,
          py: 0.75,
          fontSize: 10,
          textTransform: 'uppercase',
          color: 'text.disabled',
          letterSpacing: '0.05em',
          fontWeight: 600,
        }}
      >
        Add Step
      </Typography>
      {STEP_TYPES.map((st) => (
        <Box
          key={st.key}
          onClick={() => {
            handleAdd(st.key)
          }}
          sx={{
            display: 'flex',
            alignItems: 'center',
            gap: 1,
            px: 1.5,
            py: 0.75,
            cursor: 'pointer',
            '&:hover': { backgroundColor: 'action.hover' },
          }}
        >
          <Box
            sx={{
              width: 8,
              height: 8,
              borderRadius: '50%',
              backgroundColor: STEP_TYPE_COLORS[st.key] ?? DEFAULT_STEP_TYPE_COLOR,
              flexShrink: 0,
            }}
          />
          <Typography sx={{ fontSize: 12, color: 'text.primary' }}>
            {st.label}
          </Typography>
        </Box>
      ))}
    </Box>
  )
}

export { CanvasContextMenu }
export type { MenuPosition }
