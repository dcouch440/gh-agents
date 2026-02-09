import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useStore, workflowStore, protocolStore, canvasStore } from '@/stores'
import { STEP_TYPE_COLORS, DEFAULT_STEP_TYPE_COLOR, PROTOCOL_TYPE_COLORS } from './constants'

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

const STEP_TYPES = [
  { key: 'single', label: 'LLM Step' },
  { key: 'for_each', label: 'For-Each Step' },
  { key: 'room', label: 'Room Step' },
] as const

const PROTOCOL_LABELS: Record<string, string> = {
  decomp: 'Decomp',
  route: 'Route',
  review: 'Review',
  transform: 'Transform',
}

function CanvasContextMenu({ position, onClose }: CanvasContextMenuProps) {
  const protocolTypes = useStore(protocolStore.store, protocolStore.selectTypes)
  const allProtocols = useStore(protocolStore.store, protocolStore.selectAll)

  if (!position) return null

  const handleAdd = (event: React.MouseEvent, stepType: string, label: string) => {
    event.stopPropagation()
    event.preventDefault()
    void workflowStore.createStep({
      name: `New ${label}`,
      execution_mode: stepType,
      position_x: Math.round(position.flowX),
      position_y: Math.round(position.flowY),
    })
    onClose()
  }

  const handleAddProtocol = (event: React.MouseEvent, protocolType: string) => {
    event.stopPropagation()
    event.preventDefault()
    const label = PROTOCOL_LABELS[protocolType] ?? protocolType
    // Find the matching protocol to get its agent
    const protocol = allProtocols.find((p) => p.protocol_type === protocolType)
    const createAndLink = async () => {
      const step = await workflowStore.createStep({
        name: `New ${label}`,
        execution_mode: 'single',
        agent_id: protocol?.agent?.id,
        output_schema_id: protocol?.output_schema?.id,
        prompt_template_id: protocol?.prompt_template?.id,
        prompt_template: protocol?.prompt_template?.content ?? '',
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

  // Filter out 'default' — it's not a user-facing protocol type
  const visibleProtocolTypes = protocolTypes.filter((t) => t.name !== 'default')

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
      {position.nodeId !== undefined ? (
        <Box
          onClick={handleDelete}
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
          <Typography sx={{ fontSize: 12, color: 'error.main' }}>
            Delete Step
          </Typography>
        </Box>
      ) : (
        <>
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
              onClick={(event) => {
                handleAdd(event, st.key, st.label)
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
          {visibleProtocolTypes.length > 0 && (
            <>
              <Box sx={{ mx: 1.5, my: 0.5, borderTop: 1, borderColor: 'divider' }} />
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
                Protocol
              </Typography>
              {visibleProtocolTypes.map((pt) => (
                <Box
                  key={pt.name}
                  onClick={(event) => {
                    handleAddProtocol(event, pt.name)
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
                      backgroundColor: PROTOCOL_TYPE_COLORS[pt.name] ?? DEFAULT_STEP_TYPE_COLOR,
                      flexShrink: 0,
                    }}
                  />
                  <Typography sx={{ fontSize: 12, color: 'text.primary' }}>
                    {PROTOCOL_LABELS[pt.name] ?? pt.name}
                  </Typography>
                </Box>
              ))}
            </>
          )}
        </>
      )}
    </Box>
  )
}

export { CanvasContextMenu }
export type { MenuPosition }
