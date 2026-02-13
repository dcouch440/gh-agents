import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'

type RoomMembersTabProps = {
  stepId: string
}

function RoomMembersTab({ stepId: _stepId }: RoomMembersTabProps) {
  return (
    <Box sx={{ p: 1.5, height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
      <Typography sx={{ fontSize: 12, color: 'text.disabled', textAlign: 'center' }}>
        No members added yet. Use the chat to set up the room.
      </Typography>
    </Box>
  )
}

export { RoomMembersTab }
export type { RoomMembersTabProps }
