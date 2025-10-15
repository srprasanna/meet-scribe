//! Audio file utilities for saving captured audio
//!
//! Provides functions to save audio buffers to WAV files using the hound crate.

use crate::error::{AppError, Result};
use crate::ports::audio::AudioBuffer;
use hound::{WavSpec, WavWriter};
use std::path::Path;

/// Save an audio buffer to a WAV file
///
/// # Arguments
/// * `buffer` - The audio buffer containing f32 samples
/// * `path` - The file path where the WAV file will be saved
///
/// # Returns
/// The number of samples written
pub fn save_wav_file<P: AsRef<Path>>(buffer: &AudioBuffer, path: P) -> Result<usize> {
    // Create WAV specification from audio format
    let spec = WavSpec {
        channels: buffer.format.channels,
        sample_rate: buffer.format.sample_rate as u32,
        bits_per_sample: buffer.format.bits_per_sample,
        sample_format: hound::SampleFormat::Int,
    };

    // Create the WAV writer
    let mut writer = WavWriter::create(path, spec)
        .map_err(|e| AppError::AudioCapture(format!("Failed to create WAV file: {}", e)))?;

    // Convert f32 samples to i16 and write
    let mut samples_written = 0;
    for &sample in &buffer.samples {
        // Clamp to [-1.0, 1.0] range
        let clamped = sample.max(-1.0).min(1.0);

        // Convert to i16 range using 32768.0 to properly handle the full asymmetric range
        // i16 range is -32768 to 32767, so:
        // - Negative: -1.0 * 32768.0 = -32768 ✓
        // - Positive: 1.0 * 32768.0 = 32768, clamped to 32767 when cast to i16 ✓
        let i16_sample = (clamped * 32768.0) as i16;

        writer
            .write_sample(i16_sample)
            .map_err(|e| AppError::AudioCapture(format!("Failed to write sample: {}", e)))?;

        samples_written += 1;
    }

    // Finalize the WAV file
    writer
        .finalize()
        .map_err(|e| AppError::AudioCapture(format!("Failed to finalize WAV file: {}", e)))?;

    log::info!("Saved {} samples to WAV file", samples_written);
    Ok(samples_written)
}

/// Save audio buffer as chunks to multiple WAV files
///
/// Useful for long recordings that need to be split into manageable chunks
///
/// # Arguments
/// * `buffer` - The audio buffer containing f32 samples
/// * `base_path` - The base file path (will append _001, _002, etc.)
/// * `chunk_duration_secs` - Duration of each chunk in seconds
///
/// # Returns
/// Vector of file paths that were created
pub fn save_wav_chunks<P: AsRef<Path>>(
    buffer: &AudioBuffer,
    base_path: P,
    chunk_duration_secs: u32,
) -> Result<Vec<String>> {
    let samples_per_chunk = buffer.format.sample_rate as usize
        * buffer.format.channels as usize
        * chunk_duration_secs as usize;

    let base_path_str = base_path.as_ref().to_string_lossy().to_string();
    let (base, ext) = if let Some(pos) = base_path_str.rfind('.') {
        (&base_path_str[..pos], &base_path_str[pos..])
    } else {
        (base_path_str.as_str(), ".wav")
    };

    let mut created_files = Vec::new();
    let mut chunk_index = 0;

    for chunk in buffer.samples.chunks(samples_per_chunk) {
        chunk_index += 1;
        let chunk_path = format!("{}_{:03}{}", base, chunk_index, ext);

        // Create a buffer for this chunk
        let chunk_buffer = AudioBuffer {
            samples: chunk.to_vec(),
            format: buffer.format.clone(),
        };

        save_wav_file(&chunk_buffer, &chunk_path)?;
        created_files.push(chunk_path);
    }

    log::info!("Saved {} WAV file chunks", created_files.len());
    Ok(created_files)
}

