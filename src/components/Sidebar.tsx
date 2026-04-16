import type { Page } from "../App";

interface NavItem {
  id: Page;
  label: string;
  icon: string;
}

const navItems: NavItem[] = [
  { id: "conversations", label: "Conversations", icon: "💬" },
  { id: "memories", label: "Memories", icon: "🧠" },
  { id: "tasks", label: "Tasks", icon: "✅" },
  { id: "chat", label: "Chat", icon: "⚡" },
  { id: "rewind", label: "Rewind", icon: "⏪" },
  { id: "focus", label: "Focus", icon: "🎯" },
];

const bottomItems: NavItem[] = [
  { id: "settings", label: "Settings", icon: "⚙️" },
];

interface SidebarProps {
  activePage: Page;
  onNavigate: (page: Page) => void;
  collapsed: boolean;
  onToggleCollapse: () => void;
}

export function Sidebar({
  activePage,
  onNavigate,
  collapsed,
  onToggleCollapse,
}: SidebarProps) {
  return (
    <aside
      className={`flex flex-col border-r border-border bg-bg-secondary transition-all duration-200 ${
        collapsed ? "w-16" : "w-56"
      }`}
    >
      <div className="flex h-14 items-center gap-2 border-b border-border px-4">
        <button
          onClick={onToggleCollapse}
          className="flex h-8 w-8 items-center justify-center rounded-md text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors"
        >
          {collapsed ? "▶" : "◀"}
        </button>
        {!collapsed && (
          <span className="text-sm font-semibold tracking-wide text-accent">
            OMNISCIENT
          </span>
        )}
      </div>

      <nav className="flex flex-1 flex-col justify-between p-2">
        <div className="flex flex-col gap-1">
          {navItems.map((item) => (
            <NavButton
              key={item.id}
              item={item}
              active={activePage === item.id}
              collapsed={collapsed}
              onClick={() => onNavigate(item.id)}
            />
          ))}
        </div>

        <div className="flex flex-col gap-1">
          {bottomItems.map((item) => (
            <NavButton
              key={item.id}
              item={item}
              active={activePage === item.id}
              collapsed={collapsed}
              onClick={() => onNavigate(item.id)}
            />
          ))}
        </div>
      </nav>
    </aside>
  );
}

function NavButton({
  item,
  active,
  collapsed,
  onClick,
}: {
  item: NavItem;
  active: boolean;
  collapsed: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition-colors ${
        active
          ? "bg-accent-subtle text-accent"
          : "text-text-secondary hover:bg-bg-hover hover:text-text-primary"
      } ${collapsed ? "justify-center" : ""}`}
      title={collapsed ? item.label : undefined}
    >
      <span className="text-base">{item.icon}</span>
      {!collapsed && <span>{item.label}</span>}
    </button>
  );
}
