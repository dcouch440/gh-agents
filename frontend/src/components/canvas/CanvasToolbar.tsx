import {useState, useEffect, useCallback} from "react";
import Box from "@mui/material/Box";
import Button from "@mui/material/Button";
import Tooltip from "@mui/material/Tooltip";
import Fade from "@mui/material/Fade";
import Zoom from "@mui/material/Zoom";
import SaveOutlined from "@mui/icons-material/SaveOutlined";
import UndoOutlined from "@mui/icons-material/UndoOutlined";
import CheckCircleOutline from "@mui/icons-material/CheckCircleOutline";
import PlayArrowOutlined from "@mui/icons-material/PlayArrowOutlined";
import ErrorOutline from "@mui/icons-material/ErrorOutline";
import {useTheme} from "@mui/material/styles";
import {useStore, workflowStore} from "@/stores";
import {api} from "@/api";
import {GradientButton} from "@/components/primitives";

type RunStatus = "idle" | "running" | "completed" | "error";

function RunButton() {
  const activeWorkflowId = useStore(
    workflowStore.store,
    workflowStore.selectActiveWorkflowId,
  );
  const steps = useStore(workflowStore.store, workflowStore.selectSteps);
  const [runStatus, setRunStatus] = useState<RunStatus>("idle");

  const handleRun = useCallback(async () => {
    if (!activeWorkflowId || runStatus === "running") return;
    setRunStatus("running");
    try {
      const entryStep = steps.find((s) => s.execution_mode === "entry");
      const input = entryStep?.prompt_template.trim();
      const body = input ? {initial_input: input} : undefined;
      await api.workflows.run(activeWorkflowId, body);
      setRunStatus("completed");
      setTimeout(() => {
        setRunStatus("idle");
      }, 3000);
    } catch {
      setRunStatus("error");
      setTimeout(() => {
        setRunStatus("idle");
      }, 3000);
    }
  }, [activeWorkflowId, runStatus, steps]);

  if (!activeWorkflowId) return null;

  const runIcon =
    runStatus === "completed" ? (
      <Zoom in timeout={200}>
        <CheckCircleOutline sx={{fontSize: 16}} />
      </Zoom>
    ) : runStatus === "error" ? (
      <ErrorOutline sx={{fontSize: 16}} />
    ) : (
      <PlayArrowOutlined
        sx={{fontSize: 16, transition: "transform 0.2s ease"}}
      />
    );

  const runLabel =
    runStatus === "running"
      ? "Running..."
      : runStatus === "completed"
        ? "Started!"
        : runStatus === "error"
          ? "Failed"
          : "Run";

  const runColor =
    runStatus === "completed"
      ? "success" as const
      : runStatus === "error"
        ? "error" as const
        : "primary" as const;

  return (
    <Fade in timeout={300}>
      <Box
        sx={{
          position: "absolute",
          top: 16,
          right: 16,
          zIndex: 10,
          display: "flex",
          alignItems: "center",
          gap: 0.75,
        }}
      >
        <Tooltip
          title={
            runStatus === "running"
              ? "Workflow is running..."
              : runStatus === "completed"
                ? "Execution started successfully"
                : runStatus === "error"
                  ? "Execution failed to start"
                  : "Run this workflow"
          }
          TransitionComponent={Fade}
          enterDelay={500}
          placement="left"
        >
          <span>
            <GradientButton
              onClick={() => { void handleRun(); }}
              icon={runIcon}
              color={runColor}
              loading={runStatus === "running"}
              disabled={runStatus === "running"}
            >
              {runLabel}
            </GradientButton>
          </span>
        </Tooltip>
      </Box>
    </Fade>
  );
}

