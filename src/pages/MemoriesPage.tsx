export function MemoriesPage() {
  return (
    <>
      <div className="page-header">
        <h1 className="page-title">Memories</h1>
        <p className="page-subtitle">Facts and learnings extracted from your conversations.</p>
      </div>
      <div className="empty-state">
        <span className="material-symbols-outlined">neurology</span>
        <p className="primary-text">No memories yet</p>
        <p className="secondary-text">Memories are extracted automatically from conversations</p>
      </div>
    </>
  );
}
