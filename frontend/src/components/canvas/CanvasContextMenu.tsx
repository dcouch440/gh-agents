import { useCallback, useMemo } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import type { SxProps, Theme } from '@mui/material/styles'
import { useStore, workflowStore, protocolStore, canvasStore } from '@/stores'
import { Collections } from '@/utils/collections'
import { DEFAULT_STEP_TYPE_COLOR, STEP_TYPE_COLORS } from './constants'
import { Archetype, ARCHETYPE_CONFIGS } from './DynamicNode/archetypes'
import type { Archetype as ArchetypeType } from './DynamicNode/archetypes'

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

const SECTION_LABEL_SX: SxProps<Theme> = {
  px: 1.5,
  py: 0.75,
  fontSize: 10,
  textTransform: 'uppercase',
  color: 'text.disabled',
  letterSpacing: '0.05em',
  fontWeight: 600,
}

const COLOR_DOT_SX: SxProps<Theme> = {
  width: 8,
  height: 8,
  borderRadius: '50%',
  flexShrink: 0,
}

const ARCHETYPE_MENU_ORDER: ArchetypeType[] = [
  Archetype.DOCUMENTER,
  Archetype.TASK_FORCE,
  Archetype.ROOM,
  Archetype.BLANK,
]

function CanvasContextMenu({ position, onClose }: CanvasContextMenuProps) {
  const allProtocols = useStore(protocolStore.store, protocolStore.selectAll)
  const protocolsByType = useMemo(() => Collections.keyBy(allProtocols, (p) => p.protocol_type), [allProtocols])

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

    // For documenter, preserve existing protocol-linking behavior
    if (archetype === Archetype.DOCUMENTER) {
      const protocol = protocolsByType.get('documenter')
      const createAndLink = async () => {
        const step = await workflowStore.createStep({
          name: `New ${config.label}`,
          execution_mode: config.executionMode,
          agent_id: protocol?.agent?.id,
          output_schema_id: protocol?.output_schema?.id,
          prompt_template_id: protocol?.prompt_template?.id,
          prompt_template: '',
          position_x: Math.round(position.flowX),
          position_y: Math.round(position.flowY),
        })
        if (step && protocol) {
          canvasStore.linkStepProtocol(step.id, {
            protocolId: protocol.id,
            protocolType: protocol.protocol_type,
            protocolName: protocol.name,
            portNames: protocol.ports.map((p) => p.port_name),
          })
        }
      }
      void createAndLink()
    } else {
      void workflowStore.createStep({
        name: `New ${config.label}`,
        execution_mode: config.executionMode,
        prompt_template: '',
        position_x: Math.round(position.flowX),
        position_y: Math.round(position.flowY),
      })
    }

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
        <Box data-testid="ctx-delete-step" onClick={handleDelete} sx={MENU_ITEM_SX}>
          <Typography sx={{ fontSize: 12, color: 'error.main' }}>Delete Step</Typography>
        </Box>
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
        </>
      )}
    </Box>
  )
}

export { CanvasContextMenu }
export type { MenuPosition }
