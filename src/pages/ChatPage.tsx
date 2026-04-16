export function ChatPage() {
  return (
    <>
      <div className="page-header">
        <h1 className="page-title">Chat</h1>
        <p className="page-subtitle">Ask anything — the assistant knows your conversations and memories.</p>
      </div>
      <div className="empty-state">
        <span className="material-symbols-outlined">bolt</span>
        <p className="primary-text">Chat coming soon</p>
        <p className="secondary-text">Context-aware chat powered by your activity history</p>
      </div>
    </>
  );
}
