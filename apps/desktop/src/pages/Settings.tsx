import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

/// Settings page - configure API keys and services

interface ApiKeyStatus {
  has_key: boolean;
  masked_key?: string;
}

interface ServiceConfig {
  id?: number;
  service_type: string;
  provider: string;
  is_active: boolean;
  settings?: string;
  has_api_key: boolean;
}

// ASR Provider configurations
const ASR_PROVIDERS = {
  assemblyai: {
    name: "AssemblyAI",
    signupUrl: "https://www.assemblyai.com/",
    models: [
      { value: "best", label: "Best (Most accurate, slower)" },
      { value: "nano", label: "Nano (Fastest, less accurate)" },
    ],
  },
  deepgram: {
    name: "Deepgram",
    signupUrl: "https://deepgram.com/",
    models: [
      { value: "nova-2", label: "Nova 2 (Latest, most accurate)" },
      { value: "nova", label: "Nova (Fast and accurate)" },
      { value: "enhanced", label: "Enhanced (Good balance)" },
      { value: "base", label: "Base (Fastest)" },
    ],
  },
};

// LLM Provider configurations
const LLM_PROVIDERS = {
  openai: {
    name: "OpenAI",
    signupUrl: "https://platform.openai.com/",
    models: [
      { value: "gpt-4-turbo", label: "GPT-4 Turbo (Most capable)" },
      { value: "gpt-4", label: "GPT-4 (High quality)" },
      { value: "gpt-3.5-turbo", label: "GPT-3.5 Turbo (Fast, economical)" },
    ],
  },
  anthropic: {
    name: "Anthropic (Claude)",
    signupUrl: "https://console.anthropic.com/",
    models: [
      { value: "claude-3-opus-20240229", label: "Claude 3 Opus (Most capable)" },
      { value: "claude-3-sonnet-20240229", label: "Claude 3 Sonnet (Balanced)" },
      { value: "claude-3-haiku-20240307", label: "Claude 3 Haiku (Fastest)" },
    ],
  },
};

