import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  startTranscription,
  getTranscriptionStatus,
  getTranscripts,
  isTranscriptionAvailable,
} from "../api/transcription";
import type { Transcript } from "../types";

interface Meeting {
  id: number;
  platform: string;
  title?: string;
  start_time: number;
  end_time?: number;
  participant_count?: number;
  created_at: number;
}

const PLATFORMS = {
  teams: { label: "Microsoft Teams", icon: "🟦" },
  zoom: { label: "Zoom", icon: "🔵" },
  meet: { label: "Google Meet", icon: "🟢" },
};

function MeetingHistory() {
  const [meetings, setMeetings] = useState<Meeting[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedMeeting, setSelectedMeeting] = useState<Meeting | null>(null);
  const [transcriptionAvailable, setTranscriptionAvailable] = useState<boolean>(false);
  const [transcribingMeetingId, setTranscribingMeetingId] = useState<number | null>(null);
  const [transcripts, setTranscripts] = useState<{ [meetingId: number]: Transcript[] }>({});

  useEffect(() => {
    loadMeetings();
    checkTranscriptionAvailability();
  }, []);

  // Poll for transcription status every 3 seconds
  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const status = await getTranscriptionStatus();
        setTranscribingMeetingId(status);

        // If transcription just completed, reload transcripts for that meeting
        if (transcribingMeetingId !== null && status === null) {
          await loadTranscriptsForMeeting(transcribingMeetingId);
        }
      } catch (err) {
        console.error("Failed to check transcription status:", err);
      }
    }, 3000);

    return () => clearInterval(interval);
  }, [transcribingMeetingId]);

  const checkTranscriptionAvailability = async () => {
    try {
      const available = await isTranscriptionAvailable();
      setTranscriptionAvailable(available);
    } catch (err) {
      console.error("Failed to check transcription availability:", err);
    }
  };

  const loadMeetings = async () => {
    setLoading(true);
    setError(null);

    try {
      const history = await invoke<Meeting[]>("get_meeting_history", {
        limit: 50,
      });
      setMeetings(history);

      // Load transcripts for all meetings
      for (const meeting of history) {
        if (meeting.id && meeting.end_time) {
          await loadTranscriptsForMeeting(meeting.id);
        }
      }
    } catch (err) {
      setError(`Failed to load meetings: ${err}`);
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  const loadTranscriptsForMeeting = async (meetingId: number) => {
    try {
      const transcriptList = await getTranscripts(meetingId);
      setTranscripts((prev) => ({ ...prev, [meetingId]: transcriptList }));
    } catch (err) {
      console.error(`Failed to load transcripts for meeting ${meetingId}:`, err);
    }
  };

  const handleStartTranscription = async (meetingId: number) => {
    if (!transcriptionAvailable) {
      setError("Transcription service not configured. Please configure an ASR service in Settings.");
      return;
    }

    try {
      setError(null);
      await startTranscription(meetingId);
      setTranscribingMeetingId(meetingId);
    } catch (err) {
      setError(`Failed to start transcription: ${err}`);
      console.error(err);
    }
  };

  const handleDeleteMeeting = async (meetingId: number) => {
    if (!confirm("Are you sure you want to delete this meeting?")) {
      return;
    }

    try {
      await invoke("delete_meeting", { meetingId });
      setMeetings(meetings.filter((m) => m.id !== meetingId));
      if (selectedMeeting?.id === meetingId) {
        setSelectedMeeting(null);
      }
    } catch (err) {
      setError(`Failed to delete meeting: ${err}`);
      console.error(err);
    }
  };

  const formatDate = (timestamp: number): string => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  const formatDuration = (startTime: number, endTime?: number): string => {
    if (!endTime) return "In progress";
    const durationSeconds = endTime - startTime;
    const hours = Math.floor(durationSeconds / 3600);
    const minutes = Math.floor((durationSeconds % 3600) / 60);
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes}m`;
  };

  return (
    <div style={{ padding: "20px", maxWidth: "1200px", margin: "0 auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "20px" }}>
        <h1 style={{ margin: 0 }}>Meeting History</h1>
        <button
          onClick={loadMeetings}
          style={{
            padding: "8px 16px",
            background: "#0078d4",
            color: "white",
            border: "none",
            borderRadius: "6px",
            cursor: "pointer",
            fontSize: "14px",
          }}
        >
          🔄 Refresh
        </button>
      </div>

      {error && (
        <div
          style={{
            padding: "12px",
            background: "#fee",
            border: "1px solid #fcc",
            borderRadius: "6px",
            marginBottom: "20px",
            color: "#c33",
          }}
        >
          {error}
        </div>
      )}

      {loading ? (
        <div style={{ textAlign: "center", padding: "40px", color: "#666" }}>
          Loading meetings...
        </div>
      ) : meetings.length === 0 ? (
        <div
          style={{
            background: "white",
            padding: "40px",
            borderRadius: "8px",
            boxShadow: "0 2px 8px rgba(0,0,0,0.1)",
            textAlign: "center",
            color: "#666",
          }}
        >
          <div style={{ fontSize: "48px", marginBottom: "16px" }}>📋</div>
          <h2 style={{ marginTop: 0 }}>No Meetings Yet</h2>
          <p>Start recording your first meeting from the Active Meeting page.</p>
        </div>
      ) : (
        <div
          style={{
            background: "white",
            padding: "24px",
            borderRadius: "8px",
            boxShadow: "0 2px 8px rgba(0,0,0,0.1)",
          }}
        >
          <h2 style={{ marginTop: 0, marginBottom: "16px" }}>
            Recent Meetings ({meetings.length})
          </h2>

          <div style={{ display: "grid", gap: "12px" }}>
            {meetings.map((meeting) => {
              const platformInfo = PLATFORMS[meeting.platform as keyof typeof PLATFORMS] || {
                label: meeting.platform,
                icon: "📹",
              };

              return (
                <div
                  key={meeting.id}
                  style={{
                    padding: "16px",
                    border: "1px solid #e0e0e0",
                    borderRadius: "8px",
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                    cursor: "pointer",
                    transition: "all 0.2s",
                    background: selectedMeeting?.id === meeting.id ? "#f0f7ff" : "white",
                  }}
                  onClick={() => setSelectedMeeting(meeting)}
                  onMouseEnter={(e) => {
                    if (selectedMeeting?.id !== meeting.id) {
                      e.currentTarget.style.background = "#f9f9f9";
                    }
                  }}
                  onMouseLeave={(e) => {
                    if (selectedMeeting?.id !== meeting.id) {
                      e.currentTarget.style.background = "white";
                    }
                  }}
                >
                  <div style={{ flex: 1 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: "12px", marginBottom: "8px" }}>
                      <span style={{ fontSize: "24px" }}>{platformInfo.icon}</span>
                      <div>
                        <div style={{ fontWeight: "500", fontSize: "16px" }}>
                          {meeting.title || "Untitled Meeting"}
                        </div>
                        <div style={{ fontSize: "13px", color: "#666" }}>
                          {platformInfo.label}
                        </div>
                      </div>
                    </div>
                    <div style={{ fontSize: "13px", color: "#666", marginLeft: "36px" }}>
                      <div>
                        📅 {formatDate(meeting.start_time)}
                      </div>
                      <div style={{ marginTop: "4px" }}>
                        ⏱️ Duration: {formatDuration(meeting.start_time, meeting.end_time)}
                      </div>
                      {meeting.participant_count !== undefined && meeting.participant_count > 0 && (
                        <div style={{ marginTop: "4px" }}>
                          👥 {meeting.participant_count} participant{meeting.participant_count !== 1 ? "s" : ""}
                        </div>
                      )}
                    </div>
                  </div>

                  <div style={{ display: "flex", gap: "8px" }}>
                    {/* Transcription button - only show for completed meetings */}
                    {meeting.end_time && (() => {
                      const hasTranscripts = transcripts[meeting.id]?.length > 0;
                      const isTranscribing = transcribingMeetingId === meeting.id;

                      return (
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            if (!hasTranscripts && !isTranscribing) {
                              handleStartTranscription(meeting.id);
                            }
                          }}
                          disabled={!transcriptionAvailable || isTranscribing || hasTranscripts}
                          style={{
                            padding: "8px 16px",
                            background: hasTranscripts
                              ? "#28a745"
                              : isTranscribing
                              ? "#ffc107"
                              : transcriptionAvailable
                              ? "#6c757d"
                              : "#d3d3d3",
                            color: "white",
                            border: "none",
                            borderRadius: "6px",
                            cursor: transcriptionAvailable && !isTranscribing && !hasTranscripts ? "pointer" : "not-allowed",
                            fontSize: "13px",
                            opacity: !transcriptionAvailable ? 0.6 : 1,
                          }}
                          title={
                            !transcriptionAvailable
                              ? "Configure ASR service in Settings"
                              : isTranscribing
                              ? "Transcription in progress..."
                              : hasTranscripts
                              ? "Transcription complete"
                              : "Start transcription"
                          }
                        >
                          {isTranscribing ? "⏳ Transcribing..." : hasTranscripts ? "✓ Transcribed" : "📝 Transcribe"}
                        </button>
                      );
                    })()}
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setSelectedMeeting(meeting);
                      }}
                      style={{
                        padding: "8px 16px",
                        background: "#0078d4",
                        color: "white",
                        border: "none",
                        borderRadius: "6px",
                        cursor: "pointer",
                        fontSize: "13px",
                      }}
                    >
                      View Details
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDeleteMeeting(meeting.id);
                      }}
                      style={{
                        padding: "8px 16px",
                        background: "#dc3545",
                        color: "white",
                        border: "none",
                        borderRadius: "6px",
                        cursor: "pointer",
                        fontSize: "13px",
                      }}
                    >
                      🗑️ Delete
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Meeting Details Panel */}
      {selectedMeeting && (
        <div
          style={{
            background: "white",
            padding: "24px",
            borderRadius: "8px",
            boxShadow: "0 2px 8px rgba(0,0,0,0.1)",
            marginTop: "20px",
          }}
        >
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "16px" }}>
            <h2 style={{ margin: 0 }}>Meeting Details</h2>
            <button
              onClick={() => setSelectedMeeting(null)}
              style={{
                padding: "6px 12px",
                background: "#f0f0f0",
                border: "1px solid #ddd",
                borderRadius: "6px",
                cursor: "pointer",
                fontSize: "13px",
              }}
            >
              ✕ Close
            </button>
          </div>

          <div style={{ marginBottom: "20px" }}>
            <h3 style={{ marginTop: 0, marginBottom: "8px" }}>
              {selectedMeeting.title || "Untitled Meeting"}
            </h3>
            <div style={{ fontSize: "14px", color: "#666", lineHeight: "1.8" }}>
              <div>
                <strong>Platform:</strong>{" "}
                {PLATFORMS[selectedMeeting.platform as keyof typeof PLATFORMS]?.label || selectedMeeting.platform}
              </div>
              <div>
                <strong>Start Time:</strong> {formatDate(selectedMeeting.start_time)}
              </div>
              {selectedMeeting.end_time && (
                <div>
                  <strong>End Time:</strong> {formatDate(selectedMeeting.end_time)}
                </div>
              )}
              <div>
                <strong>Duration:</strong> {formatDuration(selectedMeeting.start_time, selectedMeeting.end_time)}
              </div>
              {selectedMeeting.participant_count !== undefined && (
                <div>
                  <strong>Participants:</strong> {selectedMeeting.participant_count}
                </div>
              )}
            </div>
          </div>

          {/* Transcripts Section */}
          {transcripts[selectedMeeting.id]?.length > 0 ? (
            <div>
              <h3 style={{ marginBottom: "12px", marginTop: "24px" }}>
                Transcript ({transcripts[selectedMeeting.id].length} segments)
              </h3>
              <div
                style={{
                  maxHeight: "400px",
                  overflowY: "auto",
                  border: "1px solid #e0e0e0",
                  borderRadius: "6px",
                  padding: "16px",
                  background: "#fafafa",
                }}
              >
                {transcripts[selectedMeeting.id].map((transcript, index) => {
                  const minutes = Math.floor(transcript.timestamp_ms / 60000);
                  const seconds = Math.floor((transcript.timestamp_ms % 60000) / 1000);
                  const timeStr = `${minutes}:${seconds.toString().padStart(2, "0")}`;

                  return (
                    <div
                      key={transcript.id || index}
                      style={{
                        marginBottom: "16px",
                        paddingBottom: "12px",
                        borderBottom: index < transcripts[selectedMeeting.id].length - 1 ? "1px solid #e0e0e0" : "none",
                      }}
                    >
                      <div style={{ display: "flex", gap: "8px", marginBottom: "6px", alignItems: "center" }}>
                        <span
                          style={{
                            fontSize: "12px",
                            color: "#666",
                            fontFamily: "monospace",
                            background: "#e0e0e0",
                            padding: "2px 6px",
                            borderRadius: "4px",
                          }}
                        >
                          {timeStr}
                        </span>
                        {transcript.confidence && (
                          <span
                            style={{
                              fontSize: "11px",
                              color: "#888",
                              background: "#f0f0f0",
                              padding: "2px 6px",
                              borderRadius: "4px",
                            }}
                          >
                            {Math.round(transcript.confidence * 100)}% confidence
                          </span>
                        )}
                      </div>
                      <div style={{ fontSize: "14px", lineHeight: "1.6", color: "#333" }}>
                        {transcript.text}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          ) : transcribingMeetingId === selectedMeeting.id ? (
            <div
              style={{
                padding: "24px",
                background: "#fff8e1",
                borderRadius: "6px",
                textAlign: "center",
                color: "#856404",
                marginTop: "24px",
                border: "1px solid #ffc107",
              }}
            >
              <div style={{ fontSize: "32px", marginBottom: "8px" }}>⏳</div>
              <p style={{ margin: 0, fontWeight: "500" }}>Transcription in progress...</p>
              <p style={{ margin: "8px 0 0 0", fontSize: "13px" }}>
                This may take a few minutes depending on meeting length
              </p>
            </div>
          ) : selectedMeeting.end_time ? (
            <div
              style={{
                padding: "16px",
                background: "#f9f9f9",
                borderRadius: "6px",
                textAlign: "center",
                color: "#666",
                marginTop: "24px",
              }}
            >
              <p style={{ margin: 0 }}>
                {transcriptionAvailable
                  ? "Click 'Transcribe' button to generate transcript with speaker diarization"
                  : "Configure an ASR service (AssemblyAI or Deepgram) in Settings to enable transcription"}
              </p>
            </div>
          ) : (
            <div
              style={{
                padding: "16px",
                background: "#f9f9f9",
                borderRadius: "6px",
                textAlign: "center",
                color: "#666",
                marginTop: "24px",
              }}
            >
              <p style={{ margin: 0 }}>Meeting is still in progress. Transcription will be available after the meeting ends.</p>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default MeetingHistory;
