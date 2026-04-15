import { useEffect, useState } from "react";
import { Bot, RefreshCw, Play, Square } from "lucide-react";
import { useApi, Agent } from "../hooks/useApi";

export function AgentManager() {
  const { getAgents } = useApi();
  const [agents, setAgents] = useState<Agent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedAgent, setSelectedAgent] = useState<Agent | null>(null);

  const loadAgents = () => {
    setLoading(true);
    getAgents()
      .then(setAgents)
      .catch((e: Error) => setError(e.message))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    loadAgents();
  }, [getAgents]);

  const statusColor = (status: Agent["status"]) => {
    switch (status) {
      case "running":
        return "text-green-600 dark:text-green-400";
      case "stopped":
        return "text-stone-400 dark:text-stone-500";
      case "error":
        return "text-red-500";
      default:
        return "text-stone-500";
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-stone-900 dark:text-stone-100">Agent Manager</h1>
          <p className="text-stone-500 text-sm mt-1">Monitor and control active agents</p>
        </div>
        <button
          onClick={loadAgents}
          className="flex items-center gap-2 px-4 py-2 border border-stone-300 dark:border-stone-600 rounded-lg text-sm font-medium text-stone-700 dark:text-stone-200 hover:bg-stone-100 dark:hover:bg-stone-700 transition-colors"
        >
          <RefreshCw size={16} />
          Refresh
        </button>
      </div>

      {error && <div className="text-red-500 text-sm">Error: {error}</div>}

      {loading ? (
        <div className="text-stone-500">Loading agents...</div>
      ) : agents.length === 0 ? (
        <div className="text-center py-12 text-stone-400 dark:text-stone-500">
          No agents found
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Agent List */}
          <div className="lg:col-span-1 space-y-3">
            <h2 className="text-xs font-semibold text-stone-500 dark:text-stone-400 uppercase tracking-wider">
              Active Agents ({agents.length})
            </h2>
            {agents.map((agent) => (
              <button
                key={agent.id}
                onClick={() => setSelectedAgent(agent)}
                className={`w-full text-left p-4 rounded-xl border transition-colors ${
                  selectedAgent?.id === agent.id
                    ? "border-primary-400 dark:border-primary-600 bg-primary-50 dark:bg-primary-900/20"
                    : "border-stone-200 dark:border-stone-700 bg-white dark:bg-stone-800 hover:border-stone-400 dark:hover:border-stone-500"
                }`}
              >
                <div className="flex items-center gap-3">
                  <div className="p-2 rounded-lg bg-stone-100 dark:bg-stone-700 text-stone-600 dark:text-stone-300">
                    <Bot size={18} />
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="font-medium text-stone-900 dark:text-stone-100 truncate">
                      {agent.name}
                    </p>
                    <p className="text-xs text-stone-400 dark:text-stone-500">
                      {agent.last_seen
                        ? `Last seen ${new Date(agent.last_seen).toLocaleString()}`
                        : "Never seen"}
                    </p>
                  </div>
                  <span className={`text-xs font-medium ${statusColor(agent.status)}`}>
                    {agent.status}
                  </span>
                </div>
              </button>
            ))}
          </div>

          {/* Agent Detail */}
          <div className="lg:col-span-2">
            {selectedAgent ? (
              <AgentDetail agent={selectedAgent} onRefresh={loadAgents} />
            ) : (
              <div className="h-full flex items-center justify-center text-stone-400 dark:text-stone-500 bg-white dark:bg-stone-800 rounded-xl border border-stone-200 dark:border-stone-700 p-12">
                Select an agent to view details
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function AgentDetail({
  agent,
  onRefresh,
}: {
  agent: Agent;
  onRefresh: () => void;
}) {
  return (
    <div className="bg-white dark:bg-stone-800 rounded-xl border border-stone-200 dark:border-stone-700 p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="p-3 rounded-xl bg-primary-100 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400">
            <Bot size={24} />
          </div>
          <div>
            <h3 className="text-lg font-bold text-stone-900 dark:text-stone-100">{agent.name}</h3>
            <p className="text-sm text-stone-500 dark:text-stone-400">ID: {agent.id}</p>
          </div>
        </div>
        <span className={`px-3 py-1 rounded-full text-sm font-medium ${
          agent.status === "running"
            ? "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400"
            : agent.status === "error"
            ? "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400"
            : "bg-stone-100 text-stone-600 dark:bg-stone-700 dark:text-stone-400"
        }`}>
          {agent.status}
        </span>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="p-4 bg-stone-50 dark:bg-stone-900/50 rounded-lg">
          <p className="text-xs text-stone-500 dark:text-stone-400 uppercase tracking-wider mb-1">
            Last Seen
          </p>
          <p className="text-sm font-medium text-stone-800 dark:text-stone-200">
            {agent.last_seen
              ? new Date(agent.last_seen).toLocaleString()
              : "Never"}
          </p>
        </div>
        <div className="p-4 bg-stone-50 dark:bg-stone-900/50 rounded-lg">
          <p className="text-xs text-stone-500 dark:text-stone-400 uppercase tracking-wider mb-1">
            Agent ID
          </p>
          <p className="text-sm font-medium text-stone-800 dark:text-stone-200 font-mono text-xs truncate">
            {agent.id}
          </p>
        </div>
      </div>

      {agent.metadata && Object.keys(agent.metadata).length > 0 && (
        <div>
          <h4 className="text-xs font-semibold text-stone-500 dark:text-stone-400 uppercase tracking-wider mb-3">
            Metadata
          </h4>
          <div className="bg-stone-50 dark:bg-stone-900/50 rounded-lg p-4">
            <pre className="text-xs text-stone-700 dark:text-stone-300 overflow-x-auto">
              {JSON.stringify(agent.metadata, null, 2)}
            </pre>
          </div>
        </div>
      )}

      {/* Controls */}
      <div className="flex items-center gap-3 pt-2">
        <button
          className="flex items-center gap-2 px-4 py-2 bg-green-600 text-white rounded-lg text-sm font-medium hover:bg-green-700 transition-colors"
        >
          <Play size={16} />
          Start
        </button>
        <button
          className="flex items-center gap-2 px-4 py-2 border border-stone-300 dark:border-stone-600 text-stone-700 dark:text-stone-200 rounded-lg text-sm font-medium hover:bg-stone-100 dark:hover:bg-stone-700 transition-colors"
        >
          <Square size={16} />
          Stop
        </button>
      </div>
    </div>
  );
}
