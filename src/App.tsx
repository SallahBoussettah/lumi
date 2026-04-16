import { useState } from "react";
import { Sidebar } from "./components/Sidebar";
import { ConversationsPage } from "./pages/ConversationsPage";
import { MemoriesPage } from "./pages/MemoriesPage";
import { TasksPage } from "./pages/TasksPage";
import { ChatPage } from "./pages/ChatPage";
import { RewindPage } from "./pages/RewindPage";
import { FocusPage } from "./pages/FocusPage";
import { SettingsPage } from "./pages/SettingsPage";

export type Page =
  | "conversations"
  | "memories"
  | "tasks"
  | "chat"
  | "rewind"
  | "focus"
  | "settings";

const pages: Record<Page, () => React.JSX.Element> = {
  conversations: ConversationsPage,
  memories: MemoriesPage,
  tasks: TasksPage,
  chat: ChatPage,
  rewind: RewindPage,
  focus: FocusPage,
  settings: SettingsPage,
};

export function App() {
  const [activePage, setActivePage] = useState<Page>("conversations");
  const ActivePage = pages[activePage];

  return (
    <div className="app-layout">
      <Sidebar activePage={activePage} onNavigate={setActivePage} />
      <main className="app-main">
        <ActivePage />
      </main>
    </div>
  );
}