function Settings() {
  // ASR state
  const [asrConfigs, setAsrConfigs] = useState<Record<string, ServiceConfig>>({});
  const [asrApiKeys, setAsrApiKeys] = useState<Record<string, string>>({});
  const [asrKeyStatuses, setAsrKeyStatuses] = useState<Record<string, ApiKeyStatus>>({});
  const [asrModels, setAsrModels] = useState<Record<string, string>>({});

  // LLM state
  const [llmConfigs, setLlmConfigs] = useState<Record<string, ServiceConfig>>({});
  const [llmApiKeys, setLlmApiKeys] = useState<Record<string, string>>({});
  const [llmKeyStatuses, setLlmKeyStatuses] = useState<Record<string, ApiKeyStatus>>({});
  const [llmModels, setLlmModels] = useState<Record<string, string>>({});

  // Loading and error states
  const [loading, setLoading] = useState<Record<string, boolean>>({});
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  // Load all configurations on mount
  useEffect(() => {
    loadAllConfigs();
  }, []);

  const loadAllConfigs = async () => {
    try {
      // Load ASR configs
      for (const provider of Object.keys(ASR_PROVIDERS)) {
        await loadConfig("asr", provider);
      }

      // Load LLM configs
      for (const provider of Object.keys(LLM_PROVIDERS)) {
        await loadConfig("llm", provider);
      }
    } catch (err) {
      console.error("Error loading configs:", err);
      setError(`Failed to load configurations: ${err}`);
    }
  };

  const loadConfig = async (serviceType: string, provider: string) => {
    try {
      // Load API key status
      const keyStatus = await invoke<ApiKeyStatus>("get_api_key_status", {
        request: { service_type: serviceType, provider },
      });

      // Load service configuration
      const config = await invoke<ServiceConfig | null>("get_service_config", {
        serviceType,
        provider,
      });

      // Parse settings to get model
      let model = "";
      if (config?.settings) {
        try {
          const settings = JSON.parse(config.settings);
          model = settings.model || "";
        } catch (e) {
          console.error("Error parsing settings:", e);
        }
      }

      // Update state
      if (serviceType === "asr") {
        setAsrKeyStatuses((prev) => ({ ...prev, [provider]: keyStatus }));
        if (config) {
          setAsrConfigs((prev) => ({ ...prev, [provider]: config }));
          setAsrModels((prev) => ({ ...prev, [provider]: model }));
        }
      } else {
        setLlmKeyStatuses((prev) => ({ ...prev, [provider]: keyStatus }));
        if (config) {
          setLlmConfigs((prev) => ({ ...prev, [provider]: config }));
          setLlmModels((prev) => ({ ...prev, [provider]: model }));
        }
      }
    } catch (err) {
      console.error(`Error loading config for ${serviceType}:${provider}:`, err);
    }
  };

  const handleSaveApiKey = async (serviceType: string, provider: string) => {
    const key = serviceType === "asr" ? asrApiKeys[provider] : llmApiKeys[provider];

    if (!key || key.trim() === "") {
      setError("Please enter an API key");
      return;
    }

    setLoading((prev) => ({ ...prev, [`${serviceType}_${provider}`]: true }));
    setError(null);
    setSuccess(null);

    try {
      // Save API key to keychain
      await invoke("save_api_key", {
        request: {
          service_type: serviceType,
          provider,
          api_key: key,
        },
      });

      // Clear the input
      if (serviceType === "asr") {
        setAsrApiKeys((prev) => ({ ...prev, [provider]: "" }));
      } else {
        setLlmApiKeys((prev) => ({ ...prev, [provider]: "" }));
      }

      // Reload config to update status
      await loadConfig(serviceType, provider);

      setSuccess(`API key saved successfully for ${provider}`);
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(`Failed to save API key: ${err}`);
    } finally {
      setLoading((prev) => ({ ...prev, [`${serviceType}_${provider}`]: false }));
    }
  };

  const handleSaveModel = async (serviceType: string, provider: string, model: string) => {
    setLoading((prev) => ({ ...prev, [`model_${serviceType}_${provider}`]: true }));
    setError(null);

    try {
      // Create settings JSON
      const settings = JSON.stringify({ model });

      // Check if config exists
      const configs = serviceType === "asr" ? asrConfigs : llmConfigs;
      const existingConfig = configs[provider];

      // Save configuration
      await invoke("save_service_config", {
        request: {
          service_type: serviceType,
          provider,
          is_active: existingConfig?.is_active || false,
          settings,
        },
      });

      // Reload config
      await loadConfig(serviceType, provider);

      setSuccess(`Model preference saved for ${provider}`);
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(`Failed to save model: ${err}`);
    } finally {
      setLoading((prev) => ({ ...prev, [`model_${serviceType}_${provider}`]: false }));
    }
  };

  const handleActivateService = async (serviceType: string, provider: string) => {
    setLoading((prev) => ({ ...prev, [`activate_${serviceType}_${provider}`]: true }));
    setError(null);

    try {
      await invoke("activate_service", {
        serviceType,
        provider,
      });

      // Reload all configs of this service type
      if (serviceType === "asr") {
        for (const p of Object.keys(ASR_PROVIDERS)) {
          await loadConfig("asr", p);
        }
      } else {
        for (const p of Object.keys(LLM_PROVIDERS)) {
          await loadConfig("llm", p);
        }
      }

      setSuccess(`${provider} activated successfully`);
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(`Failed to activate service: ${err}`);
    } finally {
      setLoading((prev) => ({ ...prev, [`activate_${serviceType}_${provider}`]: false }));
    }
  };

  const handleDeleteApiKey = async (serviceType: string, provider: string) => {
    if (!confirm(`Are you sure you want to delete the API key for ${provider}?`)) {
      return;
    }

    setLoading((prev) => ({ ...prev, [`delete_${serviceType}_${provider}`]: true }));
    setError(null);

    try {
      await invoke("delete_api_key", {
        serviceType,
        provider,
      });

      // Reload config
      await loadConfig(serviceType, provider);

      setSuccess(`API key deleted for ${provider}`);
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(`Failed to delete API key: ${err}`);
    } finally {
      setLoading((prev) => ({ ...prev, [`delete_${serviceType}_${provider}`]: false }));
    }
  };

  const renderServiceCard = (
    serviceType: string,
    provider: string,
    providerConfig: { name: string; signupUrl: string; models: Array<{ value: string; label: string }> }
  ) => {
    const configs = serviceType === "asr" ? asrConfigs : llmConfigs;
    const keyStatuses = serviceType === "asr" ? asrKeyStatuses : llmKeyStatuses;
    const apiKeys = serviceType === "asr" ? asrApiKeys : llmApiKeys;
    const models = serviceType === "asr" ? asrModels : llmModels;

    const config = configs[provider];
    const keyStatus = keyStatuses[provider];
    const apiKey = apiKeys[provider] || "";
    const selectedModel = models[provider] || "";

    const hasKey = keyStatus?.has_key || false;
    const isActive = config?.is_active || false;
    const isLoading = loading[`${serviceType}_${provider}`] || false;

    return (
      <div
        key={provider}
        style={{
          marginTop: "20px",
          padding: "20px",
          background: isActive ? "#e8f5e9" : "white",
          borderRadius: "8px",
          border: isActive ? "2px solid #4caf50" : "1px solid #ddd",
        }}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <h3 style={{ margin: 0 }}>{providerConfig.name}</h3>
          {isActive && (
            <span
              style={{
                padding: "4px 12px",
                background: "#4caf50",
                color: "white",
                borderRadius: "12px",
                fontSize: "12px",
                fontWeight: "bold",
              }}
            >
              ACTIVE
            </span>
          )}
        </div>

        <p style={{ marginTop: "8px", fontSize: "14px", color: "#666" }}>
          <a href={providerConfig.signupUrl} target="_blank" rel="noopener noreferrer">
            Sign up for {providerConfig.name}
          </a>
        </p>

        {/* API Key Section */}
        <div style={{ marginTop: "15px" }}>
          <label style={{ display: "block", fontWeight: "bold", marginBottom: "5px" }}>API Key</label>
          {hasKey ? (
            <div>
              <div
                style={{
                  padding: "8px",
                  background: "#f5f5f5",
                  borderRadius: "4px",
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                }}
              >
                <span style={{ fontFamily: "monospace" }}>{keyStatus.masked_key}</span>
                <button
                  onClick={() => handleDeleteApiKey(serviceType, provider)}
                  disabled={loading[`delete_${serviceType}_${provider}`]}
                  style={{
                    padding: "4px 12px",
                    background: "#f44336",
                    color: "white",
                    border: "none",
                    borderRadius: "4px",
                    cursor: "pointer",
                    fontSize: "12px",
                  }}
                >
                  {loading[`delete_${serviceType}_${provider}`] ? "Deleting..." : "Delete"}
                </button>
              </div>
            </div>
          ) : (
            <div style={{ display: "flex", gap: "8px" }}>
              <input
                type="password"
                placeholder="Enter API Key"
                value={apiKey}
                onChange={(e) => {
                  if (serviceType === "asr") {
                    setAsrApiKeys((prev) => ({ ...prev, [provider]: e.target.value }));
                  } else {
                    setLlmApiKeys((prev) => ({ ...prev, [provider]: e.target.value }));
                  }
                }}
                style={{ flex: 1, padding: "8px", borderRadius: "4px", border: "1px solid #ddd" }}
              />
              <button
                onClick={() => handleSaveApiKey(serviceType, provider)}
                disabled={isLoading}
                style={{
                  padding: "8px 16px",
                  background: "#2196f3",
                  color: "white",
                  border: "none",
                  borderRadius: "4px",
                  cursor: isLoading ? "not-allowed" : "pointer",
                }}
              >
                {isLoading ? "Saving..." : "Save"}
              </button>
            </div>
          )}
        </div>

        {/* Model Selection */}
        {hasKey && (
          <div style={{ marginTop: "15px" }}>
            <label style={{ display: "block", fontWeight: "bold", marginBottom: "5px" }}>Model</label>
            <select
              value={selectedModel}
              onChange={(e) => {
                const model = e.target.value;
                if (serviceType === "asr") {
                  setAsrModels((prev) => ({ ...prev, [provider]: model }));
                } else {
                  setLlmModels((prev) => ({ ...prev, [provider]: model }));
                }
                handleSaveModel(serviceType, provider, model);
              }}
              disabled={loading[`model_${serviceType}_${provider}`]}
              style={{ width: "100%", padding: "8px", borderRadius: "4px", border: "1px solid #ddd" }}
            >
              <option value="">Select a model</option>
              {providerConfig.models.map((model) => (
                <option key={model.value} value={model.value}>
                  {model.label}
                </option>
              ))}
            </select>
          </div>
        )}

        {/* Activation Button */}
        {hasKey && (
          <div style={{ marginTop: "15px" }}>
            <button
              onClick={() => handleActivateService(serviceType, provider)}
              disabled={isActive || loading[`activate_${serviceType}_${provider}`]}
              style={{
                width: "100%",
                padding: "10px",
                background: isActive ? "#9e9e9e" : "#4caf50",
                color: "white",
                border: "none",
                borderRadius: "4px",
                cursor: isActive ? "not-allowed" : "pointer",
                fontWeight: "bold",
              }}
            >
              {loading[`activate_${serviceType}_${provider}`]
                ? "Activating..."
                : isActive
                ? "Currently Active"
                : "Activate This Service"}
            </button>
          </div>
        )}
      </div>
    );
  };

  return (
    <div style={{ padding: "20px" }}>
      <h1>Settings</h1>
      <p>Configure your ASR and LLM services with API keys stored securely in your OS keychain.</p>

      {/* Error/Success Messages */}
      {error && (
        <div
          style={{
            padding: "12px",
            background: "#ffebee",
            color: "#c62828",
            borderRadius: "4px",
            marginBottom: "20px",
          }}
        >
          {error}
        </div>
      )}

      {success && (
        <div
          style={{
            padding: "12px",
            background: "#e8f5e9",
            color: "#2e7d32",
            borderRadius: "4px",
            marginBottom: "20px",
          }}
        >
          {success}
        </div>
      )}

      {/* ASR Services Section */}
      <div style={{ marginTop: "20px" }}>
        <h2>ASR Services (Transcription)</h2>
        <p>Configure your speech recognition service. Only one ASR service can be active at a time.</p>

        {Object.entries(ASR_PROVIDERS).map(([provider, config]) =>
          renderServiceCard("asr", provider, config)
        )}
      </div>

      {/* LLM Services Section */}
      <div style={{ marginTop: "40px" }}>
        <h2>LLM Services (Insights & Summaries)</h2>
        <p>Configure your language model service. Only one LLM service can be active at a time.</p>

        {Object.entries(LLM_PROVIDERS).map(([provider, config]) =>
          renderServiceCard("llm", provider, config)
        )}
      </div>

      {/* Security Note */}
      <div style={{ marginTop: "40px", padding: "15px", background: "#fff3e0", borderRadius: "8px" }}>
        <h3 style={{ marginTop: 0 }}>🔒 Security</h3>
        <p style={{ marginBottom: 0, fontSize: "14px" }}>
          Your API keys are stored securely in your operating system's keychain (Windows Credential Manager on
          Windows, Secret Service on Linux). They are never stored in plain text or in the database.
        </p>
      </div>
    </div>
  );
}

export default Settings;
