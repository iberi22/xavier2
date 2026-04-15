import { useEffect, useState, useCallback } from "react";
import { Search, Plus, Filter, ChevronLeft, ChevronRight } from "lucide-react";
import { useApi, MemoryEntry } from "../hooks/useApi";

const PAGE_SIZE = 20;

const KIND_OPTIONS = ["", "note", "fact", "preference", "context", "task", "agent"];

export function MemoryBrowser() {
  const { searchMemories, addMemory } = useApi();
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState("");
  const [memories, setMemories] = useState<MemoryEntry[]>([]);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Add memory form
  const [showAdd, setShowAdd] = useState(false);
  const [newContent, setNewContent] = useState("");
  const [newKind, setNewKind] = useState("note");
  const [adding, setAdding] = useState(false);

  const doSearch = useCallback(
    (q: string, k: string, p: number) => {
      setLoading(true);
      setError(null);
      searchMemories(q, k || undefined, PAGE_SIZE)
        .then((results) => {
          // Client-side pagination (API returns up to PAGE_SIZE)
          const start = (p - 1) * PAGE_SIZE;
          setMemories(results.slice(start, start + PAGE_SIZE));
        })
        .catch((e: Error) => setError(e.message))
        .finally(() => setLoading(false));
    },
    [searchMemories],
  );

  useEffect(() => {
    doSearch(query, kind, page);
  }, [query, kind, page, doSearch]);

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newContent.trim()) return;
    setAdding(true);
    try {
      await addMemory(newContent.trim(), newKind);
      setNewContent("");
      setShowAdd(false);
      doSearch(query, kind, page);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to add memory");
    } finally {
      setAdding(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-stone-900 dark:text-stone-100">Memory Browser</h1>
          <p className="text-stone-500 text-sm mt-1">Search and browse the shared memory store</p>
        </div>
        <button
          onClick={() => setShowAdd(!showAdd)}
          className="flex items-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-lg text-sm font-medium hover:bg-primary-700 transition-colors"
        >
          <Plus size={16} />
          Add Memory
        </button>
      </div>

      {/* Add Memory Form */}
      {showAdd && (
        <form
          onSubmit={handleAdd}
          className="bg-white dark:bg-stone-800 rounded-xl border border-stone-200 dark:border-stone-700 p-5 space-y-4"
        >
          <h2 className="font-semibold text-stone-800 dark:text-stone-100">Add New Memory</h2>
          <textarea
            value={newContent}
            onChange={(e) => setNewContent(e.target.value)}
            placeholder="Memory content..."
            rows={3}
            className="w-full px-3 py-2 rounded-lg border border-stone-300 dark:border-stone-600 bg-white dark:bg-stone-900 text-stone-900 dark:text-stone-100 text-sm resize-none focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
          <div className="flex items-center gap-3">
            <select
              value={newKind}
              onChange={(e) => setNewKind(e.target.value)}
              className="px-3 py-2 rounded-lg border border-stone-300 dark:border-stone-600 bg-white dark:bg-stone-900 text-stone-900 dark:text-stone-100 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
            >
              {KIND_OPTIONS.slice(1).map((k) => (
                <option key={k} value={k}>
                  {k}
                </option>
              ))}
            </select>
            <button
              type="submit"
              disabled={adding || !newContent.trim()}
              className="px-4 py-2 bg-primary-600 text-white rounded-lg text-sm font-medium hover:bg-primary-700 disabled:opacity-50 transition-colors"
            >
              {adding ? "Saving..." : "Save"}
            </button>
            <button
              type="button"
              onClick={() => setShowAdd(false)}
              className="px-4 py-2 text-stone-600 dark:text-stone-400 rounded-lg text-sm font-medium hover:text-stone-800 dark:hover:text-stone-200 transition-colors"
            >
              Cancel
            </button>
          </div>
        </form>
      )}

      {/* Search & Filters */}
      <div className="flex flex-col sm:flex-row gap-3">
        <div className="relative flex-1">
          <Search
            size={16}
            className="absolute left-3 top-1/2 -translate-y-1/2 text-stone-400"
          />
          <input
            type="text"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setPage(1);
            }}
            placeholder="Search memories..."
            className="w-full pl-9 pr-4 py-2 rounded-lg border border-stone-300 dark:border-stone-600 bg-white dark:bg-stone-800 text-stone-900 dark:text-stone-100 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
        </div>
        <div className="flex items-center gap-2">
          <Filter size={16} className="text-stone-400" />
          <select
            value={kind}
            onChange={(e) => {
              setKind(e.target.value);
              setPage(1);
            }}
            className="px-3 py-2 rounded-lg border border-stone-300 dark:border-stone-600 bg-white dark:bg-stone-800 text-stone-900 dark:text-stone-100 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
          >
            <option value="">All kinds</option>
            {KIND_OPTIONS.slice(1).map((k) => (
              <option key={k} value={k}>
                {k}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Error */}
      {error && <div className="text-red-500 text-sm">Error: {error}</div>}

      {/* Loading */}
      {loading && <div className="text-stone-500 text-sm">Searching...</div>}

      {/* Results */}
      {!loading && (
        <>
          {memories.length === 0 ? (
            <div className="text-center py-12 text-stone-400 dark:text-stone-500">
              No memories found
            </div>
          ) : (
            <div className="space-y-3">
              {memories.map((m) => (
                <MemoryCard key={m.id} memory={m} />
              ))}
            </div>
          )}

          {/* Pagination */}
          {memories.length > 0 && (
            <div className="flex items-center justify-center gap-3 pt-4">
              <button
                onClick={() => setPage((p) => Math.max(1, p - 1))}
                disabled={page === 1}
                className="p-2 rounded-lg border border-stone-300 dark:border-stone-600 text-stone-600 dark:text-stone-300 disabled:opacity-40 hover:bg-stone-100 dark:hover:bg-stone-700 transition-colors"
              >
                <ChevronLeft size={16} />
              </button>
              <span className="text-sm text-stone-500 dark:text-stone-400">
                Page {page}
              </span>
              <button
                onClick={() => setPage((p) => p + 1)}
                disabled={memories.length < PAGE_SIZE}
                className="p-2 rounded-lg border border-stone-300 dark:border-stone-600 text-stone-600 dark:text-stone-300 disabled:opacity-40 hover:bg-stone-100 dark:hover:bg-stone-700 transition-colors"
              >
                <ChevronRight size={16} />
              </button>
            </div>
          )}
        </>
      )}
    </div>
  );
}

