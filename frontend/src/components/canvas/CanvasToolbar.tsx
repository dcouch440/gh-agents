import {useState, useEffect, useCallback} from "react";
import Box from "@mui/material/Box";
import Button from "@mui/material/Button";
import Chip from "@mui/material/Chip";
import Collapse from "@mui/material/Collapse";
import IconButton from "@mui/material/IconButton";
import TextField from "@mui/material/TextField";
import Tooltip from "@mui/material/Tooltip";
import Typography from "@mui/material/Typography";
import CircularProgress from "@mui/material/CircularProgress";
import Fade from "@mui/material/Fade";
import Zoom from "@mui/material/Zoom";
import SaveOutlined from "@mui/icons-material/SaveOutlined";
import UndoOutlined from "@mui/icons-material/UndoOutlined";
import CheckCircleOutline from "@mui/icons-material/CheckCircleOutline";
import PlayArrowOutlined from "@mui/icons-material/PlayArrowOutlined";
import ErrorOutline from "@mui/icons-material/ErrorOutline";
import InputOutlined from "@mui/icons-material/InputOutlined";
import {useStore, workflowStore} from "@/stores";
import {api} from "@/api";
import {LS_WORKFLOW_TEST_INPUT} from "@/constants";

type RunStatus = "idle" | "running" | "completed" | "error";

