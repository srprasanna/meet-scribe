import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Box, Button, HStack, VStack, Text } from '@chakra-ui/react';
import { AudioDeviceSelector } from './AudioDeviceSelector';

/**
 * Audio Level Meter Component
 */
const AudioLevelMeter = ({ level, color }: { level: number; color: string }) => (
  <Box
    width="100%"
    height="24px"
    bg="gray.100"
    borderRadius="md"
    overflow="hidden"
    position="relative"
  >
    <Box
      height="100%"
      width={`${level}%`}
      bg={color}
      transition="width 0.1s ease-out"
    />
    <Text
      position="absolute"
      top="50%"
      left="50%"
      transform="translate(-50%, -50%)"
      fontSize="xs"
      fontWeight="bold"
      color="gray.700"
    >
      {level.toFixed(0)}%
    </Text>
  </Box>
);

/**
 * Audio Tester Component
 *
 * Allows testing speaker (loopback) and microphone capture separately
 * Shows visual audio level meter for feedback
 */
export function AudioTester() {
  const [speakerDevices, setSpeakerDevices] = useState<string[]>([]);
  const [microphoneDevices, setMicrophoneDevices] = useState<string[]>([]);
  const [selectedSpeaker, setSelectedSpeaker] = useState<string>('');
  const [selectedMicrophone, setSelectedMicrophone] = useState<string>('');

  const [isSpeakerTesting, setIsSpeakerTesting] = useState(false);
  const [isMicrophoneTesting, setIsMicrophoneTesting] = useState(false);

  const [speakerLevel, setSpeakerLevel] = useState(0);
  const [microphoneLevel, setMicrophoneLevel] = useState(0);

  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  // Simulated audio level monitoring (will be replaced with actual audio level detection)
  const levelIntervalRef = useRef<number | null>(null);

  useEffect(() => {
    loadDevices();
    return () => {
      if (levelIntervalRef.current) {
        clearInterval(levelIntervalRef.current);
      }
    };
  }, []);

  const loadDevices = async () => {
    try {
      setLoading(true);
      setError(null);

      const speakers = await invoke<string[]>('list_speaker_devices');
      const microphones = await invoke<string[]>('list_microphone_devices');

      setSpeakerDevices(speakers);
      setMicrophoneDevices(microphones);

      // Auto-select first device
      if (speakers.length > 0) {
        setSelectedSpeaker(speakers[0]);
      }
      if (microphones.length > 0) {
        setSelectedMicrophone(microphones[0]);
      }
    } catch (err) {
      setError(`Failed to load devices: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const startSpeakerTest = async () => {
    try {
      setError(null);
      setIsSpeakerTesting(true);

      // Start capturing speaker output (loopback)
      await invoke('test_speaker_capture', { deviceIndex: parseInt(selectedSpeaker) });

      // Start simulating audio levels (replace with actual audio level monitoring)
      levelIntervalRef.current = window.setInterval(() => {
        // Simulate audio levels - in reality, this should come from the audio capture
        setSpeakerLevel(Math.random() * 100);
      }, 100);

    } catch (err) {
      setError(`Speaker test failed: ${err}`);
      setIsSpeakerTesting(false);
    }
  };

  const stopSpeakerTest = async () => {
    try {
      await invoke('stop_audio_test');
      setIsSpeakerTesting(false);
      setSpeakerLevel(0);

      if (levelIntervalRef.current) {
        clearInterval(levelIntervalRef.current);
        levelIntervalRef.current = null;
      }
    } catch (err) {
      setError(`Failed to stop speaker test: ${err}`);
    }
  };

  const startMicrophoneTest = async () => {
    try {
      setError(null);
      setIsMicrophoneTesting(true);

      // Start capturing microphone input
      await invoke('test_microphone_capture', { deviceIndex: parseInt(selectedMicrophone) });

      // Start simulating audio levels
      levelIntervalRef.current = window.setInterval(() => {
        setMicrophoneLevel(Math.random() * 100);
      }, 100);

    } catch (err) {
      setError(`Microphone test failed: ${err}`);
      setIsMicrophoneTesting(false);
    }
  };

  const stopMicrophoneTest = async () => {
    try {
      await invoke('stop_audio_test');
      setIsMicrophoneTesting(false);
      setMicrophoneLevel(0);

      if (levelIntervalRef.current) {
        clearInterval(levelIntervalRef.current);
        levelIntervalRef.current = null;
      }
    } catch (err) {
      setError(`Failed to stop microphone test: ${err}`);
    }
  };

  if (loading) {
    return (
      <VStack gap={6} align="stretch">
        <Box>
          <Text fontSize="lg" fontWeight="bold" mb={2}>
            Audio Device Testing
          </Text>
          <Text fontSize="sm" color="gray.600">
            Loading audio devices...
          </Text>
        </Box>
      </VStack>
    );
  }

  // Don't render if we don't have any devices loaded
  if (speakerDevices.length === 0 && microphoneDevices.length === 0) {
    return (
      <VStack gap={6} align="stretch">
        <Box>
          <Text fontSize="lg" fontWeight="bold" mb={2}>
            Audio Device Testing
          </Text>
          <Text fontSize="sm" color="gray.600">
            No audio devices found. Please check your system audio settings.
          </Text>
        </Box>
      </VStack>
    );
  }

  return (
    <VStack gap={6} align="stretch">
      <Box>
        <Text fontSize="lg" fontWeight="bold" mb={2}>
          Audio Device Testing
        </Text>
        <Text fontSize="sm" color="gray.600" mb={4}>
          Test your speaker (loopback) and microphone capture to ensure both are working correctly.
          During meetings, both audio sources should be captured and mixed together.
        </Text>
      </Box>

      {error && (
        <Box bg="red.50" borderWidth="1px" borderColor="red.200" borderRadius="md" p={3}>
          <Text color="red.700" fontSize="sm">
            ⚠️ {error}
          </Text>
        </Box>
      )}

      {/* Speaker (Loopback) Testing */}
      {speakerDevices.length > 0 && (
        <Box
          borderWidth="1px"
          borderColor="gray.200"
          borderRadius="lg"
          p={4}
          bg={isSpeakerTesting ? 'blue.50' : 'white'}
        >
          <VStack gap={3} align="stretch">
            <HStack justify="space-between">
              <Text fontWeight="600" fontSize="md">
                🔊 Speaker Output (Loopback)
              </Text>
              {isSpeakerTesting && (
                <Box
                  bg="blue.500"
                  color="white"
                  px={2}
                  py={1}
                  borderRadius="md"
                  fontSize="xs"
                  fontWeight="bold"
                >
                  TESTING
                </Box>
              )}
            </HStack>

            <Text fontSize="sm" color="gray.600" mb={3}>
              Captures what others in the meeting are saying (speaker output)
            </Text>

            <AudioDeviceSelector
              label="Speaker Device"
              value={selectedSpeaker}
              onChange={setSelectedSpeaker}
              devices={speakerDevices}
              loading={loading}
              disabled={isSpeakerTesting}
              helpText="Select the speaker/headset that's playing the meeting audio"
            />

          {isSpeakerTesting && (
            <Box>
              <Text fontSize="sm" mb={2} fontWeight="500">
                Audio Level:
              </Text>
              <AudioLevelMeter level={speakerLevel} color="blue.500" />
            </Box>
          )}

            <Button
              colorScheme={isSpeakerTesting ? 'red' : 'blue'}
              onClick={isSpeakerTesting ? stopSpeakerTest : startSpeakerTest}
              disabled={!selectedSpeaker || isMicrophoneTesting}
            >
              {isSpeakerTesting ? 'Stop Speaker Test' : 'Start Speaker Test'}
            </Button>
          </VStack>
        </Box>
      )}

      {/* Microphone Testing */}
      {microphoneDevices.length > 0 && (
        <Box
          borderWidth="1px"
          borderColor="gray.200"
          borderRadius="lg"
          p={4}
          bg={isMicrophoneTesting ? 'green.50' : 'white'}
        >
          <VStack gap={3} align="stretch">
            <HStack justify="space-between">
              <Text fontWeight="600" fontSize="md">
                🎤 Microphone Input
              </Text>
              {isMicrophoneTesting && (
                <Box
                  bg="green.500"
                  color="white"
                  px={2}
                  py={1}
                  borderRadius="md"
                  fontSize="xs"
                  fontWeight="bold"
                >
                  TESTING
                </Box>
              )}
            </HStack>

            <Text fontSize="sm" color="gray.600" mb={3}>
              Captures your voice when you speak (microphone input)
            </Text>

            <AudioDeviceSelector
              label="Microphone Device"
              value={selectedMicrophone}
              onChange={setSelectedMicrophone}
              devices={microphoneDevices}
              loading={loading}
              disabled={isMicrophoneTesting}
              helpText="Select the microphone you're using for the meeting"
            />

          {isMicrophoneTesting && (
            <Box>
              <Text fontSize="sm" mb={2} fontWeight="500">
                Audio Level:
              </Text>
              <AudioLevelMeter level={microphoneLevel} color="green.500" />
              <Text fontSize="xs" color="gray.600" mt={2}>
                💡 Speak into your microphone to see the level meter respond
              </Text>
            </Box>
          )}

            <Button
              colorScheme={isMicrophoneTesting ? 'red' : 'green'}
              onClick={isMicrophoneTesting ? stopMicrophoneTest : startMicrophoneTest}
              disabled={!selectedMicrophone || isSpeakerTesting}
            >
              {isMicrophoneTesting ? 'Stop Microphone Test' : 'Start Microphone Test'}
            </Button>
          </VStack>
        </Box>
      )}

      {/* Important Note */}
      <Box bg="yellow.50" borderWidth="1px" borderColor="yellow.200" borderRadius="md" p={4}>
        <Text fontSize="sm" fontWeight="600" mb={2} color="yellow.900">
          ⚠️ Current Implementation Issue
        </Text>
        <Text fontSize="sm" color="yellow.800">
          Currently, only speaker output (loopback) is being captured during meetings.
          Microphone input is NOT being captured, which means your voice is not included in recordings.
          This is a known issue that needs to be fixed by implementing dual-capture mixing.
        </Text>
      </Box>
    </VStack>
  );
}
