import { useState, useEffect, useRef } from "react";
import {
  startRecording,
  stopRecording,
  getAudioLevel,
  isRecording as checkRecording,
} from "../lib/tauri";

const mockConversations = [
  { id: "1", icon: "build", title: "Omniscient Architecture Sync", overview: "Discussed memory indexing latency improvements and ambient listening protocols", time: "3h ago", tag: "work" },
  { id: "2", icon: "nightlight", title: "Evening Reflection", overview: "Personal notes on today's focus levels and evening wind-down routine", time: "5h ago", tag: "personal" },
  { id: "3", icon: "lightbulb", title: "Neural Interface Concept", overview: "Mapping memory clusters to spatial coordinates in a virtual room", time: "8h ago", tag: "idea" },
  { id: "4", icon: "groups", title: "Product Review with Team", overview: "Sidebar hover states, divider opacity adjustments, onboarding flow", time: "10h ago", tag: "work" },
  { id: "5", icon: "menu_book", title: "Book Recommendation", overview: "Marcus recommended The Overstory during afternoon chat", time: "14h ago", tag: "personal" },
];

function getGreeting(): string {
  const h = new Date().getHours();
  return h < 12 ? "Good morning" : h < 18 ? "Good afternoon" : "Good evening";
}

export function ConversationsPage() {
  const [recording, setRecording] = useState(false);
  const [audioLevel, setAudioLevel] = useState(0);
  const levelInterval = useRef<ReturnType<typeof setInterval> | null>(null);

  // Check initial recording state
  useEffect(() => {
    checkRecording().then(setRecording).catch(() => {});
  }, []);

  // Poll audio level while recording
  useEffect(() => {
    if (recording) {
      levelInterval.current = setInterval(async () => {
        try {
          const level = await getAudioLevel();
          setAudioLevel(level);
        } catch {
          // ignore
        }
      }, 100);
    } else {
      if (levelInterval.current) {
        clearInterval(levelInterval.current);
        levelInterval.current = null;
      }
      setAudioLevel(0);
    }
    return () => {
      if (levelInterval.current) clearInterval(levelInterval.current);
    };
  }, [recording]);

  async function toggleRecording() {
    try {
      if (recording) {
        await stopRecording();
        setRecording(false);
      } else {
        await startRecording();
        setRecording(true);
      }
    } catch (err) {
      console.error("Recording toggle failed:", err);
    }
  }

  return (
    <>
      <div className="page-header">
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
          <div>
            <h1 className="page-title">{getGreeting()}, Salah</h1>
            <p className="page-subtitle">5 conversations today. 4 memories extracted.</p>
          </div>
          {recording && (
            <div className="recording-pill">
              <span className="recording-dot" />
              Listening
              <span style={{ marginLeft: 4, opacity: 0.6 }}>
                {audioLevel > 0 && `${audioLevel}%`}
              </span>
            </div>
          )}
        </div>
      </div>

      <div className="stats-row">
        <div className="stat-item">
          <div className="stat-value">14</div>
          <div className="stat-label">conversations</div>
        </div>
        <div className="stat-item">
          <div className="stat-value">53</div>
          <div className="stat-label">memories</div>
        </div>
        <div className="stat-item">
          <div className="stat-value">31</div>
          <div className="stat-label">tasks</div>
        </div>
      </div>

      <div className="conversation-list">
        {mockConversations.map((c) => (
          <div key={c.id} className="conversation-row">
            <span className="material-symbols-outlined conversation-emoji">{c.icon}</span>
            <div className="conversation-content">
              <div className="conversation-title">{c.title}</div>
              <div className="conversation-overview">{c.overview}</div>
            </div>
            <div className="conversation-meta">
              <span className="conversation-time">{c.time}</span>
              <span className="conversation-tag">{c.tag}</span>
            </div>
          </div>
        ))}
      </div>

      <div className="insight-cards">
        <div className="insight-card">
          <div className="insight-card-title">Memory Consolidation</div>
          <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
            <div style={{ flex: 1, height: "4px", borderRadius: "2px", background: "rgba(255,255,255,0.06)" }}>
              <div style={{ width: "82%", height: "100%", borderRadius: "2px", background: "var(--accent)" }} />
            </div>
            <span className="insight-card-subtitle">82%</span>
          </div>
        </div>
        <div className="insight-card">
          <div className="insight-card-title">Today's Focus</div>
          <div className="focus-bars">
            {[40, 65, 30, 55, 80, 25, 60, 45].map((h, i) => (
              <div key={i} className="focus-bar" style={{ height: `${h}%`, opacity: 0.25 + (h / 100) * 0.55 }} />
            ))}
          </div>
        </div>
      </div>

      {/* Audio level visualizer when recording */}
      {recording && (
        <div style={{
          position: "fixed",
          bottom: 84,
          right: 24,
          width: 48,
          height: 4,
          borderRadius: 2,
          background: "rgba(255,255,255,0.06)",
          overflow: "hidden",
        }}>
          <div style={{
            width: `${audioLevel}%`,
            height: "100%",
            borderRadius: 2,
            background: "var(--green)",
            transition: "width 0.1s ease",
          }} />
        </div>
      )}

      <button
        className="fab"
        onClick={toggleRecording}
        style={{
          background: recording ? "var(--green)" : "var(--accent)",
          boxShadow: recording
            ? "0 4px 24px rgba(52, 211, 153, 0.35)"
            : "0 4px 24px rgba(124, 108, 240, 0.35)",
        }}
      >
        <span className="material-symbols-outlined" style={{ fontSize: "22px" }}>
          {recording ? "stop" : "mic"}
        </span>
      </button>
    </>
  );
}
