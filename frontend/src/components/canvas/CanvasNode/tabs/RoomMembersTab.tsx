import { useEffect, useRef } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore } from '@/stores'
import type { RoomStepMember } from '@/types/workflow'

type RoomMembersTabProps = {
  stepId: string
}

function RoomMembersTab({ stepId }: RoomMembersTabProps) {
  const theme = useTheme()
  const members = useStore(workflowStore.store, workflowStore.selectRoomStepMembers(stepId))
  const fetchedRef = useRef(new Set<string>())

  useEffect(() => {
    if (!fetchedRef.current.has(stepId)) {
      fetchedRef.current.add(stepId)
      void workflowStore.fetchRoomStepMembers(stepId)
    }
  }, [stepId])

  if (members.length === 0) {
    return (
      <Box sx={{ p: 1.5, height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Typography sx={{ fontSize: 12, color: 'text.disabled', textAlign: 'center' }}>
          No members added yet. Use the chat to set up the room.
        </Typography>
      </Box>
    )
  }

  return (
    <Box sx={{ p: 1.5, display: 'flex', flexDirection: 'column', gap: 1, height: '100%', overflow: 'auto' }}>
      {members.map((member: RoomStepMember) => (
        <Box
          key={member.id}
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
          <Typography sx={{ fontSize: 12, fontWeight: 600, color: 'text.primary' }}>
            {member.name}
          </Typography>
          {member.role && (
            <Typography sx={{ fontSize: 11, color: 'text.secondary', lineHeight: 1.4 }}>
              {member.role}
            </Typography>
          )}
          {member.perspective && (
            <Typography sx={{ fontSize: 11, color: 'text.disabled', lineHeight: 1.4, fontStyle: 'italic' }}>
              {member.perspective}
            </Typography>
          )}
        </Box>
      ))}
    </Box>
  )
}

export { RoomMembersTab }
export type { RoomMembersTabProps }
