// DecisionBoard.tsx - List of decisions with filtering
import { useState } from "react";
import { Button, Input, Tag as Badge } from "@openuidev/react-ui";
import { DecisionCard } from "./DecisionCard";
import { DecisionForm } from "./DecisionForm";
import type { ADR, ADRStatus, ADRPriority } from "./DecisionCard";

interface DecisionBoardProps {
  decisions: ADR[];
  token: string;
  onRefresh: () => void;
}

type FilterStatus = ADRStatus | "all";

export function DecisionBoard({ decisions, token, onRefresh }: DecisionBoardProps) {
  const [filter, setFilter] = useState<FilterStatus>("all");
  const [priorityFilter, setPriorityFilter] = useState<ADRPriority | "all">("all");
  const [search, setSearch] = useState("");
  const [showForm, setShowForm] = useState(false);
  const [editingAdr, setEditingAdr] = useState<ADR | null>(null);
  const [selectedAdr, setSelectedAdr] = useState<ADR | null>(null);

  const filtered = decisions.filter((adr) => {
    if (filter !== "all" && adr.status !== filter) return false;
    if (priorityFilter !== "all" && adr.priority !== priorityFilter) return false;
    if (search && !adr.title.toLowerCase().includes(search.toLowerCase()) &&
        !adr.context.toLowerCase().includes(search.toLowerCase())) return false;
    return true;
  });

  const proposed = decisions.filter((d) => d.status === "proposed");
  const accepted = decisions.filter((d) => d.status === "accepted");
  const deprecated = decisions.filter((d) => d.status === "deprecated");

  const handleStatusChange = async (id: string, status: ADRStatus) => {
    const adr = decisions.find((d) => d.id === id);
    if (!adr) return;

    try {
      await fetch(`/memory/sweat-operations/decisions/${id}`, {
        method: "PUT",
        headers: {
          "Content-Type": "application/json",
          "X-Xavier-Token": token,
        },
        body: JSON.stringify({ ...adr, status }),
      });
      onRefresh();
    } catch (err) {
      console.error("Failed to update status:", err);
    }
  };

  const handleCreate = async (adrData: Omit<ADR, "id" | "created_at">) => {
    const id = `D-${String(decisions.length + 1).padStart(3, "0")}`;
    const newAdr: ADR = {
      ...adrData,
      id,
      created_at: new Date().toISOString(),
    } as ADR;

    try {
      await fetch(`/memory/sweat-operations/decisions/${id}`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Xavier-Token": token,
        },
        body: JSON.stringify(newAdr),
      });
      setShowForm(false);
      onRefresh();
    } catch (err) {
      console.error("Failed to create ADR:", err);
    }
  };

  const handleEdit = async (adrData: Omit<ADR, "id" | "created_at"> & { id?: string }) => {
    if (!adrData.id) return;
    const existing = decisions.find((d) => d.id === adrData.id);
    if (!existing) return;

    try {
      await fetch(`/memory/sweat-operations/decisions/${adrData.id}`, {
        method: "PUT",
        headers: {
          "Content-Type": "application/json",
          "X-Xavier-Token": token,
        },
        body: JSON.stringify({ ...existing, ...adrData }),
      });
      setEditingAdr(null);
      onRefresh();
    } catch (err) {
      console.error("Failed to update ADR:", err);
    }
  };

  if (showForm) {
    return (
      <DecisionForm
        mode="create"
        onSubmit={handleCreate}
        onCancel={() => setShowForm(false)}
      />
    );
  }

  if (editingAdr) {
    return (
      <DecisionForm
        mode="edit"
        initial={editingAdr}
        onSubmit={handleEdit}
        onCancel={() => setEditingAdr(null)}
      />
    );
  }

  return (
    <div className="decision-board">
      <div className="board-header">
        <div className="board-title">
          <h2>Decisions Board</h2>
          <div className="decision-stats">
            <Badge style={{ background: "#fef3c7", color: "#92400e" }}>
              {proposed.length} Proposed
            </Badge>
            <Badge style={{ background: "#d1fae5", color: "#065f46" }}>
              {accepted.length} Accepted
            </Badge>
            <Badge style={{ background: "#fee2e2", color: "#991b1b" }}>
              {deprecated.length} Deprecated
            </Badge>
          </div>
        </div>
        <Button onClick={() => setShowForm(true)}>+ New ADR</Button>
      </div>

      <div className="board-filters">
        <Input
          value={search}
          onChange={(v) => setSearch(v)}
          placeholder="Search decisions..."
        />
        <div className="filter-group">
          <span>Status:</span>
          {(["all", "proposed", "accepted", "deprecated"] as FilterStatus[]).map((s) => (
            <button
              key={s}
              className={`filter-btn ${filter === s ? "active" : ""}`}
              onClick={() => setFilter(s)}
            >
              {s === "all" ? "All" : s.charAt(0).toUpperCase() + s.slice(1)}
            </button>
          ))}
        </div>
        <div className="filter-group">
          <span>Priority:</span>
          {(["all", "high", "medium", "low"] as (ADRPriority | "all")[]).map((p) => (
            <button
              key={p}
              className={`filter-btn ${priorityFilter === p ? "active" : ""}`}
              onClick={() => setPriorityFilter(p)}
            >
              {p === "all" ? "All" : p.charAt(0).toUpperCase() + p.slice(1)}
            </button>
          ))}
        </div>
      </div>

      {selectedAdr && (
        <div className="decision-detail-overlay" onClick={() => setSelectedAdr(null)}>
          <div className="decision-detail" onClick={(e) => e.stopPropagation()}>
            <DecisionCard
              adr={selectedAdr}
              onEdit={(id) => {
                setSelectedAdr(null);
                setEditingAdr(decisions.find((d) => d.id === id) ?? null);
              }}
              onAccept={(id) => handleStatusChange(id, "accepted")}
              onDeprecate={(id) => handleStatusChange(id, "deprecated")}
            />
          </div>
        </div>
      )}

      <div className="decision-list">
        {filtered.length === 0 ? (
          <div className="empty-state">No decisions match your filters</div>
        ) : (
          filtered.map((adr) => (
            <DecisionCard
              key={adr.id}
              adr={adr}
              onAccept={(id) => handleStatusChange(id, "accepted")}
              onDeprecate={(id) => handleStatusChange(id, "deprecated")}
              onEdit={(id) => setEditingAdr(decisions.find((d) => d.id === id) ?? null)}
              onView={(id) => setSelectedAdr(decisions.find((d) => d.id === id) ?? null)}
            />
          ))
        )}
      </div>
    </div>
  );
}
