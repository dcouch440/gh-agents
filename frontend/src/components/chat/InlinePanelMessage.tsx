import { useReducer, useCallback } from 'react'
import { Box, Button, InputBase, Typography } from '@mui/material'
import CheckIcon from '@mui/icons-material/Check'
import { parsePanel } from '@/components/canvas/CanvasNode/tabs/panel/parsePanel'
import type { PanelSection, PanelInteractiveItem } from '@/components/canvas/CanvasNode/tabs/panel/parsePanel'
import { PanelCheckbox } from '@/components/primitives/PanelCheckbox'
import { PanelTextInput } from '@/components/primitives/PanelTextInput'
import { TerminalBlock } from '@/components/primitives/terminal-renderer'

type InlinePanelMessageProps = {
  content: string
  submitLabel: string
  submitted: boolean
  onSubmit: (messageId: string, selections: string) => void
  messageId: string
}

type PanelState = {
  checkboxes: Map<string, boolean>
  textInputs: Map<string, string>
  notes: string
}

type PanelAction =
  | { type: 'TOGGLE'; id: string }
  | { type: 'SET_TEXT'; id: string; value: string }
  | { type: 'SET_NOTES'; value: string }

const initialState: PanelState = {
  checkboxes: new Map(),
  textInputs: new Map(),
  notes: '',
}

const panelReducer = (state: PanelState, action: PanelAction): PanelState => {
  switch (action.type) {
    case 'TOGGLE': {
      const next = new Map(state.checkboxes)
      const current = next.get(action.id) ?? false
      next.set(action.id, !current)
      return { ...state, checkboxes: next }
    }
    case 'SET_TEXT': {
      const next = new Map(state.textInputs)
      next.set(action.id, action.value)
      return { ...state, textInputs: next }
    }
    case 'SET_NOTES':
      return { ...state, notes: action.value }
  }
}

const serializeSelections = (
  sections: PanelSection[],
  state: PanelState,
): string => {
  const lines: string[] = []

  const walk = (s: PanelSection) => {
    for (const item of s.interactiveItems) {
      if (item.type === 'checkbox') {
        const checked = state.checkboxes.get(item.id) ?? item.checked
        lines.push(`- [${checked ? 'x' : ' '}] ${item.label}`)
      } else {
        const value = state.textInputs.get(item.id) ?? ''
        if (value) {
          lines.push(`- [> ${item.label}]: ${value}`)
        }
      }
    }
    for (const child of s.children) walk(child)
  }

  for (const section of sections) walk(section)

  if (state.notes.trim()) {
    lines.push('')
    lines.push(`Notes:`)
    lines.push(state.notes.trim())
  }

  return lines.length > 0 ? lines.join('\n') : '(no selections)'
}

type SectionRendererProps = {
  section: PanelSection
  state: PanelState
  dispatch: (action: PanelAction) => void
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

const renderInteractiveItem = (
  item: PanelInteractiveItem,
  state: PanelState,
  dispatch: (action: PanelAction) => void,
  disabled: boolean,
) => {
  switch (item.type) {
    case 'text_input':
      return (
        <PanelTextInput
          key={item.id}
          label={item.label}
          value={state.textInputs.get(item.id) ?? ''}
          onChange={(value) => dispatch({ type: 'SET_TEXT', id: item.id, value })}
          disabled={disabled}
        />
      )
    case 'checkbox':
      return (
        <PanelCheckbox
          key={item.id}
          label={item.label}
          checked={state.checkboxes.get(item.id) ?? item.checked}
          onChange={() => dispatch({ type: 'TOGGLE', id: item.id })}
          disabled={disabled}
        />
      )
  }
}

function InlinePanelSection({ section, state, dispatch, disabled }: SectionRendererProps) {
  const markdown = sectionToMarkdown(section)

  return (
    <Box>
      {markdown ? <TerminalBlock content={markdown} /> : null}

      {section.interactiveItems.length > 0 ? (
        <Box sx={{ py: 0.25 }}>
          {section.interactiveItems.map((item) => renderInteractiveItem(item, state, dispatch, disabled))}
        </Box>
      ) : null}

      {section.children.map((child) => (
        <InlinePanelSection
          key={child.id}
          section={child}
          state={state}
          dispatch={dispatch}
          disabled={disabled}
        />
      ))}
    </Box>
  )
}

function InlinePanelMessage({ content, submitLabel, submitted, onSubmit, messageId }: InlinePanelMessageProps) {
  const sections = parsePanel(content)
  const [state, dispatch] = useReducer(panelReducer, initialState)

  const handleSubmit = useCallback(() => {
    onSubmit(messageId, serializeSelections(sections, state))
  }, [onSubmit, messageId, sections, state])

  return (
    <Box sx={{ py: 0.25 }}>
      {sections.map((section) => (
        <InlinePanelSection
          key={section.id}
          section={section}
          state={state}
          dispatch={dispatch}
          disabled={submitted}
        />
      ))}

      {submitted && !state.notes ? null : (
        <Box
          sx={{
            display: 'flex',
            alignItems: 'baseline',
            gap: 0.75,
            py: 0.25,
            px: 0.5,
            mx: -0.5,
            opacity: submitted ? 0.6 : 1,
          }}
        >
          <Typography
            component="span"
            sx={{
              fontFamily: 'monospace',
              fontSize: '0.8125rem',
              lineHeight: 1.6,
              flexShrink: 0,
              userSelect: 'none',
            }}
          >
            [{state.notes ? 'X' : '\u00A0'}]
          </Typography>
          {submitted ? (
            <Typography
              component="span"
              sx={{
                fontFamily: 'monospace',
                fontSize: '0.8125rem',
                lineHeight: 1.6,
                whiteSpace: 'pre-wrap',
              }}
            >
              {state.notes}
            </Typography>
          ) : (
            <InputBase
              fullWidth
              multiline
              maxRows={6}
              value={state.notes}
              onChange={(e) => dispatch({ type: 'SET_NOTES', value: e.target.value })}
              placeholder="Describe your own solution"
              sx={{
                fontFamily: 'monospace',
                fontSize: '0.8125rem',
                lineHeight: 1.6,
                p: 0,
                '& .MuiInputBase-input': {
                  p: 0,
                  '&::placeholder': { opacity: 0.35, textDecoration: 'underline' },
                },
              }}
            />
          )}
        </Box>
      )}

      <Box sx={{ display: 'flex', justifyContent: 'flex-end', pt: 0.5 }}>
        {submitted ? (
          <Typography
            variant="caption"
            sx={{
              display: 'flex',
              alignItems: 'center',
              gap: 0.5,
              fontFamily: 'monospace',
              fontSize: '0.8125rem',
              color: 'text.secondary',
              opacity: 0.7,
            }}
          >
            <CheckIcon sx={{ fontSize: '0.8125rem' }} />
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
