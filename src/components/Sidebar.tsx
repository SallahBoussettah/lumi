import type { Page } from "../App";

const navItems: { id: Page; icon: string; label: string }[] = [
  { id: "conversations", icon: "forum", label: "Conversations" },
  { id: "memories", icon: "neurology", label: "Memories" },
  { id: "tasks", icon: "task_alt", label: "Tasks" },
  { id: "chat", icon: "bolt", label: "Chat" },
  { id: "rewind", icon: "history", label: "Rewind" },
  { id: "focus", icon: "track_changes", label: "Focus" },
];

interface SidebarProps {
  activePage: Page;
  onNavigate: (page: Page) => void;
}

export function Sidebar({ activePage, onNavigate }: SidebarProps) {
  return (
    <nav className="app-sidebar">
      <div className="sidebar-header">
        <div className="sidebar-logo">O</div>
        <span className="sidebar-title">Omniscient</span>
      </div>

      <div className="sidebar-section">
        <div className="sidebar-section-label">Navigate</div>
        {navItems.map((item) => (
          <button
            key={item.id}
            className={`sidebar-item ${activePage === item.id ? "active" : ""}`}
            onClick={() => onNavigate(item.id)}
          >
            <span className="material-symbols-outlined">{item.icon}</span>
            {item.label}
          </button>
        ))}
      </div>

      <div className="sidebar-spacer" />

      <button
        className={`sidebar-item ${activePage === "settings" ? "active" : ""}`}
        onClick={() => onNavigate("settings")}
      >
        <span className="material-symbols-outlined">settings</span>
        Settings
      </button>
    </nav>
  );
}
