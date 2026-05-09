import { Renderer } from "@openuidev/react-lang";
import { openuiLibrary, ThemeProvider } from "@openuidev/react-ui";
import type { CSSProperties } from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { DecisionCard } from "./components/DecisionCard";
import { ProjectCard } from "./components/ProjectCard";
import { QuestionCard } from "./components/QuestionCard";
import { designTokens, theme } from "./theme";

const combinedLibrary = {
  ...openuiLibrary,
  QuestionCard,
  DecisionCard,
  ProjectCard,
};

type ThreadSummary = {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  last_preview: string;
  message_count: number;
};

type PanelMessage = {
  id: string;
  role: string;
  plain_text: string;
  openui_lang?: string | null;
  created_at: string;
  metadata?: {
    confidence?: number;
    timings?: {
      system1_ms: number;
      system2_ms: number;
      system3_ms: number;
      total_ms: number;
    };
    components?: string[];
    rules?: string[];
    documents?: number;
    evidence?: number;
  };
};

type ThreadDetail = {
  thread: ThreadSummary;
  messages: PanelMessage[];
};

type PanelChatResponse = {
  thread: ThreadSummary;
  messages: PanelMessage[];
};

function App() {
  const [token, setToken] = useState("");
  const [draftToken, setDraftToken] = useState("");
  const [threads, setThreads] = useState<ThreadSummary[]>([]);
  const [selectedThreadId, setSelectedThreadId] = useState<string | null>(null);
  const [messages, setMessages] = useState<PanelMessage[]>([]);
  const [draft, setDraft] = useState("");
  const [health, setHealth] = useState("checking");
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const activeThread = useMemo(
    () => threads.find((item) => item.id === selectedThreadId) ?? null,
    [selectedThreadId, threads],
  );

  useEffect(() => {
    fetch("/health")
      .then((response) => response.json())
      .then((data) => setHealth(data.status ?? "unknown"))
      .catch(() => setHealth("offline"));
  }, []);

  const openThread = useCallback(
    async (threadId: string, currentToken = token) => {
      if (!currentToken) {
        return;
      }

      try {
        setError(null);
        const response = await fetch(`/panel/api/threads/${threadId}`, {
          headers: { "X-Xavier-Token": currentToken },
        });
        if (!response.ok) {
          throw new Error("Failed to load thread");
        }
        const detail = (await response.json()) as ThreadDetail;
        setSelectedThreadId(threadId);
        setMessages(detail.messages);
      } catch (cause) {
        setError(
          cause instanceof Error ? cause.message : "Failed to open thread",
        );
      }
    },
    [token],
  );

  const loadThreads = useCallback(
    async (currentToken: string) => {
      try {
        setError(null);
        const response = await fetch("/panel/api/threads", {
          headers: { "X-Xavier-Token": currentToken },
        });
        if (!response.ok) {
          throw new Error("Token rejected by Xavier");
        }
        const data = (await response.json()) as ThreadSummary[];
        setThreads(data);
        if (!selectedThreadId && data[0]) {
          void openThread(data[0].id, currentToken);
        }
      } catch (cause) {
        setError(
          cause instanceof Error ? cause.message : "Failed to load threads",
        );
      }
    },
    [openThread, selectedThreadId],
  );

  useEffect(() => {
    if (!token) {
      return;
    }
    void loadThreads(token);
  }, [token, loadThreads]);

  useEffect(() => {
    if (!token || !selectedThreadId || isLoading) {
      return;
    }

    const interval = setInterval(() => {
      if (document.visibilityState === "visible") {
        void openThread(selectedThreadId, token);
      }
    }, 10000);

    return () => clearInterval(interval);
  }, [token, selectedThreadId, isLoading, openThread]);

  async function api<T>(path: string, options?: RequestInit): Promise<T> {
    const response = await fetch(path, {
      ...options,
      headers: {
        "Content-Type": "application/json",
        "X-Xavier-Token": token,
        ...(options?.headers ?? {}),
      },
    });

    if (!response.ok) {
      throw new Error(await response.text());
    }

    return response.json() as Promise<T>;
  }

  async function createThread() {
    try {
      const thread = await api<ThreadSummary>("/panel/api/threads", {
        method: "POST",
        body: JSON.stringify({ title: "New Thread" }),
      });
      setThreads((current) => [thread, ...current]);
      setSelectedThreadId(thread.id);
      setMessages([]);
    } catch (cause) {
      setError(
        cause instanceof Error ? cause.message : "Failed to create thread",
      );
    }
  }

  async function sendMessage() {
    if (!draft.trim()) {
      return;
    }

    try {
      setIsLoading(true);
      setError(null);
      const payload = await api<PanelChatResponse>("/panel/api/chat", {
        method: "POST",
        body: JSON.stringify({
          thread_id: selectedThreadId,
          message: draft,
        }),
      });

      setDraft("");
      setSelectedThreadId(payload.thread.id);
      setMessages(payload.messages);
      setThreads((current) => {
        const next = [
          payload.thread,
          ...current.filter((item) => item.id !== payload.thread.id),
        ];
        return next;
      });
    } catch (cause) {
      setError(
        cause instanceof Error ? cause.message : "Failed to send message",
      );
    } finally {
      setIsLoading(false);
    }
  }

  if (!token) {
    return (
      <ThemeProvider mode="light" lightTheme={theme}>
        <div className="xavier-app token-screen">
          <div className="token-card">
            <p className="eyebrow">Xavier Internal Panel</p>
            <h1>OpenUI cockpit for the internal agent</h1>
            <p className="lede">
              Paste the Xavier token. The shell stays public, but every panel
              API call remains protected by <code>X-Xavier-Token</code>.
            </p>
            <div className="token-meta">
              <div className="instrument-card">
                <span className="instrument-label">Mode</span>
                <strong>Tech Brutal</strong>
              </div>
              <div className="instrument-card instrument-card-acid">
                <span className="instrument-label">Accent</span>
                <strong>Acid Yellow</strong>
              </div>
            </div>
            <textarea
              className="cx-textarea token-input"
              value={draftToken}
              onChange={(event) => setDraftToken(event.target.value)}
              placeholder="XAVIER_TOKEN"
              rows={4}
            />
            <button
              type="button"
              className="cx-button cx-button-primary"
              onClick={() => {
                setToken(draftToken.trim());
                setError(null);
              }}
            >
              Enter panel
            </button>
            {error ? <div className="error-banner">{error}</div> : null}
          </div>
        </div>
      </ThemeProvider>
    );
  }

  return (
    <ThemeProvider mode="light" lightTheme={theme}>
      <div
        className="xavier-app shell"
        style={
          {
            "--cx-bg": designTokens.color.bg,
            "--cx-surface": designTokens.color.surface,
            "--cx-surface-2": designTokens.color.surface2,
            "--cx-ink": designTokens.color.text,
            "--cx-border": designTokens.color.border,
            "--cx-acid": designTokens.color.accent,
            "--cx-acid-strong": designTokens.color.accentStrong,
            "--cx-danger": designTokens.color.danger,
            "--cx-info": designTokens.color.info,
            "--cx-success": designTokens.color.success,
            "--cx-shadow-card": designTokens.shadow.card,
            "--cx-shadow-button": designTokens.shadow.button,
            "--cx-shadow-focus": designTokens.shadow.focus,
            "--cx-radius-sm": designTokens.radius.sm,
            "--cx-radius-md": designTokens.radius.md,
            "--cx-motion-hover": designTokens.motion.hover,
            "--cx-motion-press": designTokens.motion.press,
            "--cx-font-ui": designTokens.font.ui,
            "--cx-font-mono": designTokens.font.mono,
          } as CSSProperties
        }
      >
        <aside className="sidebar">
          <div className="brand">
            <div>
              <p className="eyebrow">Xavier</p>
              <h2>Render Agent Console</h2>
            </div>
            <button
              type="button"
              className="cx-button cx-button-secondary"
              onClick={() => void createThread()}
            >
              New thread
            </button>
          </div>

          <div className="system-card">
            <div className="system-card-header">
              <span className={`health-pill health-${health}`}>{health}</span>
              <span className="system-hint">Live backend</span>
            </div>
            <p>
              Reasoning agent + render agent are split and persisted per thread.
            </p>
          </div>

          <div className="thread-list">
            {threads.map((thread) => (
              <button
                type="button"
                key={thread.id}
                className={`thread-item ${thread.id === selectedThreadId ? "thread-item-active" : ""}`}
                onClick={() => void openThread(thread.id)}
              >
                <span className="thread-item-label">Thread</span>
                <strong>{thread.title}</strong>
                <span>{thread.last_preview || "No messages yet"}</span>
                <span className="thread-item-meta">
                  {thread.message_count} messages
                </span>
              </button>
            ))}
          </div>
        </aside>

        <main className="main-pane">
          <header className="topbar">
            <div>
              <p className="eyebrow">Protected endpoint</p>
              <h1>{activeThread?.title ?? "New Thread"}</h1>
            </div>
            <div className="topbar-stats">
              <div>
                <span>Threads</span>
                <strong>{threads.length}</strong>
              </div>
              <div>
                <span>Messages</span>
                <strong>{messages.length}</strong>
              </div>
            </div>
          </header>

          {error ? <div className="error-banner">{error}</div> : null}

          <section className="message-stream">
            {messages.map((message) => (
              <article
                className={`message-card ${message.role === "assistant" ? "assistant-card" : "user-card"}`}
                key={message.id}
              >
                <div className="message-header">
                  <strong>
                    {message.role === "assistant"
                      ? "Xavier UI Agent"
                      : "Operator"}
                  </strong>
                  <span className="message-time">
                    {new Date(message.created_at).toLocaleTimeString()}
                  </span>
                </div>

                {message.role === "assistant" ? (
                  <>
                    <div className="meta-grid">
                      <MetaItem
                        label="Confidence"
                        value={formatConfidence(message.metadata?.confidence)}
                      />
                      <MetaItem
                        label="Documents"
                        value={String(message.metadata?.documents ?? 0)}
                      />
                      <MetaItem
                        label="Evidence"
                        value={String(message.metadata?.evidence ?? 0)}
                      />
                      <MetaItem
                        label="Latency"
                        value={`${message.metadata?.timings?.total_ms ?? 0} ms`}
                      />
                    </div>

                    <div className="rules-panel">
                      <div>
                        <h3>Render rules</h3>
                        <div className="tag-row">
                          {(message.metadata?.rules ?? []).map((item) => (
                            <span key={item} className="tag">
                              {item}
                            </span>
                          ))}
                        </div>
                      </div>
                      <div>
                        <h3>Components</h3>
                        <div className="tag-row">
                          {(message.metadata?.components ?? []).map((item) => (
                            <span key={item} className="tag tag-accent">
                              {item}
                            </span>
                          ))}
                        </div>
                      </div>
                    </div>

                    {message.openui_lang ? (
                      <div className="render-surface">
                        <div className="render-surface-header">
                          <span className="render-surface-title">
                            OpenUI Render Surface
                          </span>
                          <span className="render-surface-title render-surface-mode">
                            Structured output
                          </span>
                        </div>
                        <Renderer
                          response={message.openui_lang}
                          library={combinedLibrary}
                          isStreaming={false}
                        />
                      </div>
                    ) : null}

                    <div className="plain-text">{message.plain_text}</div>
                  </>
                ) : (
                  <div className="plain-text">{message.plain_text}</div>
                )}
              </article>
            ))}
            {isLoading ? (
              <div className="loading-block">Thinking and rendering…</div>
            ) : null}
          </section>

          <footer className="composer">
            <textarea
              className="cx-textarea"
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
              placeholder="Ask Xavier for memory, code, or a structured answer..."
              rows={4}
            />
            <button
              type="button"
              className="cx-button cx-button-primary"
              onClick={() => void sendMessage()}
            >
              Send
            </button>
          </footer>
        </main>
      </div>
    </ThemeProvider>
  );
}

function MetaItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="meta-item">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function formatConfidence(value?: number) {
  if (typeof value !== "number") {
    return "n/a";
  }
  return `${Math.round(value * 100)}%`;
}

export default App;
