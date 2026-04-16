export function SettingsPage() {
  return (
    <div className="flex h-full flex-col p-6">
      <h1 className="text-lg font-semibold text-text-primary">Settings</h1>
      <p className="mt-2 text-sm text-text-secondary">
        Configure your AI providers, audio devices, and preferences.
      </p>

      <div className="mt-6 flex flex-col gap-6">
        <SettingsSection title="LLM Provider">
          <SettingsRow label="Provider" value="Ollama (local)" />
          <SettingsRow label="Model" value="Not configured" />
          <SettingsRow label="API URL" value="http://localhost:11434" />
        </SettingsSection>

        <SettingsSection title="Audio">
          <SettingsRow label="Input Device" value="Default" />
          <SettingsRow label="System Audio" value="Disabled" />
        </SettingsSection>

        <SettingsSection title="Screen Capture">
          <SettingsRow label="Capture Interval" value="3 seconds" />
          <SettingsRow label="OCR Engine" value="Tesseract" />
        </SettingsSection>
      </div>
    </div>
  );
}

function SettingsSection({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-xl border border-border bg-bg-secondary p-4">
      <h2 className="mb-3 text-sm font-medium text-text-primary">{title}</h2>
      <div className="flex flex-col gap-2">{children}</div>
    </div>
  );
}

function SettingsRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between rounded-lg px-3 py-2 hover:bg-bg-hover transition-colors">
      <span className="text-xs text-text-secondary">{label}</span>
      <span className="text-xs text-text-muted">{value}</span>
    </div>
  );
}
