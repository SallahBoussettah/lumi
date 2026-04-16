import { useState, useEffect, useCallback } from "react";
import {
  getMemoryDetail,
  updateMemory,
  dismissMemory,
  deleteMemory,
} from "../lib/tauri";
import type { MemoryDetail } from "../lib/tauri";

interface Props {
  memoryId: string;
  onBack: () => void;
  onOpenConversation: (id: string) => void;
  onDeleted: () => void;
}

function formatDate(iso: string): string {
  const d = new Date(iso + "Z");
  return d.toLocaleString(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

const CATEGORY_LABEL: Record<string, string> = {
  system: "About you",
  interesting: "Worth knowing",
  manual: "Saved manually",
};

export function MemoryDetailPage({
  memoryId,
  onBack,
  onOpenConversation,
  onDeleted,
}: Props) {
  const [detail, setDetail] = useState<MemoryDetail | null>(null);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [draftCategory, setDraftCategory] = useState("system");
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    try {
      const d = await getMemoryDetail(memoryId);
      setDetail(d);
      setDraft(d.memory.content);
      setDraftCategory(d.memory.category);
    } catch (e) {
      console.error(e);
    }
  }, [memoryId]);

  useEffect(() => {
    load();
  }, [load]);

  async function handleSave() {
    if (!detail) return;
    if (!draft.trim()) return;
    setSaving(true);
    try {
      const contentChanged = draft.trim() !== detail.memory.content;
      const categoryChanged = draftCategory !== detail.memory.category;
      await updateMemory(
        memoryId,
        contentChanged ? draft.trim() : null,
        categoryChanged ? draftCategory : null
      );
      setEditing(false);
      await load();
    } catch (e) {
      console.error(e);
    }
    setSaving(false);
  }

  async function handleDismiss() {
    if (!confirm("Dismiss this memory? It'll be hidden from views and chat.")) return;
    try {
      await dismissMemory(memoryId);
      onDeleted();
    } catch (e) {
      console.error(e);
    }
  }

  async function handleDelete() {
    if (!confirm("Permanently delete this memory? This cannot be undone.")) return;
    try {
      await deleteMemory(memoryId);
      onDeleted();
    } catch (e) {
      console.error(e);
    }
  }

  if (!detail) {
    return (
      <div style={{ padding: "var(--space-12)", textAlign: "center", color: "var(--text-3)" }}>
        Loading…
      </div>
    );
  }

  const m = detail.memory;
  const src = detail.source_conversation;

  return (
    <>
      <button
        onClick={onBack}
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: 6,
          background: "transparent",
          border: "none",
          color: "var(--text-3)",
          fontFamily: "inherit",
          fontSize: "var(--text-sm)",
          cursor: "pointer",
          padding: "4px 8px",
          marginLeft: -8,
          marginBottom: "var(--space-4)",
          borderRadius: "var(--r-control)",
          transition: "all var(--dur-quick) var(--ease-out)",
        }}
        onMouseEnter={(e) => (e.currentTarget.style.color = "var(--text-1)")}
        onMouseLeave={(e) => (e.currentTarget.style.color = "var(--text-3)")}
      >
        <span className="material-symbols-outlined" style={{ fontSize: 16 }}>
          arrow_back
        </span>
        Memories
      </button>

      <header className="page-header">
        <div style={{ display: "flex", alignItems: "flex-start", gap: 16 }}>
          <div className="conv-icon" style={{ width: 44, height: 44 }}>
            <span className="material-symbols-outlined" style={{ fontSize: 22 }}>
              {m.category === "system" ? "person" : "auto_awesome"}
            </span>
          </div>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
              <span className="conv-tag">{CATEGORY_LABEL[m.category] || m.category}</span>
              {m.manually_added && (
                <span className="conv-tag" style={{ color: "var(--text-3)" }}>
                  saved manually
                </span>
              )}
            </div>

            {editing ? (
              <>
                <textarea
                  value={draft}
                  onChange={(e) => setDraft(e.target.value)}
                  rows={3}
                  style={{
                    width: "100%",
                    background: "var(--surface-card)",
                    border: "1px solid var(--accent)",
                    borderRadius: "var(--r-control)",
                    color: "var(--text-1)",
                    fontFamily: "var(--font-display)",
                    fontSize: "var(--text-xl)",
                    padding: "12px 14px",
                    resize: "vertical",
                    outline: "none",
                    lineHeight: 1.4,
                  }}
                />
                <div style={{ display: "flex", gap: 8, marginTop: 12, alignItems: "center" }}>
                  <select
                    value={draftCategory}
                    onChange={(e) => setDraftCategory(e.target.value)}
                    style={{
                      background: "var(--surface-card)",
                      border: "1px solid var(--border-faint)",
                      color: "var(--text-2)",
                      borderRadius: "var(--r-control)",
                      padding: "6px 10px",
                      fontSize: "var(--text-sm)",
                      fontFamily: "inherit",
                    }}
                  >
                    <option value="system">About you</option>
                    <option value="interesting">Worth knowing</option>
                  </select>
                  <button
                    className="filter-pill active"
                    onClick={handleSave}
                    disabled={saving || !draft.trim()}
                  >
                    {saving ? "Saving…" : "Save"}
                  </button>
                  <button
                    className="filter-pill"
                    onClick={() => {
                      setEditing(false);
                      setDraft(m.content);
                      setDraftCategory(m.category);
                    }}
                  >
                    Cancel
                  </button>
                </div>
              </>
            ) : (
              <h1
                className="page-title"
                style={{ fontWeight: 400, lineHeight: 1.3, marginBottom: 12 }}
              >
                {m.content}
              </h1>
            )}

            <p className="page-subtitle">
              {m.created_at !== m.updated_at && "Edited "}
              {formatDate(m.updated_at)}
            </p>
          </div>

          {!editing && (
            <div style={{ display: "flex", gap: 6, flexShrink: 0 }}>
              <button className="filter-pill" onClick={() => setEditing(true)}>
                Edit
              </button>
              <button className="filter-pill" onClick={handleDismiss}>
                Dismiss
              </button>
              <button
                className="filter-pill"
                onClick={handleDelete}
                style={{ color: "var(--semantic-error)" }}
              >
                Delete
              </button>
            </div>
          )}
        </div>
      </header>

      {/* Source conversation */}
      <section className="date-section">
        <div className="date-section-label">Source</div>
        {src ? (
          <div
            className="conv-row"
            onClick={() => onOpenConversation(src.id)}
          >
            <div className="conv-icon">
              <span className="material-symbols-outlined">forum</span>
            </div>
            <div className="conv-body">
              <div className="conv-title">{src.title || "Untitled conversation"}</div>
              <div className="conv-overview">{src.overview || ""}</div>
            </div>
            <div className="conv-meta">
              <span className="conv-time">{formatDate(src.started_at).split(",")[0]}</span>
              {src.category && <span className="conv-tag">{src.category}</span>}
            </div>
          </div>
        ) : (
          <p style={{ fontSize: "var(--text-sm)", color: "var(--text-4)" }}>
            {m.manually_added
              ? "Saved manually — no source conversation."
              : "Source conversation no longer exists."}
          </p>
        )}
      </section>

      {/* Tip */}
      <section style={{ marginTop: "var(--space-8)" }}>
        <p
          style={{
            fontSize: "var(--text-sm)",
            color: "var(--text-3)",
            fontStyle: "italic",
          }}
        >
          You can also fix this from chat — try saying "change [memory keyword] to [correct
          version]" or "forget [memory keyword]".
        </p>
      </section>
    </>
  );
}
