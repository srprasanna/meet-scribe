import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MarkdownContent } from "../components/MarkdownContent";
import {
  startTranscription,
  getTranscriptionStatus,
  getTranscripts,
  isTranscriptionAvailable,
  deleteTranscripts,
} from "../api/transcription";
import {
  generateMeetingInsights,
  getMeetingInsights,
  deleteMeetingInsights,
  type StoredInsight,
} from "../api/insights";
import {
  getSpeakerSummary,
  linkSpeakerToParticipant,
  unlinkSpeaker,
  deleteMeetingParticipants,
  type SpeakerSummary,
} from "../api/participant";
import type { Transcript, InsightType, ServiceConfig } from "../types";
import {
  DialogRoot,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogBody,
  DialogFooter,
  DialogBackdrop,
  DialogCloseTrigger,
  Button,
  HStack,
  Text,
} from "@chakra-ui/react";

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

const INSIGHT_TYPES: { type: InsightType; label: string; icon: string }[] = [
  { type: "summary", label: "Summary", icon: "📋" },
  { type: "action_item", label: "Action Items", icon: "✅" },
  { type: "key_point", label: "Key Points", icon: "💡" },
  { type: "decision", label: "Decisions", icon: "🎯" },
];

function MeetingHistory() {
  const [meetings, setMeetings] = useState<Meeting[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedMeeting, setSelectedMeeting] = useState<Meeting | null>(null);
  const [transcriptionAvailable, setTranscriptionAvailable] = useState<boolean>(false);
  const [transcribingMeetingId, setTranscribingMeetingId] = useState<number | null>(null);
  const [transcripts, setTranscripts] = useState<{ [meetingId: number]: Transcript[] }>({});
  const [loadingTranscripts, setLoadingTranscripts] = useState<{ [meetingId: number]: boolean }>({});

  // Insights state
  const [insights, setInsights] = useState<{ [meetingId: number]: StoredInsight[] }>({});
  const [loadingInsights, setLoadingInsights] = useState<{ [meetingId: number]: boolean }>({});
  const [generatingInsights, setGeneratingInsights] = useState<number | null>(null);
  const [llmConfig, setLlmConfig] = useState<{ provider: string; model: string } | null>(null);
  const [llmAvailable, setLlmAvailable] = useState<boolean>(false);

  // Participant mapping state
  const [showSpeakerModal, setShowSpeakerModal] = useState<boolean>(false);
  const [speakerSummaries, setSpeakerSummaries] = useState<SpeakerSummary[]>([]);
  const [loadingSpeakers, setLoadingSpeakers] = useState<boolean>(false);
  const [editingSpeaker, setEditingSpeaker] = useState<string | null>(null);
  const [participantName, setParticipantName] = useState<string>("");
  const [participantEmail, setParticipantEmail] = useState<string>("");

  // Confirmation dialog states
  const [regenerateTranscriptDialog, setRegenerateTranscriptDialog] = useState<{
    open: boolean;
    meetingId: number | null;
  }>({ open: false, meetingId: null });

  const [regenerateInsightsDialog, setRegenerateInsightsDialog] = useState<{
    open: boolean;
    meetingId: number | null;
  }>({ open: false, meetingId: null });

  const [deleteMeetingDialog, setDeleteMeetingDialog] = useState<{
    open: boolean;
    meetingId: number | null;
  }>({ open: false, meetingId: null });

  const [unlinkSpeakerDialog, setUnlinkSpeakerDialog] = useState<{
    open: boolean;
    speakerLabel: string | null;
  }>({ open: false, speakerLabel: null });

  useEffect(() => {
    loadMeetings();
    checkTranscriptionAvailability();
    checkLlmAvailability();
  }, []);

  // Lazy load transcripts and insights when a meeting is selected
  useEffect(() => {
    if (selectedMeeting?.id && selectedMeeting.end_time) {
      // Only load if we don't already have transcripts for this meeting
      if (!transcripts[selectedMeeting.id]) {
        loadTranscriptsForMeeting(selectedMeeting.id);
      }
      // Load insights if not already loaded
      if (!insights[selectedMeeting.id]) {
        loadInsightsForMeeting(selectedMeeting.id);
      }
    }
  }, [selectedMeeting]);

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

  const checkLlmAvailability = async () => {
    try {
      // Get active LLM service config
      const activeConfig = await invoke<ServiceConfig | null>("get_active_service_config", {
        serviceType: "llm",
      });
      if (activeConfig) {
        const settings = activeConfig.settings ? JSON.parse(activeConfig.settings) : {};
        setLlmConfig({
          provider: activeConfig.provider,
          model: settings.model || "",
        });
        setLlmAvailable(!!settings.model);
      } else {
        setLlmAvailable(false);
      }
    } catch (err) {
      console.error("Failed to check LLM availability:", err);
      setLlmAvailable(false);
    }
  };

  const loadInsightsForMeeting = async (meetingId: number) => {
    setLoadingInsights((prev) => ({ ...prev, [meetingId]: true }));
    try {
      const response = await getMeetingInsights(meetingId);
      setInsights((prev) => ({ ...prev, [meetingId]: response.insights }));
    } catch (err) {
      console.error(`Failed to load insights for meeting ${meetingId}:`, err);
    } finally {
      setLoadingInsights((prev) => ({ ...prev, [meetingId]: false }));
    }
  };

  const openRegenerateTranscriptDialog = (meetingId: number) => {
    setRegenerateTranscriptDialog({ open: true, meetingId });
  };

  const handleRegenerateTranscripts = async () => {
    const meetingId = regenerateTranscriptDialog.meetingId;
    setRegenerateTranscriptDialog({ open: false, meetingId: null });

    if (!meetingId) return;

    try {
      setError(null);
      // Delete existing transcripts
      await deleteTranscripts(meetingId);
      // Clear from state
      setTranscripts((prev) => ({ ...prev, [meetingId]: [] }));
      // Also delete insights since they're based on the old transcript
      await deleteMeetingInsights(meetingId);
      setInsights((prev) => ({ ...prev, [meetingId]: [] }));
      // Delete participant mappings since speaker labels may be different
      await deleteMeetingParticipants(meetingId);
      // Start new transcription
      await startTranscription(meetingId);
      setTranscribingMeetingId(meetingId);
    } catch (err) {
      setError(`Failed to regenerate transcript: ${err}`);
      console.error(err);
    }
  };

  const openRegenerateInsightsDialog = (meetingId: number) => {
    setRegenerateInsightsDialog({ open: true, meetingId });
  };

  const handleRegenerateInsights = async () => {
    const meetingId = regenerateInsightsDialog.meetingId;
    setRegenerateInsightsDialog({ open: false, meetingId: null });

    if (!meetingId) return;

    try {
      setError(null);
      // Delete existing insights
      await deleteMeetingInsights(meetingId);
      setInsights((prev) => ({ ...prev, [meetingId]: [] }));
      // Generate new insights
      await handleGenerateInsights(meetingId);
    } catch (err) {
      setError(`Failed to regenerate insights: ${err}`);
      console.error(err);
    }
  };

  const handleGenerateInsights = async (meetingId: number) => {
    if (!llmConfig || !llmConfig.model) {
      setError("LLM service not configured. Please configure an LLM service in Settings and select a model.");
      return;
    }

    try {
      setError(null);
      setGeneratingInsights(meetingId);

      const response = await generateMeetingInsights({
        meeting_id: meetingId,
        provider: llmConfig.provider,
        model: llmConfig.model,
        insight_types: ["summary", "action_item", "key_point", "decision"],
      });

      setInsights((prev) => ({ ...prev, [meetingId]: response.insights }));
    } catch (err) {
      setError(`Failed to generate insights: ${err}`);
      console.error(err);
    } finally {
      setGeneratingInsights(null);
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

      // Don't load transcripts upfront - use lazy loading instead
      // Transcripts will be loaded only when a meeting is selected
    } catch (err) {
      setError(`Failed to load meetings: ${err}`);
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  const loadTranscriptsForMeeting = async (meetingId: number) => {
    setLoadingTranscripts((prev) => ({ ...prev, [meetingId]: true }));
    try {
      const transcriptList = await getTranscripts(meetingId);
      setTranscripts((prev) => ({ ...prev, [meetingId]: transcriptList }));
    } catch (err) {
      console.error(`Failed to load transcripts for meeting ${meetingId}:`, err);
    } finally {
      setLoadingTranscripts((prev) => ({ ...prev, [meetingId]: false }));
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

  const openDeleteMeetingDialog = (meetingId: number) => {
    setDeleteMeetingDialog({ open: true, meetingId });
  };

  const handleDeleteMeeting = async () => {
    const meetingId = deleteMeetingDialog.meetingId;
    setDeleteMeetingDialog({ open: false, meetingId: null });

    if (!meetingId) return;

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

  const handleManageSpeakers = async (meetingId: number) => {
    setShowSpeakerModal(true);
    setLoadingSpeakers(true);
    try {
      const summaries = await getSpeakerSummary(meetingId);
      setSpeakerSummaries(summaries);
    } catch (err) {
      setError(`Failed to load speaker summary: ${err}`);
      console.error(err);
    } finally {
      setLoadingSpeakers(false);
    }
  };

  const handleLinkSpeaker = async (speakerLabel: string) => {
    if (!selectedMeeting || !participantName.trim()) {
      return;
    }

    try {
      await linkSpeakerToParticipant({
        meeting_id: selectedMeeting.id,
        speaker_label: speakerLabel,
        participant_name: participantName.trim(),
        participant_email: participantEmail.trim() || undefined,
      });

      // Reload speaker summaries to show updated mapping
      const summaries = await getSpeakerSummary(selectedMeeting.id);
      setSpeakerSummaries(summaries);

      // Reload transcripts to reflect participant linkage
      await loadTranscriptsForMeeting(selectedMeeting.id);

      // Clear form
      setEditingSpeaker(null);
      setParticipantName("");
      setParticipantEmail("");
    } catch (err) {
      setError(`Failed to link speaker: ${err}`);
      console.error(err);
    }
  };

  const openUnlinkSpeakerDialog = (speakerLabel: string) => {
    setUnlinkSpeakerDialog({ open: true, speakerLabel });
  };

  const handleUnlinkSpeaker = async () => {
    const speakerLabel = unlinkSpeakerDialog.speakerLabel;
    setUnlinkSpeakerDialog({ open: false, speakerLabel: null });

    if (!selectedMeeting || !speakerLabel) {
      return;
    }

    try {
      await unlinkSpeaker(selectedMeeting.id, speakerLabel);

      // Reload speaker summaries
      const summaries = await getSpeakerSummary(selectedMeeting.id);
      setSpeakerSummaries(summaries);

      // Reload transcripts
      await loadTranscriptsForMeeting(selectedMeeting.id);
    } catch (err) {
      setError(`Failed to unlink speaker: ${err}`);
      console.error(err);
    }
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
                        openDeleteMeetingDialog(meeting.id);
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
          {loadingTranscripts[selectedMeeting.id] ? (
            <div
              style={{
                padding: "24px",
                background: "#f0f7ff",
                borderRadius: "6px",
                textAlign: "center",
                color: "#0078d4",
                marginTop: "24px",
                border: "1px solid #b3d9ff",
              }}
            >
              <div style={{ fontSize: "32px", marginBottom: "8px" }}>📖</div>
              <p style={{ margin: 0, fontWeight: "500" }}>Loading transcripts...</p>
            </div>
          ) : transcripts[selectedMeeting.id]?.length > 0 ? (
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: "24px", marginBottom: "12px" }}>
                <h3 style={{ margin: 0 }}>
                  Transcript ({transcripts[selectedMeeting.id].length} segments)
                </h3>
                <div style={{ display: "flex", gap: "8px" }}>
                  <button
                    onClick={() => handleManageSpeakers(selectedMeeting.id)}
                    style={{
                      padding: "6px 12px",
                      background: "#0078d4",
                      color: "white",
                      border: "none",
                      borderRadius: "4px",
                      cursor: "pointer",
                      fontSize: "12px",
                    }}
                    title="Map speaker labels to participant names"
                  >
                    👥 Manage Speakers
                  </button>
                  <button
                    onClick={() => openRegenerateTranscriptDialog(selectedMeeting.id)}
                    disabled={transcribingMeetingId === selectedMeeting.id}
                    style={{
                      padding: "6px 12px",
                      background: "#6c757d",
                      color: "white",
                      border: "none",
                      borderRadius: "4px",
                      cursor: transcribingMeetingId === selectedMeeting.id ? "not-allowed" : "pointer",
                      fontSize: "12px",
                    }}
                    title="Re-transcribe with different settings or ASR provider"
                  >
                    🔄 Regenerate Transcript
                  </button>
                </div>
              </div>
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
                        {(transcript.participant_name || transcript.speaker_label) && (
                          <span
                            style={{
                              fontSize: "11px",
                              color: transcript.participant_name ? "#28a745" : "#0078d4",
                              background: transcript.participant_name ? "#e6f7ed" : "#e6f3ff",
                              padding: "2px 8px",
                              borderRadius: "4px",
                              fontWeight: "500",
                            }}
                          >
                            {transcript.participant_name ? "👤" : "🎙️"} {transcript.participant_name || transcript.speaker_label}
                          </span>
                        )}
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

          {/* Insights Section - only show if transcripts exist */}
          {transcripts[selectedMeeting.id]?.length > 0 && (
            <div style={{ marginTop: "24px" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "12px" }}>
                <h3 style={{ margin: 0 }}>
                  AI Insights
                  {insights[selectedMeeting.id]?.length > 0 && ` (${insights[selectedMeeting.id].length})`}
                </h3>
                <div style={{ display: "flex", gap: "8px" }}>
                  {insights[selectedMeeting.id]?.length > 0 && (
                    <button
                      onClick={() => openRegenerateInsightsDialog(selectedMeeting.id)}
                      disabled={generatingInsights === selectedMeeting.id}
                      style={{
                        padding: "6px 12px",
                        background: "#6c757d",
                        color: "white",
                        border: "none",
                        borderRadius: "4px",
                        cursor: generatingInsights === selectedMeeting.id ? "not-allowed" : "pointer",
                        fontSize: "12px",
                      }}
                      title="Re-generate insights with different LLM or settings"
                    >
                      🔄 Regenerate
                    </button>
                  )}
                  {!insights[selectedMeeting.id]?.length && (
                    <button
                      onClick={() => handleGenerateInsights(selectedMeeting.id)}
                      disabled={!llmAvailable || generatingInsights === selectedMeeting.id}
                      style={{
                        padding: "8px 16px",
                        background: generatingInsights === selectedMeeting.id
                          ? "#ffc107"
                          : llmAvailable
                          ? "#6f42c1"
                          : "#d3d3d3",
                        color: "white",
                        border: "none",
                        borderRadius: "6px",
                        cursor: llmAvailable && generatingInsights !== selectedMeeting.id ? "pointer" : "not-allowed",
                        fontSize: "13px",
                        fontWeight: "500",
                      }}
                      title={
                        !llmAvailable
                          ? "Configure LLM service in Settings"
                          : generatingInsights === selectedMeeting.id
                          ? "Generating insights..."
                          : "Generate AI insights from transcript"
                      }
                    >
                      {generatingInsights === selectedMeeting.id ? "⏳ Generating..." : "🤖 Generate Insights"}
                    </button>
                  )}
                </div>
              </div>

              {loadingInsights[selectedMeeting.id] ? (
                <div
                  style={{
                    padding: "24px",
                    background: "#f0f7ff",
                    borderRadius: "6px",
                    textAlign: "center",
                    color: "#0078d4",
                  }}
                >
                  Loading insights...
                </div>
              ) : generatingInsights === selectedMeeting.id ? (
                <div
                  style={{
                    padding: "24px",
                    background: "#fff8e1",
                    borderRadius: "6px",
                    textAlign: "center",
                    color: "#856404",
                    border: "1px solid #ffc107",
                  }}
                >
                  <div style={{ fontSize: "32px", marginBottom: "8px" }}>🤖</div>
                  <p style={{ margin: 0, fontWeight: "500" }}>Generating AI insights...</p>
                  <p style={{ margin: "8px 0 0 0", fontSize: "13px" }}>
                    Analyzing transcript with {llmConfig?.provider} ({llmConfig?.model})
                  </p>
                </div>
              ) : insights[selectedMeeting.id]?.length > 0 ? (
                <div style={{ display: "grid", gap: "16px" }}>
                  {INSIGHT_TYPES.map(({ type, label, icon }) => {
                    const typeInsights = insights[selectedMeeting.id].filter((i) => i.insight_type === type);
                    if (typeInsights.length === 0) return null;

                    return (
                      <div
                        key={type}
                        style={{
                          border: "1px solid #e0e0e0",
                          borderRadius: "8px",
                          overflow: "hidden",
                        }}
                      >
                        <div
                          style={{
                            background: type === "summary" ? "#e8f5e9" :
                                        type === "action_item" ? "#fff3e0" :
                                        type === "key_point" ? "#e3f2fd" :
                                        "#fce4ec",
                            padding: "12px 16px",
                            fontWeight: "600",
                            display: "flex",
                            alignItems: "center",
                            gap: "8px",
                          }}
                        >
                          <span>{icon}</span>
                          <span>{label}</span>
                        </div>
                        <div style={{ padding: "16px" }}>
                          {typeInsights.map((insight) => (
                            <div
                              key={insight.id}
                              style={{
                                fontSize: "14px",
                              }}
                            >
                              <MarkdownContent content={insight.content} />
                            </div>
                          ))}
                        </div>
                      </div>
                    );
                  })}
                </div>
              ) : (
                <div
                  style={{
                    padding: "16px",
                    background: "#f9f9f9",
                    borderRadius: "6px",
                    textAlign: "center",
                    color: "#666",
                  }}
                >
                  <p style={{ margin: 0 }}>
                    {llmAvailable
                      ? "Click 'Generate Insights' to analyze this transcript with AI"
                      : "Configure an LLM service (OpenAI, Anthropic, Google, or Groq) in Settings to enable AI insights"}
                  </p>
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {/* Speaker Mapping Modal */}
      {showSpeakerModal && selectedMeeting && (
        <div
          style={{
            position: "fixed",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: "rgba(0, 0, 0, 0.5)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 1000,
          }}
          onClick={() => setShowSpeakerModal(false)}
        >
          <div
            style={{
              background: "white",
              borderRadius: "8px",
              padding: "24px",
              maxWidth: "700px",
              width: "90%",
              maxHeight: "80vh",
              overflowY: "auto",
              boxShadow: "0 4px 20px rgba(0,0,0,0.3)",
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "20px" }}>
              <h2 style={{ margin: 0 }}>👥 Manage Speakers</h2>
              <button
                onClick={() => setShowSpeakerModal(false)}
                style={{
                  background: "none",
                  border: "none",
                  fontSize: "24px",
                  cursor: "pointer",
                  color: "#666",
                }}
              >
                ✕
              </button>
            </div>

            <p style={{ color: "#666", marginBottom: "20px", fontSize: "14px" }}>
              Map detected speaker labels to actual participant names. This helps identify who said what in the transcript.
            </p>

            {loadingSpeakers ? (
              <div style={{ textAlign: "center", padding: "40px", color: "#666" }}>
                Loading speakers...
              </div>
            ) : speakerSummaries.length === 0 ? (
              <div
                style={{
                  textAlign: "center",
                  padding: "40px",
                  background: "#f9f9f9",
                  borderRadius: "6px",
                  color: "#666",
                }}
              >
                <p style={{ margin: 0 }}>No speakers detected in this meeting.</p>
                <p style={{ margin: "8px 0 0 0", fontSize: "13px" }}>
                  Speakers are detected during transcription with diarization enabled.
                </p>
              </div>
            ) : (
              <div style={{ display: "grid", gap: "16px" }}>
                {speakerSummaries.map((speaker) => (
                  <div
                    key={speaker.speaker_label}
                    style={{
                      border: "1px solid #e0e0e0",
                      borderRadius: "6px",
                      padding: "16px",
                      background: speaker.participant ? "#f0f7ff" : "#fafafa",
                    }}
                  >
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "start", marginBottom: "12px" }}>
                      <div>
                        <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "4px" }}>
                          <span style={{ fontWeight: "600", color: "#0078d4" }}>
                            🎙️ {speaker.speaker_label}
                          </span>
                          <span style={{ fontSize: "12px", color: "#666" }}>
                            ({speaker.transcript_count} segments)
                          </span>
                        </div>
                        {speaker.participant && (
                          <div style={{ fontSize: "14px", color: "#333", marginTop: "4px" }}>
                            <strong>{speaker.participant.name}</strong>
                            {speaker.participant.email && (
                              <span style={{ color: "#666", marginLeft: "8px" }}>
                                {speaker.participant.email}
                              </span>
                            )}
                          </div>
                        )}
                      </div>
                      {speaker.participant ? (
                        <button
                          onClick={() => openUnlinkSpeakerDialog(speaker.speaker_label)}
                          style={{
                            padding: "4px 8px",
                            background: "#dc3545",
                            color: "white",
                            border: "none",
                            borderRadius: "4px",
                            cursor: "pointer",
                            fontSize: "11px",
                          }}
                        >
                          Unlink
                        </button>
                      ) : (
                        <button
                          onClick={() => {
                            setEditingSpeaker(speaker.speaker_label);
                            setParticipantName("");
                            setParticipantEmail("");
                          }}
                          style={{
                            padding: "4px 8px",
                            background: "#28a745",
                            color: "white",
                            border: "none",
                            borderRadius: "4px",
                            cursor: "pointer",
                            fontSize: "11px",
                          }}
                        >
                          Link
                        </button>
                      )}
                    </div>

                    {/* Sample transcripts */}
                    <div style={{ fontSize: "13px", color: "#666", marginBottom: "8px" }}>
                      <strong>Sample transcripts:</strong>
                    </div>
                    <div style={{ fontSize: "12px", color: "#555", lineHeight: "1.6" }}>
                      {speaker.sample_transcripts.map((text, idx) => (
                        <div
                          key={idx}
                          style={{
                            marginBottom: "4px",
                            paddingLeft: "8px",
                            borderLeft: "2px solid #e0e0e0",
                          }}
                        >
                          "{text.length > 100 ? text.substring(0, 100) + "..." : text}"
                        </div>
                      ))}
                    </div>

                    {/* Link form */}
                    {editingSpeaker === speaker.speaker_label && (
                      <div
                        style={{
                          marginTop: "12px",
                          padding: "12px",
                          background: "white",
                          borderRadius: "4px",
                          border: "1px solid #0078d4",
                        }}
                      >
                        <div style={{ marginBottom: "8px" }}>
                          <label
                            style={{
                              display: "block",
                              marginBottom: "4px",
                              fontSize: "12px",
                              fontWeight: "500",
                            }}
                          >
                            Participant Name *
                          </label>
                          <input
                            type="text"
                            value={participantName}
                            onChange={(e) => setParticipantName(e.target.value)}
                            placeholder="Enter participant name"
                            style={{
                              width: "100%",
                              padding: "6px 8px",
                              border: "1px solid #ccc",
                              borderRadius: "4px",
                              fontSize: "13px",
                            }}
                          />
                        </div>
                        <div style={{ marginBottom: "12px" }}>
                          <label
                            style={{
                              display: "block",
                              marginBottom: "4px",
                              fontSize: "12px",
                              fontWeight: "500",
                            }}
                          >
                            Email (optional)
                          </label>
                          <input
                            type="email"
                            value={participantEmail}
                            onChange={(e) => setParticipantEmail(e.target.value)}
                            placeholder="Enter email address"
                            style={{
                              width: "100%",
                              padding: "6px 8px",
                              border: "1px solid #ccc",
                              borderRadius: "4px",
                              fontSize: "13px",
                            }}
                          />
                        </div>
                        <div style={{ display: "flex", gap: "8px" }}>
                          <button
                            onClick={() => handleLinkSpeaker(speaker.speaker_label)}
                            disabled={!participantName.trim()}
                            style={{
                              padding: "6px 12px",
                              background: participantName.trim() ? "#0078d4" : "#ccc",
                              color: "white",
                              border: "none",
                              borderRadius: "4px",
                              cursor: participantName.trim() ? "pointer" : "not-allowed",
                              fontSize: "12px",
                            }}
                          >
                            Save
                          </button>
                          <button
                            onClick={() => {
                              setEditingSpeaker(null);
                              setParticipantName("");
                              setParticipantEmail("");
                            }}
                            style={{
                              padding: "6px 12px",
                              background: "#6c757d",
                              color: "white",
                              border: "none",
                              borderRadius: "4px",
                              cursor: "pointer",
                              fontSize: "12px",
                            }}
                          >
                            Cancel
                          </button>
                        </div>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Regenerate Transcript Confirmation Dialog */}
      <DialogRoot
        open={regenerateTranscriptDialog.open}
        onOpenChange={(e) => e.open ? null : setRegenerateTranscriptDialog({ open: false, meetingId: null })}
      >
        <DialogBackdrop />
        <DialogContent
          maxW="md"
          mx="auto"
          my="auto"
          position="fixed"
          top="50%"
          left="50%"
          transform="translate(-50%, -50%)"
        >
          <DialogHeader p={4}>
            <DialogTitle>Regenerate Transcript</DialogTitle>
            <DialogCloseTrigger onClick={() => setRegenerateTranscriptDialog({ open: false, meetingId: null })} />
          </DialogHeader>
          <DialogBody p={4} pt={0}>
            <Text>
              Are you sure you want to regenerate the transcript? This will delete the existing transcript and speaker mappings.
            </Text>
          </DialogBody>
          <DialogFooter p={4} pt={0}>
            <HStack gap={3}>
              <Button variant="outline" onClick={() => setRegenerateTranscriptDialog({ open: false, meetingId: null })} px={4} py={2}>
                Cancel
              </Button>
              <Button colorScheme="red" onClick={handleRegenerateTranscripts} px={4} py={2}>
                Regenerate
              </Button>
            </HStack>
          </DialogFooter>
        </DialogContent>
      </DialogRoot>

      {/* Regenerate Insights Confirmation Dialog */}
      <DialogRoot
        open={regenerateInsightsDialog.open}
        onOpenChange={(e) => e.open ? null : setRegenerateInsightsDialog({ open: false, meetingId: null })}
      >
        <DialogBackdrop />
        <DialogContent
          maxW="md"
          mx="auto"
          my="auto"
          position="fixed"
          top="50%"
          left="50%"
          transform="translate(-50%, -50%)"
        >
          <DialogHeader p={4}>
            <DialogTitle>Regenerate Insights</DialogTitle>
            <DialogCloseTrigger onClick={() => setRegenerateInsightsDialog({ open: false, meetingId: null })} />
          </DialogHeader>
          <DialogBody p={4} pt={0}>
            <Text>
              Are you sure you want to regenerate the insights? This will delete the existing insights.
            </Text>
          </DialogBody>
          <DialogFooter p={4} pt={0}>
            <HStack gap={3}>
              <Button variant="outline" onClick={() => setRegenerateInsightsDialog({ open: false, meetingId: null })} px={4} py={2}>
                Cancel
              </Button>
              <Button colorScheme="red" onClick={handleRegenerateInsights} px={4} py={2}>
                Regenerate
              </Button>
            </HStack>
          </DialogFooter>
        </DialogContent>
      </DialogRoot>

      {/* Delete Meeting Confirmation Dialog */}
      <DialogRoot
        open={deleteMeetingDialog.open}
        onOpenChange={(e) => e.open ? null : setDeleteMeetingDialog({ open: false, meetingId: null })}
      >
        <DialogBackdrop />
        <DialogContent
          maxW="md"
          mx="auto"
          my="auto"
          position="fixed"
          top="50%"
          left="50%"
          transform="translate(-50%, -50%)"
        >
          <DialogHeader p={4}>
            <DialogTitle>Delete Meeting</DialogTitle>
            <DialogCloseTrigger onClick={() => setDeleteMeetingDialog({ open: false, meetingId: null })} />
          </DialogHeader>
          <DialogBody p={4} pt={0}>
            <Text>
              Are you sure you want to delete this meeting? This action cannot be undone.
            </Text>
          </DialogBody>
          <DialogFooter p={4} pt={0}>
            <HStack gap={3}>
              <Button variant="outline" onClick={() => setDeleteMeetingDialog({ open: false, meetingId: null })} px={4} py={2}>
                Cancel
              </Button>
              <Button colorScheme="red" onClick={handleDeleteMeeting} px={4} py={2}>
                Delete
              </Button>
            </HStack>
          </DialogFooter>
        </DialogContent>
      </DialogRoot>

      {/* Unlink Speaker Confirmation Dialog */}
      <DialogRoot
        open={unlinkSpeakerDialog.open}
        onOpenChange={(e) => e.open ? null : setUnlinkSpeakerDialog({ open: false, speakerLabel: null })}
      >
        <DialogBackdrop />
        <DialogContent
          maxW="md"
          mx="auto"
          my="auto"
          position="fixed"
          top="50%"
          left="50%"
          transform="translate(-50%, -50%)"
        >
          <DialogHeader p={4}>
            <DialogTitle>Unlink Speaker</DialogTitle>
            <DialogCloseTrigger onClick={() => setUnlinkSpeakerDialog({ open: false, speakerLabel: null })} />
          </DialogHeader>
          <DialogBody p={4} pt={0}>
            <Text>
              Unlink <strong>{unlinkSpeakerDialog.speakerLabel}</strong> from participant?
            </Text>
          </DialogBody>
          <DialogFooter p={4} pt={0}>
            <HStack gap={3}>
              <Button variant="outline" onClick={() => setUnlinkSpeakerDialog({ open: false, speakerLabel: null })} px={4} py={2}>
                Cancel
              </Button>
              <Button colorScheme="red" onClick={handleUnlinkSpeaker} px={4} py={2}>
                Unlink
              </Button>
            </HStack>
          </DialogFooter>
        </DialogContent>
      </DialogRoot>
    </div>
  );
}

export default MeetingHistory;
