export function SettingsPage() {
  return (
    <>
      <div className="page-header">
        <h1 className="page-title">Settings</h1>
        <p className="page-subtitle">Configure providers, audio, and screen capture.</p>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">AI Provider</div>
        <div className="settings-card">
          <div className="settings-row"><span className="label">Provider</span><span className="value">Ollama (local)</span></div>
          <div className="settings-row"><span className="label">Model</span><span className="value">Not configured</span></div>
          <div className="settings-row"><span className="label">Endpoint</span><span className="value">http://localhost:11434</span></div>
        </div>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">Audio</div>
        <div className="settings-card">
          <div className="settings-row"><span className="label">Input device</span><span className="value">Default</span></div>
          <div className="settings-row"><span className="label">System audio</span><span className="value">Off</span></div>
          <div className="settings-row"><span className="label">VAD sensitivity</span><span className="value">Medium</span></div>
        </div>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">Screen Capture</div>
        <div className="settings-card">
          <div className="settings-row"><span className="label">Interval</span><span className="value">3 seconds</span></div>
          <div className="settings-row"><span className="label">OCR</span><span className="value">Tesseract</span></div>
        </div>
      </div>
    </>
  );
}
