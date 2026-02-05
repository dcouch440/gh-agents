import { useParams } from 'react-router-dom'
import { useState } from 'react'
import { Box, Typography, Dialog, DialogTitle, DialogContent, DialogActions } from '@mui/material'
import { useAgent } from '@/hooks/useAgents'
import { useAgentDocuments } from '@/hooks/useAgentDocuments'
import { useDocuments } from '@/hooks/useDocuments'
import { useToolRouter } from '@/hooks/useToolRouter'
import { useToolRouterMutations } from '@/hooks/useToolRouterMutations'
import { useRouterModes } from '@/hooks/useRouterModes'
import { useRouterModeMutations } from '@/hooks/useRouterModeMutations'
import { useTools } from '@/hooks/useTools'
import { api } from '@/api'
import {
  PageHeader,
  Card,
  LoadingSpinner,
  ErrorMessage,
  EmptyState,
  DataTable,
  Button,
  type Column,
} from '@/components/primitives'
import {
  NoRouterState,
  RouterInfoCard,
  RouterModesList,
  RouterFormDialog,
  ModeFormDialog,
  ToolAssignmentDialog,
} from '@/components/routers'
import type { DocumentListItem } from '@/types/document'
import type { RouterMode, CreateToolRouterRequest, CreateRouterModeRequest } from '@/types'

