import { useState, useEffect } from "react";
import { Sidebar } from "./components/Sidebar";
import { ConversationsPage } from "./pages/ConversationsPage";
import { ConversationDetailPage } from "./pages/ConversationDetailPage";
import { MemoriesPage } from "./pages/MemoriesPage";
import { MemoryDetailPage } from "./pages/MemoryDetailPage";
import { TasksPage } from "./pages/TasksPage";
import { ChatPage } from "./pages/ChatPage";
import { RewindPage } from "./pages/RewindPage";
import { FocusPage } from "./pages/FocusPage";
import { SettingsPage } from "./pages/SettingsPage";
import { isRecording as checkRecording } from "./lib/tauri";

export type Page =
  | "conversations"
  | "memories"
  | "tasks"
  | "chat"
  | "rewind"
  | "focus"
  | "settings";

function todWarmth(): number {
  const h = new Date().getHours() + new Date().getMinutes() / 60;
  return Math.max(0, 1 - Math.abs(h - 13) / 12);
}

export function App() {
  const [activePage, setActivePage] = useState<Page>("conversations");
  const [selectedConversationId, setSelectedConversationId] = useState<string | null>(null);
  const [selectedMemoryId, setSelectedMemoryId] = useState<string | null>(null);
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    const apply = () => {
      document.documentElement.style.setProperty("--tod-warmth", String(todWarmth()));
    };
    apply();
    const id = setInterval(apply, 5 * 60 * 1000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    const tick = () => {
      checkRecording().then(setRecording).catch(() => {});
    };
    tick();
    const id = setInterval(tick, 1500);
    return () => clearInterval(id);
  }, []);

  function navigate(page: Page) {
    setActivePage(page);
    setSelectedConversationId(null);
    setSelectedMemoryId(null);
  }

  function openConversation(id: string) {
    setActivePage("conversations");
    setSelectedConversationId(id);
    setSelectedMemoryId(null);
  }

  function openMemory(id: string) {
    setActivePage("memories");
    setSelectedMemoryId(id);
    setSelectedConversationId(null);
  }

  function clearDetail() {
    setSelectedConversationId(null);
    setSelectedMemoryId(null);
  }

  function renderPage() {
    if (activePage === "conversations" && selectedConversationId) {
      return (
        <ConversationDetailPage
          conversationId={selectedConversationId}
          onBack={clearDetail}
          onDeleted={clearDetail}
        />
      );
    }
    if (activePage === "memories" && selectedMemoryId) {
      return (
        <MemoryDetailPage
          memoryId={selectedMemoryId}
          onBack={clearDetail}
          onOpenConversation={openConversation}
          onDeleted={clearDetail}
        />
      );
    }

    switch (activePage) {
      case "conversations":
        return <ConversationsPage onOpenConversation={openConversation} />;
      case "memories":
        return <MemoriesPage onOpenMemory={openMemory} />;
      case "tasks":
        return <TasksPage />;
      case "chat":
        return <ChatPage />;
      case "rewind":
        return <RewindPage />;
      case "focus":
        return <FocusPage />;
      case "settings":
        return <SettingsPage />;
    }
  }

  return (
    <div className={`app-layout ${recording ? "is-listening" : ""}`}>
      <Sidebar activePage={activePage} onNavigate={navigate} />
      <main className="app-main">
        <div className="app-content">{renderPage()}</div>
      </main>
    </div>
  );
}
