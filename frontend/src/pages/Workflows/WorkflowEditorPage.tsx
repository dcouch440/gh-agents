import {useEffect} from "react";
import {useParams, useNavigate} from "react-router-dom";
import Box from "@mui/material/Box";
import {
  useStore,
  workflowStore,
  agentStore,
  outputSchemaStore,
  protocolStore,
} from "@/stores";
import {WorkflowCanvas} from "@/components/canvas";

function WorkflowEditorPage() {
  const {id} = useParams<{id: string}>();
  const navigate = useNavigate();
  const loading = useStore(workflowStore.store, workflowStore.selectLoading);

  useEffect(() => {
    if (!id) {
      void navigate("/workflows");
      return;
    }
    void workflowStore.loadWorkflow(id);
    void agentStore.fetchAll();
    void outputSchemaStore.fetchIfStale();
    void protocolStore.fetchAll();
    return () => {
      workflowStore.clearActive();
    };
  }, [id, navigate]);

  if (loading) {
    return null;
  }

  return (
    <Box
      sx={{
        width: "100%",
        height: "100%",
        backgroundColor: (theme) => theme.palette.custom.cavityBg,
      }}
    >
      <WorkflowCanvas />
    </Box>
  );
}

export {WorkflowEditorPage};
