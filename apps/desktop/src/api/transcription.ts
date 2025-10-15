/**
 * Transcription API - Frontend bindings for transcription Tauri commands
 */

import { invoke } from "@tauri-apps/api/core";
import type { Transcript, TranscriptionConfig } from "../types";

/**
 * Start transcription for a completed meeting
 *
 * @param meetingId - The ID of the meeting to transcribe
 * @param config - Optional transcription configuration
 * @returns Promise that resolves when transcription starts
 */
export async function startTranscription(
  meetingId: number,
  config?: TranscriptionConfig
): Promise<void> {
  return invoke("start_transcription", {
    meetingId,
    config: config || {
      enable_diarization: true,
      language: "en",
    },
  });
}

/**
 * Get the current transcription status
 *
 * @returns Promise that resolves to the meeting ID being transcribed, or null if none
 */
export async function getTranscriptionStatus(): Promise<number | null> {
  return invoke("get_transcription_status");
}

/**
 * Get transcripts for a meeting
 *
 * @param meetingId - The ID of the meeting
 * @returns Promise that resolves to array of transcript segments
 */
export async function getTranscripts(meetingId: number): Promise<Transcript[]> {
  return invoke("get_transcripts", { meetingId });
}

/**
 * Check if transcription is available
 *
 * Checks if an ASR service is configured and ready to use.
 *
 * @returns Promise that resolves to true if transcription is available
 */
export async function isTranscriptionAvailable(): Promise<boolean> {
  return invoke("is_transcription_available");
}
