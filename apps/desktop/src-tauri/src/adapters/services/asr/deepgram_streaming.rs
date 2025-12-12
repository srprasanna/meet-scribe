//! Deepgram streaming transcription implementation
//!
//! Implements real-time transcription with speaker diarization using Deepgram's WebSocket API.
//! Reference: https://developers.deepgram.com/docs/live-streaming-audio

use crate::error::{AppError, Result};
use crate::ports::transcription::{
    StreamingSession, StreamingTranscriptionCallback, TranscriptionConfig, TranscriptionSegment,
};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const DEEPGRAM_STREAMING_URL: &str = "wss://api.deepgram.com/v1/listen";

/// Deepgram streaming session
pub struct DeepgramStreamingSession {
    /// WebSocket write sink
    ws_sender: Arc<Mutex<Option<futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        Message,
    >>>>,

    /// Session active status
    is_active: Arc<Mutex<bool>>,

    /// Handle to the receiver task
    receiver_task: Option<tokio::task::JoinHandle<()>>,
}

impl DeepgramStreamingSession {
    /// Create a new Deepgram streaming session
    pub async fn new(
        api_key: String,
        config: &TranscriptionConfig,
        callback: Box<dyn StreamingTranscriptionCallback>,
    ) -> Result<Self> {
        log::info!("Starting Deepgram streaming session");

        // Build WebSocket URL with query parameters
        let model = config.model.as_deref().unwrap_or("nova-2-meeting");

        let mut url = format!("{}?model={}", DEEPGRAM_STREAMING_URL, model);

        // Add diarization if enabled
        if config.enable_diarization {
            url.push_str("&diarize=true");
        }

        // Add utterances for better segmentation
        url.push_str("&utterances=true");

        // Add punctuation
        url.push_str("&punctuate=true");

        // Add interim results for real-time feedback
        url.push_str("&interim_results=true");

        // Add language if specified
        if let Some(lang) = &config.language {
            url.push_str(&format!("&language={}", lang));
        }

        // Add encoding and sample rate (Deepgram expects these)
        url.push_str("&encoding=linear16&sample_rate=16000&channels=1");

        log::info!("Connecting to Deepgram WebSocket: {}", url);

        // Create authorization header
        let request = tokio_tungstenite::tungstenite::http::Request::builder()
            .uri(&url)
            .header("Authorization", format!("Token {}", api_key))
            .body(())
            .map_err(|e| AppError::Transcription(format!("Failed to build request: {}", e)))?;

        // Connect to WebSocket
        let (ws_stream, _) = connect_async(request)
            .await
            .map_err(|e| AppError::Transcription(format!("WebSocket connection failed: {}", e)))?;

        log::info!("Connected to Deepgram WebSocket");

        // Split the WebSocket into sender and receiver
        let (write, mut read) = ws_stream.split();

        let ws_sender = Arc::new(Mutex::new(Some(write)));
        let is_active = Arc::new(Mutex::new(true));

        // Spawn a task to receive messages from the WebSocket
        let is_active_clone = Arc::clone(&is_active);
        let receiver_task = tokio::spawn(async move {
            while let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        log::debug!("Received Deepgram message: {}", text);

                        // Parse the Deepgram response
                        match serde_json::from_str::<DeepgramStreamingResponse>(&text) {
                            Ok(response) => {
                                // Check if this is a final transcript or interim
                                let is_final = response.is_final.unwrap_or(false);

                                // Process the response
                                if let Some(ref channel_data) = response.channel {
                                    if let Some(alternative) = channel_data.alternatives.first() {
                                        // Extract segment information from alternative
                                        if !alternative.transcript.is_empty() {
                                            let segment = TranscriptionSegment {
                                                text: alternative.transcript.clone(),
                                                start_ms: (response.start.unwrap_or(0.0) * 1000.0) as i64,
                                                end_ms: ((response.start.unwrap_or(0.0) + response.duration.unwrap_or(0.0)) * 1000.0) as i64,
                                                speaker_label: None, // Will be populated from utterances if available
                                                confidence: Some(alternative.confidence),
                                            };

                                            if is_final {
                                                callback.on_transcript(segment).await;
                                            } else {
                                                callback.on_interim_transcript(segment).await;
                                            }
                                        }

                                        // Process utterances (for diarization)
                                        if let Some(ref utterances) = alternative.utterances {
                                            for utterance in utterances {
                                                let segment = TranscriptionSegment {
                                                    text: utterance.transcript.clone(),
                                                    start_ms: (utterance.start * 1000.0) as i64,
                                                    end_ms: (utterance.end * 1000.0) as i64,
                                                    speaker_label: Some(format!("Speaker {}", utterance.speaker)),
                                                    confidence: Some(utterance.confidence),
                                                };

                                                callback.on_transcript(segment).await;
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to parse Deepgram response: {}", e);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        log::info!("Deepgram WebSocket closed");
                        *is_active_clone.lock().await = false;
                        callback.on_close().await;
                        break;
                    }
                    Err(e) => {
                        log::error!("WebSocket error: {}", e);
                        callback.on_error(e.to_string()).await;
                        *is_active_clone.lock().await = false;
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(Self {
            ws_sender,
            is_active,
            receiver_task: Some(receiver_task),
        })
    }
}

#[async_trait]
impl StreamingSession for DeepgramStreamingSession {
    async fn send_audio(&mut self, audio_chunk: &[u8]) -> Result<()> {
        let mut sender = self.ws_sender.lock().await;

        if let Some(ws) = sender.as_mut() {
            ws.send(Message::Binary(audio_chunk.to_vec()))
                .await
                .map_err(|e| AppError::Transcription(format!("Failed to send audio: {}", e)))?;
            Ok(())
        } else {
            Err(AppError::Transcription("WebSocket connection is closed".to_string()))
        }
    }

    async fn flush(&mut self) -> Result<()> {
        // Deepgram automatically processes all buffered audio when the connection closes
        // We can optionally send a flush message, but it's not required
        log::info!("Flushing Deepgram streaming session");
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        log::info!("Closing Deepgram streaming session");

        *self.is_active.lock().await = false;

        // Send close frame
        let mut sender = self.ws_sender.lock().await;
        if let Some(mut ws) = sender.take() {
            let _ = ws.send(Message::Close(None)).await;
            let _ = ws.close().await;
        }

        // Wait for receiver task to complete
        if let Some(task) = self.receiver_task.take() {
            let _ = task.await;
        }

        Ok(())
    }

    fn is_active(&self) -> bool {
        // We need to use try_lock here since this is a sync method
        // In a real-world scenario, you might want to use a different pattern
        self.is_active.try_lock().map(|guard| *guard).unwrap_or(false)
    }
}

impl Drop for DeepgramStreamingSession {
    fn drop(&mut self) {
        // Attempt to close gracefully
        if let Some(task) = self.receiver_task.take() {
            task.abort();
        }
    }
}

// ===== Deepgram Streaming API Response Types =====

#[derive(Debug, Deserialize)]
struct DeepgramStreamingResponse {
    #[serde(rename = "type")]
    message_type: Option<String>,
    channel: Option<Channel>,
    is_final: Option<bool>,
    start: Option<f64>,
    duration: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct Channel {
    alternatives: Vec<Alternative>,
}

#[derive(Debug, Deserialize)]
struct Alternative {
    transcript: String,
    confidence: f32,
    utterances: Option<Vec<Utterance>>,
}

#[derive(Debug, Deserialize)]
struct Utterance {
    transcript: String,
    start: f64,
    end: f64,
    confidence: f32,
    speaker: u32,
}
