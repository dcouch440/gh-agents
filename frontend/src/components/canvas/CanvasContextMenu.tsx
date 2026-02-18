import { useCallback } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import type { SxProps, Theme } from '@mui/material/styles'
import { workflowStore, canvasStore, shareStore } from '@/stores'
import { DEFAULT_STEP_TYPE_COLOR, STEP_TYPE_COLORS, SECTION_LABEL_SX, COLOR_DOT_SX } from './constants'
import { Archetype, ARCHETYPE_CONFIGS, resolveArchetype } from './DynamicNode/archetypes'
import type { Archetype as ArchetypeType } from './DynamicNode/archetypes'
import { buildShareableFields } from './buildShareableFields'
import { buildProtocolsByStep } from './canvasContextMenuUtils'

const VIEWPORT_PADDING = 8

type MenuPosition = {
  x: number
  y: number
  flowX: number
  flowY: number
  nodeId?: string
} | null

type CanvasContextMenuProps = {
  position: MenuPosition
  onClose: () => void
}

const MENU_ITEM_SX: SxProps<Theme> = {
  display: 'flex',
  alignItems: 'center',
  gap: 1,
  px: 1.5,
  py: 0.75,
  cursor: 'pointer',
  '&:hover': { backgroundColor: 'action.hover' },
}


const ARCHETYPE_MENU_ORDER: ArchetypeType[] = [
  Archetype.WORKFORCE,
  Archetype.ROOM,
]

function CanvasContextMenu({ position, onClose }: CanvasContextMenuProps) {
  // Callback ref: clamp menu position to stay within viewport after mount
  const menuRef = useCallback((node: HTMLDivElement | null) => {
    if (!node) return
    const rect = node.getBoundingClientRect()
    const vw = window.innerWidth
    const vh = window.innerHeight
    if (rect.right > vw - VIEWPORT_PADDING) {
      node.style.left = `${vw - rect.width - VIEWPORT_PADDING}px`
    }
    if (rect.bottom > vh - VIEWPORT_PADDING) {
      node.style.top = `${vh - rect.height - VIEWPORT_PADDING}px`
    }
  }, [])

  if (!position) return null

  const handleAddArchetype = (event: React.MouseEvent, archetype: ArchetypeType) => {
    event.stopPropagation()
    event.preventDefault()
    const config = ARCHETYPE_CONFIGS[archetype]

    void workflowStore.createStep({
      name: `New ${config.label}`,
      execution_mode: config.executionMode,
      prompt_template: '',
      position_x: Math.round(position.flowX),
      position_y: Math.round(position.flowY),
    })

    onClose()
  }

  const handleAddContext = (event: React.MouseEvent) => {
    event.stopPropagation()
    event.preventDefault()
    void workflowStore.createStep({
      name: 'Context',
      execution_mode: 'context',
      position_x: Math.round(position.flowX),
      position_y: Math.round(position.flowY),
    })
    onClose()
  }

  const handleAddInput = (event: React.MouseEvent) => {
    event.stopPropagation()
    event.preventDefault()
    // Enforce single-input constraint on the frontend
    const steps = workflowStore.selectSteps(workflowStore.store.getState())
    const hasInput = steps.some((s) => s.execution_mode === 'input')
    if (hasInput) {
      onClose()
      return
    }
    void workflowStore.createStep({
      name: 'Input',
      execution_mode: 'input',
      position_x: Math.round(position.flowX),
      position_y: Math.round(position.flowY),
    })
    onClose()
  }

  const handleShare = (event: React.MouseEvent) => {
    event.stopPropagation()
    event.preventDefault()
    if (!position.nodeId) return

    const state = workflowStore.store.getState()

    // Regular step nodes
    const step = state.steps.byId.get(position.nodeId)
    if (!step) return

    const protocolsByStep = buildProtocolsByStep(canvasStore.store.getState().stepProtocols)
    const archetype = resolveArchetype(step, protocolsByStep, position.nodeId)

    const rosterAgents = state.rosterByStep[position.nodeId] ?? []
    const roomMembers = state.roomMembersByStep[position.nodeId] ?? []

    const fields = buildShareableFields({
      stepId: position.nodeId,
      step,
      archetype,
      rosterAgents,
      roomMembers,
    })

    shareStore.enterShareMode(position.nodeId, fields)
    onClose()
  }

  const handleDelete = (event: React.MouseEvent) => {
    event.stopPropagation()
    event.preventDefault()
    if (position.nodeId) {
      void workflowStore.deleteStep(position.nodeId)
    }
    onClose()
  }

  return (
    <Box
      ref={menuRef}
      data-testid="canvas-context-menu"
      onMouseDown={(e) => {
        e.stopPropagation()
      }}
      sx={{
        position: 'fixed',
        left: position.x,
        top: position.y,
        zIndex: 1000,
        backgroundColor: 'background.paper',
        border: 1,
        borderColor: 'divider',
        borderRadius: '8px',
        boxShadow: (theme) => (theme.palette.mode === 'dark' ? '0 4px 24px rgba(0, 0, 0, 0.4)' : '0 4px 24px rgba(45, 27, 14, 0.14)'),
        minWidth: 160,
        py: 0.5,
      }}
    >
      {position.nodeId !== undefined ? (
        <>
          <Box data-testid="ctx-share-step" onClick={handleShare} sx={MENU_ITEM_SX}>
            <Typography sx={{ fontSize: 12, color: 'text.primary' }}>Share</Typography>
          </Box>
          <Box sx={{ mx: 1.5, my: 0.5, borderTop: 1, borderColor: 'divider' }} />
          <Box data-testid="ctx-delete-step" onClick={handleDelete} sx={MENU_ITEM_SX}>
            <Typography sx={{ fontSize: 12, color: 'error.main' }}>Delete Step</Typography>
          </Box>
        </>
      ) : (
        <>
          <Typography sx={SECTION_LABEL_SX}>Archetypes</Typography>
          {ARCHETYPE_MENU_ORDER.map((archetype) => {
            const config = ARCHETYPE_CONFIGS[archetype]
            return (
              <Box
                key={archetype}
                data-testid={`ctx-add-${archetype}`}
                onClick={(event) => {
                  handleAddArchetype(event, archetype)
                }}
                sx={MENU_ITEM_SX}
              >
                <Box sx={{ ...COLOR_DOT_SX, backgroundColor: config.color }} />
                <Typography sx={{ fontSize: 12, color: 'text.primary' }}>{config.label}</Typography>
              </Box>
            )
          })}
          <Box sx={{ mx: 1.5, my: 0.5, borderTop: 1, borderColor: 'divider' }} />
          <Typography sx={SECTION_LABEL_SX}>Utilities</Typography>
          <Box data-testid="ctx-add-context" onClick={handleAddContext} sx={MENU_ITEM_SX}>
            <Box sx={{ ...COLOR_DOT_SX, backgroundColor: STEP_TYPE_COLORS['context'] ?? DEFAULT_STEP_TYPE_COLOR }} />
            <Typography sx={{ fontSize: 12, color: 'text.primary' }}>Context</Typography>
          </Box>
          <Box data-testid="ctx-add-input" onClick={handleAddInput} sx={MENU_ITEM_SX}>
            <Box sx={{ ...COLOR_DOT_SX, backgroundColor: STEP_TYPE_COLORS['input'] ?? DEFAULT_STEP_TYPE_COLOR }} />
            <Typography sx={{ fontSize: 12, color: 'text.primary' }}>Input</Typography>
          </Box>
        </>
      )}
    </Box>
  )
}

export { CanvasContextMenu }
export type { MenuPosition }
