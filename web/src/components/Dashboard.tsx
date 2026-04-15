import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { Brain, Bot, Activity, Plus, Search, Settings } from "lucide-react";
import { useApi, HealthStatus, MemoryStats } from "../hooks/useApi";

export function Dashboard() {
  const { getHealth, getMemoryStats } = useApi();
  const [health, setHealth] = useState<HealthStatus | null>(null);
  const [stats, setStats] = useState<MemoryStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([getHealth(), getMemoryStats()])
      .then(([h, s]) => {
        setHealth(h);
        setStats(s);
      })
      .catch((e: Error) => setError(e.message))
      .finally(() => setLoading(false));
  }, [getHealth, getMemoryStats]);

  if (loading) {
    return <div className="text-stone-500">Loading...</div>;
  }

  if (error) {
    return <div className="text-red-500">Error: {error}</div>;
  }

  const statusColor =
    health?.status === "ok" || health?.status === "healthy"
      ? "text-green-600 dark:text-green-400"
      : "text-yellow-600 dark:text-yellow-400";

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-stone-900 dark:text-stone-100">Dashboard</h1>
          <p className="text-stone-500 text-sm mt-1">System overview and quick actions</p>
        </div>
        <span className={`text-sm font-medium ${statusColor}`}>
          ● {health?.status ?? "unknown"}
        </span>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          icon={Brain}
          label="Total Memories"
          value={stats?.total ?? 0}
          color="text-primary-600"
        />
        <StatCard
          icon={Bot}
          label="Active Agents"
          value={stats?.by_kind?.agent ?? 0}
          color="text-blue-600"
        />
        <StatCard
          icon={Activity}
          label="Uptime"
          value={health ? formatUptime(health.uptime_seconds) : "—"}
          color="text-emerald-600"
        />
        <StatCard
          icon={Brain}
          label="Memory Used"
          value={health ? `${health.memory_mb} MB` : "—"}
          color="text-amber-600"
        />
      </div>

      {/* Memory by kind */}
      {stats && (
        <div className="bg-white dark:bg-stone-800 rounded-xl border border-stone-200 dark:border-stone-700 p-6">
          <h2 className="text-sm font-semibold text-stone-600 dark:text-stone-300 uppercase tracking-wider mb-4">
            Memories by Kind
          </h2>
          <div className="flex flex-wrap gap-3">
            {Object.entries(stats.by_kind).map(([kind, count]) => (
              <span
                key={kind}
                className="inline-flex items-center gap-2 px-3 py-1.5 bg-stone-100 dark:bg-stone-700 rounded-full text-sm text-stone-700 dark:text-stone-200"
              >
                <span className="font-medium">{count}</span>
                <span className="text-stone-400 dark:text-stone-500">{kind}</span>
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Quick Actions */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        <QuickAction
          to="/memory"
          icon={Search}
          label="Search Memories"
          description="Query the knowledge base"
        />
        <QuickAction
          to="/memory"
          icon={Plus}
          label="Add Memory"
          description="Store new information"
        />
        <QuickAction
          to="/settings"
          icon={Settings}
          label="Settings"
          description="Configure the dashboard"
        />
      </div>
    </div>
  );
}

function StatCard({
  icon: Icon,
  label,
  value,
  color,
}: {
  icon: React.ElementType;
  label: string;
  value: string | number;
  color: string;
}) {
  return (
    <div className="bg-white dark:bg-stone-800 rounded-xl border border-stone-200 dark:border-stone-700 p-5">
      <div className="flex items-center gap-3 mb-3">
        <div className={`p-2 rounded-lg bg-stone-100 dark:bg-stone-700 ${color}`}>
          <Icon size={18} />
        </div>
        <span className="text-xs font-medium text-stone-500 dark:text-stone-400 uppercase tracking-wider">
          {label}
        </span>
      </div>
      <p className="text-2xl font-bold text-stone-900 dark:text-stone-100">{value}</p>
    </div>
  );
}

function QuickAction({
  to,
  icon: Icon,
  label,
  description,
}: {
  to: string;
  icon: React.ElementType;
  label: string;
  description: string;
}) {
  return (
    <Link
      to={to}
      className="flex items-center gap-4 p-4 bg-white dark:bg-stone-800 rounded-xl border border-stone-200 dark:border-stone-700 hover:border-primary-400 dark:hover:border-primary-600 transition-colors group"
    >
      <div className="p-2.5 rounded-lg bg-primary-50 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 group-hover:bg-primary-100 dark:group-hover:bg-primary-900/50 transition-colors">
        <Icon size={20} />
      </div>
      <div>
        <p className="font-medium text-stone-900 dark:text-stone-100">{label}</p>
        <p className="text-sm text-stone-500 dark:text-stone-400">{description}</p>
      </div>
    </Link>
  );
}

function formatUptime(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  return `${h}h ${m}m`;
}
