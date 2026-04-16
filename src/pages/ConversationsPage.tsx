export function ConversationsPage() {
  return (
    <div className="flex h-full flex-col p-6">
      <h1 className="text-lg font-semibold text-text-primary">Conversations</h1>
      <p className="mt-2 text-sm text-text-secondary">
        Your transcribed conversations will appear here.
      </p>
      <div className="mt-8 flex flex-1 items-center justify-center">
        <div className="text-center">
          <div className="text-4xl">💬</div>
          <p className="mt-3 text-sm text-text-muted">No conversations yet</p>
          <p className="mt-1 text-xs text-text-muted">
            Start recording to capture your first conversation
          </p>
        </div>
      </div>
    </div>
  );
}
