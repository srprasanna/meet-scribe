import { BrowserRouter, Routes, Route, Link } from "react-router-dom";
import Dashboard from "./pages/Dashboard";
import ActiveMeeting from "./pages/ActiveMeeting";
import MeetingHistory from "./pages/MeetingHistory";
import Settings from "./pages/Settings";
import "./App.css";

function App() {
  return (
    <BrowserRouter>
      <div className="app">
        <nav className="sidebar">
          <h1>Meet Scribe</h1>
          <ul>
            <li>
              <Link to="/">Dashboard</Link>
            </li>
            <li>
              <Link to="/meeting">Active Meeting</Link>
            </li>
            <li>
              <Link to="/history">History</Link>
            </li>
            <li>
              <Link to="/settings">Settings</Link>
            </li>
          </ul>
        </nav>

        <main className="main-content">
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/meeting" element={<ActiveMeeting />} />
            <Route path="/history" element={<MeetingHistory />} />
            <Route path="/settings" element={<Settings />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}

export default App;
