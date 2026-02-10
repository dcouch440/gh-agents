import { useEffect, useMemo, useState, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import { Box, Button, Typography } from '@mui/material'
import DeleteIcon from '@mui/icons-material/Delete'
import WorkshopIcon from '@mui/icons-material/Science'
import VisibilityIcon from '@mui/icons-material/Visibility'
import { FadeIn } from '@/components/animation'
import { PageHeader, Table, StatusBadge, ActionMenu, ConfirmModal, type TableColumn, type MenuAction } from '@/components/primitives'
import { useConfirmModal } from '@/hooks/useConfirmModal'
import { useStore, agentStore, sessionStore } from '@/stores'
import { api } from '@/api'
import type { Agent } from '@/types/agent'

function AgentsPage() {
  const navigate = useNavigate()
  const agents = useStore(agentStore.store, agentStore.selectAll)
  const agentsLoading = useStore(agentStore.store, agentStore.selectLoading)
  const agentsError = useStore(agentStore.store, agentStore.selectError)
  const sessions = useStore(sessionStore.store, sessionStore.selectAll)
  const sessionsLoading = useStore(sessionStore.store, sessionStore.selectLoading)
  const confirmModal = useConfirmModal()
  const { openConfirm } = confirmModal

  useEffect(() => {
    void agentStore.fetchAll()
    void sessionStore.fetchAll()
  }, [])
  const [creatingSession, setCreatingSession] = useState<string | null>(null)

  // Build a Map for O(1) agent→session lookups
  const sessionsByAgentId = useMemo(() => {
    const map = new Map<string, (typeof sessions)[number]>()
    for (const s of sessions) {
      if (s.mode_id === 'workshop') {
        map.set(s.agent_id, s)
      }
    }
    return map
  }, [sessions])

  const handleAgentClick = useCallback(
    async (agentId: string, sessionId?: string) => {
      if (sessionId) {
        // Open existing workshop session
        void navigate(`/agents/workshop/${sessionId}`)
      } else {
        // Create a new session for this existing agent
        setCreatingSession(agentId)
        try {
          const session = await api.sessions.create({
            mode_id: 'workshop',
            agent_id: agentId,
            title: 'Agent Workshop',
          })
          void navigate(`/agents/workshop/${session.id}`)
        } catch (err) {
          console.error('Failed to create session:', err)
        } finally {
          setCreatingSession(null)
        }
      }
    },
    [navigate],
  )

  const handleNewWorkshop = useCallback(() => {
    void navigate('/agents/workshop')
  }, [navigate])

  const handleDeleteAgent = useCallback(
    (agent: Agent) => {
      openConfirm({
        title: 'Delete Agent',
        message: `Are you sure you want to delete "${agent.name}"? This action cannot be undone.`,
        confirmText: 'Delete',
        confirmColor: 'error',
        onConfirm: async () => {
          await agentStore.remove(agent.id)
        },
      })
    },
    [openConfirm],
  )

  const loading = agentsLoading || sessionsLoading

  const tableAgents = agents

  // Define table columns
  const columns: TableColumn<Agent>[] = useMemo(
    () => [
      {
        key: 'name',
        header: 'Name',
        sortable: true,
        width: 200,
        render: (agent) => (
          <Typography variant="body2" fontWeight={500}>
            {agent.name}
          </Typography>
        ),
      },
      {
        key: 'system_prompt',
        header: 'System Prompt',
        truncate: true,
        width: 300,
        render: (agent) => (
          <Typography variant="body2" color="text.secondary">
            {agent.system_prompt || 'No system prompt'}
          </Typography>
        ),
      },
      {
        key: 'model',
        header: 'Model',
        sortable: true,
        width: 220,
        render: (agent) => (
          <Typography variant="body2">
            {agent.model_provider}/{agent.model_id}
          </Typography>
        ),
      },
      {
        key: 'temperature',
        header: 'Temperature',
        sortable: true,
        align: 'right' as const,
        width: 120,
        render: (agent) => <Typography variant="body2">{agent.model_temperature}</Typography>,
      },
      {
        key: 'max_tokens',
        header: 'Max Tokens',
        sortable: true,
        align: 'right' as const,
        width: 120,
        render: (agent) => <Typography variant="body2">{agent.model_max_tokens.toLocaleString()}</Typography>,
      },
      {
        key: 'status',
        header: 'Status',
        sortable: true,
        width: 120,
        render: (agent) => <StatusBadge status={agent.status} />,
      },
      {
        key: 'actions',
        header: 'Actions',
        width: 80,
        align: 'center' as const,
        render: (agent) => {
          const session = sessionsByAgentId.get(agent.id)
          const isCreating = creatingSession === agent.id

          const actions: MenuAction[] = [
            {
              key: 'workshop',
              label: session ? 'Open Workshop' : 'Start Workshop',
              icon: <WorkshopIcon fontSize="small" />,
              onClick: () => {
                void handleAgentClick(agent.id, session?.id)
              },
              disabled: isCreating,
              dividerAfter: true,
            },
            {
              key: 'view',
              label: 'View Details',
              icon: <VisibilityIcon fontSize="small" />,
              onClick: () => {
                void navigate(`/agents/${agent.id}`)
              },
            },
            {
              key: 'delete',
              label: 'Delete',
              icon: <DeleteIcon fontSize="small" />,
              onClick: () => {
                void handleDeleteAgent(agent)
              },
              color: 'error' as const,
            },
          ]

          return <ActionMenu actions={actions} ariaLabel={`Actions for ${agent.name}`} />
        },
      },
    ],
    [sessionsByAgentId, creatingSession, handleAgentClick, handleDeleteAgent, navigate],
  )

  return (
    <FadeIn>
      <Box>
        <PageHeader title="Agents">
          <Button variant="contained" onClick={handleNewWorkshop}>
            New Workshop
          </Button>
        </PageHeader>

        <Table
          data={tableAgents}
          keyExtractor={(agent) => agent.id}
          columns={columns}
          loading={loading}
          error={agentsError}
          emptyMessage="No agents yet. Create your first one in the workshop!"
          enableSorting
          enableSearch
          enablePagination
          searchPlaceholder="Search agents by name, model, or prompt..."
          searchFields={['name', 'model_id', 'model_provider', 'system_prompt']}
          defaultSortColumn="name"
          defaultSortDirection="asc"
          defaultPageSize={25}
          pageSizeOptions={[10, 25, 50, 100]}
          onRowClick={(agent) => {
            const session = sessionsByAgentId.get(agent.id)
            void handleAgentClick(agent.id, session?.id)
          }}
          stickyHeader
          density="normal"
        />

        <ConfirmModal
          open={confirmModal.open}
          onClose={confirmModal.closeConfirm}
          onConfirm={confirmModal.handleConfirm}
          title={confirmModal.title}
          message={confirmModal.message}
          confirmText={confirmModal.confirmText}
          cancelText={confirmModal.cancelText}
          confirmColor={confirmModal.confirmColor}
          loading={confirmModal.loading}
          error={confirmModal.error}
        />
      </Box>
    </FadeIn>
  )
}

export { AgentsPage }
