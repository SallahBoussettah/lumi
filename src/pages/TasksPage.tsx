export function TasksPage() {
  return (
    <>
      <div className="page-header">
        <h1 className="page-title">Tasks</h1>
        <p className="page-subtitle">Action items extracted from your conversations.</p>
      </div>
      <div className="empty-state">
        <span className="material-symbols-outlined">task_alt</span>
        <p className="primary-text">No tasks yet</p>
        <p className="secondary-text">Mention something to do in a conversation and it shows up here</p>
      </div>
    </>
  );
}
