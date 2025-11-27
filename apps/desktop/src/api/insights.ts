/**
 * Insights API - Frontend bindings for LLM insight generation Tauri commands
 */

import { invoke } from "@tauri-apps/api/core";
import type { InsightType } from "../types";

/**
 * Stored insight returned from the backend
 */
export interface StoredInsight {
  id: number;
  meeting_id: number;
  insight_type: InsightType;
  content: string;
  created_at: number;
}

/**
 * Response containing insights
 */
export interface MeetingInsightsResponse {
  insights: StoredInsight[];
}

/**
 * Request to generate insights for a meeting
 */
export interface GenerateMeetingInsightsRequest {
  meeting_id: number;
  provider: string;
  model: string;
  insight_types: InsightType[];
  temperature?: number;
  max_tokens?: number;
}

/**
 * Generate insights for a meeting using the configured LLM
 *
 * @param request - The insight generation request
 * @returns Promise that resolves to generated and stored insights
 */
export async function generateMeetingInsights(
  request: GenerateMeetingInsightsRequest
): Promise<MeetingInsightsResponse> {
  return invoke("generate_meeting_insights", { request });
}

/**
 * Get stored insights for a meeting
 *
 * @param meetingId - The ID of the meeting
 * @returns Promise that resolves to array of stored insights
 */
export async function getMeetingInsights(
  meetingId: number
): Promise<MeetingInsightsResponse> {
  return invoke("get_meeting_insights", { meetingId });
}

/**
 * Default prompt info
 */
export interface PromptInfo {
  insight_type: InsightType;
  prompt: string;
}

/**
 * Get default prompt templates
 *
 * @param insightType - Optional specific insight type, or all if not provided
 * @returns Promise that resolves to array of prompt templates
 */
export async function getDefaultPrompts(
  insightType?: InsightType
): Promise<{ prompts: PromptInfo[] }> {
  return invoke("get_default_prompts", {
    request: { insight_type: insightType || null },
  });
}

/**
 * List all supported LLM providers
 *
 * @returns Promise that resolves to array of provider names
 */
export async function listLlmProviders(): Promise<string[]> {
  return invoke("list_llm_providers");
}

/**
 * Delete all insights for a meeting
 *
 * This allows regenerating insights by first deleting existing ones.
 *
 * @param meetingId - The ID of the meeting
 * @returns Promise that resolves when insights are deleted
 */
export async function deleteMeetingInsights(meetingId: number): Promise<void> {
  return invoke("delete_meeting_insights", { meetingId });
}