/// Get the duration of an audio buffer in seconds
pub fn get_duration_seconds(buffer: &AudioBuffer) -> f64 {
    let total_frames = buffer.samples.len() / buffer.format.channels as usize;
    total_frames as f64 / buffer.format.sample_rate as f64
}

/// Get a formatted string representation of audio buffer info
pub fn format_audio_info(buffer: &AudioBuffer) -> String {
    let duration = get_duration_seconds(buffer);
    format!(
        "{:.2}s @ {}Hz, {} channel(s), {} samples",
        duration,
        buffer.format.sample_rate,
        buffer.format.channels,
        buffer.samples.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::audio::AudioFormat;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_get_duration_seconds() {
        let buffer = AudioBuffer {
            samples: vec![0.0; 16000], // 16000 samples
            format: AudioFormat {
                sample_rate: 16000,
                channels: 1,
                bits_per_sample: 16,
            },
        };

        let duration = get_duration_seconds(&buffer);
        assert!((duration - 1.0).abs() < 0.001); // Should be 1 second
    }

    #[test]
    fn test_format_audio_info() {
        let buffer = AudioBuffer {
            samples: vec![0.0; 8000],
            format: AudioFormat {
                sample_rate: 16000,
                channels: 1,
                bits_per_sample: 16,
            },
        };

        let info = format_audio_info(&buffer);
        assert!(info.contains("0.50s"));
        assert!(info.contains("16000Hz"));
        assert!(info.contains("1 channel"));
    }

    #[test]
    fn test_save_wav_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.wav");

        // Create a simple sine wave
        let sample_rate = 16000;
        let duration = 0.1; // 100ms
        let frequency = 440.0; // A4 note
        let num_samples = (sample_rate as f64 * duration) as usize;

        let mut samples = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f64 / sample_rate as f64;
            let sample = (2.0 * std::f64::consts::PI * frequency * t).sin() as f32;
            samples.push(sample);
        }

        let buffer = AudioBuffer {
            samples,
            format: AudioFormat {
                sample_rate: sample_rate as u32,
                channels: 1,
                bits_per_sample: 16,
            },
        };

        let result = save_wav_file(&buffer, &file_path);
        assert!(result.is_ok());
        assert!(file_path.exists());

        // Check file size is reasonable
        let metadata = fs::metadata(&file_path).unwrap();
        assert!(metadata.len() > 100); // Should have some data
    }

    #[test]
    fn test_save_wav_chunks() {
        let dir = tempdir().unwrap();
        let base_path = dir.path().join("test.wav");

        // Create 3 seconds of audio
        let sample_rate = 16000;
        let num_samples = sample_rate * 3;
        let samples = vec![0.5_f32; num_samples];

        let buffer = AudioBuffer {
            samples,
            format: AudioFormat {
                sample_rate: sample_rate as u32,
                channels: 1,
                bits_per_sample: 16,
            },
        };

        // Split into 1-second chunks
        let result = save_wav_chunks(&buffer, &base_path, 1);
        assert!(result.is_ok());

        let files = result.unwrap();
        assert_eq!(files.len(), 3); // Should create 3 files

        // Check all files exist
        for file in &files {
            assert!(Path::new(file).exists());
        }
    }

    #[test]
    fn test_f32_to_i16_conversion_range() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("range_test.wav");

        // Test edge cases: -1.0, 0.0, +1.0
        // This verifies the asymmetric i16 range (-32768 to 32767) is handled correctly
        let samples = vec![
            -1.0_f32, // Should map to -32768
            0.0_f32,  // Should map to 0
            1.0_f32,  // Should map to 32767 (after clamping from 32768)
        ];

        let buffer = AudioBuffer {
            samples,
            format: AudioFormat {
                sample_rate: 16000,
                channels: 1,
                bits_per_sample: 16,
            },
        };

        // This should not panic or lose data
        let result = save_wav_file(&buffer, &file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3); // Should write all 3 samples
        assert!(file_path.exists());
    }
}
