import {useMemo, useState, useCallback} from "react";
import {useNavigate} from "react-router-dom";
import {Box, Button, Typography} from "@mui/material";
import {FadeIn} from "@/components/animation";
import {
  PageHeader,
  Table,
  StatusBadge,
  type TableColumn,
} from "@/components/primitives";
import {useAgents} from "@/hooks/useAgents";
import {useSessions} from "@/hooks/useSessions";
import {api} from "@/api";
import type {Agent} from "@/types/agent";

function AgentsPage() {
  const navigate = useNavigate();
  const {agents, loading: agentsLoading, error: agentsError} = useAgents();
  const {sessions, loading: sessionsLoading} = useSessions();
  const [creatingSession, setCreatingSession] = useState<string | null>(null);

  // Match agents with their workshop sessions
  const agentsWithSessions = useMemo(() => {
    // Filter out draft agents (temporary unsaved workshops)
    const finalizedAgents = agents.filter(
      (agent) => !agent.name.startsWith("[Workshop Draft]"),
    );

    return finalizedAgents.map((agent) => {
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
    void navigate("/agents/workshop");
  }, [navigate]);

  const loading = agentsLoading || sessionsLoading;

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
        key: 'workshop',
        header: 'Workshop',
        width: 140,
        render: (agent) => {
          const {session} = agentsWithSessions.find((a) => a.agent.id === agent.id) ?? {};
          const isCreating = creatingSession === agent.id;
          return (
            <Button
              size="small"
              variant={session ? 'contained' : 'outlined'}
              disabled={isCreating}
              onClick={(e) => {
                e.stopPropagation();
                void handleAgentClick(agent.id, session?.id);
              }}
              sx={{minWidth: 120}}
            >
              {isCreating ? 'Creating...' : session ? 'Open Workshop' : 'Start Workshop'}
            </Button>
          );
        },
      },
    ],
    [agentsWithSessions, creatingSession, handleAgentClick],
  );

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
            const {session} = agentsWithSessions.find((a) => a.agent.id === agent.id) ?? {};
            void handleAgentClick(agent.id, session?.id);
          }}
          stickyHeader
          density="normal"
        />
      </Box>
    </FadeIn>
  );
}

export {AgentsPage};
