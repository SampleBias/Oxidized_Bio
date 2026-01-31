/**
 * Settings Page
 * Allows users to configure API keys and preferences
 */
import { useState, useCallback } from 'preact/hooks';
import { route } from 'preact-router';
import { useSettings, ProviderInfo, ProviderStatus } from '../hooks/useSettings';
import { Button } from '../components/ui/Button';
import { IconButton } from '../components/ui/IconButton';

interface SettingsPageProps {
  path?: string;
}

// Provider card component
interface ProviderCardProps {
  provider: ProviderInfo;
  status: ProviderStatus | undefined;
  isDefault: boolean;
  onSetDefault: () => void;
  onSaveKey: (key: string, model?: string) => Promise<void>;
  onTest: () => Promise<void>;
  testResult: { success: boolean; message: string } | null;
  isSaving: boolean;
}

function ProviderCard({
  provider,
  status,
  isDefault,
  onSetDefault,
  onSaveKey,
  onTest,
  testResult,
  isSaving,
}: ProviderCardProps) {
  const [apiKey, setApiKey] = useState('');
  const [selectedModel, setSelectedModel] = useState(status?.default_model || provider.models[0]?.id || '');
  const [isEditing, setIsEditing] = useState(false);
  const [localSaving, setLocalSaving] = useState(false);

  const handleSave = async () => {
    setLocalSaving(true);
    await onSaveKey(apiKey, selectedModel);
    setApiKey('');
    setIsEditing(false);
    setLocalSaving(false);
  };

  const handleClear = async () => {
    setLocalSaving(true);
    await onSaveKey('', selectedModel);
    setLocalSaving(false);
  };

  return (
    <div className="provider-card">
      <div className="provider-header">
        <div className="provider-info">
          <h3>{provider.name}</h3>
          <p className="provider-description">{provider.description}</p>
        </div>
        <div className="provider-badges">
          {isDefault && <span className="badge badge-primary">Default</span>}
          {status?.has_key && <span className="badge badge-success">Configured</span>}
        </div>
      </div>

      <div className="provider-body">
        {/* API Key Section */}
        <div className="form-group">
          <label>API Key</label>
          {status?.has_key && !isEditing ? (
            <div className="key-display">
              <span className="key-hint">{status.key_hint}</span>
              <div className="key-actions">
                <button className="btn-text" onClick={() => setIsEditing(true)}>
                  Change
                </button>
                <button className="btn-text btn-danger" onClick={handleClear}>
                  Remove
                </button>
              </div>
            </div>
          ) : (
            <div className="key-input">
              <input
                type="password"
                placeholder={`Enter ${provider.name} API key...`}
                value={apiKey}
                onChange={(e) => setApiKey((e.target as HTMLInputElement).value)}
                className="input"
              />
              {isEditing && (
                <button className="btn-text" onClick={() => setIsEditing(false)}>
                  Cancel
                </button>
              )}
            </div>
          )}
        </div>

        {/* Model Selection */}
        <div className="form-group">
          <label>Default Model</label>
          <select
            value={selectedModel}
            onChange={(e) => setSelectedModel((e.target as HTMLSelectElement).value)}
            className="select"
          >
            {provider.models.map((model) => (
              <option key={model.id} value={model.id}>
                {model.name}
                {model.supports_vision && ' 👁'}
                {model.context_length && ` (${Math.floor(model.context_length / 1000)}k ctx)`}
              </option>
            ))}
          </select>
        </div>

        {/* Test Result */}
        {testResult && (
          <div className={`test-result ${testResult.success ? 'success' : 'error'}`}>
            {testResult.success ? '✓' : '✗'} {testResult.message}
          </div>
        )}
      </div>

      <div className="provider-footer">
        <div className="footer-left">
          {provider.docs_url && (
            <a href={provider.docs_url} target="_blank" rel="noopener noreferrer" className="btn-text">
              Documentation ↗
            </a>
          )}
        </div>
        <div className="footer-right">
          {!isDefault && status?.has_key && (
            <Button variant="secondary" onClick={onSetDefault} disabled={isSaving}>
              Set as Default
            </Button>
          )}
          {status?.has_key && (
            <Button variant="secondary" onClick={onTest} disabled={isSaving}>
              Test Connection
            </Button>
          )}
          {(apiKey || isEditing) && (
            <Button variant="primary" onClick={handleSave} disabled={localSaving || !apiKey}>
              {localSaving ? 'Saving...' : 'Save Key'}
            </Button>
          )}
        </div>
      </div>

      <style>{`
        .provider-card {
          background: var(--bg-secondary, #1a1a1a);
          border: 1px solid var(--border-color, #333);
          border-radius: 12px;
          padding: 20px;
          margin-bottom: 16px;
        }
        .provider-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 16px;
        }
        .provider-info h3 {
          margin: 0 0 4px 0;
          color: var(--text-primary, #fff);
          font-size: 1.1rem;
        }
        .provider-description {
          margin: 0;
          color: var(--text-secondary, #a1a1a1);
          font-size: 0.9rem;
        }
        .provider-badges {
          display: flex;
          gap: 8px;
        }
        .badge {
          padding: 4px 8px;
          border-radius: 4px;
          font-size: 0.75rem;
          font-weight: 500;
        }
        .badge-primary {
          background: var(--accent-color, #3b82f6);
          color: white;
        }
        .badge-success {
          background: #22c55e;
          color: white;
        }
        .provider-body {
          margin-bottom: 16px;
        }
        .form-group {
          margin-bottom: 12px;
        }
        .form-group label {
          display: block;
          margin-bottom: 6px;
          color: var(--text-secondary, #a1a1a1);
          font-size: 0.85rem;
        }
        .key-display {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 10px 14px;
          background: var(--bg-primary, #0a0a0a);
          border: 1px solid var(--border-color, #333);
          border-radius: 8px;
        }
        .key-hint {
          font-family: monospace;
          color: var(--text-secondary, #a1a1a1);
        }
        .key-actions {
          display: flex;
          gap: 12px;
        }
        .key-input {
          display: flex;
          gap: 8px;
          align-items: center;
        }
        .input {
          flex: 1;
          padding: 10px 14px;
          background: var(--bg-primary, #0a0a0a);
          border: 1px solid var(--border-color, #333);
          border-radius: 8px;
          color: var(--text-primary, #fff);
          font-size: 0.9rem;
        }
        .input:focus {
          outline: none;
          border-color: var(--accent-color, #3b82f6);
        }
        .select {
          width: 100%;
          padding: 10px 14px;
          background: var(--bg-primary, #0a0a0a);
          border: 1px solid var(--border-color, #333);
          border-radius: 8px;
          color: var(--text-primary, #fff);
          font-size: 0.9rem;
        }
        .select:focus {
          outline: none;
          border-color: var(--accent-color, #3b82f6);
        }
        .btn-text {
          background: none;
          border: none;
          color: var(--accent-color, #3b82f6);
          cursor: pointer;
          font-size: 0.85rem;
          padding: 4px 8px;
        }
        .btn-text:hover {
          text-decoration: underline;
        }
        .btn-danger {
          color: #ef4444;
        }
        .test-result {
          padding: 10px 14px;
          border-radius: 8px;
          font-size: 0.85rem;
          margin-top: 8px;
        }
        .test-result.success {
          background: rgba(34, 197, 94, 0.1);
          border: 1px solid rgba(34, 197, 94, 0.3);
          color: #22c55e;
        }
        .test-result.error {
          background: rgba(239, 68, 68, 0.1);
          border: 1px solid rgba(239, 68, 68, 0.3);
          color: #ef4444;
        }
        .provider-footer {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding-top: 16px;
          border-top: 1px solid var(--border-color, #333);
        }
        .footer-right {
          display: flex;
          gap: 8px;
        }
      `}</style>
    </div>
  );
}

