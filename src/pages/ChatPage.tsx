export function ChatPage() {
  return (
    <div className="flex h-full flex-col p-6">
      <h1 className="text-lg font-semibold text-text-primary">Chat</h1>
      <p className="mt-2 text-sm text-text-secondary">
        Ask your AI assistant anything — it knows your conversations and memories.
      </p>
      <div className="mt-8 flex flex-1 items-center justify-center">
        <div className="text-center">
          <div className="text-4xl">⚡</div>
          <p className="mt-3 text-sm text-text-muted">Chat coming soon</p>
          <p className="mt-1 text-xs text-text-muted">
            RAG-powered chat with full context of your activity
          </p>
        </div>
      </div>
    </div>
  );
}
