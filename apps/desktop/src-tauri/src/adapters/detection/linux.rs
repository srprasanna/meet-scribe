//! Linux AT-SPI participant detector
//!
//! Uses the AT-SPI (Assistive Technology Service Provider Interface) to detect
//! meeting participants from Teams, Zoom, and Google Meet windows on Linux.

use crate::domain::models::Platform;
use crate::error::{AppError, Result};
use crate::ports::detection::{
    DetectedMeeting, DetectedParticipant, DetectionConfig, DetectionMethod, DetectionResult,
    ParticipantDetectorPort,
};
use async_trait::async_trait;
use std::collections::HashSet;
use std::process::Command;

/// Linux AT-SPI participant detector
///
/// Uses AT-SPI D-Bus interface to query accessibility information from applications.
/// Falls back to wmctrl/xdotool for window enumeration if needed.
pub struct AtSpiDetector {
    /// Whether AT-SPI is available on this system
    atspi_available: bool,
}

impl AtSpiDetector {
    /// Creates a new AT-SPI detector
    pub fn new() -> Self {
        let atspi_available = Self::check_atspi_available();
        Self { atspi_available }
    }

    /// Checks if AT-SPI is available on the system
    fn check_atspi_available() -> bool {
        // Check if atspi-bus-launcher or at-spi2-registryd is running
        if let Ok(output) = Command::new("pgrep")
            .args(["-x", "at-spi2-registryd"])
            .output()
        {
            if output.status.success() {
                return true;
            }
        }

        // Also check for the AT-SPI D-Bus service
        if let Ok(output) = Command::new("dbus-send")
            .args([
                "--session",
                "--dest=org.a11y.Bus",
                "--print-reply",
                "/org/a11y/bus",
                "org.freedesktop.DBus.Peer.Ping",
            ])
            .output()
        {
            return output.status.success();
        }

        false
    }

