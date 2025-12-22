import { NativeSelectRoot, NativeSelectField } from '@chakra-ui/react';

interface AudioDeviceSelectorProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  devices: string[];
  loading?: boolean;
  disabled?: boolean;
  helpText?: string;
}

/**
 * Reusable Audio Device Selector Component
 *
 * Displays a dropdown for selecting audio devices (speakers or microphones)
 */
export function AudioDeviceSelector({
  label,
  value,
  onChange,
  devices,
  loading = false,
  disabled = false,
  helpText,
}: AudioDeviceSelectorProps) {
  return (
    <div>
      <label
        style={{
          display: 'block',
          marginBottom: '8px',
          fontWeight: '500',
          fontSize: '14px',
        }}
      >
        {label}
      </label>
      <NativeSelectRoot>
        <NativeSelectField
          value={value}
          onChange={(e) => onChange(e.target.value)}
          disabled={loading || disabled}
        >
          {loading ? (
            <option>Loading devices...</option>
          ) : devices.length === 0 ? (
            <option>No devices found</option>
          ) : (
            devices.map((device) => (
              <option key={device} value={device}>
                {device}
              </option>
            ))
          )}
        </NativeSelectField>
      </NativeSelectRoot>
      {helpText && (
        <div style={{ fontSize: '12px', color: '#666', marginTop: '4px' }}>
          {helpText}
        </div>
      )}
    </div>
  );
}
