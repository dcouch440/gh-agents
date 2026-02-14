import { useCallback, useEffect, useMemo } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Popover from '@mui/material/Popover'
import Checkbox from '@mui/material/Checkbox'
import type { SxProps, Theme } from '@mui/material/styles'
import { useStore, workflowStore, canvasStore, contextMentionStore } from '@/stores'
import type { PickableEntity, PickableEntityKind } from '@/stores/contextMentionStore'
import { Collections } from '@/utils/collections'
import { Archetype, ARCHETYPE_CONFIGS } from './DynamicNode/archetypes'
import { STEP_TYPE_COLORS } from './constants'
import type { WorkflowStep } from '@/types/workflow'

// ── Types ────────────────────────────────────────────────────────────────────

type ContextMentionMenuProps = {
  anchorEl: HTMLElement | null
  onClose: () => void
  stepId: string
}

type MenuEntity = {
  entity: PickableEntity
  color: string
  group: string
}

// ── Styling ──────────────────────────────────────────────────────────────────

const SECTION_LABEL_SX: SxProps<Theme> = {
  px: 1.5,
  py: 0.75,
  fontSize: 10,
  textTransform: 'uppercase',
  color: 'text.disabled',
  letterSpacing: '0.05em',
  fontWeight: 600,
}

const MENU_ITEM_SX: SxProps<Theme> = {
  display: 'flex',
  alignItems: 'center',
  gap: 0.5,
  px: 1,
  py: 0.25,
  cursor: 'pointer',
  '&:hover': { backgroundColor: 'action.hover' },
}

