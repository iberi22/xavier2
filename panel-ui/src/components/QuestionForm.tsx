import { useState } from "react";
import { Button, Card, Input, TextArea, Select } from "@openuidev/react-ui";
import type { Question } from "./QuestionCard";

interface QuestionFormProps {
  initialData?: Partial<Question>;
  onSubmit: (data: Omit<Question, "id" | "created_at" | "updated_at">) => void;
  onCancel?: () => void;
  isLoading?: boolean;
}

const defaultData = {
  title: "",
  content: "",
  priority: "this_week" as Question["priority"],
  project: "",
  category: "negocio" as Question["category"],
  status: "open" as Question["status"],
};

export function QuestionForm({ initialData, onSubmit, onCancel, isLoading }: QuestionFormProps) {
  const [formData, setFormData] = useState({ ...defaultData, ...initialData });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit(formData);
  };

  const handleChange = (field: keyof typeof formData, value: string) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
  };

  return (
    <Card style={{ background: "#fffdf8", padding: "24px" }}>
      <form onSubmit={handleSubmit}>
        <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
          <div>
            <span style={{ fontWeight: 600, color: "#111827", display: "block", marginBottom: "6px" }}>
              Título *
            </span>
            <Input
              value={formData.title}
              onChange={(e) => handleChange("title", e.target.value)}
              placeholder="¿Cuál es la pregunta?"
              required
              style={{ width: "100%" }}
            />
          </div>

          <div>
            <span style={{ fontWeight: 600, color: "#111827", display: "block", marginBottom: "6px" }}>
              Descripción *
            </span>
            <TextArea
              value={formData.content}
              onChange={(e) => handleChange("content", e.target.value)}
              placeholder="Describe el contexto y los detalles de la pregunta..."
              required
              rows={4}
              style={{ width: "100%", resize: "vertical" }}
            />
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "16px" }}>
            <div>
              <span style={{ fontWeight: 600, color: "#111827", display: "block", marginBottom: "6px" }}>
                Prioridad
              </span>
              <Select
                value={formData.priority}
                onChange={(e) => handleChange("priority", e.target.value)}
                style={{ width: "100%" }}
              >
                <option value="urgent">🔴 Urgente</option>
                <option value="this_week">🟡 Esta Semana</option>
                <option value="backlog">🟢 Backlog</option>
              </Select>
            </div>

            <div>
              <span style={{ fontWeight: 600, color: "#111827", display: "block", marginBottom: "6px" }}>
                Categoría
              </span>
              <Select
                value={formData.category}
                onChange={(e) => handleChange("category", e.target.value)}
                style={{ width: "100%" }}
              >
                <option value="negocio">Negocio</option>
                <option value="tecnico">Técnico</option>
                <option value="ventas">Ventas</option>
                <option value="legal">Legal</option>
                <option value="producto">Producto</option>
              </Select>
            </div>

            <div>
              <span style={{ fontWeight: 600, color: "#111827", display: "block", marginBottom: "6px" }}>
                Proyecto
              </span>
              <Input
                value={formData.project}
                onChange={(e) => handleChange("project", e.target.value)}
                placeholder="Nombre del proyecto"
                style={{ width: "100%" }}
              />
            </div>
          </div>

          {initialData?.id && (
            <div>
              <span style={{ fontWeight: 600, color: "#111827", display: "block", marginBottom: "6px" }}>
                Respuesta
              </span>
              <TextArea
                value={formData.answer || ""}
                onChange={(e) => handleChange("answer", e.target.value)}
                placeholder="Agrega la respuesta a esta pregunta..."
                rows={3}
                style={{ width: "100%", resize: "vertical" }}
              />
            </div>
          )}

          <div style={{ display: "flex", gap: "12px", justifyContent: "flex-end", marginTop: "8px" }}>
            {onCancel && (
              <Button type="button" variant="secondary" onClick={onCancel} disabled={isLoading}>
                Cancelar
              </Button>
            )}
            <Button type="submit" disabled={isLoading || !formData.title || !formData.content}>
              {isLoading ? "Guardando..." : initialData?.id ? "Actualizar" : "Crear Pregunta"}
            </Button>
          </div>
        </div>
      </form>
    </Card>
  );
}

export default QuestionForm;
