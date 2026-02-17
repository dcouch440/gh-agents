import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useStore, workflowStore } from '@/stores'
import { DetailShell } from './DetailShell'

type MemberDetailProps = {
  artifactId: string
  onClose: () => void
}

function MemberDetail({ artifactId, onClose }: MemberDetailProps) {
  const member = useStore(workflowStore.store, workflowStore.selectRoomMemberById(artifactId))

  if (!member) {
    return (
      <DetailShell title="Member" accentColor="#a78bfa" onClose={onClose}>
        <Typography sx={{ color: 'text.disabled' }}>Member not found</Typography>
      </DetailShell>
    )
  }

  return (
    <DetailShell title={member.name} accentColor="#a78bfa" onClose={onClose}>
      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {member.role && (
          <Box>
            <Typography sx={{ fontSize: 11, fontWeight: 700, color: 'text.secondary', mb: 0.5, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
              Role
            </Typography>
            <Typography sx={{ fontSize: 13, color: 'text.primary', lineHeight: 1.5 }}>
              {member.role}
            </Typography>
          </Box>
        )}
        {member.perspective && (
          <Box>
            <Typography sx={{ fontSize: 11, fontWeight: 700, color: 'text.secondary', mb: 0.5, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
              Perspective
            </Typography>
            <Typography sx={{ fontSize: 13, color: 'text.primary', lineHeight: 1.5 }}>
              {member.perspective}
            </Typography>
          </Box>
        )}
      </Box>
    </DetailShell>
  )
}

export { MemberDetail }
export type { MemberDetailProps }
