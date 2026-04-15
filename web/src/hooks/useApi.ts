import { useCallback, useState } from "react";

const API_BASE = "/api";

export interface MemoryEntry {
  id: string;
  content: string;
  kind: string;
  priority: string;
  source: string;
  created_at: string;
}

export interface MemoryStats {
  total: number;
  by_kind: Record<string, number>;
  by_priority: Record<string, number>;
}

export interface HealthStatus {
  status: string;
  uptime_seconds: number;
  memory_mb: number;
}

export interface Agent {
  id: string;
  name: string;
  status: "running" | "stopped" | "error";
  last_seen: string;
  metadata?: Record<string, unknown>;
}

async function api<T>(path: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...(options?.headers ?? {}),
    },
  });
  if (!response.ok) {
    throw new Error(await response.text());
  }
  return response.json() as Promise<T>;
}

export function useApi() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const getHealth = useCallback(async (): Promise<HealthStatus> => {
    return api<HealthStatus>("/health");
  }, []);

  const searchMemories = useCallback(
    async (query: string, kind?: string, limit = 20): Promise<MemoryEntry[]> => {
      const params = new URLSearchParams({ q: query, limit: String(limit) });
      if (kind) params.set("kind", kind);
      return api<MemoryEntry[]>(`/memory/search?${params}`);
    },
    [],
  );

  const getMemoryStats = useCallback(async (): Promise<MemoryStats> => {
    return api<MemoryStats>("/memory/stats");
  }, []);

  const addMemory = useCallback(
    async (content: string, kind = "note", priority = "medium", source = "web") => {
      return api<MemoryEntry>("/memory/add", {
        method: "POST",
        body: JSON.stringify({ content, kind, priority, source }),
      });
    },
    [],
  );

  const getAgents = useCallback(async (): Promise<Agent[]> => {
    return api<Agent[]>("/agents");
  }, []);

  return {
    loading,
    error,
    setError,
    getHealth,
    searchMemories,
    getMemoryStats,
    addMemory,
    getAgents,
  };
}
