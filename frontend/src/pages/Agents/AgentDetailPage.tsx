import { useParams } from 'react-router-dom'
import { useState, useEffect, useCallback, useMemo } from 'react'
import { Box, Typography, Dialog, DialogTitle, DialogContent, DialogActions } from '@mui/material'
import { useStore, documentStore, agentStore } from '@/stores'
import { PageHeader, Card, LoadingSpinner, ErrorMessage, EmptyState, DataTable, Button, type Column } from '@/components/primitives'
import { Collections } from '@/utils/collections'
import type { DocumentListItem } from '@/types/document'

function AgentDetailPage() {
  const { id } = useParams()

  // Agent data from store
  const agent = useStore(agentStore.store, agentStore.selectById(id ?? ''))
  const agentLoading = useStore(agentStore.store, agentStore.selectLoading)
  const agentError = useStore(agentStore.store, agentStore.selectError)

  // Agent context documents from store
  const agentDocs = useStore(agentStore.store, agentStore.selectContext(id ?? ''))
  const [docsLoading, setDocsLoading] = useState(true)
  const [docsError, setDocsError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)

  const allDocuments = useStore(documentStore.store, documentStore.selectAll)
  const allDocsLoading = useStore(documentStore.store, documentStore.selectLoading)
  const [showAddDialog, setShowAddDialog] = useState(false)

  useEffect(() => {
    void documentStore.fetchAll()
  }, [])

  // Fetch agent
  useEffect(() => {
    if (!id) return
    void agentStore.fetchOne(id)
  }, [id])

  // Fetch agent context documents
  useEffect(() => {
    if (!id) return
    setDocsLoading(true)
    setDocsError(null)
    agentStore
      .fetchContext(id)
      .catch((e: unknown) => setDocsError(e instanceof Error ? e.message : 'Failed to load context'))
      .finally(() => setDocsLoading(false))
  }, [id])

  const addDocument = useCallback(
    async (documentId: string) => {
      if (!id) return
      const currentIdSet = Collections.toSetBy(agentDocs, (d) => d.id)
      if (currentIdSet.has(documentId)) return
      const currentIds = Collections.mapBy(agentDocs, (d) => d.id)
      setSaving(true)
      try {
        await agentStore.setContext(id, [...currentIds, documentId])
      } finally {
        setSaving(false)
      }
    },
    [id, agentDocs],
  )

  const removeDocument = useCallback(
    async (documentId: string) => {
      if (!id) return
      const currentIds = Collections.filterMap(agentDocs, (d) => (d.id !== documentId ? d.id : null))
      setSaving(true)
      try {
        await agentStore.setContext(id, currentIds)
      } finally {
        setSaving(false)
      }
    },
    [id, agentDocs],
  )

  const availableDocuments = useMemo(() => {
    const assignedIds = Collections.toSetBy(agentDocs, (ad) => ad.id)
    return Collections.filterMap(allDocuments, (doc) => (!assignedIds.has(doc.id) ? doc : null))
  }, [allDocuments, agentDocs])

  if (!id) {
    return <ErrorMessage message="No agent ID provided" />
  }

  if (agentLoading || docsLoading) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '256px' }}>
        <LoadingSpinner size="lg" />
      </Box>
    )
  }

  if (agentError) {
    return <ErrorMessage message={agentError} />
  }

  if (!agent) {
    return <ErrorMessage message="Agent not found" />
  }

  // Document handlers
  const handleAddDocument = async (documentId: string) => {
    await addDocument(documentId)
    setShowAddDialog(false)
  }

  const handleRemoveDocument = async (documentId: string) => {
    if (confirm('Remove this document from agent context?')) {
      await removeDocument(documentId)
    }
  }

  const docColumns: Column<DocumentListItem>[] = [
    { key: 'title', header: 'Title', render: (doc) => doc.title },
    { key: 'doc_type', header: 'Type', render: (doc) => doc.doc_type ?? 'N/A' },
    { key: 'ref_tag', header: 'Ref Tag', render: (doc) => doc.ref_tag ?? 'N/A' },
    {
      key: 'actions',
      header: 'Actions',
      render: (doc) => (
        <Button
          variant="danger"
          size="small"
          onClick={() => {
            void handleRemoveDocument(doc.id)
          }}
          disabled={saving}
        >
          Remove
        </Button>
      ),
    },
  ]

  return (
    <Box>
      <PageHeader title={agent.name}>
        <Typography variant="body2" color="text.secondary">
          {agent.model_id}
        </Typography>
      </PageHeader>

      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
        <Card title="Agent Details">
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
            <Box>
              <Typography variant="caption" color="text.secondary" component="div" sx={{ mb: 0.5 }}>
                System Prompt
              </Typography>
              <Box sx={{ p: 1.5, bgcolor: 'background.default', borderRadius: 1, border: 1, borderColor: 'divider' }}>
                <Typography variant="body2" component="pre" sx={{ whiteSpace: 'pre-wrap', fontFamily: 'monospace', m: 0 }}>
                  {agent.system_prompt}
                </Typography>
              </Box>
            </Box>
            <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 2 }}>
              <Box>
                <Typography variant="caption" color="text.secondary" component="div">
                  Model Provider
                </Typography>
                <Typography variant="body2">{agent.model_provider}</Typography>
              </Box>
              <Box>
                <Typography variant="caption" color="text.secondary" component="div">
                  Status
                </Typography>
                <Typography variant="body2">{agent.status}</Typography>
              </Box>
              <Box>
                <Typography variant="caption" color="text.secondary" component="div">
                  Max Tokens
                </Typography>
                <Typography variant="body2">{agent.model_max_tokens}</Typography>
              </Box>
              <Box>
                <Typography variant="caption" color="text.secondary" component="div">
                  Temperature
                </Typography>
                <Typography variant="body2">{agent.model_temperature}</Typography>
              </Box>
            </Box>
          </Box>
        </Card>

        <Card
          title="Agent Context Documents"
          actions={
            <Button variant="primary" size="small" onClick={() => setShowAddDialog(true)} disabled={saving || allDocsLoading}>
              Add Document
            </Button>
          }
        >
          {docsError ? <ErrorMessage message={docsError} /> : null}
          {agentDocs.length === 0 ? (
            <EmptyState message="No context documents attached to this agent" />
          ) : (
            <DataTable rows={agentDocs} columns={docColumns} rowKey={(d) => d.id} />
          )}
        </Card>

      </Box>

      {/* Add Document Dialog */}
      <Dialog open={showAddDialog} onClose={() => setShowAddDialog(false)} maxWidth="sm" fullWidth>
        <DialogTitle>Add Document to Agent Context</DialogTitle>
        <DialogContent dividers sx={{ p: 2 }}>
          {availableDocuments.length === 0 ? (
            <EmptyState message="All documents are already attached to this agent" />
          ) : (
            <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
              {availableDocuments.map((doc) => (
                <Box
                  key={doc.id}
                  onClick={() => {
                    void handleAddDocument(doc.id)
                  }}
                  sx={{
                    p: 1.5,
                    border: 1,
                    borderColor: 'divider',
                    borderRadius: 1,
                    cursor: 'pointer',
                    '&:hover': { bgcolor: 'action.hover' },
                  }}
                >
                  <Typography variant="body2" sx={{ fontWeight: 500 }}>
                    {doc.title}
                  </Typography>
                  <Typography variant="caption" color="text.secondary">
                    {doc.doc_type ?? 'N/A'} {doc.ref_tag ? `• ${doc.ref_tag}` : ''}
                  </Typography>
                </Box>
              ))}
            </Box>
          )}
        </DialogContent>
        <DialogActions sx={{ px: 3, py: 2 }}>
          <Button variant="secondary" onClick={() => setShowAddDialog(false)}>
            Cancel
          </Button>
        </DialogActions>
      </Dialog>

    </Box>
  )
}

export { AgentDetailPage }
