import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Chip from '@mui/material/Chip'
import { useStore, workflowStore } from '@/stores'
import { DetailShell } from './DetailShell'

type DocumentDetailProps = {
  artifactId: string
  onClose: () => void
}

function DocumentDetail({ artifactId, onClose }: DocumentDetailProps) {
  const doc = useStore(workflowStore.store, workflowStore.selectDocumentDefById(artifactId))
  const contentByDefId = useStore(workflowStore.store, workflowStore.selectDocumentContentByDefId)

  if (!doc) {
    return (
      <DetailShell title="Document" accentColor="#D4793E" onClose={onClose}>
        <Typography sx={{ color: 'text.disabled' }}>Document not found</Typography>
      </DetailShell>
    )
  }

  const content = contentByDefId[artifactId] ?? null

  return (
    <DetailShell title={doc.name} accentColor="#D4793E" onClose={onClose}>
      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {doc.description && (
          <Typography sx={{ fontSize: 13, color: 'text.secondary', lineHeight: 1.5 }}>
            {doc.description}
          </Typography>
        )}
        <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
          <Chip label={`Target: ~${doc.target_length} chars`} size="small" variant="outlined" />
          {doc.document_id !== null && <Chip label="Generated" size="small" color="success" variant="outlined" />}
        </Box>
        {content !== null ? (
          <Box
            sx={{
              p: 2,
              borderRadius: '8px',
              border: 1,
              borderColor: 'divider',
              backgroundColor: (theme) => theme.palette.custom.hoverOverlay,
              whiteSpace: 'pre-wrap',
              fontSize: 13,
              lineHeight: 1.6,
              color: 'text.primary',
              fontFamily: 'monospace',
            }}
          >
            {content}
          </Box>
        ) : (
          <Typography sx={{ fontSize: 12, color: 'text.disabled', fontStyle: 'italic' }}>
            No content generated yet
          </Typography>
        )}
      </Box>
    </DetailShell>
  )
}

export { DocumentDetail }
export type { DocumentDetailProps }
