import {useEffect, useMemo, useState, useCallback} from 'react'
import {useNavigate} from 'react-router-dom'
import {Box, Button, Typography} from '@mui/material'
import DeleteIcon from '@mui/icons-material/Delete'
import WorkshopIcon from '@mui/icons-material/Science'
import VisibilityIcon from '@mui/icons-material/Visibility'
import {FadeIn} from '@/components/animation'
import {
  PageHeader,
  Table,
  StatusBadge,
  ActionMenu,
  ConfirmModal,
  type TableColumn,
  type MenuAction,
} from '@/components/primitives'
import {useSessions} from '@/hooks/useSessions'
import {useConfirmModal} from '@/hooks/useConfirmModal'
import {useStore, agentStore} from '@/stores'
import {api} from '@/api'
import type {Agent} from '@/types/agent'

function AgentsPage() {
  const navigate = useNavigate()
  const agents = useStore(agentStore.store, agentStore.selectAll)
  const agentsLoading = useStore(agentStore.store, agentStore.selectLoading)
  const agentsError = useStore(agentStore.store, agentStore.selectError)
  const {sessions, loading: sessionsLoading} = useSessions()
  const confirm = useConfirmModal()

  useEffect(() => { void agentStore.fetchAll() }, [])
  const [creatingSession, setCreatingSession] = useState<string | null>(null)

  // Match agents with their workshop sessions
  const agentsWithSessions = useMemo(() => {
    return agents.map((agent) => {
      // Find workshop session for this agent
      // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
      const session = sessions?.find(
        (s) => s.mode_id === "workshop" && s.agent_id === agent.id,
      );
      return {agent, session};
    });
  }, [agents, sessions]);

  const handleAgentClick = useCallback(
    async (agentId: string, sessionId?: string) => {
      if (sessionId) {
        // Open existing workshop session
        void navigate(`/agents/workshop/${sessionId}`);
      } else {
        // Create a new session for this existing agent
        setCreatingSession(agentId);
        try {
          const session = await api.sessions.create({
            mode_id: "workshop",
            agent_id: agentId,
            title: "Agent Workshop",
          });
          void navigate(`/agents/workshop/${session.id}`);
        } catch (err) {
          console.error("Failed to create session:", err);
        } finally {
          setCreatingSession(null);
        }
      }
    },
    [navigate],
  );

  const handleNewWorkshop = useCallback(() => {
    void navigate('/agents/workshop')
  }, [navigate])

  const handleDeleteAgent = useCallback(
    async (agent: Agent) => {
      confirm.openConfirm({
        title: 'Delete Agent',
        message: `Are you sure you want to delete "${agent.name}"? This action cannot be undone.`,
        confirmText: 'Delete',
        confirmColor: 'error',
      })

      const confirmed = await confirm.confirmAsync(async () => {
        await agentStore.remove(agent.id)
      })

      if (confirmed) {
        // Agent deleted successfully
      }
    },
    [confirm],
  )

  const loading = agentsLoading || sessionsLoading

  // Extract just the agents from agentsWithSessions for the table
  const tableAgents = useMemo(
    () => agentsWithSessions.map(({agent}) => agent),
    [agentsWithSessions],
  );

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
        render: (agent) => (
          <Typography variant="body2">{agent.model_temperature}</Typography>
        ),
      },
      {
        key: 'max_tokens',
        header: 'Max Tokens',
        sortable: true,
        align: 'right' as const,
        width: 120,
        render: (agent) => (
          <Typography variant="body2">
            {agent.model_max_tokens.toLocaleString()}
          </Typography>
        ),
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
          const {session} =
            agentsWithSessions.find((a) => a.agent.id === agent.id) ?? {}
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

          return (
            <ActionMenu
              actions={actions}
              ariaLabel={`Actions for ${agent.name}`}
            />
          )
        },
      },
    ],
    [agentsWithSessions, creatingSession, handleAgentClick, handleDeleteAgent, navigate],
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
            const {session} =
              agentsWithSessions.find((a) => a.agent.id === agent.id) ?? {}
            void handleAgentClick(agent.id, session?.id)
          }}
          stickyHeader
          density="normal"
        />

        <ConfirmModal
          open={confirm.open}
          onClose={confirm.closeConfirm}
          onConfirm={confirm.onConfirm}
          title={confirm.title}
          message={confirm.message}
          confirmText={confirm.confirmText}
          cancelText={confirm.cancelText}
          confirmColor={confirm.confirmColor}
          loading={confirm.loading}
          error={confirm.error}
        />
      </Box>
    </FadeIn>
  )
}

export {AgentsPage};