function RunButton() {
  const activeWorkflowId = useStore(
    workflowStore.store,
    workflowStore.selectActiveWorkflowId,
  );
  const [runStatus, setRunStatus] = useState<RunStatus>("idle");
  const [inputExpanded, setInputExpanded] = useState(false);
  const [testInput, setTestInput] = useState("");
  const [prevWorkflowId, setPrevWorkflowId] = useState<string | null>(null);

  // Sync test input from localStorage when active workflow changes (React-approved pattern)
  if (activeWorkflowId !== prevWorkflowId) {
    setPrevWorkflowId(activeWorkflowId);
    if (activeWorkflowId) {
      const stored = localStorage.getItem(
        LS_WORKFLOW_TEST_INPUT + activeWorkflowId,
      );
      setTestInput(stored ?? "");
    } else {
      setTestInput("");
    }
  }

  const handleTestInputChange = useCallback(
    (value: string) => {
      setTestInput(value);
      if (activeWorkflowId) {
        localStorage.setItem(
          LS_WORKFLOW_TEST_INPUT + activeWorkflowId,
          value,
        );
      }
    },
    [activeWorkflowId],
  );

  const handleRun = useCallback(async () => {
    if (!activeWorkflowId || runStatus === "running") return;
    setRunStatus("running");
    try {
      const trimmed = testInput.trim();
      const body = trimmed ? {initial_input: trimmed} : undefined;
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
  }, [activeWorkflowId, runStatus, testInput]);

  if (!activeWorkflowId) return null;

  const icon =
    runStatus === "running" ? (
      <CircularProgress size={14} thickness={5} color="inherit" />
    ) : runStatus === "completed" ? (
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

  const label =
    runStatus === "running"
      ? "Running..."
      : runStatus === "completed"
        ? "Started!"
        : runStatus === "error"
          ? "Failed"
          : "Run";

  const bgGradient =
    runStatus === "completed"
      ? "linear-gradient(135deg, #10b981 0%, #059669 100%)"
      : runStatus === "error"
        ? "linear-gradient(135deg, #ef4444 0%, #dc2626 100%)"
        : "linear-gradient(135deg, #10b981 0%, #059669 100%)";

  const shadow =
    runStatus === "completed"
      ? "0 4px 12px rgba(16, 185, 129, 0.4)"
      : runStatus === "error"
        ? "0 4px 12px rgba(239, 68, 68, 0.4)"
        : "0 4px 12px rgba(16, 185, 129, 0.4)";

  return (
    <Fade in timeout={300}>
      <Box
        sx={{
          position: "absolute",
          top: 16,
          right: 16,
          zIndex: 10,
          display: "flex",
          flexDirection: "column",
          alignItems: "flex-end",
          gap: 1,
        }}
      >
        <Box sx={{display: "flex", gap: 0.75, alignItems: "center"}}>
          <Tooltip
            title={inputExpanded ? "Hide test input" : "Add test input"}
            TransitionComponent={Fade}
            enterDelay={500}
            placement="left"
          >
            <IconButton
              size="small"
              onClick={() => {
                setInputExpanded((prev) => !prev);
              }}
              sx={{
                width: 32,
                height: 32,
                borderRadius: "10px",
                backgroundColor: inputExpanded
                  ? "rgba(59, 130, 246, 0.2)"
                  : "rgba(12, 16, 24, 0.85)",
                backdropFilter: "blur(12px)",
                border: "1px solid",
                borderColor: inputExpanded
                  ? "rgba(59, 130, 246, 0.4)"
                  : "rgba(240, 246, 252, 0.1)",
                color: inputExpanded
                  ? "#60a5fa"
                  : "rgba(240, 246, 252, 0.7)",
                transition: "all 0.2s ease",
                "&:hover": {
                  backgroundColor: inputExpanded
                    ? "rgba(59, 130, 246, 0.3)"
                    : "rgba(240, 246, 252, 0.08)",
                  borderColor: inputExpanded
                    ? "rgba(59, 130, 246, 0.5)"
                    : "rgba(240, 246, 252, 0.2)",
                },
              }}
            >
              <InputOutlined sx={{fontSize: 16}} />
            </IconButton>
          </Tooltip>

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
              <Button
                size="small"
                variant="contained"
                startIcon={icon}
                onClick={() => {
                  void handleRun();
                }}
                disabled={runStatus === "running"}
                sx={{
                  fontSize: 13,
                  fontWeight: 600,
                  textTransform: "none",
                  px: 2.5,
                  py: 0.75,
                  minWidth: 90,
                  background: bgGradient,
                  boxShadow: shadow,
                  borderRadius: "12px",
                  transition: "all 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
                  "&:hover": {
                    background:
                      runStatus === "completed"
                        ? "linear-gradient(135deg, #059669 0%, #047857 100%)"
                        : runStatus === "error"
                          ? "linear-gradient(135deg, #dc2626 0%, #b91c1c 100%)"
                          : "linear-gradient(135deg, #059669 0%, #047857 100%)",
                    boxShadow:
                      runStatus === "completed"
                        ? "0 6px 16px rgba(16, 185, 129, 0.5)"
                        : runStatus === "error"
                          ? "0 6px 16px rgba(239, 68, 68, 0.5)"
                          : "0 6px 16px rgba(16, 185, 129, 0.5)",
                    transform: "translateY(-1px)",
                    "& .MuiSvgIcon-root": {
                      transform: "scale(1.1)",
                    },
                  },
                  "&:active": {
                    transform: "translateY(0) scale(0.98)",
                  },
                  "&.Mui-disabled": {
                    background: "rgba(16, 185, 129, 0.3)",
                    color: "rgba(255, 255, 255, 0.5)",
                  },
                }}
              >
                {label}
              </Button>
            </span>
          </Tooltip>
        </Box>

        <Collapse in={inputExpanded} timeout={200}>
          <Box
            sx={{
              width: 320,
              p: 1.5,
              borderRadius: "12px",
              backgroundColor: "rgba(12, 16, 24, 0.92)",
              backdropFilter: "blur(16px)",
              border: "1px solid",
              borderColor: "rgba(240, 246, 252, 0.1)",
              boxShadow: "0 8px 32px rgba(0, 0, 0, 0.4)",
            }}
          >
            <Box
              sx={{
                display: "flex",
                alignItems: "center",
                gap: 1,
                mb: 1,
              }}
            >
              <Typography
                variant="caption"
                sx={{
                  color: "rgba(240, 246, 252, 0.6)",
                  fontWeight: 600,
                  fontSize: 11,
                  letterSpacing: "0.05em",
                  textTransform: "uppercase",
                }}
              >
                Test Input
              </Typography>
              <Chip
                label="DEV"
                size="small"
                sx={{
                  height: 18,
                  fontSize: 9,
                  fontWeight: 700,
                  letterSpacing: "0.08em",
                  backgroundColor: "rgba(251, 191, 36, 0.15)",
                  color: "#fbbf24",
                  borderRadius: "4px",
                  "& .MuiChip-label": {px: 0.75},
                }}
              />
            </Box>
            <TextField
              multiline
              rows={4}
              fullWidth
              placeholder="Paste test input here... Available as {input} in entry step prompts"
              value={testInput}
              onChange={(e) => {
                handleTestInputChange(e.target.value);
              }}
              sx={{
                "& .MuiOutlinedInput-root": {
                  fontSize: 12,
                  fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
                  color: "rgba(240, 246, 252, 0.9)",
                  backgroundColor: "rgba(0, 0, 0, 0.3)",
                  borderRadius: "8px",
                  "& fieldset": {
                    borderColor: "rgba(240, 246, 252, 0.08)",
                  },
                  "&:hover fieldset": {
                    borderColor: "rgba(240, 246, 252, 0.15)",
                  },
                  "&.Mui-focused fieldset": {
                    borderColor: "rgba(59, 130, 246, 0.5)",
                    borderWidth: 1,
                  },
                },
                "& .MuiOutlinedInput-input::placeholder": {
                  color: "rgba(240, 246, 252, 0.3)",
                  fontSize: 11,
                },
              }}
            />
          </Box>
        </Collapse>
      </Box>
    </Fade>
  );
}

function SaveDiscardBar() {
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
          backgroundColor: "rgba(12, 16, 24, 0.92)",
          backdropFilter: "blur(16px)",
          border: "1px solid",
          borderColor: "rgba(240, 246, 252, 0.12)",
          boxShadow:
            "0 8px 32px rgba(0, 0, 0, 0.5), 0 1px 2px rgba(0, 0, 0, 0.3), inset 0 1px 0 rgba(255, 255, 255, 0.05)",
          transition: "all 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
          "&:hover": {
            borderColor: "rgba(240, 246, 252, 0.18)",
            boxShadow:
              "0 12px 40px rgba(0, 0, 0, 0.6), 0 2px 4px rgba(0, 0, 0, 0.3), inset 0 1px 0 rgba(255, 255, 255, 0.08)",
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
                borderColor: "rgba(240, 246, 252, 0.14)",
                color: "rgba(240, 246, 252, 0.9)",
                px: 2,
                py: 0.75,
                minWidth: 100,
                transition: "all 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
                "&:hover": {
                  borderColor: "rgba(240, 246, 252, 0.3)",
                  backgroundColor: "rgba(255, 255, 255, 0.08)",
                  "& .MuiSvgIcon-root": {
                    transform: "rotate(-30deg)",
                  },
                },
                "&:active": {
                  transform: "scale(0.98)",
                },
                "&.Mui-disabled": {
                  borderColor: "rgba(240, 246, 252, 0.06)",
                  color: "rgba(240, 246, 252, 0.3)",
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
            <Button
              size="small"
              variant="contained"
              startIcon={
                saving ? (
                  <CircularProgress size={14} thickness={5} color="inherit" />
                ) : justSaved ? (
                  <Zoom in timeout={200}>
                    <CheckCircleOutline sx={{fontSize: 16}} />
                  </Zoom>
                ) : (
                  <SaveOutlined
                    sx={{
                      fontSize: 16,
                      transition: "transform 0.2s ease",
                    }}
                  />
                )
              }
              onClick={() => {
                void handleSave();
              }}
              disabled={saving}
              sx={{
                fontSize: 13,
                fontWeight: 600,
                textTransform: "none",
                px: 2.5,
                py: 0.75,
                minWidth: 100,
                background: justSaved
                  ? "linear-gradient(135deg, #10b981 0%, #059669 100%)"
                  : "linear-gradient(135deg, #3b82f6 0%, #2563eb 100%)",
                boxShadow: justSaved
                  ? "0 4px 12px rgba(16, 185, 129, 0.4)"
                  : "0 4px 12px rgba(59, 130, 246, 0.4)",
                transition: "all 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
                "&:hover": {
                  background: justSaved
                    ? "linear-gradient(135deg, #059669 0%, #047857 100%)"
                    : "linear-gradient(135deg, #2563eb 0%, #1d4ed8 100%)",
                  boxShadow: justSaved
                    ? "0 6px 16px rgba(16, 185, 129, 0.5)"
                    : "0 6px 16px rgba(59, 130, 246, 0.5)",
                  transform: "translateY(-1px)",
                  "& .MuiSvgIcon-root": {
                    transform: "scale(1.1)",
                  },
                },
                "&:active": {
                  transform: "translateY(0) scale(0.98)",
                  boxShadow: justSaved
                    ? "0 2px 8px rgba(16, 185, 129, 0.4)"
                    : "0 2px 8px rgba(59, 130, 246, 0.4)",
                },
                "&.Mui-disabled": {
                  background: "rgba(59, 130, 246, 0.3)",
                  color: "rgba(255, 255, 255, 0.5)",
                },
              }}
            >
              {saving ? "Saving..." : justSaved ? "Saved!" : "Save"}
            </Button>
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
