import { useReducer, useCallback } from 'react'
import { Box, Button, Typography } from '@mui/material'
import CheckIcon from '@mui/icons-material/Check'
import { parsePanel } from '@/components/canvas/CanvasNode/tabs/panel/parsePanel'
import type { PanelSection } from '@/components/canvas/CanvasNode/tabs/panel/parsePanel'
import { PanelCheckbox } from '@/components/primitives/PanelCheckbox'
import { TerminalBlock } from '@/components/primitives/terminal-renderer'

type InlinePanelMessageProps = {
  content: string
  submitLabel: string
  submitted: boolean
  onSubmit: (messageId: string, selections: string) => void
  messageId: string
}

type SelectionAction = { type: 'TOGGLE'; id: string }

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

type SectionRendererProps = {
  section: PanelSection
  selections: Map<string, boolean>
  onToggle: (id: string) => void
  disabled: boolean
}

const sectionToMarkdown = (section: PanelSection): string => {
  const parts: string[] = []
  if (section.title) {
    const prefix = '#'.repeat(section.depth + 1)
    parts.push(`${prefix} ${section.title}`)
  }
  if (section.bodyMarkdown) {
    parts.push(section.bodyMarkdown)
  }
  return parts.join('\n\n')
}

function InlinePanelSection({ section, selections, onToggle, disabled }: SectionRendererProps) {
  const markdown = sectionToMarkdown(section)

  return (
    <Box>
      {markdown ? <TerminalBlock content={markdown} /> : null}

      {section.interactiveItems.length > 0 ? (
        <Box sx={{ pl: 0.5, py: 0.5 }}>
          {section.interactiveItems.map((item) => (
            <PanelCheckbox
              key={item.id}
              label={item.label}
              checked={selections.get(item.id) ?? item.checked}
              onChange={() => onToggle(item.id)}
              disabled={disabled}
            />
          ))}
        </Box>
      ) : null}

      {section.children.map((child) => (
        <InlinePanelSection
          key={child.id}
          section={child}
          selections={selections}
          onToggle={onToggle}
          disabled={disabled}
        />
      ))}
    </Box>
  )
}

function InlinePanelMessage({ content, submitLabel, submitted, onSubmit, messageId }: InlinePanelMessageProps) {
  const sections = parsePanel(content)
  const [selections, dispatchSelection] = useReducer(selectionReducer, new Map<string, boolean>())

  const handleToggle = useCallback((id: string) => {
    dispatchSelection({ type: 'TOGGLE', id })
  }, [])

  const handleSubmit = useCallback(() => {
    onSubmit(messageId, serializeSelections(sections, selections))
  }, [onSubmit, messageId, sections, selections])

  return (
    <Box sx={{ py: 0.25 }}>
      {sections.map((section) => (
        <InlinePanelSection
          key={section.id}
          section={section}
          selections={selections}
          onToggle={handleToggle}
          disabled={submitted}
        />
      ))}

      <Box sx={{ display: 'flex', justifyContent: 'flex-end', pt: 0.5 }}>
        {submitted ? (
          <Typography
            variant="caption"
            sx={{
              display: 'flex',
              alignItems: 'center',
              gap: 0.5,
              color: 'text.secondary',
              opacity: 0.7,
            }}
          >
            <CheckIcon sx={{ fontSize: '0.875rem' }} />
            Submitted
          </Typography>
        ) : (
          <Button size="small" variant="contained" onClick={handleSubmit}>
            {submitLabel}
          </Button>
        )}
      </Box>
    </Box>
  )
}

export { InlinePanelMessage }
export type { InlinePanelMessageProps }