function AgentDetailPage() {
  const { id } = useParams()
  const { agent, loading: agentLoading, error: agentError, reload: reloadAgent } = useAgent(id ?? null)
  const {
    documents: agentDocs,
    loading: docsLoading,
    error: docsError,
    saving,
    addDocument,
    removeDocument,
  } = useAgentDocuments(id ?? null)
  const { documents: allDocuments, loading: allDocsLoading } = useDocuments()
  const [showAddDialog, setShowAddDialog] = useState(false)

  // Router & Modes hooks
  const toolRouter = useToolRouter(agent?.router_id ?? null)
  const toolRouterMutations = useToolRouterMutations()
  const modes = useRouterModes(agent?.router_id ?? null)
  const modeMutations = useRouterModeMutations()
  const { tools: allTools } = useTools()

  // Router dialog state
  const [showRouterForm, setShowRouterForm] = useState(false)
  const [editingRouter, setEditingRouter] = useState(false)

  // Mode dialog state
  const [showModeForm, setShowModeForm] = useState(false)
  const [editingMode, setEditingMode] = useState<RouterMode | null>(null)

  // Tool assignment dialog state
  const [showToolAssignment, setShowToolAssignment] = useState(false)
  const [toolAssignmentTarget, setToolAssignmentTarget] = useState<{ type: 'router' | 'mode'; id: string } | null>(null)
  const [assignedToolIds, setAssignedToolIds] = useState<string[]>([])

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

  const availableDocuments = allDocuments.filter(
    (doc) => !agentDocs.some((ad) => ad.id === doc.id),
  )

  const docColumns: Column<DocumentListItem>[] = [
    { key: 'title', header: 'Title', render: (doc) => doc.title },
    { key: 'doc_type', header: 'Type', render: (doc) => doc.doc_type ?? 'N/A' },
    { key: 'ref_tag', header: 'Ref Tag', render: (doc) => doc.ref_tag ?? 'N/A' },
    {
      key: 'actions',
      header: 'Actions',
      render: (doc) => (
        <Button variant="danger" size="small" onClick={() => { void handleRemoveDocument(doc.id) }} disabled={saving}>
          Remove
        </Button>
      ),
    },
  ]

  // Router handlers
  const handleCreateRouter = async (data: CreateToolRouterRequest) => {
    const newRouter = await toolRouterMutations.createRouter(data)
    await api.agents.update(id, { router_id: newRouter.id })
    setShowRouterForm(false)
    await reloadAgent()
  }

  const handleUpdateRouter = async (data: CreateToolRouterRequest) => {
    if (!agent.router_id) return
    await toolRouterMutations.updateRouter(agent.router_id, data)
    setShowRouterForm(false)
    setEditingRouter(false)
    await toolRouter.reload()
  }

  const handleDeleteRouter = async () => {
    if (!agent.router_id) return
    if (!confirm('Delete this router and all its modes?')) return
    await toolRouterMutations.deleteRouter(agent.router_id)
    await api.agents.update(id, {})
    await reloadAgent()
  }

  // Mode handlers
  const handleCreateMode = async (data: CreateRouterModeRequest) => {
    if (!agent.router_id) return
    await modeMutations.createMode(agent.router_id, data)
    setShowModeForm(false)
    await modes.reload()
  }

  const handleUpdateMode = async (data: CreateRouterModeRequest) => {
    if (!editingMode) return
    await modeMutations.updateMode(editingMode.id, data)
    setEditingMode(null)
    setShowModeForm(false)
    await modes.reload()
  }

  const handleDeleteMode = async (mode: RouterMode) => {
    if (!confirm(`Delete mode "${mode.display_name}"?`)) return
    await modeMutations.deleteMode(mode.id)
    await modes.reload()
  }

  // Tool assignment handlers
  const handleOpenRouterTools = async () => {
    if (!agent.router_id) return
    setToolAssignmentTarget({ type: 'router', id: agent.router_id })
    try {
      const tools = await toolRouterMutations.loadRouterTools(agent.router_id)
      setAssignedToolIds(tools.map((t) => t.id))
    } catch {
      setAssignedToolIds([])
    }
    setShowToolAssignment(true)
  }

  const handleOpenModeTools = async (mode: RouterMode) => {
    setToolAssignmentTarget({ type: 'mode', id: mode.id })
    try {
      const tools = await modeMutations.loadModeTools(mode.id)
      setAssignedToolIds(tools.map((t) => t.id))
    } catch {
      setAssignedToolIds([])
    }
    setShowToolAssignment(true)
  }

  const handleSaveTools = async (toolIds: string[]) => {
    if (!toolAssignmentTarget) return
    if (toolAssignmentTarget.type === 'router') {
      await toolRouterMutations.saveRouterTools(toolAssignmentTarget.id, { tool_ids: toolIds })
    } else {
      await modeMutations.saveModeTools(toolAssignmentTarget.id, { tool_ids: toolIds })
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
    setShowModeForm(true)
  }

  const handleOpenEditMode = (mode: RouterMode) => {
    setEditingMode(mode)
    setShowModeForm(true)
  }

  const routerFormInitialValues = editingRouter && toolRouter.router
    ? {
        name: toolRouter.router.name,
        description: toolRouter.router.description ?? undefined,
        system_prompt: toolRouter.router.system_prompt,
        model_id: toolRouter.router.model_id,
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
        <Typography variant="body2" color="text.secondary">{agent.model_id}</Typography>
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
            <Button
              variant="primary"
              size="small"
              onClick={() => setShowAddDialog(true)}
              disabled={saving || allDocsLoading}
            >
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
            <NoRouterState
              onCreateRouter={() => setShowRouterForm(true)}
              creating={toolRouterMutations.creating}
            />
          ) : toolRouter.loading ? (
            <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
              <LoadingSpinner size="md" />
            </Box>
          ) : toolRouter.error ? (
            <ErrorMessage message={toolRouter.error} />
          ) : toolRouter.router ? (
            <>
              <RouterInfoCard
                router={toolRouter.router}
                onEdit={handleOpenEditRouter}
                onDelete={() => { void handleDeleteRouter() }}
                onManageTools={() => { void handleOpenRouterTools() }}
              />
              {modes.loading ? (
                <Box sx={{ display: 'flex', justifyContent: 'center', py: 2 }}>
                  <LoadingSpinner size="sm" />
                </Box>
              ) : modes.error ? (
                <ErrorMessage message={modes.error} />
              ) : (
                <RouterModesList
                  modes={modes.modes}
                  onEditMode={handleOpenEditMode}
                  onDeleteMode={(m) => { void handleDeleteMode(m) }}
                  onManageTools={(m) => { void handleOpenModeTools(m) }}
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
                  onClick={() => { void handleAddDocument(doc.id) }}
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
        onClose={() => { setShowRouterForm(false); setEditingRouter(false) }}
        onSubmit={(data) => { void (editingRouter ? handleUpdateRouter(data) : handleCreateRouter(data)) }}
        initialValues={routerFormInitialValues}
        saving={toolRouterMutations.creating || toolRouterMutations.updating}
        title={editingRouter ? 'Edit Router' : 'Create Router'}
      />

      {/* Mode Form Dialog */}
      <ModeFormDialog
        open={showModeForm}
        onClose={() => { setShowModeForm(false); setEditingMode(null) }}
        onSubmit={(data) => { void (editingMode ? handleUpdateMode(data) : handleCreateMode(data)) }}
        initialValues={modeFormInitialValues}
        saving={modeMutations.creating || modeMutations.updating}
        title={editingMode ? 'Edit Mode' : 'Create Mode'}
      />

      {/* Tool Assignment Dialog */}
      <ToolAssignmentDialog
        open={showToolAssignment}
        onClose={() => { setShowToolAssignment(false); setToolAssignmentTarget(null) }}
        onSave={(toolIds) => { void handleSaveTools(toolIds) }}
        allTools={allTools}
        assignedToolIds={assignedToolIds}
        saving={toolRouterMutations.savingTools || modeMutations.savingTools}
        loading={toolRouterMutations.loadingTools || modeMutations.loadingTools}
        title={toolAssignmentTarget?.type === 'router' ? 'Router Tools' : 'Mode Tools'}
      />
    </Box>
  )
}

export { AgentDetailPage }
