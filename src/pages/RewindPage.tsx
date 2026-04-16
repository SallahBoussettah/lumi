export function RewindPage() {
  return (
    <div className="flex h-full flex-col p-6">
      <h1 className="text-lg font-semibold text-text-primary">Rewind</h1>
      <p className="mt-2 text-sm text-text-secondary">
        Browse through your screen history — search anything you've seen.
      </p>
      <div className="mt-8 flex flex-1 items-center justify-center">
        <div className="text-center">
          <div className="text-4xl">⏪</div>
          <p className="mt-3 text-sm text-text-muted">Rewind coming soon</p>
          <p className="mt-1 text-xs text-text-muted">
            Screen capture with OCR and full-text search
          </p>
        </div>
      </div>
    </div>
  );
}
