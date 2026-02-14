import { useParams } from 'react-router-dom'
import { useState, useEffect, useCallback, useMemo } from 'react'
import { Box, Typography, Dialog, DialogTitle, DialogContent, DialogActions } from '@mui/material'
import { useStore, toolStore, documentStore, agentStore, toolRouterStore } from '@/stores'
import { api } from '@/api'
import { PageHeader, Card, LoadingSpinner, ErrorMessage, EmptyState, DataTable, Button, type Column } from '@/components/primitives'
import {
  NoRouterState,
  RouterInfoCard,
  RouterModesList,
  RouterFormDialog,
  ModeFormDialog,
  ToolAssignmentDialog,
} from '@/components/routers'
import { Collections } from '@/utils/collections'
import type { DocumentListItem } from '@/types/document'
import type { RouterMode, CreateToolRouterRequest, CreateRouterModeRequest } from '@/types'

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

  // Router data from store
  const routerId = agent?.router_id ?? null
  const router = useStore(toolRouterStore.store, toolRouterStore.selectById(routerId ?? ''))
  const routerLoading = useStore(toolRouterStore.store, toolRouterStore.selectLoading)
  const routerError = useStore(toolRouterStore.store, toolRouterStore.selectError)
  const routerModes = useStore(toolRouterStore.store, toolRouterStore.selectModes(routerId ?? ''))
  const [modesLoading, setModesLoading] = useState(false)
  const [modesError, setModesError] = useState<string | null>(null)
  const [creating, setCreating] = useState(false)
  const [savingTools, setSavingTools] = useState(false)
  const [loadingToolsFlag, setLoadingToolsFlag] = useState(false)

  const allTools = useStore(toolStore.store, toolStore.selectAll)

  useEffect(() => {
    void toolStore.fetchAll()
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

  // Fetch router + modes when agent has a router
  useEffect(() => {
    if (!routerId) return
    void toolRouterStore.fetchOne(routerId)
    setModesLoading(true)
    setModesError(null)
    toolRouterStore
      .fetchModes(routerId)
      .catch((e: unknown) => setModesError(e instanceof Error ? e.message : 'Failed to load modes'))
      .finally(() => setModesLoading(false))
  }, [routerId])

  const reloadAgent = useCallback(async () => {
    if (!id) return
    await agentStore.fetchOne(id)
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

  const reloadModes = useCallback(async () => {
    if (!routerId) return
    setModesLoading(true)
    setModesError(null)
    try {
      await toolRouterStore.fetchModes(routerId)
    } catch (e) {
      setModesError(e instanceof Error ? e.message : 'Failed to load modes')
    } finally {
      setModesLoading(false)
    }
  }, [routerId])

  // Router dialog state
  const [showRouterForm, setShowRouterForm] = useState(false)
  const [editingRouter, setEditingRouter] = useState(false)

  // Mode dialog state
  const [showModeForm, setShowModeForm] = useState(false)
  const [editingMode, setEditingMode] = useState<RouterMode | null>(null)
  const [editingModeToolIds, setEditingModeToolIds] = useState<string[]>([])

  // Tool assignment dialog state
  const [showToolAssignment, setShowToolAssignment] = useState(false)
  const [toolAssignmentTarget, setToolAssignmentTarget] = useState<{ type: 'router' | 'mode'; id: string } | null>(null)
  const [assignedToolIds, setAssignedToolIds] = useState<string[]>([])

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

  // Router handlers
  const handleCreateRouter = async (data: CreateToolRouterRequest) => {
    setCreating(true)
    try {
      const newRouter = await toolRouterStore.create(data)
      await api.agents.update(id, { router_id: newRouter.id })
      setShowRouterForm(false)
      await reloadAgent()
    } finally {
      setCreating(false)
    }
  }

  const handleUpdateRouter = async (data: CreateToolRouterRequest) => {
    if (!agent.router_id) return
    await toolRouterStore.update(agent.router_id, data)
    setShowRouterForm(false)
    setEditingRouter(false)
  }

  const handleDeleteRouter = async () => {
    if (!agent.router_id) return
    if (!confirm('Delete this router and all its modes?')) return
    await toolRouterStore.remove(agent.router_id)
    await api.agents.update(id, {})
    await reloadAgent()
  }

  // Mode handlers
  const handleCreateMode = async (data: CreateRouterModeRequest, toolIds: string[]) => {
    if (!agent.router_id) return
    const newMode = await toolRouterStore.createMode(agent.router_id, data)
    if (toolIds.length > 0) {
      await toolRouterStore.setModeTools(newMode.id, { tool_ids: toolIds })
    }
    setShowModeForm(false)
    await reloadModes()
  }

  const handleUpdateMode = async (data: CreateRouterModeRequest, toolIds: string[]) => {
    if (!editingMode) return
    await toolRouterStore.updateMode(editingMode.id, data)
    await toolRouterStore.setModeTools(editingMode.id, { tool_ids: toolIds })
    setEditingMode(null)
    setShowModeForm(false)
    await reloadModes()
  }

  const handleDeleteMode = async (mode: RouterMode) => {
    if (!confirm(`Delete mode "${mode.display_name}"?`)) return
    await toolRouterStore.deleteMode(mode.id)
    await reloadModes()
  }

  // Tool assignment handlers
  const handleOpenRouterTools = async () => {
    if (!agent.router_id) return
    setToolAssignmentTarget({ type: 'router', id: agent.router_id })
    setLoadingToolsFlag(true)
    try {
      const tools = await toolRouterStore.fetchRouterTools(agent.router_id)
      setAssignedToolIds(tools.map((t) => t.id))
    } catch {
      setAssignedToolIds([])
    } finally {
      setLoadingToolsFlag(false)
    }
    setShowToolAssignment(true)
  }

  const handleOpenModeTools = async (mode: RouterMode) => {
    setToolAssignmentTarget({ type: 'mode', id: mode.id })
    setLoadingToolsFlag(true)
    try {
      const tools = await toolRouterStore.fetchModeTools(mode.id)
      setAssignedToolIds(tools.map((t) => t.id))
    } catch {
      setAssignedToolIds([])
    } finally {
      setLoadingToolsFlag(false)
    }
    setShowToolAssignment(true)
  }

  const handleSaveTools = async (toolIds: string[]) => {
    if (!toolAssignmentTarget) return
    setSavingTools(true)
    try {
      if (toolAssignmentTarget.type === 'router') {
        await toolRouterStore.setRouterTools(toolAssignmentTarget.id, { tool_ids: toolIds })
      } else {
        await toolRouterStore.setModeTools(toolAssignmentTarget.id, { tool_ids: toolIds })
      }
    } finally {
      setSavingTools(false)
    }
    setShowToolAssignment(false)
    setToolAssignmentTarget(null)
  }

  const handleOpenEditRouter = () => {
    setEditingRouter(true)
    setShowRouterForm(true)
  }

  const handleOpenCreateMode = () => {
    setEditingMode(null)
    setEditingModeToolIds([])
    setShowModeForm(true)
  }

  const handleOpenEditMode = async (mode: RouterMode) => {
    setEditingMode(mode)
    try {
      const tools = await toolRouterStore.fetchModeTools(mode.id)
      setEditingModeToolIds(tools.map((t) => t.id))
    } catch {
      setEditingModeToolIds([])
    }
    setShowModeForm(true)
  }

  const routerFormInitialValues =
    editingRouter && router
      ? {
          name: router.name,
          description: router.description ?? undefined,
          system_prompt: router.system_prompt,
          model_id: router.model_id,
        }
      : null

  const modeFormInitialValues = editingMode
    ? {
        mode_key: editingMode.mode_key,
        display_name: editingMode.display_name,
        description: editingMode.description,
        system_prompt: editingMode.system_prompt,
        temperature: editingMode.temperature,
        max_tokens: editingMode.max_tokens,
        append_to_agent_system_prompt: editingMode.append_to_agent_system_prompt,
        append_to_agent_tools: editingMode.append_to_agent_tools,
        display_order: editingMode.display_order,
      }
    : null

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

        <Card
          title="Tool Router & Modes"
          actions={
            agent.router_id ? (
              <Button variant="primary" size="small" onClick={handleOpenCreateMode}>
                Add Mode
              </Button>
            ) : undefined
          }
        >
          {!agent.router_id ? (
            <NoRouterState onCreateRouter={() => setShowRouterForm(true)} creating={creating} />
          ) : routerLoading ? (
            <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
              <LoadingSpinner size="md" />
            </Box>
          ) : routerError ? (
            <ErrorMessage message={routerError} />
          ) : router ? (
            <>
              <RouterInfoCard
                router={router}
                onEdit={handleOpenEditRouter}
                onDelete={() => {
                  void handleDeleteRouter()
                }}
                onManageTools={() => {
                  void handleOpenRouterTools()
                }}
              />
              {modesLoading ? (
                <Box sx={{ display: 'flex', justifyContent: 'center', py: 2 }}>
                  <LoadingSpinner size="sm" />
                </Box>
              ) : modesError ? (
                <ErrorMessage message={modesError} />
              ) : (
                <RouterModesList
                  modes={routerModes}
                  onEditMode={(m) => {
                    void handleOpenEditMode(m)
                  }}
                  onDeleteMode={(m) => {
                    void handleDeleteMode(m)
                  }}
                  onManageTools={(m) => {
                    void handleOpenModeTools(m)
                  }}
                />
              )}
            </>
          ) : null}
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

      {/* Router Form Dialog */}
      <RouterFormDialog
        open={showRouterForm}
        onClose={() => {
          setShowRouterForm(false)
          setEditingRouter(false)
        }}
        onSubmit={(data) => {
          void (editingRouter ? handleUpdateRouter(data) : handleCreateRouter(data))
        }}
        initialValues={routerFormInitialValues}
        saving={creating}
        title={editingRouter ? 'Edit Router' : 'Create Router'}
      />

      {/* Mode Form Dialog */}
      <ModeFormDialog
        open={showModeForm}
        onClose={() => {
          setShowModeForm(false)
          setEditingMode(null)
        }}
        onSubmit={(data, toolIds) => {
          void (editingMode ? handleUpdateMode(data, toolIds) : handleCreateMode(data, toolIds))
        }}
        initialValues={modeFormInitialValues}
        allTools={allTools}
        initialToolIds={editingModeToolIds}
        saving={false}
        title={editingMode ? 'Edit Mode' : 'Create Mode'}
      />

      {/* Tool Assignment Dialog */}
      <ToolAssignmentDialog
        open={showToolAssignment}
        onClose={() => {
          setShowToolAssignment(false)
          setToolAssignmentTarget(null)
        }}
        onSave={(toolIds) => {
          void handleSaveTools(toolIds)
        }}
        allTools={allTools}
        assignedToolIds={assignedToolIds}
        saving={savingTools}
        loading={loadingToolsFlag}
        title={toolAssignmentTarget?.type === 'router' ? 'Router Tools' : 'Mode Tools'}
      />
    </Box>
  )
}

export { AgentDetailPage }
