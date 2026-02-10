import { useState } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Button from '@mui/material/Button'
import InputBase from '@mui/material/InputBase'
import AddOutlined from '@mui/icons-material/AddOutlined'
import { useTheme } from '@mui/material/styles'
import type { DocumentDef, CreateDocumentDefRequest } from '@/types/workflow'

type DocumentsTabProps = {
  documents: DocumentDef[]
  adding: boolean
  onAdd: () => void
  onSubmitNew: (body: CreateDocumentDefRequest) => void
  onCancelAdd: () => void
  onRemove: (id: string) => void
}

function InlineAddForm({ onSubmit, onCancel }: { onSubmit: (body: CreateDocumentDefRequest) => void; onCancel: () => void }) {
  const theme = useTheme()
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [targetLength, setTargetLength] = useState(1000)

  const isValid = name.trim().length > 0

  const handleSubmit = () => {
    if (!isValid) return
    onSubmit({
      name: name.trim(),
      description: description.trim() || undefined,
      target_length: targetLength,
    })
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && isValid) {
      e.preventDefault()
      handleSubmit()
    }
    if (e.key === 'Escape') {
      e.preventDefault()
      onCancel()
    }
  }

  return (
    <Box
      sx={{
        p: 1,
        borderRadius: '8px',
        border: 1,
        borderColor: 'primary.main',
        backgroundColor: theme.palette.custom.hoverOverlay,
        display: 'flex',
        flexDirection: 'column',
        gap: 0.75,
      }}
    >
      <InputBase
        autoFocus
        value={name}
        onChange={(e) => {
          setName(e.target.value)
        }}
        onKeyDown={handleKeyDown}
        placeholder="Document name"
        sx={{
          fontSize: 12,
          fontWeight: 600,
          px: 0.75,
          py: 0.25,
          borderRadius: '4px',
          border: 1,
          borderColor: 'divider',
          backgroundColor: 'background.paper',
        }}
      />
      <InputBase
        value={description}
        onChange={(e) => {
          setDescription(e.target.value)
        }}
        onKeyDown={handleKeyDown}
        placeholder="Description (optional)"
        multiline
        rows={2}
        sx={{
          fontSize: 11,
          px: 0.75,
          py: 0.25,
          borderRadius: '4px',
          border: 1,
          borderColor: 'divider',
          backgroundColor: 'background.paper',
        }}
      />
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
        <Typography sx={{ fontSize: 10, color: 'text.secondary', whiteSpace: 'nowrap' }}>Target length:</Typography>
        <InputBase
          value={targetLength}
          onChange={(e) => {
            const n = parseInt(e.target.value, 10)
            if (!isNaN(n) && n >= 0) setTargetLength(n)
          }}
          onKeyDown={handleKeyDown}
          type="number"
          sx={{
            fontSize: 11,
            px: 0.5,
            py: 0.25,
            borderRadius: '4px',
            border: 1,
            borderColor: 'divider',
            backgroundColor: 'background.paper',
            width: 70,
            '& input': { textAlign: 'right' },
          }}
        />
      </Box>
      <Box sx={{ display: 'flex', gap: 0.5, justifyContent: 'flex-end' }}>
        <Button size="small" onClick={onCancel} sx={{ fontSize: 11, textTransform: 'none', minWidth: 0, px: 1 }}>
          Cancel
        </Button>
        <Button
          size="small"
          variant="contained"
          onClick={handleSubmit}
          disabled={!isValid}
          sx={{ fontSize: 11, textTransform: 'none', minWidth: 0, px: 1 }}
        >
          Add
        </Button>
      </Box>
    </Box>
  )
}

function DocumentsTab({ documents, adding, onAdd, onSubmitNew, onCancelAdd, onRemove }: DocumentsTabProps) {
  const theme = useTheme()

  return (
    <Box sx={{ p: 1.5, display: 'flex', flexDirection: 'column', gap: 1, height: '100%', overflow: 'auto' }}>
      {adding && <InlineAddForm onSubmit={onSubmitNew} onCancel={onCancelAdd} />}

      {!adding && documents.length === 0 && (
        <Box sx={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <Typography sx={{ fontSize: 12, color: 'text.disabled' }}>No documents configured</Typography>
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
            <Typography sx={{ fontSize: 12, fontWeight: 600, color: 'text.primary' }}>{doc.name}</Typography>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
              <Typography sx={{ fontSize: 10, color: 'text.disabled' }}>{doc.target_length} chars</Typography>
              <Box
                component="button"
                onClick={() => {
                  onRemove(doc.id)
                }}
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
          {doc.description && <Typography sx={{ fontSize: 11, color: 'text.secondary', lineHeight: 1.4 }}>{doc.description}</Typography>}
        </Box>
      ))}

      {!adding && (
        <Button variant="outlined" size="small" startIcon={<AddOutlined />} onClick={onAdd} sx={{ alignSelf: 'stretch' }}>
          Add Document
        </Button>
      )}
    </Box>
  )
}

export { DocumentsTab }
export type { DocumentsTabProps }
