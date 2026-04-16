const conversations = [
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
  return (
    <>
      <div className="page-header">
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
          <div>
            <h1 className="page-title">{getGreeting()}, Salah</h1>
            <p className="page-subtitle">5 conversations today. 4 memories extracted.</p>
          </div>
          <div className="recording-pill">
            <span className="recording-dot" />
            Listening
          </div>
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
        {conversations.map((c) => (
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

      <button className="fab">
        <span className="material-symbols-outlined" style={{ fontSize: "22px" }}>mic</span>
      </button>
    </>
  );
}
