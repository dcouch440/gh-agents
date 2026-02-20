import { useReducer, useCallback } from 'react'
import { Box, Button, Stack } from '@mui/material'
import { parsePanel } from './parsePanel'
import { PanelSectionRenderer } from './PanelSectionRenderer'
import type { PanelSection } from './parsePanel'

type PanelOverlayProps = {
  content: string
  submitLabel: string
  onSubmit: (selections: string) => void
  onDismiss: () => void
}

// Selection state: Map<itemId, boolean>
type SelectionAction =
  | { type: 'TOGGLE'; id: string }

const selectionReducer = (state: Map<string, boolean>, action: SelectionAction): Map<string, boolean> => {
  const next = new Map(state)
  const current = next.get(action.id) ?? false
  next.set(action.id, !current)
  return next
}

const serializeSelections = (
  sections: PanelSection[],
  selections: Map<string, boolean>,
): string => {
  const lines: string[] = []

  const walk = (s: PanelSection) => {
    for (const item of s.interactiveItems) {
      const checked = selections.get(item.id) ?? item.checked
      lines.push(`- [${checked ? 'x' : ' '}] ${item.label}`)
    }
    for (const child of s.children) walk(child)
  }

  for (const section of sections) walk(section)
  return lines.length > 0 ? lines.join('\n') : '(no selections)'
}

function PanelOverlay({ content, submitLabel, onSubmit, onDismiss }: PanelOverlayProps) {
  const sections = parsePanel(content)
  const [selections, dispatchSelection] = useReducer(selectionReducer, new Map<string, boolean>())

  const handleToggle = useCallback((id: string) => {
    dispatchSelection({ type: 'TOGGLE', id })
  }, [])

  const handleSubmit = useCallback(() => {
    onSubmit(serializeSelections(sections, selections))
  }, [onSubmit, sections, selections])

  return (
    <Box
      sx={{
        position: 'absolute',
        inset: 0,
        zIndex: 10,
        display: 'flex',
        flexDirection: 'column',
        bgcolor: 'rgba(0, 0, 0, 0.4)',
        backdropFilter: 'blur(2px)',
      }}
    >
      <Box
        sx={{
          flex: 1,
          overflow: 'auto',
          p: 2,
        }}
      >
        <Stack spacing={1.5}>
          {sections.map((section) => (
            <PanelSectionRenderer
              key={section.id}
              section={section}
              selections={selections}
              onToggle={handleToggle}
            />
          ))}
        </Stack>
      </Box>

      <Box
        sx={{
          px: 2,
          py: 1.5,
          borderTop: 1,
          borderColor: 'divider',
          bgcolor: 'background.paper',
          display: 'flex',
          justifyContent: 'flex-end',
          gap: 1,
        }}
      >
        <Button size="small" onClick={onDismiss}>
          Dismiss
        </Button>
        <Button size="small" variant="contained" onClick={handleSubmit}>
          {submitLabel}
        </Button>
      </Box>
    </Box>
  )
}

export { PanelOverlay }
export type { PanelOverlayProps }
