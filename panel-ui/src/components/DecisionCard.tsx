import { designTokens } from "../theme";

export type ADRStatus = "proposed" | "accepted" | "deprecated";
export type ADRPriority = "high" | "medium" | "low";

export interface ADR {
  id: string;
  title: string;
  context: string;
  decision: string;
  consequences_positive: string[];
  consequences_negative: string[];
  status: ADRStatus;
  priority: ADRPriority;
  project?: string;
  blockers?: string[];
  deadline?: string;
  created_at: string;
  decided_at?: string;
  decided_by?: string;
}

interface DecisionCardProps {
  adr: ADR;
  onAccept?: (id: string) => void;
  onDeprecate?: (id: string) => void;
  onEdit?: (id: string) => void;
  onView?: (id: string) => void;
}

const statusConfig = {
  proposed: { label: "PROPOSED", bg: "#fff3b0", color: "#7a5d00" },
  accepted: { label: "ACCEPTED", bg: "#bff2da", color: "#0c7b4f" },
  deprecated: { label: "DEPRECATED", bg: "#ffd1d1", color: "#8d2020" },
};

const priorityConfig = {
  high: { label: "HIGH", color: "#dc2626" },
  medium: { label: "MEDIUM", color: "#d8a800" },
  low: { label: "LOW", color: "#16a34a" },
};

export function DecisionCard(props: DecisionCardProps | { adr: string | ADR }) {
  const { onAccept, onDeprecate, onEdit, onView } = props as DecisionCardProps;
  let adr = (props as DecisionCardProps).adr;

  if (typeof adr === "string") {
    try {
      adr = JSON.parse(adr) as ADR;
    } catch {
      return <div>Error parsing decision data</div>;
    }
  }

  const status = statusConfig[adr.status];
  const priority = priorityConfig[adr.priority];
  const handleView = () => onView?.(adr.id);

  return (
    <div
      className="decision-card"
      style={{ borderLeft: `6px solid ${priority.color}` }}
    >
      <div className="decision-header">
        <div className="decision-badges">
          <span
            className="cx-badge"
            style={{ background: status.bg, color: status.color }}
          >
            {status.label}
          </span>
          <span
            className="cx-badge"
            style={{ background: `${priority.color}20`, color: priority.color }}
          >
            {priority.label}
          </span>
          {adr.project ? (
            <span className="cx-badge cx-badge-neutral">{adr.project}</span>
          ) : null}
        </div>
        <span className="decision-date">
          {new Date(adr.created_at).toLocaleDateString()}
        </span>
      </div>

      <button
        type="button"
        className="decision-title-button"
        onClick={handleView}
      >
        <span className="decision-title">
          {adr.id}: {adr.title}
        </span>
      </button>

      <p className="decision-context">{adr.context}</p>

      {adr.decision ? (
        <div className="decision-decision">
          <strong>Decision:</strong> {adr.decision}
        </div>
      ) : null}

      {adr.blockers && adr.blockers.length > 0 ? (
        <div className="decision-blockers">
          <strong>Blockers:</strong>
          <ul>
            {adr.blockers.map((b, i) => (
              <li key={`${adr.id}-blocker-${b}-${i}`}>{b}</li>
            ))}
          </ul>
        </div>
      ) : null}

      {adr.consequences_positive.length > 0 ? (
        <div
          className="decision-pros"
          style={{ background: "rgba(25,195,125,0.12)" }}
        >
          <strong style={{ color: designTokens.color.success }}>
            Positives
          </strong>
          <ul>
            {adr.consequences_positive.map((p, i) => (
              <li key={`${adr.id}-pro-${p}-${i}`}>{p}</li>
            ))}
          </ul>
        </div>
      ) : null}

      {adr.consequences_negative.length > 0 ? (
        <div
          className="decision-cons"
          style={{ background: "rgba(255,92,92,0.12)" }}
        >
          <strong style={{ color: designTokens.color.danger }}>
            Negatives
          </strong>
          <ul>
            {adr.consequences_negative.map((n, i) => (
              <li key={`${adr.id}-con-${n}-${i}`}>{n}</li>
            ))}
          </ul>
        </div>
      ) : null}

      <div className="decision-actions">
        {adr.status === "proposed" ? (
          <>
            <button
              type="button"
              className="cx-button cx-button-primary"
              onClick={() => onAccept?.(adr.id)}
            >
              Accept
            </button>
            <button
              type="button"
              className="cx-button cx-button-danger"
              onClick={() => onDeprecate?.(adr.id)}
            >
              Deprecate
            </button>
          </>
        ) : null}
        <button
          type="button"
          className="cx-button cx-button-secondary"
          onClick={() => onEdit?.(adr.id)}
        >
          Edit
        </button>
        <button
          type="button"
          className="cx-button cx-button-muted"
          onClick={() => onView?.(adr.id)}
        >
          View Full
        </button>
      </div>
    </div>
  );
}