function MemoryCard({ memory }: { memory: MemoryEntry }) {
  const [expanded, setExpanded] = useState(false);

  const kindColor: Record<string, string> = {
    note: "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300",
    fact: "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-300",
    preference: "bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-300",
    context: "bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-300",
    task: "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-300",
    agent: "bg-stone-100 text-stone-700 dark:bg-stone-700 dark:text-stone-300",
  };

  return (
    <div className="bg-white dark:bg-stone-800 rounded-xl border border-stone-200 dark:border-stone-700 p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-2 flex-wrap">
            <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${kindColor[memory.kind] ?? "bg-stone-100 dark:bg-stone-700 text-stone-600 dark:text-stone-300"}`}>
              {memory.kind}
            </span>
            {memory.source && (
              <span className="text-xs text-stone-400 dark:text-stone-500">{memory.source}</span>
            )}
            <span className="text-xs text-stone-400 dark:text-stone-500 ml-auto">
              {new Date(memory.created_at).toLocaleDateString()}
            </span>
          </div>
          <p
            className={`text-sm text-stone-700 dark:text-stone-200 ${!expanded && "line-clamp-2"}`}
          >
            {memory.content}
          </p>
          {memory.content.length > 200 && (
            <button
              onClick={() => setExpanded(!expanded)}
              className="text-xs text-primary-600 dark:text-primary-400 mt-1 hover:underline"
            >
              {expanded ? "Show less" : "Show more"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