const COLOR_DOT_SX: SxProps<Theme> = {
  width: 8,
  height: 8,
  borderRadius: '50%',
  flexShrink: 0,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

const resolveStepColor = (step: WorkflowStep, protocolType: string | null): string => {
  if (step.execution_mode === 'context') return STEP_TYPE_COLORS['context'] ?? '#7d8590'
  if (step.execution_mode === 'documenter' || protocolType === 'documenter') return ARCHETYPE_CONFIGS[Archetype.DOCUMENTER].color
  if (step.execution_mode === 'task_force') return ARCHETYPE_CONFIGS[Archetype.TASK_FORCE].color
  if (step.execution_mode === 'room') return ARCHETYPE_CONFIGS[Archetype.ROOM].color
  return ARCHETYPE_CONFIGS[Archetype.BLANK].color
}

const resolveGroup = (step: WorkflowStep, protocolType: string | null): string => {
  if (step.execution_mode === 'context') return 'Context Nodes'
  if (step.execution_mode === 'documenter' || protocolType === 'documenter') return 'Documenters'
  if (step.execution_mode === 'task_force') return 'Task Forces'
  if (step.execution_mode === 'room') return 'Rooms'
  return 'Steps'
}

const resolveKind = (step: WorkflowStep): PickableEntityKind =>
  step.execution_mode === 'context' ? 'context-node' : 'workflow-step'

const buildEntity = (step: WorkflowStep): PickableEntity => ({
  kind: resolveKind(step),
  id: step.id,
  name: step.name ?? 'Unnamed',
  summary: `${step.execution_mode} step`,
  data: {
    execution_mode: step.execution_mode,
    prompt_template: step.prompt_template,
    description: step.description,
    content: step.execution_mode === 'context' ? step.prompt_template : undefined,
  },
})

const GROUP_ORDER = ['Context Nodes', 'Documenters', 'Task Forces', 'Rooms', 'Steps']

// ── Component ────────────────────────────────────────────────────────────────

function ContextMentionMenu({ anchorEl, onClose, stepId }: ContextMentionMenuProps) {
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)
  const stepProtocols = useStore(canvasStore.store, canvasStore.selectStepProtocols)
  const mentions = useStore(contextMentionStore.store, contextMentionStore.selectMentions(stepId))
  const checkedIds = useMemo(() => new Set(mentions.map((m) => m.entityId)), [mentions])

  const menuEntities = useMemo((): MenuEntity[] => {
    const items: MenuEntity[] = []
    for (const step of steps) {
      if (step.id === stepId) continue
      const link = stepProtocols[step.id]
      const protocolType = link?.protocolType ?? null
      items.push({
        entity: buildEntity(step),
        color: resolveStepColor(step, protocolType),
        group: resolveGroup(step, protocolType),
      })
    }
    return items
  }, [steps, stepId, stepProtocols])

  const grouped = useMemo(() => {
    const groups = Collections.groupBy(menuEntities, (e) => e.group)
    return GROUP_ORDER.filter((g) => groups.has(g)).map((g) => ({
      label: g,
      items: groups.get(g)!,
    }))
  }, [menuEntities])

  const handleToggle = useCallback(
    (item: MenuEntity) => {
      if (checkedIds.has(item.entity.id)) {
        contextMentionStore.removeByEntityId(stepId, item.entity.id)
      } else {
        contextMentionStore.addMention(stepId, item.entity, item.color)
      }
    },
    [stepId, checkedIds],
  )

  // Close on right-click anywhere
  useEffect(() => {
    if (!anchorEl) return
    const handleContextMenu = (e: MouseEvent) => {
      e.preventDefault()
      onClose()
    }
    document.addEventListener('contextmenu', handleContextMenu)
    return () => {
      document.removeEventListener('contextmenu', handleContextMenu)
    }
  }, [anchorEl, onClose])

  // Close on ESC
  useEffect(() => {
    if (!anchorEl) return
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose()
      }
    }
    document.addEventListener('keydown', handleKeyDown)
    return () => {
      document.removeEventListener('keydown', handleKeyDown)
    }
  }, [anchorEl, onClose])

  return (
    <Popover
      open={anchorEl !== null}
      anchorEl={anchorEl}
      onClose={() => {
        // Only close from our custom handlers (right-click, ESC)
        // Do nothing here so clicks on canvas don't close it
      }}
      disableAutoFocus
      disableEnforceFocus
      disableRestoreFocus
      hideBackdrop
      anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
      transformOrigin={{ vertical: 'top', horizontal: 'left' }}
      slotProps={{
        paper: {
          sx: {
            backgroundColor: 'background.paper',
            border: 1,
            borderColor: 'divider',
            borderRadius: '8px',
            boxShadow: (theme) =>
              theme.palette.mode === 'dark' ? '0 4px 24px rgba(0, 0, 0, 0.4)' : '0 4px 24px rgba(45, 27, 14, 0.14)',
            minWidth: 220,
            maxHeight: 320,
            overflowY: 'auto',
            py: 0.5,
          },
        },
      }}
    >
      {grouped.length === 0 ? (
        <Typography sx={{ px: 1.5, py: 1, fontSize: 11, color: 'text.secondary' }}>No other nodes available</Typography>
      ) : (
        grouped.map((group, gi) => (
          <Box key={group.label}>
            {gi > 0 && <Box sx={{ mx: 1.5, my: 0.5, borderTop: 1, borderColor: 'divider' }} />}
            <Typography sx={SECTION_LABEL_SX}>{group.label}</Typography>
            {group.items.map((item) => (
              <Box
                key={item.entity.id}
                onClick={() => {
                  handleToggle(item)
                }}
                sx={MENU_ITEM_SX}
              >
                <Checkbox
                  checked={checkedIds.has(item.entity.id)}
                  size="small"
                  tabIndex={-1}
                  disableRipple
                  sx={{
                    p: 0.25,
                    '& .MuiSvgIcon-root': { fontSize: 16 },
                    color: item.color,
                    '&.Mui-checked': { color: item.color },
                  }}
                />
                <Box sx={{ ...COLOR_DOT_SX, backgroundColor: item.color }} />
                <Typography
                  sx={{
                    fontSize: 12,
                    color: 'text.primary',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {item.entity.name}
                </Typography>
              </Box>
            ))}
          </Box>
        ))
      )}
    </Popover>
  )
}

export { ContextMentionMenu }
export type { ContextMentionMenuProps }
