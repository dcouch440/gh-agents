import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import {DESIGN} from '@/constants'
import type {VariableCompletion} from '@/utils/variableContext'

type VariableChipStripProps = {
  completions: VariableCompletion[]
  onCopy: ((label: string) => void) | null
}

function VariableChipStrip({completions, onCopy}: VariableChipStripProps) {
  if (completions.length === 0) return null

  const groups = new Map<string, VariableCompletion[]>()
  for (const c of completions) {
    const list = groups.get(c.section)
    if (list) {
      list.push(c)
    } else {
      groups.set(c.section, [c])
    }
  }

  return (
    <Box
      sx={{
        px: '16px',
        py: '8px',
        borderBottom: 1,
        borderColor: 'divider',
        maxHeight: 120,
        overflow: 'auto',
        flexShrink: 0,
      }}
    >
      {[...groups.entries()].map(([section, items]) => (
        <Box key={section} sx={{mb: groups.size > 1 ? 0.5 : 0}}>
          <Typography
            sx={{
              fontSize: 9,
              fontWeight: 600,
              letterSpacing: '0.06em',
              textTransform: 'uppercase',
              color: 'text.disabled',
              mb: '3px',
              lineHeight: 1,
            }}
          >
            {section}
          </Typography>
          <Box sx={{display: 'flex', flexWrap: 'wrap', gap: 0.5}}>
            {items.map((c) => (
              <Box
                key={c.label}
                onClick={onCopy !== null ? () => { onCopy(c.label) } : undefined}
                title="Click to copy"
                sx={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  px: '6px',
                  py: '2px',
                  borderRadius: '4px',
                  backgroundColor: `${DESIGN.SYN_VARIABLE}12`,
                  border: 1,
                  borderColor: `${DESIGN.SYN_VARIABLE}30`,
                  cursor: onCopy !== null ? 'pointer' : 'default',
                  transition: 'all 120ms ease',
                  userSelect: 'none',
                  '&:hover': onCopy !== null
                    ? {
                        backgroundColor: `${DESIGN.SYN_VARIABLE}22`,
                        borderColor: `${DESIGN.SYN_VARIABLE}50`,
                      }
                    : {},
                }}
              >
                <Typography
                  sx={{
                    fontSize: 10,
                    fontFamily: 'monospace',
                    fontWeight: 500,
                    color: DESIGN.SYN_VARIABLE,
                    lineHeight: 1,
                    whiteSpace: 'nowrap',
                  }}
                >
                  {c.label}
                </Typography>
              </Box>
            ))}
          </Box>
        </Box>
      ))}
    </Box>
  )
}

export {VariableChipStrip}
export type {VariableChipStripProps}
