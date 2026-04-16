export function MemoriesPage() {
  return (
    <div className="flex h-full flex-col p-6">
      <h1 className="text-lg font-semibold text-text-primary">Memories</h1>
      <p className="mt-2 text-sm text-text-secondary">
        Facts and learnings extracted from your conversations and screen activity.
      </p>
      <div className="mt-8 flex flex-1 items-center justify-center">
        <div className="text-center">
          <div className="text-4xl">🧠</div>
          <p className="mt-3 text-sm text-text-muted">No memories yet</p>
          <p className="mt-1 text-xs text-text-muted">
            Memories are extracted automatically from your conversations
          </p>
        </div>
      </div>
    </div>
  );
}
