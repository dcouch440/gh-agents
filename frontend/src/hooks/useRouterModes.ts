import { useState, useEffect, useCallback } from "react";
import type { RouterMode } from "@/types";
import { api } from "@/api";

const useRouterModes = (routerId: string | null) => {
  const [modes, setModes] = useState<RouterMode[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!routerId) {
      setModes([]);
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const data = await api.routerModes.listByRouter(routerId);
      setModes(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load router modes");
    } finally {
      setLoading(false);
    }
  }, [routerId]);

  useEffect(() => {
    if (!routerId) {
      setModes([]);
      setLoading(false);
      return;
    }
    let cancelled = false;
    const run = async () => {
      await load();
      if (cancelled) return;
    };
    void run();
    return () => {
      cancelled = true;
    };
  }, [load, routerId]);

  return { modes, loading, error, reload: load };
};

export { useRouterModes };
