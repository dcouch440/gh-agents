import { useState, useCallback } from "react";
import { api } from "@/api";
import type {
  CreateRouterModeRequest,
  UpdateRouterModeRequest,
  RouterMode,
  SetModeToolsRequest,
  Tool,
} from "@/types";

const useRouterModeMutations = () => {
  const [creating, setCreating] = useState(false);
  const [updating, setUpdating] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [loadingTools, setLoadingTools] = useState(false);
  const [savingTools, setSavingTools] = useState(false);
  const [toolsError, setToolsError] = useState<string | null>(null);

  const createMode = useCallback(
    async (
      routerId: string,
      body: CreateRouterModeRequest
    ): Promise<RouterMode> => {
      setCreating(true);
      try {
        return await api.routerModes.createForRouter(routerId, body);
      } finally {
        setCreating(false);
      }
    },
    []
  );

  const updateMode = useCallback(
    async (
      modeId: string,
      body: UpdateRouterModeRequest
    ): Promise<RouterMode> => {
      setUpdating(true);
      try {
        return await api.routerModes.update(modeId, body);
      } finally {
        setUpdating(false);
      }
    },
    []
  );

  const deleteMode = useCallback(async (modeId: string): Promise<void> => {
    setDeleting(true);
    try {
      await api.routerModes.delete(modeId);
    } finally {
      setDeleting(false);
    }
  }, []);

  const loadModeTools = useCallback(async (modeId: string): Promise<Tool[]> => {
    setLoadingTools(true);
    setToolsError(null);
    try {
      return await api.routerModes.getTools(modeId);
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Failed to load mode tools";
      setToolsError(msg);
      throw e;
    } finally {
      setLoadingTools(false);
    }
  }, []);

  const saveModeTools = useCallback(
    async (modeId: string, body: SetModeToolsRequest): Promise<void> => {
      setSavingTools(true);
      setToolsError(null);
      try {
        await api.routerModes.setTools(modeId, body);
      } catch (e) {
        const msg =
          e instanceof Error ? e.message : "Failed to save mode tools";
        setToolsError(msg);
        throw e;
      } finally {
        setSavingTools(false);
      }
    },
    []
  );

  return {
    createMode,
    creating,
    updateMode,
    updating,
    deleteMode,
    deleting,
    loadModeTools,
    saveModeTools,
    loadingTools,
    savingTools,
    toolsError,
  };
};

export { useRouterModeMutations };
