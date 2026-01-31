/**
 * Settings hook - manages user settings and API keys
 */
import { useState, useEffect, useCallback } from 'preact/hooks';

export interface ProviderStatus {
  has_key: boolean;
  key_hint: string | null;
  default_model: string | null;
  enabled: boolean;
}

export interface Settings {
  default_provider: string;
  openai: ProviderStatus;
  anthropic: ProviderStatus;
  google: ProviderStatus;
  openrouter: ProviderStatus;
  glm: ProviderStatus;
  theme: 'dark' | 'light' | 'system';
}

export interface ProviderInfo {
  id: string;
  name: string;
  description: string;
  models: {
    id: string;
    name: string;
    context_length?: number;
    supports_vision?: boolean;
  }[];
  docs_url?: string;
}

export interface UpdateSettingsRequest {
  default_provider?: string;
  openai_key?: string;
  openai_model?: string;
  anthropic_key?: string;
  anthropic_model?: string;
  google_key?: string;
  google_model?: string;
  openrouter_key?: string;
  openrouter_model?: string;
  glm_key?: string;
  glm_model?: string;
  theme?: 'dark' | 'light' | 'system';
}

export function useSettings() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<Record<string, { success: boolean; message: string } | null>>({});

  // Load settings on mount
  useEffect(() => {
    loadSettings();
    loadProviders();
  }, []);

  const loadSettings = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch('/api/settings');
      if (!response.ok) {
        throw new Error('Failed to load settings');
      }
      const data = await response.json();
      setSettings(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load settings');
      console.error('Failed to load settings:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const loadProviders = useCallback(async () => {
    try {
      const response = await fetch('/api/settings/providers');
      if (!response.ok) {
        throw new Error('Failed to load providers');
      }
      const data = await response.json();
      setProviders(data);
    } catch (err) {
      console.error('Failed to load providers:', err);
    }
  }, []);

  const updateSettings = useCallback(async (updates: UpdateSettingsRequest) => {
    setIsSaving(true);
    setError(null);
    try {
      const response = await fetch('/api/settings', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(updates),
      });
      
      const data = await response.json();
      
      if (!response.ok) {
        throw new Error(data.error || 'Failed to save settings');
      }
      
      setSettings(data.settings);
      return { success: true, message: data.message };
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to save settings';
      setError(message);
      return { success: false, message };
    } finally {
      setIsSaving(false);
    }
  }, []);

  const testProvider = useCallback(async (providerId: string) => {
    setTestResults(prev => ({ ...prev, [providerId]: null }));
    try {
      const response = await fetch(`/api/settings/test/${providerId}`, {
        method: 'POST',
      });
      const data = await response.json();
      const result = {
        success: data.success,
        message: data.success ? data.message : data.error,
      };
      setTestResults(prev => ({ ...prev, [providerId]: result }));
      return result;
    } catch (err) {
      const result = {
        success: false,
        message: err instanceof Error ? err.message : 'Connection test failed',
      };
      setTestResults(prev => ({ ...prev, [providerId]: result }));
      return result;
    }
  }, []);

  const clearTestResult = useCallback((providerId: string) => {
    setTestResults(prev => ({ ...prev, [providerId]: null }));
  }, []);

  return {
    settings,
    providers,
    isLoading,
    isSaving,
    error,
    testResults,
    loadSettings,
    updateSettings,
    testProvider,
    clearTestResult,
  };
}
