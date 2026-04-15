import { designTokens } from "../theme";

export interface Question {
  id: string;
  title: string;
  content: string;
  priority: "urgent" | "this_week" | "backlog";
  project: string;
  category: "negocio" | "tecnico" | "ventas" | "legal" | "producto";
  status: "open" | "answered" | "dismissed";
  answer?: string;
  created_at: string;
  updated_at: string;
}

interface QuestionCardProps {
  question: Question;
  onEdit?: (question: Question) => void;
  onDelete?: (id: string) => void;
  onStatusChange?: (id: string, status: Question["status"]) => void;
}

const priorityConfig = {
  urgent: { label: "URGENTE", color: "#ff6b6b", bg: "rgba(255,107,107,0.15)" },
  this_week: {
    label: "ESTA SEMANA",
    color: "#d8a800",
    bg: "rgba(242,242,48,0.22)",
  },
  backlog: { label: "BACKLOG", color: "#19c37d", bg: "rgba(25,195,125,0.15)" },
};

const statusConfig = {
  open: { label: "Abierta", color: designTokens.color.info },
  answered: { label: "Respondida", color: designTokens.color.success },
  dismissed: { label: "Descartada", color: designTokens.color.muted },
};

const categoryConfig = {
  negocio: "NEGOCIO",
  tecnico: "TECNICO",
  ventas: "VENTAS",
  legal: "LEGAL",
  producto: "PRODUCTO",
};

export function QuestionCard(
  props: QuestionCardProps | { question: string | Question },
) {
  const { onEdit, onDelete, onStatusChange } = props as QuestionCardProps;
  let question = (props as QuestionCardProps).question;

  if (typeof question === "string") {
    try {
      question = JSON.parse(question) as Question;
    } catch {
      return <div>Error parsing question data</div>;
    }
  }

  const priority = priorityConfig[question.priority];
  const status = statusConfig[question.status];

  return (
    <div
      className="question-card-shell"
      style={{ borderLeft: `6px solid ${priority.color}` }}
    >
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-start",
          gap: "12px",
        }}
      >
        <div style={{ flex: 1 }}>
          <div className="card-badges" style={{ marginBottom: "8px" }}>
            <span
              className="cx-badge"
              style={{ background: priority.bg, color: priority.color }}
            >
              {priority.label}
            </span>
            <span
              className="cx-badge"
              style={{ background: "#dbeafe", color: designTokens.color.info }}
            >
              {categoryConfig[question.category]}
            </span>
            <span
              className="cx-badge"
              style={{ background: `${status.color}20`, color: status.color }}
            >
              {status.label}
            </span>
          </div>

          <div className="cx-card-title">{question.title}</div>
          <div className="cx-card-copy">{question.content}</div>

          <div className="cx-meta-row">
            <span>PROJECT {question.project}</span>
            <span>ID {question.id}</span>
            <span>
              {new Date(question.created_at).toLocaleDateString("es-ES")}
            </span>
          </div>

          {question.answer && (
            <div
              className="cx-answer-box"
              style={{ background: "rgba(25,195,125,0.12)" }}
            >
              <div
                style={{
                  fontWeight: 800,
                  color: designTokens.color.success,
                  fontSize: "0.85rem",
                  marginBottom: "4px",
                }}
              >
                RESPUESTA
              </div>
              <div className="cx-card-copy">{question.answer}</div>
            </div>
          )}
        </div>

        <div
          className="cx-card-actions"
          style={{ flexDirection: "column", minWidth: "118px" }}
        >
          {onStatusChange && (
            <>
              {question.status === "open" && (
                <button
                  type="button"
                  className="cx-button cx-button-primary"
                  onClick={() => onStatusChange(question.id, "answered")}
                >
                  Responder
                </button>
              )}
              {question.status === "answered" && (
                <button
                  type="button"
                  className="cx-button cx-button-secondary"
                  onClick={() => onStatusChange(question.id, "open")}
                >
                  Reabrir
                </button>
              )}
              <button
                type="button"
                className="cx-button cx-button-muted"
                onClick={() => onStatusChange(question.id, "dismissed")}
              >
                Descartar
              </button>
            </>
          )}
          {onEdit && (
            <button
              type="button"
              className="cx-button cx-button-secondary"
              onClick={() => onEdit(question)}
            >
              Editar
            </button>
          )}
          {onDelete && (
            <button
              type="button"
              className="cx-button cx-button-danger"
              onClick={() => onDelete(question.id)}
            >
              Eliminar
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

export default QuestionCard;
