import { Checkbox, FormControlLabel, Box, Typography } from '@mui/material'
import type { Tool } from '@/types'

type ToolCheckListProps = {
  tools: readonly Tool[]
  selectedIds: ReadonlySet<string>
  onToggle: (toolId: string) => void
  disabled: boolean
}

function ToolCheckList({ tools, selectedIds, onToggle, disabled }: ToolCheckListProps) {
  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5 }}>
      {tools.map((tool) => (
        <FormControlLabel
          key={tool.id}
          control={
            <Checkbox checked={selectedIds.has(tool.id)} onChange={() => onToggle(tool.id)} size="small" disabled={disabled} />
          }
          label={
            <Box>
              <Typography variant="body2" sx={{ fontWeight: 500 }}>
                {tool.name}
              </Typography>
              {tool.description ? (
                <Typography variant="caption" color="text.secondary">
                  {tool.description}
                </Typography>
              ) : null}
            </Box>
          }
        />
      ))}
    </Box>
  )
}

export { ToolCheckList }
export type { ToolCheckListProps }