    /// Enumerates windows using wmctrl or xdotool
    fn enumerate_windows(&self) -> Result<Vec<(u64, String, u32, String)>> {
        let mut windows = Vec::new();

        // Try wmctrl first (more widely available)
        if let Ok(output) = Command::new("wmctrl").args(["-l", "-p"]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if let Some(window_info) = self.parse_wmctrl_line(line) {
                        windows.push(window_info);
                    }
                }
                return Ok(windows);
            }
        }

        // Fall back to xdotool
        if let Ok(output) = Command::new("xdotool")
            .args(["search", "--name", "."])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for window_id_str in stdout.lines() {
                    if let Ok(window_id) = window_id_str.parse::<u64>() {
                        if let Some(info) = self.get_window_info_xdotool(window_id) {
                            windows.push(info);
                        }
                    }
                }
                return Ok(windows);
            }
        }

        // If neither tool is available, return empty list with warning
        log::warn!("Neither wmctrl nor xdotool available for window enumeration");
        Ok(windows)
    }

    /// Parses a line from wmctrl -l -p output
    fn parse_wmctrl_line(&self, line: &str) -> Option<(u64, String, u32, String)> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return None;
        }

        // Format: 0x12345678  0 12345  hostname Title of Window
        let window_id = u64::from_str_radix(parts[0].trim_start_matches("0x"), 16).ok()?;
        let _desktop = parts[1];
        let pid: u32 = parts[2].parse().ok()?;
        let _hostname = parts[3];

        // Title is everything after the hostname
        let title = parts[4..].join(" ");

        // Get window class using xprop if available
        let class = self.get_window_class(window_id).unwrap_or_default();

        Some((window_id, title, pid, class))
    }

    /// Gets window info using xdotool
    fn get_window_info_xdotool(&self, window_id: u64) -> Option<(u64, String, u32, String)> {
        // Get window name
        let name_output = Command::new("xdotool")
            .args(["getwindowname", &window_id.to_string()])
            .output()
            .ok()?;

        if !name_output.status.success() {
            return None;
        }

        let title = String::from_utf8_lossy(&name_output.stdout)
            .trim()
            .to_string();

        // Get window PID
        let pid_output = Command::new("xdotool")
            .args(["getwindowpid", &window_id.to_string()])
            .output()
            .ok()?;

        let pid: u32 = if pid_output.status.success() {
            String::from_utf8_lossy(&pid_output.stdout)
                .trim()
                .parse()
                .unwrap_or(0)
        } else {
            0
        };

        let class = self.get_window_class(window_id).unwrap_or_default();

        Some((window_id, title, pid, class))
    }

    /// Gets window class using xprop
    fn get_window_class(&self, window_id: u64) -> Option<String> {
        let output = Command::new("xprop")
            .args(["-id", &format!("0x{:x}", window_id), "WM_CLASS"])
            .output()
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse: WM_CLASS(STRING) = "class", "Class"
            if let Some(start) = stdout.find('"') {
                let rest = &stdout[start + 1..];
                if let Some(end) = rest.find('"') {
                    return Some(rest[..end].to_string());
                }
            }
        }
        None
    }

    /// Identifies the platform from window title and class
    fn identify_platform(title: &str, class: &str) -> Option<Platform> {
        let title_lower = title.to_lowercase();
        let class_lower = class.to_lowercase();

        // Microsoft Teams (Electron app on Linux)
        if title_lower.contains("microsoft teams")
            || title_lower.contains("| teams")
            || class_lower.contains("teams")
            || class_lower.contains("microsoft teams")
        {
            return Some(Platform::Teams);
        }

        // Zoom
        if title_lower.contains("zoom meeting")
            || title_lower.contains("zoom")
            || class_lower.contains("zoom")
        {
            return Some(Platform::Zoom);
        }

        // Google Meet (runs in browser)
        if title_lower.contains("meet.google.com")
            || title_lower.contains("google meet")
            || (title_lower.contains("meet") && title_lower.contains("google"))
        {
            return Some(Platform::Meet);
        }

        None
    }

    /// Uses AT-SPI to get accessible children of a window
    fn get_atspi_children(&self, window_id: u64, platform: &Platform) -> Result<Vec<String>> {
        let mut participants = HashSet::new();

        // Use gdbus or python-atspi to query accessibility tree
        // This is a simplified implementation using accerciser's approach
        let script = format!(
            r#"
import gi
gi.require_version('Atspi', '2.0')
from gi.repository import Atspi

def find_participants(obj, platform, depth=0, participants=set()):
    if depth > 20:
        return participants

    try:
        name = obj.get_name()
        role = obj.get_role_name()

        # Look for list items and text elements that might be participant names
        if role in ['list item', 'label', 'text', 'table cell']:
            if name and len(name) > 1 and len(name) < 100:
                # Filter out common UI elements
                skip_words = ['mute', 'camera', 'video', 'share', 'chat',
                              'participants', 'leave', 'end', 'settings',
                              'more', 'options', 'reactions']
                name_lower = name.lower()
                if not any(word in name_lower for word in skip_words):
                    if any(c.isalpha() for c in name):
                        participants.add(name)

        # Recurse into children
        for i in range(obj.get_child_count()):
            child = obj.get_child_at_index(i)
            if child:
                find_participants(child, platform, depth + 1, participants)
    except:
        pass

    return participants

# Find the window by ID
desktop = Atspi.get_desktop(0)
for i in range(desktop.get_child_count()):
    app = desktop.get_child_at_index(i)
    if app:
        for j in range(app.get_child_count()):
            window = app.get_child_at_index(j)
            if window:
                # Check if this is our target window
                # Note: AT-SPI window IDs don't always match X11 window IDs
                participants = find_participants(window, "{}", 0, set())
                if participants:
                    for p in sorted(participants):
                        print(p)
"#,
            match platform {
                Platform::Teams => "teams",
                Platform::Zoom => "zoom",
                Platform::Meet => "meet",
            }
        );

        // Execute the Python script
        if let Ok(output) = Command::new("python3").args(["-c", &script]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let name = line.trim();
                    if !name.is_empty() && self.is_likely_participant_name(name, platform) {
                        participants.insert(name.to_string());
                    }
                }
            }
        }

        Ok(participants.into_iter().collect())
    }

    /// Determines if a string is likely a participant name
    fn is_likely_participant_name(&self, name: &str, platform: &Platform) -> bool {
        let name = name.trim();

        if name.len() < 2 || name.len() > 100 {
            return false;
        }

        let skip_patterns = [
            "mute",
            "unmute",
            "camera",
            "video",
            "share",
            "screen",
            "chat",
            "participants",
            "people",
            "reactions",
            "leave",
            "end",
            "meeting",
            "settings",
            "more",
            "options",
            "raise hand",
            "pin",
            "spotlight",
            "remove",
            "admit",
            "waiting",
            "lobby",
            "recording",
            "live",
            "call",
            "join",
            "audio",
            "speaker",
            "microphone",
            "minimize",
            "maximize",
            "close",
            "search",
            "filter",
            "invite",
            "copy",
            "link",
        ];

        let name_lower = name.to_lowercase();
        for pattern in skip_patterns.iter() {
            if name_lower == *pattern {
                return false;
            }
        }

        match platform {
            Platform::Teams => {
                if name_lower.starts_with("microsoft")
                    || name_lower.contains("teams")
                    || name_lower.contains("new meeting")
                {
                    return false;
                }
            }
            Platform::Zoom => {
                if name_lower.starts_with("zoom") || name_lower.contains("breakout") {
                    return false;
                }
            }
            Platform::Meet => {
                if name_lower.contains("google") || name_lower.contains("present") {
                    return false;
                }
            }
        }

        name.chars().any(|c| c.is_alphabetic())
    }

    /// Checks if a name indicates it's the current user
    fn is_self_indicator(name: &str) -> bool {
        let lower = name.to_lowercase();
        lower.contains("(you)") || lower.contains("(me)") || lower.ends_with(" (you)")
    }

    /// Removes self-indicators from a participant name
    fn clean_participant_name(name: &str) -> String {
        name.replace("(You)", "")
            .replace("(you)", "")
            .replace("(Me)", "")
            .replace("(me)", "")
            .trim()
            .to_string()
    }
}

