/// Settings page - configure API keys and services
function Settings() {
  return (
    <div>
      <h1>Settings</h1>
      <p>Configure your ASR and LLM services here.</p>

      <div style={{ marginTop: "20px", padding: "20px", background: "white", borderRadius: "8px" }}>
        <h2>ASR Services (Transcription)</h2>
        <p>Configure your speech recognition service API keys.</p>

        <div style={{ marginTop: "15px" }}>
          <h3>AssemblyAI</h3>
          <p><a href="https://www.assemblyai.com/" target="_blank">Sign up for AssemblyAI</a></p>
          <input
            type="password"
            placeholder="Enter API Key"
            style={{ width: "100%", padding: "8px", marginTop: "5px" }}
            disabled
          />
          <p style={{ fontSize: "12px", color: "#666", marginTop: "5px" }}>
            TODO: Implement keyring storage in Phase 1
          </p>
        </div>

        <div style={{ marginTop: "15px" }}>
          <h3>Deepgram</h3>
          <p><a href="https://deepgram.com/" target="_blank">Sign up for Deepgram</a></p>
          <input
            type="password"
            placeholder="Enter API Key"
            style={{ width: "100%", padding: "8px", marginTop: "5px" }}
            disabled
          />
        </div>
      </div>

      <div style={{ marginTop: "20px", padding: "20px", background: "white", borderRadius: "8px" }}>
        <h2>LLM Services (Insights & Summaries)</h2>
        <p>Configure your language model service API keys.</p>

        <div style={{ marginTop: "15px" }}>
          <h3>OpenAI</h3>
          <p><a href="https://platform.openai.com/" target="_blank">Get OpenAI API key</a></p>
          <input
            type="password"
            placeholder="Enter API Key"
            style={{ width: "100%", padding: "8px", marginTop: "5px" }}
            disabled
          />
        </div>

        <div style={{ marginTop: "15px" }}>
          <h3>Anthropic (Claude)</h3>
          <p><a href="https://console.anthropic.com/" target="_blank">Get Anthropic API key</a></p>
          <input
            type="password"
            placeholder="Enter API Key"
            style={{ width: "100%", padding: "8px", marginTop: "5px" }}
            disabled
          />
        </div>
      </div>

      <div style={{ marginTop: "20px", padding: "20px", background: "white", borderRadius: "8px" }}>
        <h2>TODO: Phase 1 (Keyring Implementation)</h2>
        <ul>
          <li>Implement secure API key storage using OS keychain</li>
          <li>Add Tauri commands for saving/retrieving keys</li>
          <li>Hook up these inputs to keychain storage</li>
          <li>Add service activation toggles</li>
          <li>Add model selection dropdowns</li>
        </ul>
      </div>
    </div>
  );
}

export default Settings;
