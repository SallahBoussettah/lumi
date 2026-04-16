import { useState, useEffect } from "react";
import {
  getActionItems,
  toggleActionItem,
  deleteActionItem,
  clearCompletedTasks,
} from "../lib/tauri";
import type { ActionItemData } from "../lib/tauri";

function formatDueDate(iso: string | null): string | null {
  if (!iso) return null;
  const d = new Date(iso);
  if (isNaN(d.getTime())) return null;
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const dayMs = 86400000;
  const dayDiff = Math.round((d.getTime() - today.getTime()) / dayMs);

  if (dayDiff === 0) return "today";
  if (dayDiff === 1) return "tomorrow";
  if (dayDiff === -1) return "yesterday";
  if (dayDiff > 1 && dayDiff <= 7) {
    return d.toLocaleDateString(undefined, { weekday: "long" });
  }
  if (dayDiff < 0) return "overdue";
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

export function TasksPage() {
  const [tasks, setTasks] = useState<ActionItemData[]>([]);

  useEffect(() => {
    load();
  }, []);

  async function load() {
    try {
      const data = (await getActionItems()) as unknown as ActionItemData[];
      setTasks(data);
    } catch {
      /* ignore */
    }
  }

  async function handleToggle(id: string, currentCompleted: boolean) {
    try {
      await toggleActionItem(id, !currentCompleted);
      setTasks((prev) =>
        prev.map((t) => (t.id === id ? { ...t, completed: !currentCompleted } : t))
      );
    } catch {
      /* ignore */
    }
  }

  async function handleDelete(id: string) {
    try {
      await deleteActionItem(id);
      setTasks((prev) => prev.filter((t) => t.id !== id));
    } catch {
      /* ignore */
    }
  }

  async function handleClearCompleted() {
    if (!confirm("Delete all completed tasks?")) return;
    try {
      await clearCompletedTasks();
      setTasks((prev) => prev.filter((t) => !t.completed));
    } catch {
      /* ignore */
    }
  }

  const pending = tasks.filter((t) => !t.completed);
  const done = tasks.filter((t) => t.completed);

  return (
    <>
      <header className="page-header">
        <div className="page-header-row">
          <div>
            <h1 className="page-title">Tasks</h1>
            <p className="page-subtitle">
              Things you mentioned needing to do. Check them off as you go.
            </p>
          </div>
          {done.length > 0 && (
            <button
              className="filter-pill"
              onClick={handleClearCompleted}
              style={{ color: "var(--text-3)" }}
            >
              Clear {done.length} completed
            </button>
          )}
        </div>
      </header>

      {tasks.length === 0 ? (
        <div className="empty">
          <div className="empty-mark">
            <span className="material-symbols-outlined">task_alt</span>
          </div>
          <p className="empty-voice">No tasks waiting on you.</p>
          <p className="empty-hint">
            When you say something like "remind me to..." or "I need to..." in a
            conversation, I'll capture it here.
          </p>
        </div>
      ) : (
        <>
          {pending.length > 0 && (
            <section className="date-section">
              <div className="date-section-label">{pending.length} pending</div>
              {pending.map((task) => (
                <TaskRow
                  key={task.id}
                  task={task}
                  onToggle={handleToggle}
                  onDelete={handleDelete}
                />
              ))}
            </section>
          )}

          {done.length > 0 && (
            <section className="date-section">
              <div className="date-section-label">Completed</div>
              {done.map((task) => (
                <TaskRow
                  key={task.id}
                  task={task}
                  onToggle={handleToggle}
                  onDelete={handleDelete}
                />
              ))}
            </section>
          )}
        </>
      )}
    </>
  );
}

function TaskRow({
  task,
  onToggle,
  onDelete,
}: {
  task: ActionItemData;
  onToggle: (id: string, completed: boolean) => void;
  onDelete: (id: string) => void;
}) {
  const due = formatDueDate(task.due_at);
  const isOverdue = due === "overdue" || due === "yesterday";
  const isDueToday = due === "today";

  return (
    <div className="conv-row" style={{ alignItems: "center" }}>
      <div
        className="conv-icon"
        onClick={() => onToggle(task.id, task.completed)}
        style={{
          background: task.completed ? "transparent" : "var(--accent-faint)",
          color: task.completed ? "var(--semantic-active)" : "var(--accent)",
          cursor: "pointer",
        }}
      >
        <span className="material-symbols-outlined">
          {task.completed ? "check_circle" : "radio_button_unchecked"}
        </span>
      </div>
      <div
        className="conv-body"
        onClick={() => onToggle(task.id, task.completed)}
        style={{ cursor: "pointer" }}
      >
        <div
          className="conv-title"
          style={{
            fontWeight: 400,
            color: task.completed ? "var(--text-3)" : "var(--text-1)",
            textDecoration: task.completed ? "line-through" : "none",
          }}
        >
          {task.description}
        </div>
        {due && !task.completed && (
          <div
            className="conv-overview"
            style={{
              color: isOverdue
                ? "var(--semantic-error)"
                : isDueToday
                  ? "var(--accent)"
                  : "var(--text-3)",
              marginTop: 2,
            }}
          >
            <span
              className="material-symbols-outlined"
              style={{ fontSize: 12, verticalAlign: "middle", marginRight: 4 }}
            >
              schedule
            </span>
            {due}
          </div>
        )}
      </div>
      <div className="conv-meta" style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
        <span className={`priority priority-${task.priority}`}>{task.priority}</span>
        <button
          className="row-action"
          onClick={(e) => {
            e.stopPropagation();
            onDelete(task.id);
          }}
          title="Delete task"
        >
          <span className="material-symbols-outlined">delete</span>
        </button>
      </div>
    </div>
  );
}
