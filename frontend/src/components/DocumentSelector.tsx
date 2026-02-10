import { useState, useEffect } from 'react'
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  Box,
  Typography,
  Checkbox,
  Chip,
  Collapse,
  List,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
} from '@mui/material'
import { useStore, documentStore } from '@/stores'
import { LoadingSpinner, EmptyState } from '@/components/primitives'
import { api } from '@/api'
import type { DocumentListItem, Document } from '@/types/document'

type DocumentSelectorProps = {
  selectedIds: string[]
  onSelectionChange: (selectedIds: string[]) => void
  open: boolean
  onClose: () => void
}

function DocumentSelector({ selectedIds, onSelectionChange, open, onClose }: DocumentSelectorProps) {
  const documents = useStore(documentStore.store, documentStore.selectAll)
  const loading = useStore(documentStore.store, documentStore.selectLoading)

  useEffect(() => {
    void documentStore.fetchAll()
  }, [])
  const [localSelectedIds, setLocalSelectedIds] = useState<Set<string>>(() => new Set(selectedIds))
  const [expandedId, setExpandedId] = useState<string | null>(null)
  const [loadedDocs, setLoadedDocs] = useState<Map<string, Document>>(new Map())
  const [loadingDocId, setLoadingDocId] = useState<string | null>(null)

  if (!open) return null

  const handleToggle = (documentId: string) => {
    setLocalSelectedIds((prev) => {
      const next = new Set(prev)
      if (next.has(documentId)) next.delete(documentId)
      else next.add(documentId)
      return next
    })
  }

  const handleSave = () => {
    onSelectionChange([...localSelectedIds])
    onClose()
  }

  const handleCancel = () => {
    setLocalSelectedIds(new Set(selectedIds))
    onClose()
  }

  const toggleExpand = (docId: string, e: React.MouseEvent) => {
    e.stopPropagation()
    const newExpandedId = expandedId === docId ? null : docId
    setExpandedId(newExpandedId)

    // Load document content if expanding and not already loaded
    if (newExpandedId && !loadedDocs.has(newExpandedId)) {
      setLoadingDocId(newExpandedId)
      void api.documents
        .get(newExpandedId)
        .then((doc) => {
          setLoadedDocs((prev) => new Map(prev).set(newExpandedId, doc))
        })
        .catch((err: unknown) => {
          console.error('Failed to load document:', err)
        })
        .finally(() => {
          setLoadingDocId(null)
        })
    }
  }

  const getDocumentContent = (docId: string): string => {
    if (loadingDocId === docId) {
      return 'Loading document content...'
    }
    const fullDoc = loadedDocs.get(docId)
    if (fullDoc?.content) {
      return fullDoc.content
    }
    const listItem = documents.find((d) => d.id === docId)
    return listItem?.summary ?? 'No content available'
  }

  const renderDocumentRow = (doc: DocumentListItem) => {
    const isSelected = localSelectedIds.has(doc.id)
    const isExpanded = expandedId === doc.id

    return (
      <ListItem
        key={doc.id}
        disablePadding
        sx={{
          flexDirection: 'column',
          alignItems: 'stretch',
          borderBottom: 1,
          borderColor: 'divider',
        }}
      >
        <ListItemButton
          selected={isSelected}
          onClick={(e) => {
            toggleExpand(doc.id, e)
          }}
          sx={{
            py: 1.25,
            px: 1.5,
          }}
        >
          <ListItemIcon sx={{ minWidth: 36 }}>
            <Checkbox
              checked={isSelected}
              edge="start"
              tabIndex={-1}
              disableRipple
              onChange={() => handleToggle(doc.id)}
              onClick={(e) => e.stopPropagation()}
            />
          </ListItemIcon>
          <ListItemText
            primary={
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5 }}>
                <Typography variant="body2" sx={{ fontWeight: 500 }}>
                  {doc.title}
                </Typography>
                {doc.doc_type ? <Chip label={doc.doc_type} size="small" variant="outlined" /> : null}
                {doc.ref_tag ? (
                  <Typography variant="caption" color="text.secondary">
                    {doc.ref_tag}
                  </Typography>
                ) : null}
              </Box>
            }
          />
        </ListItemButton>
        <Collapse in={isExpanded} timeout="auto" unmountOnExit>
          <Box
            sx={{
              py: 1.5,
              px: 2,
              pl: 6,
              bgcolor: 'background.default',
              borderLeft: 3,
              borderColor: 'divider',
              ml: 1.5,
              maxHeight: 300,
              overflowY: 'auto',
            }}
          >
            <Typography
              variant="body2"
              component="pre"
              sx={{
                fontFamily: 'monospace',
                fontSize: '0.75rem',
                whiteSpace: 'pre-wrap',
                wordWrap: 'break-word',
                m: 0,
              }}
            >
              {getDocumentContent(doc.id)}
            </Typography>
          </Box>
        </Collapse>
      </ListItem>
    )
  }

  return (
    <Dialog
      open={open}
      onClose={handleCancel}
      maxWidth="md"
      fullWidth
      PaperProps={{
        sx: {
          maxHeight: '85vh',
        },
      }}
    >
      <DialogTitle>
        <Typography variant="h6" component="div" sx={{ fontWeight: 600 }}>
          Select Documents
        </Typography>
        <Typography variant="body2" color="text.secondary">
          Choose documents to attach as agent context ({localSelectedIds.size} selected)
        </Typography>
      </DialogTitle>

      <DialogContent
        dividers
        sx={{
          p: 0,
          bgcolor: 'background.default',
        }}
      >
        {loading ? (
          <Box sx={{ display: 'flex', justifyContent: 'center', py: 7.5 }}>
            <LoadingSpinner size="md" />
          </Box>
        ) : documents.length === 0 ? (
          <Box sx={{ p: 5 }}>
            <EmptyState message="No documents available" />
          </Box>
        ) : (
          <List disablePadding>{documents.map(renderDocumentRow)}</List>
        )}
      </DialogContent>

      <DialogActions sx={{ px: 3, py: 2 }}>
        <Button onClick={handleCancel} variant="outlined" color="inherit">
          Cancel
        </Button>
        <Button onClick={handleSave} variant="contained" color="primary">
          Save Selection
        </Button>
      </DialogActions>
    </Dialog>
  )
}

export { DocumentSelector }
export type { DocumentSelectorProps }
