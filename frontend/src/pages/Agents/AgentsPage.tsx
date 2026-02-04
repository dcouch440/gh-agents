import {useMemo, useState} from "react";
import {useNavigate} from "react-router-dom";
import {
  Box,
  Button,
  Card,
  CardContent,
  CardActions,
  Typography,
  CircularProgress,
  Alert,
  Grid,
} from "@mui/material";
import {PageHeader} from "@/components/primitives";
import {useAgents} from "@/hooks/useAgents";
import {useSessions} from "@/hooks/useSessions";
import {api} from "@/api";

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

  const handleAgentClick = async (agentId: string, sessionId?: string) => {
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
  };

  const handleNewWorkshop = () => {
    void navigate("/agents/workshop");
  };

  const loading = agentsLoading || sessionsLoading;

  return (
    <Box>
      <PageHeader title="Agents">
        <Button variant="contained" onClick={handleNewWorkshop}>
          New Workshop
        </Button>
      </PageHeader>

      {agentsError && (
        <Alert severity="error" sx={{mb: 2}}>
          {agentsError}
        </Alert>
      )}

      {loading ? (
        <Box sx={{display: "flex", justifyContent: "center", py: 4}}>
          <CircularProgress />
        </Box>
      ) : agentsWithSessions.length === 0 ? (
        <Box sx={{textAlign: "center", py: 4}}>
          <Typography variant="body1" color="text.secondary">
            No agents yet. Create your first one in the workshop!
          </Typography>
        </Box>
      ) : (
        <Grid container spacing={2}>
          {agentsWithSessions.map(({agent, session}) => (
            <Grid item xs={12} sm={6} md={4} key={agent.id}>
              <Card
                sx={{
                  height: "100%",
                  display: "flex",
                  flexDirection: "column",
                  cursor: "pointer",
                  transition: "transform 0.2s, box-shadow 0.2s",
                  "&:hover": {
                    transform: "translateY(-4px)",
                    boxShadow: 4,
                  },
                }}
                onClick={() => void handleAgentClick(agent.id, session?.id)}
              >
                <CardContent sx={{flexGrow: 1}}>
                  <Typography variant="h6" gutterBottom>
                    {agent.name}
                  </Typography>
                  <Typography
                    variant="body2"
                    color="text.secondary"
                    sx={{
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      display: "-webkit-box",
                      WebkitLineClamp: 3,
                      WebkitBoxOrient: "vertical",
                    }}
                  >
                    {agent.system_prompt || "No system prompt"}
                  </Typography>
                  <Typography variant="caption" color="text.secondary" sx={{mt: 1, display: "block"}}>
                    Model: {agent.model_id}
                  </Typography>
                </CardContent>
                <CardActions>
                  <Button
                    size="small"
                    fullWidth
                    variant={session ? "contained" : "outlined"}
                    disabled={creatingSession === agent.id}
                  >
                    {creatingSession === agent.id
                      ? "Creating..."
                      : session
                        ? "Open Workshop"
                        : "Start Workshop"}
                  </Button>
                </CardActions>
              </Card>
            </Grid>
          ))}
        </Grid>
      )}
    </Box>
  );
}

export {AgentsPage};