impl Default for AtSpiDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ParticipantDetectorPort for AtSpiDetector {
    async fn list_active_meetings(&self) -> Result<Vec<DetectedMeeting>> {
        let windows = self.enumerate_windows()?;

        let meetings: Vec<DetectedMeeting> = windows
            .into_iter()
            .filter_map(|(window_id, title, pid, class)| {
                Self::identify_platform(&title, &class).map(|platform| DetectedMeeting {
                    platform,
                    window_title: Some(title),
                    process_id: pid,
                    window_handle: window_id,
                })
            })
            .collect();

        Ok(meetings)
    }

    async fn detect_participants(
        &self,
        meeting: &DetectedMeeting,
        config: &DetectionConfig,
    ) -> Result<DetectionResult> {
        let mut warnings = Vec::new();

        // Try AT-SPI first if available
        let participant_names = if self.atspi_available {
            match self.get_atspi_children(meeting.window_handle, &meeting.platform) {
                Ok(names) => names,
                Err(e) => {
                    warnings.push(format!("AT-SPI detection failed: {}", e));
                    Vec::new()
                }
            }
        } else {
            warnings.push(
                "AT-SPI is not available. Please ensure at-spi2-core is installed and running."
                    .to_string(),
            );
            Vec::new()
        };

        if participant_names.is_empty() {
            warnings.push(
                "No participants detected. The participant panel may be closed or accessibility support may be limited.".to_string()
            );
        }

        // Convert names to DetectedParticipant structs
        let participants: Vec<DetectedParticipant> = participant_names
            .into_iter()
            .filter(|name| config.include_self || !Self::is_self_indicator(name))
            .map(|name| {
                let is_self = Self::is_self_indicator(&name);
                let clean_name = Self::clean_participant_name(&name);
                DetectedParticipant {
                    name: clean_name,
                    is_self,
                    is_speaking: None,
                    has_video: None,
                    is_muted: None,
                }
            })
            .collect();

        let confidence = if participants.is_empty() {
            0.0
        } else if participants.len() > 1 {
            0.7 // Slightly lower confidence than Windows due to more complex accessibility stack
        } else {
            0.4
        };

        Ok(DetectionResult {
            meeting: meeting.clone(),
            participants,
            method: DetectionMethod::AtSpi,
            confidence,
            warnings,
        })
    }

    async fn auto_detect(&self, config: &DetectionConfig) -> Result<Option<DetectionResult>> {
        let meetings = self.list_active_meetings().await?;

        let meeting = if let Some(ref target) = config.target_platform {
            meetings.into_iter().find(|m| &m.platform == target)
        } else {
            meetings.into_iter().next()
        };

        match meeting {
            Some(m) => {
                let result = self.detect_participants(&m, config).await?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }

    fn detection_method(&self) -> DetectionMethod {
        DetectionMethod::AtSpi
    }

    fn is_available(&self) -> bool {
        self.atspi_available
    }
}
