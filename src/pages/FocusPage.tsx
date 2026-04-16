export function FocusPage() {
  return (
    <div className="flex h-full flex-col p-6">
      <h1 className="text-lg font-semibold text-text-primary">Focus</h1>
      <p className="mt-2 text-sm text-text-secondary">
        Track your focus sessions and get productivity insights.
      </p>
      <div className="mt-8 flex flex-1 items-center justify-center">
        <div className="text-center">
          <div className="text-4xl">🎯</div>
          <p className="mt-3 text-sm text-text-muted">Focus tracking coming soon</p>
          <p className="mt-1 text-xs text-text-muted">
            AI monitors your activity and helps you stay on task
          </p>
        </div>
      </div>
    </div>
  );
}
