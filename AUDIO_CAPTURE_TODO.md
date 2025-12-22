# Audio Capture Implementation TODO

## Critical Issue

**Problem**: Currently, only speaker output (loopback) is being captured during meetings. The user's microphone input is NOT being captured, which means their voice is not included in the recording.

**Impact**: Meeting recordings are incomplete - they capture what others are saying (speaker output) but NOT what the meeting host is saying (microphone input).

## Current Implementation

See [apps/desktop/src-tauri/src/commands/meeting.rs:76-79](apps/desktop/src-tauri/src/commands/meeting.rs#L76-L79):

```rust
// For now, we use the speaker device for capture (loopback mode on Windows)
// TODO: Use microphone_device for direct microphone input if needed
let mut audio_capture = state.audio_capture.lock().await;
match audio_capture.start_capture(request.speaker_device).await {
```

The `StartMeetingRequest` already includes both `speaker_device` and `microphone_device`, but only the speaker device is being used.

## Required Solution: Dual-Capture Audio Mixing

To properly capture meeting audio, we need to:

### 1. Capture TWO audio streams simultaneously:
   - **Speaker Output (Loopback)**: What others in the meeting are saying
     - Windows: WASAPI loopback mode on render endpoint
     - Linux: PulseAudio monitor source

   - **Microphone Input**: What the user is saying
     - Windows: WASAPI capture mode on capture endpoint
     - Linux: PulseAudio capture from microphone device

### 2. Mix the two streams together:
   - Both streams need to be resampled to the same format if necessary
   - Audio samples need to be mixed (added together with proper level normalization)
   - Mixed audio is then encoded to WAV and saved to file

## Implementation Plan

### Phase 1: Add Microphone Capture Port
1. Extend `AudioCapturePort` trait to support dual capture:
   ```rust
   async fn start_dual_capture(
       &mut self,
       speaker_device: Option<String>,
       microphone_device: Option<String>,
   ) -> Result<()>;
   ```

2. Update `WasapiAudioCapture` to support simultaneous capture:
   - Create two capture threads (one for loopback, one for microphone)
   - Implement audio mixing logic
   - Handle format conversion if speaker and mic have different formats

### Phase 2: Implement Windows Dual Capture
File: [apps/desktop/src-tauri/src/adapters/audio/windows.rs](apps/desktop/src-tauri/src/adapters/audio/windows.rs)

**Key Changes**:
1. Add microphone capture endpoint initialization
2. Create separate capture thread for microphone
3. Implement audio mixer:
   ```rust
   fn mix_audio_samples(speaker_buffer: &[f32], mic_buffer: &[f32]) -> Vec<f32> {
       // Mix samples with proper gain control
       // May need to handle different buffer sizes
   }
   ```
4. Handle synchronization between two audio streams
5. Write mixed audio to output buffer

**Technical Considerations**:
- **Sample Rate**: May differ between devices - need resampling
- **Buffer Sizes**: Likely different - need ring buffer
- **Latency**: Mic and speaker may have different latencies
- **Gain Control**: Prevent clipping when mixing

### Phase 3: Implement Linux Dual Capture
File: [apps/desktop/src-tauri/src/adapters/audio/linux.rs](apps/desktop/src-tauri/src/adapters/audio/linux.rs)

Similar approach using PulseAudio/PipeWire for both monitor source and microphone capture.

### Phase 4: Update Meeting Command
File: [apps/desktop/src-tauri/src/commands/meeting.rs:74-79](apps/desktop/src-tauri/src/commands/meeting.rs#L74-L79)

```rust
// Start dual audio capture (speaker + microphone)
let mut audio_capture = state.audio_capture.lock().await;
match audio_capture.start_dual_capture(
    request.speaker_device,
    request.microphone_device
).await {
    // ...
}
```

### Phase 5: Audio Testing Interface
**DONE** ✅ Added Audio Testing tab in Settings:
- File: [apps/desktop/src/components/AudioTester.tsx](apps/desktop/src/components/AudioTester.tsx)
- Allows testing speaker and microphone capture separately
- Shows visual audio level meter
- Backend commands added (stubs): `test_speaker_capture`, `test_microphone_capture`, `stop_audio_test`

**TODO**: Implement actual audio level monitoring in backend

## Testing Audio Capture

### Using the Audio Testing Tab

1. Go to Settings → Audio Testing tab
2. Test Speaker Capture:
   - Select speaker device
   - Click "Start Speaker Test"
   - Play audio on your computer
   - Observe audio level meter

3. Test Microphone Capture:
   - Select microphone device
   - Click "Start Microphone Test"
   - Speak into microphone
   - Observe audio level meter

4. During meetings:
   - Both speaker AND microphone should be captured
   - Verify by checking the recording file after the meeting

### Verification Checklist

After implementing dual capture:
- [ ] Speaker output is captured (others' voices)
- [ ] Microphone input is captured (user's voice)
- [ ] Audio levels are properly balanced
- [ ] No clipping or distortion
- [ ] Audio stays synchronized
- [ ] Works with different device formats
- [ ] Handles device disconnection gracefully

## References

### Windows WASAPI Resources
- [WASAPI Loopback Recording](https://docs.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording)
- [Capturing a Stream](https://docs.microsoft.com/en-us/windows/win32/coreaudio/capturing-a-stream)

### Audio Mixing
- [Digital Audio Mixing](https://www.voegler.eu/pub/audio/digital-audio.html)
- Ensure proper gain staging to prevent clipping
- Consider using soft clipping or limiting

### Rust Audio Libraries
- `cpal` - Cross-platform audio I/O (consider for future refactoring)
- `rubato` - Audio resampling
- `dasp` - Digital audio signal processing

## Priority

**CRITICAL** - This is a blocker for production use. Meeting recordings without the host's voice are essentially useless for most use cases.

Recommended: Implement Phase 1-4 before the next release.
