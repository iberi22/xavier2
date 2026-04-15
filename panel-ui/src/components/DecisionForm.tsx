// DecisionForm.tsx - Form for creating/editing ADRs
import { useState } from "react";
import { Button, Input, TextArea, Tag as Badge } from "@openuidev/react-ui";
import type { ADR, ADRStatus, ADRPriority } from "./DecisionCard";

interface DecisionFormProps {
  initial?: Partial<ADR>;
  onSubmit: (adr: Omit<ADR, "id" | "created_at"> & { id?: string }) => void;
  onCancel: () => void;
  mode: "create" | "edit";
}

export function DecisionForm({ initial, onSubmit, onCancel, mode }: DecisionFormProps) {
  const [title, setTitle] = useState(initial?.title ?? "");
  const [context, setContext] = useState(initial?.context ?? "");
  const [decision, setDecision] = useState(initial?.decision ?? "");
  const [pros, setPros] = useState(initial?.consequences_positive?.join("\n") ?? "");
  const [cons, setCons] = useState(initial?.consequences_negative?.join("\n") ?? "");
  const [status, setStatus] = useState<ADRStatus>(initial?.status ?? "proposed");
  const [priority, setPriority] = useState<ADRPriority>(initial?.priority ?? "medium");
  const [project, setProject] = useState(initial?.project ?? "");
  const [blockers, setBlockers] = useState(initial?.blockers?.join("\n") ?? "");
  const [deadline, setDeadline] = useState(initial?.deadline ?? "");

  const handleSubmit = () => {
    if (!title.trim() || !context.trim()) return;

    const adr: Omit<ADR, "id" | "created_at"> & { id?: string } = {
      title: title.trim(),
      context: context.trim(),
      decision: decision.trim(),
      consequences_positive: pros.split("\n").map((p) => p.trim()).filter(Boolean),
      consequences_negative: cons.split("\n").map((c) => c.trim()).filter(Boolean),
      status,
      priority,
      project: project.trim() || undefined,
      blockers: blockers.split("\n").map((b) => b.trim()).filter(Boolean),
      deadline: deadline.trim() || undefined,
    };

    if (mode === "edit" && initial?.id) {
      adr.id = initial.id;
    }

    onSubmit(adr);
  };

  return (
    <div className="decision-form">
      <h2>{mode === "create" ? "New ADR" : "Edit ADR"}</h2>

      <div className="form-group">
        <label>Title *</label>
        <Input
          value={title}
          onChange={(v) => setTitle(v)}
          placeholder="e.g., ADR-001: Use Rust for Synapse runtime"
        />
      </div>

      <div className="form-row">
        <div className="form-group">
          <label>Priority</label>
          <div className="button-group">
            {(["high", "medium", "low"] as ADRPriority[]).map((p) => (
              <button
                key={p}
                type="button"
                className={`toggle-btn ${priority === p ? "active" : ""}`}
                onClick={() => setPriority(p)}
              >
                {p.toUpperCase()}
              </button>
            ))}
          </div>
        </div>

        <div className="form-group">
          <label>Status</label>
          <div className="button-group">
            {(["proposed", "accepted", "deprecated"] as ADRStatus[]).map((s) => (
              <button
                key={s}
                type="button"
                className={`toggle-btn ${status === s ? "active" : ""}`}
                onClick={() => setStatus(s)}
              >
                {s.toUpperCase()}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="form-group">
        <label>Project</label>
        <Input
          value={project}
          onChange={(v) => setProject(v)}
          placeholder="e.g., Synapse, ManteniApp"
        />
      </div>

      <div className="form-group">
        <label>Context *</label>
        <TextArea
          value={context}
          onChange={(v) => setContext(v)}
          placeholder="Describe the problem or situation..."
          rows={4}
        />
      </div>

      <div className="form-group">
        <label>Decision</label>
        <TextArea
          value={decision}
          onChange={(v) => setDecision(v)}
          placeholder="What was decided?"
          rows={3}
        />
      </div>

      <div className="form-row">
        <div className="form-group">
          <label>Positive Consequences (one per line)</label>
          <TextArea
            value={pros}
            onChange={(v) => setPros(v)}
            placeholder={"Better performance\nLower costs"}
            rows={3}
          />
        </div>

        <div className="form-group">
          <label>Negative Consequences (one per line)</label>
          <TextArea
            value={cons}
            onChange={(v) => setCons(v)}
            placeholder={"Higher initial investment\nSteeper learning curve"}
            rows={3}
          />
        </div>
      </div>

      <div className="form-group">
        <label>Blockers (one per line)</label>
        <TextArea
          value={blockers}
          onChange={(v) => setBlockers(v)}
          placeholder={"Awaiting hardware partner confirmation"}
          rows={2}
        />
      </div>

      <div className="form-group">
        <label>Deadline</label>
        <Input
          value={deadline}
          onChange={(v) => setDeadline(v)}
          placeholder="YYYY-MM-DD"
        />
      </div>

      <div className="form-actions">
        <Button onClick={handleSubmit}>
          {mode === "create" ? "Create ADR" : "Save Changes"}
        </Button>
        <Button variant="secondary" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}