function SaveDiscardBar() {
  const theme = useTheme();
  const isDark = theme.palette.mode === "dark";
  const dirty = useStore(workflowStore.store, workflowStore.selectDirty);
  const [saving, setSaving] = useState(false);
  const [justSaved, setJustSaved] = useState(false);

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      await workflowStore.saveAllDirtySteps();
      setJustSaved(true);
      setTimeout(() => {
        setJustSaved(false);
      }, 2000);
    } finally {
      setSaving(false);
    }
  }, []);

  const handleDiscard = useCallback(() => {
    void workflowStore.revertSteps();
  }, []);

  // Cmd+S / Ctrl+S keyboard shortcut
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "s") {
        e.preventDefault();
        if (dirty && !saving) {
          void handleSave();
        }
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [dirty, saving, handleSave]);

  if (!dirty) return null;

  return (
    <Zoom in={dirty} timeout={300}>
      <Box
        sx={{
          position: "absolute",
          bottom: 24,
          left: "50%",
          transform: "translateX(-50%)",
          zIndex: 10,
          display: "flex",
          gap: 1.5,
          px: 2,
          py: 1.25,
          borderRadius: "16px",
          backgroundColor: theme.palette.custom.floatingPanelBg,
          backdropFilter: "blur(16px)",
          border: "1px solid",
          borderColor: theme.palette.custom.floatingPanelBorder,
          boxShadow: isDark
            ? "0 8px 32px rgba(0, 0, 0, 0.5), 0 1px 2px rgba(0, 0, 0, 0.3), inset 0 1px 0 rgba(255, 255, 255, 0.05)"
            : "0 8px 32px rgba(45, 27, 14, 0.12), 0 1px 2px rgba(45, 27, 14, 0.06)",
          transition: "all 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
          "&:hover": {
            borderColor: theme.palette.custom.borderHover,
            boxShadow: isDark
              ? "0 12px 40px rgba(0, 0, 0, 0.6), 0 2px 4px rgba(0, 0, 0, 0.3), inset 0 1px 0 rgba(255, 255, 255, 0.08)"
              : "0 12px 40px rgba(45, 27, 14, 0.16), 0 2px 4px rgba(45, 27, 14, 0.08)",
          },
        }}
      >
        <Tooltip
          title="Discard changes (Esc)"
          TransitionComponent={Fade}
          enterDelay={500}
          placement="top"
        >
          <span>
            <Button
              size="small"
              variant="outlined"
              startIcon={
                <UndoOutlined
                  sx={{
                    fontSize: 16,
                    transition: "transform 0.2s ease",
                  }}
                />
              }
              onClick={handleDiscard}
              disabled={saving}
              sx={{
                fontSize: 13,
                fontWeight: 500,
                textTransform: "none",
                borderColor: theme.palette.custom.floatingPanelBorder,
                color: theme.palette.text.primary,
                px: 2,
                py: 0.75,
                minWidth: 100,
                transition: "all 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
                "&:hover": {
                  borderColor: theme.palette.custom.borderHover,
                  backgroundColor: theme.palette.custom.activeTintStrong,
                  "& .MuiSvgIcon-root": {
                    transform: "rotate(-30deg)",
                  },
                },
                "&:active": {
                  transform: "scale(0.98)",
                },
                "&.Mui-disabled": {
                  borderColor: theme.palette.divider,
                  color: theme.palette.text.disabled,
                },
              }}
            >
              Discard
            </Button>
          </span>
        </Tooltip>

        <Tooltip
          title={justSaved ? "Saved!" : "Save changes (\u2318S)"}
          TransitionComponent={Fade}
          enterDelay={500}
          placement="top"
        >
          <span>
            <GradientButton
              onClick={() => { void handleSave(); }}
              loading={saving}
              disabled={saving}
              color={justSaved ? "success" : "primary"}
              icon={
                justSaved ? (
                  <Zoom in timeout={200}>
                    <CheckCircleOutline sx={{fontSize: 16}} />
                  </Zoom>
                ) : (
                  <SaveOutlined
                    sx={{ fontSize: 16, transition: "transform 0.2s ease" }}
                  />
                )
              }
              minWidth={100}
            >
              {saving ? "Saving..." : justSaved ? "Saved!" : "Save"}
            </GradientButton>
          </span>
        </Tooltip>
      </Box>
    </Zoom>
  );
}

function CanvasToolbar() {
  return (
    <>
      <RunButton />
      <SaveDiscardBar />
    </>
  );
}

export {CanvasToolbar};
