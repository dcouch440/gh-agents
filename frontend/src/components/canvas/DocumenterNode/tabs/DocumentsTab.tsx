import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Button from '@mui/material/Button'
import AddOutlined from '@mui/icons-material/AddOutlined'
import { useTheme } from '@mui/material/styles'

type DocumentDef = {
  id: string
  name: string
  description: string
  targetLength: number
}

type DocumentsTabProps = {
  documents: DocumentDef[]
  onAdd: () => void
  onRemove: (id: string) => void
}

function DocumentsTab({ documents, onAdd, onRemove }: DocumentsTabProps) {
  const theme = useTheme()

  return (
    <Box sx={{ p: 1.5, display: 'flex', flexDirection: 'column', gap: 1, height: '100%', overflow: 'auto' }}>
      {documents.length === 0 && (
        <Box sx={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <Typography sx={{ fontSize: 12, color: 'text.disabled' }}>
            No documents configured
          </Typography>
        </Box>
      )}

      {documents.map((doc) => (
        <Box
          key={doc.id}
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
            <Typography sx={{ fontSize: 12, fontWeight: 600, color: 'text.primary' }}>
              {doc.name}
            </Typography>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
              <Typography sx={{ fontSize: 10, color: 'text.disabled' }}>
                {doc.targetLength} chars
              </Typography>
              <Box
                component="button"
                onClick={() => { onRemove(doc.id) }}
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
          </Box>
          <Typography sx={{ fontSize: 11, color: 'text.secondary', lineHeight: 1.4 }}>
            {doc.description}
          </Typography>
        </Box>
      ))}

      <Button
        variant="outlined"
        size="small"
        startIcon={<AddOutlined />}
        onClick={onAdd}
        sx={{ alignSelf: 'stretch' }}
      >
        Add Document
      </Button>
    </Box>
  )
}

export { DocumentsTab }
export type { DocumentsTabProps, DocumentDef }
