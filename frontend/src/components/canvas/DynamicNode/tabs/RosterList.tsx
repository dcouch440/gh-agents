import { useState, useCallback } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Button from '@mui/material/Button'
import Chip from '@mui/material/Chip'
import AddOutlined from '@mui/icons-material/AddOutlined'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore } from '@/stores'
import type { RosterAgent } from '@/types/workflow'
import { RosterAddForm } from './RosterAddForm'

type RosterListProps = {
  stepId: string
  entityLabel: string
}

function RosterList({ stepId, entityLabel }: RosterListProps) {
  const theme = useTheme()
  const roster = useStore(workflowStore.store, workflowStore.selectStepRoster(stepId))
  const [adding, setAdding] = useState(false)

  const handleRemove = useCallback(
    (agentId: string) => {
      void workflowStore.deleteRosterAgent(stepId, agentId)
    },
    [stepId],
  )

  const handleSubmitNew = useCallback(
    (name: string, roleDescription: string) => {
      void workflowStore.createRosterAgent(stepId, {
        name: name.trim(),
        role_description: roleDescription.trim() || undefined,
        execution_order: roster.length,
      })
      setAdding(false)
    },
    [stepId, roster.length],
  )

  const lowerLabel = entityLabel.toLowerCase()

  return (
    <Box sx={{ p: 1.5, display: 'flex', flexDirection: 'column', gap: 1, height: '100%', overflow: 'auto' }}>
      {adding && (
        <RosterAddForm
          onSubmit={handleSubmitNew}
          onCancel={() => { setAdding(false) }}
        />
      )}

      {!adding && roster.length === 0 && (
        <Box sx={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <Typography sx={{ fontSize: 12, color: 'text.disabled' }}>No {lowerLabel}s configured yet</Typography>
        </Box>
      )}

      {roster.map((agent: RosterAgent) => (
        <Box
          key={agent.id}
          sx={{
            p: 1.5,
            borderRadius: '8px',
            border: 1,
            borderColor: 'divider',
            backgroundColor: theme.palette.custom.hoverOverlay,
            display: 'flex',
            flexDirection: 'column',
            gap: 0.5,
          }}
        >
          <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <Typography sx={{ fontSize: 12, fontWeight: 600, color: 'text.primary' }}>{agent.name}</Typography>
            <Box
              component="button"
              onClick={() => { handleRemove(agent.id) }}
              sx={{
                all: 'unset',
                cursor: 'pointer',
                fontSize: 12,
                color: 'text.disabled',
                lineHeight: 1,
                '&:hover': { color: 'error.main' },
              }}
            >
              &times;
            </Box>
          </Box>
          {agent.role_description && (
            <Typography sx={{ fontSize: 11, color: 'text.secondary', lineHeight: 1.4 }}>
              {agent.role_description}
            </Typography>
          )}
          {agent.capabilities.length > 0 && (
            <Box sx={{ display: 'flex', gap: 0.5, flexWrap: 'wrap' }}>
              {agent.capabilities.map((cap) => (
                <Chip key={cap} label={cap} size="small" sx={{ fontSize: 10, height: 18 }} />
              ))}
            </Box>
          )}
        </Box>
      ))}

      {!adding && (
        <Button
          variant="outlined"
          size="small"
          startIcon={<AddOutlined />}
          onClick={() => { setAdding(true) }}
          sx={{ alignSelf: 'stretch' }}
        >
          Add {entityLabel}
        </Button>
      )}
    </Box>
  )
}

export { RosterList }
export type { RosterListProps }
