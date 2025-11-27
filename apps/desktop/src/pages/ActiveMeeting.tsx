import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface MeetingStatus {
  meeting_id: number | null;
  is_recording: boolean;
  platform: string | null;
  title: string | null;
  start_time: number | null;
  duration_seconds: number | null;
}

interface AudioCaptureStatus {
  is_capturing: boolean;
  device: string | null;
  format: {
    sample_rate: number;
    channels: number;
    bits_per_sample: number;
  };
}

interface Participant {
  id: number;
  name: string;
  email?: string;
  speaker_label?: string;
}

interface TranscriptSegment {
  id: number;
  participant_id?: number;
  participant_name?: string;
  timestamp_ms: number;
  text: string;
  confidence?: number;
}

const PLATFORMS = [
  { value: "teams", label: "Microsoft Teams", icon: "🟦" },
  { value: "zoom", label: "Zoom", icon: "🔵" },
  { value: "meet", label: "Google Meet", icon: "🟢" },
];

function ActiveMeeting() {
  const [selectedPlatform, setSelectedPlatform] = useState<string>("teams");
  const [meetingTitle, setMeetingTitle] = useState<string>("");
  const [selectedSpeakerDevice, setSelectedSpeakerDevice] = useState<string>("0: Default Communication Device");
  const [selectedMicrophoneDevice, setSelectedMicrophoneDevice] = useState<string>("");
  const [speakerDevices, setSpeakerDevices] = useState<string[]>([]);
  const [microphoneDevices, setMicrophoneDevices] = useState<string[]>([]);
  const [loadingDevices, setLoadingDevices] = useState<boolean>(false);
  const [meetingStatus, setMeetingStatus] = useState<MeetingStatus>({
    meeting_id: null,
    is_recording: false,
    platform: null,
    title: null,
    start_time: null,
    duration_seconds: null,
  });
  const [audioStatus, setAudioStatus] = useState<AudioCaptureStatus>({
    is_capturing: false,
    device: null,
    format: {
      sample_rate: 16000,
      channels: 1,
      bits_per_sample: 16,
    },
  });
  const [participants, setParticipants] = useState<Participant[]>([]);
  const [transcript, setTranscript] = useState<TranscriptSegment[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState<boolean>(false);

  // Load audio devices on mount
  useEffect(() => {
    const loadAudioDevices = async () => {
      setLoadingDevices(true);
      try {
        const speakers = await invoke<string[]>("list_speaker_devices");
        const microphones = await invoke<string[]>("list_microphone_devices");

        setSpeakerDevices(speakers);
        setMicrophoneDevices(microphones);

        if (speakers.length > 0) {
          setSelectedSpeakerDevice(speakers[0]); // Default to first speaker device
        }
        if (microphones.length > 0) {
          setSelectedMicrophoneDevice(microphones[0]); // Default to first microphone device
        }
      } catch (err) {
        console.error("Failed to load audio devices:", err);
        setError(`Failed to load audio devices: ${err}`);
      } finally {
        setLoadingDevices(false);
      }
    };

    loadAudioDevices();
  }, []);

  // Poll meeting status periodically
  useEffect(() => {
    const pollStatus = async () => {
      try {
        const status = await invoke<MeetingStatus>("get_meeting_status");
        setMeetingStatus(status);

        const audioStat = await invoke<AudioCaptureStatus>("get_audio_capture_status");
        setAudioStatus(audioStat);
      } catch (err) {
        console.error("Failed to get meeting status:", err);
      }
    };

    // Poll every 1 second if recording
    const interval = setInterval(pollStatus, 1000);
    pollStatus(); // Initial poll

    return () => clearInterval(interval);
  }, []);

  const handleStartMeeting = async () => {
    if (!selectedPlatform) {
      setError("Please select a meeting platform");
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const meetingId = await invoke<number>("start_meeting", {
        request: {
          platform: selectedPlatform,
          title: meetingTitle || null,
          speaker_device: selectedSpeakerDevice,
          microphone_device: selectedMicrophoneDevice,
        },
      });

      console.log("Meeting started with ID:", meetingId);

      // Update status
      setMeetingStatus({
        meeting_id: meetingId,
        is_recording: true,
        platform: selectedPlatform,
        title: meetingTitle || null,
        start_time: Date.now() / 1000,
        duration_seconds: 0,
      });
    } catch (err) {
      setError(`Failed to start meeting: ${err}`);
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  const handleStopMeeting = async () => {
    if (!meetingStatus.meeting_id) {
      setError("No active meeting to stop");
      return;
    }

    setLoading(true);
    setError(null);

    try {
      await invoke("stop_meeting", {
        meetingId: meetingStatus.meeting_id,
      });

      console.log("Meeting stopped");

      // Reset status
      setMeetingStatus({
        meeting_id: null,
        is_recording: false,
        platform: null,
        title: null,
        start_time: null,
        duration_seconds: null,
      });
      setTranscript([]);
      setParticipants([]);
    } catch (err) {
      setError(`Failed to stop meeting: ${err}`);
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  const formatDuration = (seconds: number | null): string => {
    if (!seconds) return "00:00:00";
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);
    return `${hours.toString().padStart(2, "0")}:${minutes.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  };

  const formatTimestamp = (timestampMs: number): string => {
    const seconds = Math.floor(timestampMs / 1000);
    const minutes = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${minutes.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  };

  return (
    <div style={{ padding: "20px", maxWidth: "1200px", margin: "0 auto" }}>
      <h1 style={{ marginBottom: "20px" }}>Active Meeting</h1>

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

      {/* Meeting Controls */}
      <div
        style={{
          background: "white",
          padding: "24px",
          borderRadius: "8px",
          boxShadow: "0 2px 8px rgba(0,0,0,0.1)",
          marginBottom: "20px",
        }}
      >
        <h2 style={{ marginTop: 0, marginBottom: "20px" }}>Meeting Controls</h2>

        {!meetingStatus.is_recording ? (
          <>
            {/* Platform Selection */}
            <div style={{ marginBottom: "16px" }}>
              <label
                style={{
                  display: "block",
                  marginBottom: "8px",
                  fontWeight: "500",
                }}
              >
                Meeting Platform
              </label>
              <div style={{ display: "flex", gap: "12px" }}>
                {PLATFORMS.map((platform) => (
                  <button
                    key={platform.value}
                    onClick={() => setSelectedPlatform(platform.value)}
                    style={{
                      padding: "12px 20px",
                      border: selectedPlatform === platform.value ? "2px solid #0078d4" : "2px solid #ddd",
                      borderRadius: "6px",
                      background: selectedPlatform === platform.value ? "#e6f2ff" : "white",
                      cursor: "pointer",
                      fontSize: "14px",
                      display: "flex",
                      alignItems: "center",
                      gap: "8px",
                      transition: "all 0.2s",
                    }}
                  >
                    <span style={{ fontSize: "20px" }}>{platform.icon}</span>
                    {platform.label}
                  </button>
                ))}
              </div>
            </div>

            {/* Meeting Title */}
            <div style={{ marginBottom: "16px" }}>
              <label
                htmlFor="meetingTitle"
                style={{
                  display: "block",
                  marginBottom: "8px",
                  fontWeight: "500",
                }}
              >
                Meeting Title (Optional)
              </label>
              <input
                id="meetingTitle"
                type="text"
                value={meetingTitle}
                onChange={(e) => setMeetingTitle(e.target.value)}
                placeholder="e.g., Weekly Team Sync"
                style={{
                  width: "100%",
                  padding: "10px",
                  border: "1px solid #ddd",
                  borderRadius: "6px",
                  fontSize: "14px",
                }}
              />
            </div>

            {/* Speaker Device Selection */}
            <div style={{ marginBottom: "16px" }}>
              <label
                htmlFor="speakerDevice"
                style={{
                  display: "block",
                  marginBottom: "8px",
                  fontWeight: "500",
                }}
              >
                Speaker Device
              </label>
              <select
                id="speakerDevice"
                value={selectedSpeakerDevice}
                onChange={(e) => setSelectedSpeakerDevice(e.target.value)}
                disabled={loadingDevices}
                style={{
                  width: "100%",
                  padding: "10px",
                  border: "1px solid #ddd",
                  borderRadius: "6px",
                  fontSize: "14px",
                  cursor: loadingDevices ? "not-allowed" : "pointer",
                  background: loadingDevices ? "#f5f5f5" : "white",
                }}
              >
                {loadingDevices ? (
                  <option>Loading devices...</option>
                ) : speakerDevices.length === 0 ? (
                  <option>No speaker devices found</option>
                ) : (
                  speakerDevices.map((device) => (
                    <option key={device} value={device}>
                      {device}
                    </option>
                  ))
                )}
              </select>
              <div style={{ fontSize: "12px", color: "#666", marginTop: "4px" }}>
                Select the speaker/headset that's playing the meeting audio
              </div>
            </div>

            {/* Microphone Device Selection */}
            <div style={{ marginBottom: "20px" }}>
              <label
                htmlFor="microphoneDevice"
                style={{
                  display: "block",
                  marginBottom: "8px",
                  fontWeight: "500",
                }}
              >
                Microphone Device
              </label>
              <select
                id="microphoneDevice"
                value={selectedMicrophoneDevice}
                onChange={(e) => setSelectedMicrophoneDevice(e.target.value)}
                disabled={loadingDevices}
                style={{
                  width: "100%",
                  padding: "10px",
                  border: "1px solid #ddd",
                  borderRadius: "6px",
                  fontSize: "14px",
                  cursor: loadingDevices ? "not-allowed" : "pointer",
                  background: loadingDevices ? "#f5f5f5" : "white",
                }}
              >
                {loadingDevices ? (
                  <option>Loading devices...</option>
                ) : microphoneDevices.length === 0 ? (
                  <option>No microphone devices found</option>
                ) : (
                  microphoneDevices.map((device) => (
                    <option key={device} value={device}>
                      {device}
                    </option>
                  ))
                )}
              </select>
              <div style={{ fontSize: "12px", color: "#666", marginTop: "4px" }}>
                Select the microphone you're using for the meeting
              </div>
            </div>

            {/* Start Button */}
            <button
              onClick={handleStartMeeting}
              disabled={loading}
              style={{
                padding: "12px 32px",
                background: loading ? "#ccc" : "#0078d4",
                color: "white",
                border: "none",
                borderRadius: "6px",
                cursor: loading ? "not-allowed" : "pointer",
                fontSize: "16px",
                fontWeight: "500",
                display: "flex",
                alignItems: "center",
                gap: "8px",
              }}
            >
              <span style={{ fontSize: "20px" }}>🔴</span>
              {loading ? "Starting..." : "Start Recording"}
            </button>
          </>
        ) : (
          <>
            {/* Recording Status */}
            <div style={{ marginBottom: "20px" }}>
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "12px",
                  marginBottom: "12px",
                }}
              >
                <div
                  style={{
                    width: "12px",
                    height: "12px",
                    background: "#ff0000",
                    borderRadius: "50%",
                    animation: "pulse 1.5s ease-in-out infinite",
                  }}
                />
                <span style={{ fontSize: "18px", fontWeight: "500" }}>
                  Recording in Progress
                </span>
              </div>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "16px" }}>
                <div>
                  <div style={{ fontSize: "12px", color: "#666", marginBottom: "4px" }}>
                    Platform
                  </div>
                  <div style={{ fontSize: "16px", fontWeight: "500" }}>
                    {PLATFORMS.find((p) => p.value === meetingStatus.platform)?.label || meetingStatus.platform}
                  </div>
                </div>
                <div>
                  <div style={{ fontSize: "12px", color: "#666", marginBottom: "4px" }}>
                    Duration
                  </div>
                  <div style={{ fontSize: "16px", fontWeight: "500", fontFamily: "monospace" }}>
                    {formatDuration(meetingStatus.duration_seconds)}
                  </div>
                </div>
                <div>
                  <div style={{ fontSize: "12px", color: "#666", marginBottom: "4px" }}>
                    Title
                  </div>
                  <div style={{ fontSize: "16px", fontWeight: "500" }}>
                    {meetingStatus.title || "Untitled Meeting"}
                  </div>
                </div>
              </div>
            </div>

            {/* Audio Status */}
            <div
              style={{
                padding: "12px",
                background: "#f5f5f5",
                borderRadius: "6px",
                marginBottom: "20px",
                fontSize: "13px",
              }}
            >
              <strong>Audio:</strong>{" "}
              {audioStatus.is_capturing ? "Capturing" : "Not Capturing"} |{" "}
              <strong>Device:</strong> {audioStatus.device || "Default"} |{" "}
              <strong>Format:</strong> {audioStatus.format.sample_rate}Hz,{" "}
              {audioStatus.format.channels}ch, {audioStatus.format.bits_per_sample}bit
            </div>

            {/* Stop Button */}
            <button
              onClick={handleStopMeeting}
              disabled={loading}
              style={{
                padding: "12px 32px",
                background: loading ? "#ccc" : "#dc3545",
                color: "white",
                border: "none",
                borderRadius: "6px",
                cursor: loading ? "not-allowed" : "pointer",
                fontSize: "16px",
                fontWeight: "500",
                display: "flex",
                alignItems: "center",
                gap: "8px",
              }}
            >
              <span style={{ fontSize: "20px" }}>⏹️</span>
              {loading ? "Stopping..." : "Stop Recording"}
            </button>
          </>
        )}
      </div>

      {/* Participants Panel */}
      {meetingStatus.is_recording && (
        <div
          style={{
            background: "white",
            padding: "24px",
            borderRadius: "8px",
            boxShadow: "0 2px 8px rgba(0,0,0,0.1)",
            marginBottom: "20px",
          }}
        >
          <h2 style={{ marginTop: 0, marginBottom: "16px" }}>
            Participants ({participants.length})
          </h2>
          {participants.length === 0 ? (
            <div style={{ color: "#666", fontStyle: "italic" }}>
              No participants detected yet. Participant detection is in progress...
            </div>
          ) : (
            <div style={{ display: "grid", gap: "12px" }}>
              {participants.map((participant) => (
                <div
                  key={participant.id}
                  style={{
                    padding: "12px",
                    background: "#f9f9f9",
                    borderRadius: "6px",
                    display: "flex",
                    alignItems: "center",
                    gap: "12px",
                  }}
                >
                  <div
                    style={{
                      width: "40px",
                      height: "40px",
                      borderRadius: "50%",
                      background: "#0078d4",
                      color: "white",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      fontSize: "18px",
                      fontWeight: "500",
                    }}
                  >
                    {participant.name.charAt(0).toUpperCase()}
                  </div>
                  <div>
                    <div style={{ fontWeight: "500" }}>{participant.name}</div>
                    {participant.email && (
                      <div style={{ fontSize: "12px", color: "#666" }}>
                        {participant.email}
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Transcript Panel */}
      {meetingStatus.is_recording && (
        <div
          style={{
            background: "white",
            padding: "24px",
            borderRadius: "8px",
            boxShadow: "0 2px 8px rgba(0,0,0,0.1)",
          }}
        >
          <h2 style={{ marginTop: 0, marginBottom: "16px" }}>
            Live Transcript
          </h2>
          {transcript.length === 0 ? (
            <div style={{ color: "#666", fontStyle: "italic", textAlign: "center", padding: "40px" }}>
              Waiting for transcription to start...
              <br />
              <span style={{ fontSize: "12px", marginTop: "8px", display: "block" }}>
                Transcription will appear here in real-time once configured in Settings
              </span>
            </div>
          ) : (
            <div
              style={{
                maxHeight: "400px",
                overflowY: "auto",
                border: "1px solid #eee",
                borderRadius: "6px",
                padding: "16px",
              }}
            >
              {transcript.map((segment) => (
                <div
                  key={segment.id}
                  style={{
                    marginBottom: "16px",
                    paddingBottom: "16px",
                    borderBottom: "1px solid #f0f0f0",
                  }}
                >
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "12px",
                      marginBottom: "8px",
                    }}
                  >
                    <span
                      style={{
                        fontSize: "12px",
                        color: "#666",
                        fontFamily: "monospace",
                      }}
                    >
                      {formatTimestamp(segment.timestamp_ms)}
                    </span>
                    <span style={{ fontWeight: "500", color: "#0078d4" }}>
                      {segment.participant_name || "Unknown Speaker"}
                    </span>
                    {segment.confidence && (
                      <span
                        style={{
                          fontSize: "11px",
                          color: "#999",
                        }}
                      >
                        ({Math.round(segment.confidence * 100)}%)
                      </span>
                    )}
                  </div>
                  <div style={{ lineHeight: "1.6", color: "#333" }}>
                    {segment.text}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      <style>{`
        @keyframes pulse {
          0%, 100% {
            opacity: 1;
          }
          50% {
            opacity: 0.5;
          }
        }
      `}</style>
    </div>
  );
}

export default ActiveMeeting;
