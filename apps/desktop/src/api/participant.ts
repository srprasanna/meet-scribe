/**
 * Participant API - Frontend bindings for participant mapping Tauri commands
 */

import { invoke } from "@tauri-apps/api/core";

/**
 * Participant information
 */
export interface ParticipantInfo {
  id: number;
  name: string;
  email?: string;
}

/**
 * Speaker summary with sample transcripts
 */
export interface SpeakerSummary {
  speaker_label: string;
  transcript_count: number;
  sample_transcripts: string[];
  participant?: ParticipantInfo;
}

/**
 * Request to link a speaker to a participant
 */
export interface LinkSpeakerRequest {
  meeting_id: number;
  speaker_label: string;
  participant_name: string;
  participant_email?: string;
}

/**
 * Get summary of all speakers in a meeting with sample transcripts
 *
 * @param meetingId - The ID of the meeting
 * @returns Promise that resolves to array of speaker summaries
 */
export async function getSpeakerSummary(
  meetingId: number
): Promise<SpeakerSummary[]> {
  return invoke("get_speaker_summary", { meetingId });
}

/**
 * Link a speaker label to a participant (create or update)
 *
 * @param request - The link speaker request
 * @returns Promise that resolves to the participant ID
 */
export async function linkSpeakerToParticipant(
  request: LinkSpeakerRequest
): Promise<number> {
  return invoke("link_speaker_to_participant", { request });
}

/**
 * Unlink a speaker from a participant (remove mapping)
 *
 * @param meetingId - The ID of the meeting
 * @param speakerLabel - The speaker label to unlink
 * @returns Promise that resolves when the speaker is unlinked
 */
export async function unlinkSpeaker(
  meetingId: number,
  speakerLabel: string
): Promise<void> {
  return invoke("unlink_speaker", { meetingId, speakerLabel });
}

/**
 * Delete all participants for a meeting
 * This is useful when regenerating transcripts to start fresh
 *
 * @param meetingId - The ID of the meeting
 * @returns Promise that resolves when all participants are deleted
 */
export async function deleteMeetingParticipants(
  meetingId: number
): Promise<void> {
  return invoke("delete_meeting_participants", { meetingId });
}
