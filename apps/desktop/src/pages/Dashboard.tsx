/// Dashboard page - main landing page
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

function Dashboard() {
  const [version, setVersion] = useState<string>("");
  const [dbHealth, setDbHealth] = useState<string>("");

  useEffect(() => {
    // Test Tauri commands
    invoke<string>("get_version")
      .then((v) => setVersion(v))
      .catch(console.error);

    invoke<string>("check_db_health")
      .then((health) => setDbHealth(health))
      .catch((err) => setDbHealth(`Error: ${err}`));
  }, []);

  return (
    <div>
      <h1>Dashboard</h1>
      <p>Welcome to Meet Scribe - Your bot-free meeting assistant</p>

      <div style={{ marginTop: "20px", padding: "20px", background: "white", borderRadius: "8px" }}>
        <h2>System Status</h2>
        <p><strong>Version:</strong> {version || "Loading..."}</p>
        <p><strong>Database:</strong> {dbHealth || "Checking..."}</p>
      </div>

      <div style={{ marginTop: "20px", padding: "20px", background: "white", borderRadius: "8px" }}>
        <h2>Quick Start</h2>
        <ol>
          <li>Configure your API keys in <strong>Settings</strong></li>
          <li>Start a meeting in Teams, Zoom, or Google Meet</li>
          <li>Click <strong>Active Meeting</strong> to begin recording</li>
        </ol>
      </div>
    </div>
  );
}

export default Dashboard;
