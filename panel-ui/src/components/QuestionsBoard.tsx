import { useState, useEffect, useMemo } from "react";
import { Button, Select, Tag as Badge } from "@openuidev/react-ui";
import { QuestionCard, type Question } from "./QuestionCard";
import { QuestionForm } from "./QuestionForm";

interface QuestionsBoardProps {
  token: string;
  xavierUrl?: string;
}

type FilterState = {
  priority: Question["priority"] | "all";
  status: Question["status"] | "all";
  category: Question["category"] | "all";
  project: string;
};

const defaultFilters: FilterState = {
  priority: "all",
  status: "open",
  category: "all",
  project: "",
};

export function QuestionsBoard({ token, xavierUrl = "http://localhost:8003" }: QuestionsBoardProps) {
  const [questions, setQuestions] = useState<Question[]>([]);
  const [filters, setFilters] = useState<FilterState>(defaultFilters);
  const [showForm, setShowForm] = useState(false);
  const [editingQuestion, setEditingQuestion] = useState<Question | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void loadQuestions();
  }, []);

  async function loadQuestions() {
    try {
      setIsLoading(true);
      setError(null);

      const response = await fetch(`${xavierUrl}/memory/search`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Xavier-Token": token,
        },
        body: JSON.stringify({
          query: "sweat-operations questions",
          limit: 100,
        }),
      });

      if (!response.ok) {
        throw new Error("Failed to load questions from Xavier");
      }

      const data = await response.json() as { results?: Array<{ memory?: Question }> };
      const loadedQuestions: Question[] = [];

      if (data.results) {
        for (const result of data.results) {
          if (result.memory) {
            try {
              // Handle case where memory.content is a JSON string
              const memory = typeof result.memory === 'string'
                ? JSON.parse(result.memory)
                : result.memory;

              if (memory.id && memory.title) {
                loadedQuestions.push(memory);
              }
            } catch (e) {
              // Skip invalid JSON
            }
          }
        }
      }

      setQuestions(loadedQuestions);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load questions");
    } finally {
      setIsLoading(false);
    }
  }

  async function saveQuestion(data: Omit<Question, "id" | "created_at" | "updated_at">) {
    try {
      setIsLoading(true);
      setError(null);

      const now = new Date().toISOString();
      const question: Question = {
        ...data,
        id: editingQuestion?.id || `q-${Date.now()}`,
        created_at: editingQuestion?.created_at || now,
        updated_at: now,
      };

      const response = await fetch(`${xavierUrl}/memory/add`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Xavier-Token": token,
        },
        body: JSON.stringify({
          path: `sweat-operations/questions/${question.id}`,
          content: JSON.stringify(question),
          metadata: {
            type: "question",
            priority: question.priority,
            status: question.status,
            category: question.category,
            project: question.project,
          },
        }),
      });

      if (!response.ok) {
        throw new Error("Failed to save question to Xavier");
      }

      await loadQuestions();
      setShowForm(false);
      setEditingQuestion(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save question");
    } finally {
      setIsLoading(false);
    }
  }

  async function deleteQuestion(id: string) {
    try {
      setIsLoading(true);
      const response = await fetch(`${xavierUrl}/memory/delete`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Xavier-Token": token,
        },
        body: JSON.stringify({
          path: `sweat-operations/questions/${id}`,
        }),
      });

      if (!response.ok) {
        throw new Error("Failed to delete question");
      }

      await loadQuestions();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to delete question");
    } finally {
      setIsLoading(false);
    }
  }

  async function updateStatus(id: string, status: Question["status"]) {
    const question = questions.find((q) => q.id === id);
    if (question) {
      await saveQuestion({ ...question, status, answer: question.answer });
    }
  }

  const filteredQuestions = useMemo(() => {
    return questions.filter((q) => {
      if (filters.priority !== "all" && q.priority !== filters.priority) return false;
      if (filters.status !== "all" && q.status !== filters.status) return false;
      if (filters.category !== "all" && q.category !== filters.category) return false;
      if (filters.project && !q.project.toLowerCase().includes(filters.project.toLowerCase())) return false;
      return true;
    });
  }, [questions, filters]);

  const stats = useMemo(() => ({
    total: questions.length,
    urgent: questions.filter((q) => q.priority === "urgent").length,
    thisWeek: questions.filter((q) => q.priority === "this_week").length,
    backlog: questions.filter((q) => q.priority === "backlog").length,
    answered: questions.filter((q) => q.status === "answered").length,
  }), [questions]);

  const handleEdit = (question: Question) => {
    setEditingQuestion(question);
    setShowForm(true);
  };

  const handleCancelForm = () => {
    setShowForm(false);
    setEditingQuestion(null);
  };

  return (
    <div style={{ padding: "20px" }}>
      {/* Header */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "24px" }}>
        <div>
          <h2 style={{ fontSize: "1.5rem", fontWeight: 700, color: "#111827", margin: 0 }}>Questions Board</h2>
          <p style={{ color: "#6b7280", fontSize: "0.9rem", margin: 0 }}>
            {stats.total} preguntas | {stats.urgent} urgentes | {stats.answered} respondidas
          </p>
        </div>
        <Button onClick={() => setShowForm(true)} disabled={showForm}>
          + Nueva Pregunta
        </Button>
      </div>

      {/* Stats Cards */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: "12px", marginBottom: "24px" }}>
        <div style={{ background: "rgba(255,107,107,0.15)", padding: "16px", borderRadius: "12px", textAlign: "center" }}>
          <div style={{ fontSize: "2rem", fontWeight: 700, color: "#ff6b6b" }}>{stats.urgent}</div>
          <span style={{ fontSize: "0.8rem", color: "#6b7280" }}>Urgentes</span>
        </div>
        <div style={{ background: "rgba(255,217,61,0.15)", padding: "16px", borderRadius: "12px", textAlign: "center" }}>
          <div style={{ fontSize: "2rem", fontWeight: 700, color: "#ffd93d" }}>{stats.thisWeek}</div>
          <span style={{ fontSize: "0.8rem", color: "#6b7280" }}>Esta Semana</span>
        </div>
        <div style={{ background: "rgba(82,183,136,0.15)", padding: "16px", borderRadius: "12px", textAlign: "center" }}>
          <div style={{ fontSize: "2rem", fontWeight: 700, color: "#52b788" }}>{stats.backlog}</div>
          <span style={{ fontSize: "0.8rem", color: "#6b7280" }}>Backlog</span>
        </div>
        <div style={{ background: "rgba(0,212,255,0.15)", padding: "16px", borderRadius: "12px", textAlign: "center" }}>
          <div style={{ fontSize: "2rem", fontWeight: 700, color: "#00d4ff" }}>{stats.answered}</div>
          <span style={{ fontSize: "0.8rem", color: "#6b7280" }}>Respondidas</span>
        </div>
      </div>

      {/* Filters */}
      <div style={{ display: "flex", gap: "12px", marginBottom: "20px", flexWrap: "wrap" }}>
        <Select
          value={filters.priority}
          onChange={(e) => setFilters((f) => ({ ...f, priority: e.target.value as FilterState["priority"] }))}
        >
          <option value="all">Todas Prioridades</option>
          <option value="urgent">🔴 Urgente</option>
          <option value="this_week">🟡 Esta Semana</option>
          <option value="backlog">🟢 Backlog</option>
        </Select>

        <Select
          value={filters.status}
          onChange={(e) => setFilters((f) => ({ ...f, status: e.target.value as FilterState["status"] }))}
        >
          <option value="all">Todos los Estados</option>
          <option value="open">Abiertas</option>
          <option value="answered">Respondidas</option>
          <option value="dismissed">Descartadas</option>
        </Select>

        <Select
          value={filters.category}
          onChange={(e) => setFilters((f) => ({ ...f, category: e.target.value as FilterState["category"] }))}
        >
          <option value="all">Todas Categorías</option>
          <option value="negocio">Negocio</option>
          <option value="tecnico">Técnico</option>
          <option value="ventas">Ventas</option>
          <option value="legal">Legal</option>
          <option value="producto">Producto</option>
        </Select>

        <input
          type="text"
          placeholder="Filtrar por proyecto..."
          value={filters.project}
          onChange={(e) => setFilters((f) => ({ ...f, project: e.target.value }))}
          style={{
            padding: "8px 12px",
            border: "1px solid #d6d0c4",
            borderRadius: "8px",
            fontSize: "0.9rem",
            minWidth: "200px",
          }}
        />
      </div>

      {/* Error Banner */}
      {error && (
        <div style={{
          background: "rgba(255,107,107,0.15)",
          color: "#ff6b6b",
          padding: "12px 16px",
          borderRadius: "8px",
          marginBottom: "16px",
        }}>
          {error}
        </div>
      )}

      {/* Form Modal */}
      {showForm && (
        <div style={{
          position: "fixed",
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          background: "rgba(0,0,0,0.5)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          zIndex: 1000,
          padding: "20px",
        }}>
          <div style={{ background: "#fff", borderRadius: "16px", maxWidth: "700px", width: "100%", maxHeight: "90vh", overflow: "auto" }}>
            <div style={{ padding: "20px", borderBottom: "1px solid #e5e7eb" }}>
              <h3 style={{ fontSize: "1.2rem", fontWeight: 600, margin: 0 }}>
                {editingQuestion ? "Editar Pregunta" : "Nueva Pregunta"}
              </h3>
            </div>
            <div style={{ padding: "20px" }}>
              <QuestionForm
                initialData={editingQuestion || undefined}
                onSubmit={saveQuestion}
                onCancel={handleCancelForm}
                isLoading={isLoading}
              />
            </div>
          </div>
        </div>
      )}

      {/* Questions List */}
      {isLoading && questions.length === 0 ? (
        <div style={{ textAlign: "center", padding: "40px", color: "#6b7280" }}>
          Cargando preguntas...
        </div>
      ) : filteredQuestions.length === 0 ? (
        <div style={{ textAlign: "center", padding: "40px", color: "#6b7280" }}>
          {questions.length === 0
            ? "No hay preguntas aún. ¡Crea la primera!"
            : "No hay preguntas que coincidan con los filtros."}
        </div>
      ) : (
        <div>
          {filteredQuestions.map((question) => (
            <QuestionCard
              key={question.id}
              question={question}
              onEdit={handleEdit}
              onDelete={deleteQuestion}
              onStatusChange={updateStatus}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export default QuestionsBoard;
