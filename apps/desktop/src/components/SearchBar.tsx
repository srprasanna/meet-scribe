import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Transcript {
  id: number;
  meeting_id: number;
  participant_id?: number;
  participant_name?: string;
  speaker_label?: string;
  timestamp_ms: number;
  text: string;
  confidence?: number;
  created_at: number;
}

interface TranscriptSearchResult {
  transcript: Transcript;
  meeting_title?: string;
  meeting_platform: string;
  rank: number;
}

interface Insight {
  id: number;
  meeting_id: number;
  insight_type: string;
  content: string;
  metadata?: string;
  created_at: number;
}

interface InsightSearchResult {
  insight: Insight;
  meeting_title?: string;
  meeting_platform: string;
  rank: number;
}

interface Meeting {
  id: number;
  platform: string;
  title?: string;
  start_time: number;
  end_time?: number;
  participant_count?: number;
  audio_file_path?: string;
  created_at: number;
}

interface SearchResults {
  transcripts: TranscriptSearchResult[];
  insights: InsightSearchResult[];
  meetings: Meeting[];
}

interface SearchBarProps {
  onMeetingSelect?: (meetingId: number) => void;
}

export function SearchBar({ onMeetingSelect }: SearchBarProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResults | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<"all" | "transcripts" | "insights" | "meetings">("all");
  const [isOpen, setIsOpen] = useState(false);

  // Debounced search function
  useEffect(() => {
    const timeoutId = setTimeout(async () => {
      if (query.trim().length < 2) {
        setResults(null);
        setIsOpen(false);
        return;
      }

      setLoading(true);
      setError(null);
      setIsOpen(true);

      try {
        const searchResults = await invoke<SearchResults>("search_all", {
          query: query.trim(),
          limit: 50,
        });
        setResults(searchResults);
      } catch (err) {
        setError(`Search failed: ${err}`);
        console.error(err);
      } finally {
        setLoading(false);
      }
    }, 300); // 300ms debounce

    return () => clearTimeout(timeoutId);
  }, [query]);

  const formatTimestamp = (ms: number): string => {
    const secs = Math.floor(ms / 1000);
    const mins = Math.floor(secs / 60);
    const hours = Math.floor(mins / 60);
    const remainingMins = mins % 60;
    const remainingSecs = secs % 60;

    if (hours > 0) {
      return `${hours}:${remainingMins.toString().padStart(2, "0")}:${remainingSecs.toString().padStart(2, "0")}`;
    }
    return `${remainingMins}:${remainingSecs.toString().padStart(2, "0")}`;
  };

  const formatDate = (timestamp: number): string => {
    return new Date(timestamp * 1000).toLocaleDateString();
  };

  const totalResults =
    (results?.transcripts.length || 0) + (results?.insights.length || 0) + (results?.meetings.length || 0);

  return (
    <div style={{ position: "relative", width: "100%", maxWidth: "600px" }}>
      {/* Search Input */}
      <div style={{ position: "relative" }}>
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search meetings, transcripts, insights... (Ctrl+K)"
          style={{
            width: "100%",
            padding: "10px 40px 10px 16px",
            fontSize: "14px",
            border: "1px solid #ddd",
            borderRadius: "8px",
            outline: "none",
            transition: "border-color 0.2s",
          }}
          onFocus={() => query.trim().length >= 2 && setIsOpen(true)}
        />
        {loading && (
          <div
            style={{
              position: "absolute",
              right: "12px",
              top: "50%",
              transform: "translateY(-50%)",
              fontSize: "16px",
            }}
          >
            ⏳
          </div>
        )}
        {query && !loading && (
          <button
            onClick={() => {
              setQuery("");
              setResults(null);
              setIsOpen(false);
            }}
            style={{
              position: "absolute",
              right: "12px",
              top: "50%",
              transform: "translateY(-50%)",
              background: "none",
              border: "none",
              cursor: "pointer",
              fontSize: "16px",
              padding: "4px",
            }}
          >
            ✕
          </button>
        )}
      </div>

      {/* Search Results Dropdown */}
      {isOpen && results && totalResults > 0 && (
        <div
          style={{
            position: "absolute",
            top: "calc(100% + 4px)",
            left: 0,
            right: 0,
            background: "white",
            border: "1px solid #ddd",
            borderRadius: "8px",
            boxShadow: "0 4px 12px rgba(0,0,0,0.15)",
            maxHeight: "500px",
            overflow: "hidden",
            zIndex: 1000,
          }}
        >
          {/* Tab Navigation */}
          <div style={{ display: "flex", borderBottom: "1px solid #eee", padding: "8px" }}>
            <button
              onClick={() => setActiveTab("all")}
              style={{
                flex: 1,
                padding: "8px",
                background: activeTab === "all" ? "#0078d4" : "transparent",
                color: activeTab === "all" ? "white" : "#333",
                border: "none",
                borderRadius: "4px",
                cursor: "pointer",
                fontSize: "13px",
                fontWeight: activeTab === "all" ? "bold" : "normal",
              }}
            >
              All ({totalResults})
            </button>
            <button
              onClick={() => setActiveTab("transcripts")}
              style={{
                flex: 1,
                padding: "8px",
                background: activeTab === "transcripts" ? "#0078d4" : "transparent",
                color: activeTab === "transcripts" ? "white" : "#333",
                border: "none",
                borderRadius: "4px",
                cursor: "pointer",
                fontSize: "13px",
                fontWeight: activeTab === "transcripts" ? "bold" : "normal",
              }}
            >
              Transcripts ({results.transcripts.length})
            </button>
            <button
              onClick={() => setActiveTab("insights")}
              style={{
                flex: 1,
                padding: "8px",
                background: activeTab === "insights" ? "#0078d4" : "transparent",
                color: activeTab === "insights" ? "white" : "#333",
                border: "none",
                borderRadius: "4px",
                cursor: "pointer",
                fontSize: "13px",
                fontWeight: activeTab === "insights" ? "bold" : "normal",
              }}
            >
              Insights ({results.insights.length})
            </button>
            <button
              onClick={() => setActiveTab("meetings")}
              style={{
                flex: 1,
                padding: "8px",
                background: activeTab === "meetings" ? "#0078d4" : "transparent",
                color: activeTab === "meetings" ? "white" : "#333",
                border: "none",
                borderRadius: "4px",
                cursor: "pointer",
                fontSize: "13px",
                fontWeight: activeTab === "meetings" ? "bold" : "normal",
              }}
            >
              Meetings ({results.meetings.length})
            </button>
          </div>

          {/* Results List */}
          <div style={{ maxHeight: "400px", overflowY: "auto" }}>
            {/* Transcripts */}
            {(activeTab === "all" || activeTab === "transcripts") &&
              results.transcripts.map((result, idx) => (
                <div
                  key={`transcript-${idx}`}
                  onClick={() => {
                    onMeetingSelect?.(result.transcript.meeting_id);
                    setIsOpen(false);
                  }}
                  style={{
                    padding: "12px 16px",
                    borderBottom: "1px solid #f0f0f0",
                    cursor: "pointer",
                    transition: "background 0.2s",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = "#f8f9fa";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = "white";
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "4px" }}>
                    <span style={{ fontSize: "12px", color: "#666" }}>📝 Transcript</span>
                    <span style={{ fontSize: "11px", color: "#999" }}>•</span>
                    <span style={{ fontSize: "12px", color: "#666" }}>
                      {result.meeting_title || "Untitled"} ({result.meeting_platform})
                    </span>
                    <span style={{ fontSize: "11px", color: "#999" }}>•</span>
                    <span style={{ fontSize: "12px", color: "#666" }}>
                      {formatTimestamp(result.transcript.timestamp_ms)}
                    </span>
                  </div>
                  <div style={{ fontSize: "13px", color: "#333" }}>
                    <strong>{result.transcript.participant_name || result.transcript.speaker_label || "Unknown"}:</strong>{" "}
                    {result.transcript.text.substring(0, 150)}
                    {result.transcript.text.length > 150 && "..."}
                  </div>
                </div>
              ))}

            {/* Insights */}
            {(activeTab === "all" || activeTab === "insights") &&
              results.insights.map((result, idx) => (
                <div
                  key={`insight-${idx}`}
                  onClick={() => {
                    onMeetingSelect?.(result.insight.meeting_id);
                    setIsOpen(false);
                  }}
                  style={{
                    padding: "12px 16px",
                    borderBottom: "1px solid #f0f0f0",
                    cursor: "pointer",
                    transition: "background 0.2s",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = "#f8f9fa";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = "white";
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "4px" }}>
                    <span style={{ fontSize: "12px", color: "#666" }}>
                      {result.insight.insight_type === "summary"
                        ? "📋"
                        : result.insight.insight_type === "action_item"
                        ? "✅"
                        : result.insight.insight_type === "key_point"
                        ? "💡"
                        : "🎯"}{" "}
                      {result.insight.insight_type.replace("_", " ").toUpperCase()}
                    </span>
                    <span style={{ fontSize: "11px", color: "#999" }}>•</span>
                    <span style={{ fontSize: "12px", color: "#666" }}>
                      {result.meeting_title || "Untitled"} ({result.meeting_platform})
                    </span>
                  </div>
                  <div style={{ fontSize: "13px", color: "#333" }}>
                    {result.insight.content.substring(0, 150)}
                    {result.insight.content.length > 150 && "..."}
                  </div>
                </div>
              ))}

            {/* Meetings */}
            {(activeTab === "all" || activeTab === "meetings") &&
              results.meetings.map((meeting, idx) => (
                <div
                  key={`meeting-${idx}`}
                  onClick={() => {
                    onMeetingSelect?.(meeting.id);
                    setIsOpen(false);
                  }}
                  style={{
                    padding: "12px 16px",
                    borderBottom: "1px solid #f0f0f0",
                    cursor: "pointer",
                    transition: "background 0.2s",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = "#f8f9fa";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = "white";
                  }}
                >
                  <div style={{ fontSize: "14px", fontWeight: "bold", color: "#333", marginBottom: "4px" }}>
                    {meeting.title || "Untitled Meeting"}
                  </div>
                  <div style={{ fontSize: "12px", color: "#666" }}>
                    {meeting.platform.toUpperCase()} • {formatDate(meeting.start_time)}
                    {meeting.participant_count && ` • ${meeting.participant_count} participants`}
                  </div>
                </div>
              ))}
          </div>
        </div>
      )}

      {/* No Results */}
      {isOpen && results && totalResults === 0 && (
        <div
          style={{
            position: "absolute",
            top: "calc(100% + 4px)",
            left: 0,
            right: 0,
            background: "white",
            border: "1px solid #ddd",
            borderRadius: "8px",
            boxShadow: "0 4px 12px rgba(0,0,0,0.15)",
            padding: "20px",
            textAlign: "center",
            color: "#666",
            zIndex: 1000,
          }}
        >
          No results found for "{query}"
        </div>
      )}

      {/* Error */}
      {error && (
        <div
          style={{
            position: "absolute",
            top: "calc(100% + 4px)",
            left: 0,
            right: 0,
            background: "#fff3cd",
            border: "1px solid #ffc107",
            borderRadius: "8px",
            padding: "12px",
            color: "#856404",
            zIndex: 1000,
          }}
        >
          {error}
        </div>
      )}
    </div>
  );
}