export function SettingsPage({}: SettingsPageProps) {
  const {
    settings,
    providers,
    isLoading,
    isSaving,
    error,
    testResults,
    updateSettings,
    testProvider,
  } = useSettings();

  const handleSaveKey = useCallback(async (providerId: string, key: string, model?: string) => {
    const updates: Record<string, string | undefined> = {};
    updates[`${providerId}_key`] = key;
    if (model) {
      updates[`${providerId}_model`] = model;
    }
    await updateSettings(updates);
  }, [updateSettings]);

  const handleSetDefault = useCallback(async (providerId: string) => {
    await updateSettings({ default_provider: providerId });
  }, [updateSettings]);

  const getProviderStatus = (providerId: string) => {
    if (!settings) return undefined;
    return settings[providerId as keyof typeof settings] as ProviderStatus | undefined;
  };

  if (isLoading) {
    return (
      <div className="settings-page">
        <div className="loading">Loading settings...</div>
      </div>
    );
  }

  return (
    <div className="settings-page">
      <header className="settings-header">
        <button className="back-button" onClick={() => route('/chat')}>
          ← Back to Chat
        </button>
        <h1>Settings</h1>
        <p className="subtitle">Configure your LLM providers and preferences</p>
      </header>

      {error && (
        <div className="error-banner">
          {error}
        </div>
      )}

      <section className="settings-section">
        <h2>LLM Providers</h2>
        <p className="section-description">
          Configure API keys for the AI providers you want to use. Your keys are encrypted and stored securely.
        </p>

        <div className="providers-grid">
          {providers.map((provider) => (
            <ProviderCard
              key={provider.id}
              provider={provider}
              status={getProviderStatus(provider.id)}
              isDefault={settings?.default_provider === provider.id}
              onSetDefault={() => handleSetDefault(provider.id)}
              onSaveKey={(key, model) => handleSaveKey(provider.id, key, model)}
              onTest={() => testProvider(provider.id)}
              testResult={testResults[provider.id] || null}
              isSaving={isSaving}
            />
          ))}
        </div>
      </section>

      <section className="settings-section">
        <h2>Preferences</h2>
        
        <div className="preference-item">
          <label>Theme</label>
          <select
            value={settings?.theme || 'dark'}
            onChange={(e) => updateSettings({ theme: (e.target as HTMLSelectElement).value as any })}
            className="select"
          >
            <option value="dark">Dark</option>
            <option value="light">Light</option>
            <option value="system">System</option>
          </select>
        </div>
      </section>

      <style>{`
        .settings-page {
          min-height: 100vh;
          background: var(--bg-primary, #0a0a0a);
          padding: 24px;
          max-width: 900px;
          margin: 0 auto;
        }
        .settings-header {
          margin-bottom: 32px;
        }
        .back-button {
          background: none;
          border: none;
          color: var(--accent-color, #3b82f6);
          cursor: pointer;
          font-size: 0.9rem;
          padding: 8px 0;
          margin-bottom: 16px;
        }
        .back-button:hover {
          text-decoration: underline;
        }
        .settings-header h1 {
          margin: 0 0 8px 0;
          color: var(--text-primary, #fff);
          font-size: 2rem;
        }
        .subtitle {
          margin: 0;
          color: var(--text-secondary, #a1a1a1);
        }
        .error-banner {
          background: rgba(239, 68, 68, 0.1);
          border: 1px solid rgba(239, 68, 68, 0.3);
          color: #ef4444;
          padding: 12px 16px;
          border-radius: 8px;
          margin-bottom: 24px;
        }
        .settings-section {
          margin-bottom: 32px;
        }
        .settings-section h2 {
          color: var(--text-primary, #fff);
          font-size: 1.3rem;
          margin: 0 0 8px 0;
        }
        .section-description {
          color: var(--text-secondary, #a1a1a1);
          margin: 0 0 20px 0;
          font-size: 0.9rem;
        }
        .providers-grid {
          display: flex;
          flex-direction: column;
          gap: 16px;
        }
        .preference-item {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 16px;
          background: var(--bg-secondary, #1a1a1a);
          border: 1px solid var(--border-color, #333);
          border-radius: 12px;
        }
        .preference-item label {
          color: var(--text-primary, #fff);
          font-size: 0.95rem;
        }
        .preference-item .select {
          width: 200px;
        }
        .loading {
          display: flex;
          align-items: center;
          justify-content: center;
          height: 50vh;
          color: var(--text-secondary, #a1a1a1);
        }
      `}</style>
    </div>
  );
}
