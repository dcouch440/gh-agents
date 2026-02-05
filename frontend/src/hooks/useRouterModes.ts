import { useState, useEffect, useCallback } from "react";
import type { RouterMode } from "@/types";
import { api } from "@/api";

const useRouterModes = (routerId: string) => {
  const [modes, setModes] = useState<RouterMode[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
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
    let cancelled = false;
    const run = async () => {
      await load();
      if (cancelled) return;
    };
    void run();
    return () => {
      cancelled = true;
    };
  }, [load]);

  return { modes, loading, error, reload: load };
};

export { useRouterModes };
