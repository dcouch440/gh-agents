import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Chip from '@mui/material/Chip'
import { useStore, workflowStore } from '@/stores'
import { DetailShell } from './DetailShell'
import type { RoomStepMember } from '@/types/workflow'

type RoomDetailProps = {
  stepId: string
  onClose: () => void
}

function RoomDetail({ stepId, onClose }: RoomDetailProps) {
  const step = useStore(workflowStore.store, workflowStore.selectStepById(stepId))
  const roomMembersByStep = useStore(workflowStore.store, workflowStore.selectRoomMembersByStep)
  const members = roomMembersByStep[stepId] ?? []

  const title = step?.name ?? 'Room'

  return (
    <DetailShell title={title} accentColor="#a78bfa" onClose={onClose}>
      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {step?.description && (
          <Typography sx={{ fontSize: 13, color: 'text.secondary', lineHeight: 1.5 }}>
            {step.description}
          </Typography>
        )}
        <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
          <Chip label={`${members.length} member${members.length !== 1 ? 's' : ''}`} size="small" variant="outlined" />
        </Box>
        {members.length > 0 ? (
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1.5 }}>
            {members.map((member: RoomStepMember) => (
              <Box
                key={member.id}
                sx={{
                  p: 1.5,
                  borderRadius: '8px',
                  border: 1,
                  borderColor: 'divider',
                  backgroundColor: (theme) => theme.palette.custom.hoverOverlay,
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
        ) : (
          <Typography sx={{ fontSize: 12, color: 'text.disabled', fontStyle: 'italic' }}>
            No members added yet
          </Typography>
        )}
      </Box>
    </DetailShell>
  )
}

export { RoomDetail }
export type { RoomDetailProps }
