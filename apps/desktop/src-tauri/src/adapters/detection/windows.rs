//! Windows UI Automation participant detector
//!
//! Uses the Windows UI Automation API to detect meeting participants from
//! Teams, Zoom, and Google Meet windows.

use crate::domain::models::Platform;
use crate::error::{AppError, Result};
use crate::ports::detection::{
    DetectedMeeting, DetectedParticipant, DetectionConfig, DetectionMethod, DetectionResult,
    ParticipantDetectorPort,
};
use async_trait::async_trait;
use std::collections::HashSet;
use windows::core::VARIANT;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationTreeWalker, TreeScope_Descendants,
    UIA_ControlTypePropertyId, UIA_ListItemControlTypeId, UIA_TextControlTypeId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
};

/// Windows UI Automation participant detector
///
/// Note: COM objects are created on-demand in a blocking context since they
/// are apartment-threaded and cannot be shared across async contexts.
pub struct UiAutomationDetector;

impl UiAutomationDetector {
    /// Creates a new UI Automation detector
    pub fn new() -> Self {
        Self
    }

    /// Enumerates all visible windows and returns meeting-related ones
    /// Returns (window_handle_as_u64, title, class_name, process_id)
    fn enumerate_meeting_windows() -> Result<Vec<(u64, String, String, u32)>> {
        let mut windows: Vec<(u64, String, String, u32)> = Vec::new();

        unsafe {
            let windows_ptr = &mut windows as *mut Vec<(u64, String, String, u32)>;

            EnumWindows(Some(enum_windows_callback), LPARAM(windows_ptr as isize))
                .map_err(|e| AppError::Detection(format!("Failed to enumerate windows: {}", e)))?;
        }

        Ok(windows)
    }

    /// Identifies the platform from window title and class name
    fn identify_platform(title: &str, class_name: &str) -> Option<Platform> {
        let title_lower = title.to_lowercase();
        let class_lower = class_name.to_lowercase();

        // Microsoft Teams
        if title_lower.contains("microsoft teams")
            || title_lower.contains("| teams")
            || class_lower.contains("teams")
        {
            return Some(Platform::Teams);
        }

        // Zoom
        if title_lower.contains("zoom meeting")
            || title_lower.contains("zoom")
            || class_lower.contains("zoomwebviewhost")
            || class_lower.contains("zplayer")
        {
            return Some(Platform::Zoom);
        }

        // Google Meet (runs in Chrome/Edge)
        if title_lower.contains("meet.google.com")
            || title_lower.contains("google meet")
            || (title_lower.contains("meet") && title_lower.contains("google"))
        {
            return Some(Platform::Meet);
        }

        None
    }

    /// Finds participant list elements in a UI tree (blocking, runs in spawn_blocking)
    fn find_participants_blocking(window_handle: u64, platform: Platform) -> Result<Vec<String>> {
        let mut participants = HashSet::new();

        unsafe {
            // Initialize COM for this thread
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            // Create UI Automation instance
            let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)
                .map_err(|e| {
                    AppError::Detection(format!("Failed to create UI Automation: {}", e))
                })?;

            let hwnd = HWND(window_handle as *mut std::ffi::c_void);

            // Get root element for the window
            let root = automation.ElementFromHandle(hwnd).map_err(|e| {
                AppError::Detection(format!("Failed to get element from window: {}", e))
            })?;

            // Create condition for list items
            let list_item_condition = automation
                .CreatePropertyCondition(
                    UIA_ControlTypePropertyId,
                    &VARIANT::from(UIA_ListItemControlTypeId.0),
                )
                .map_err(|e| AppError::Detection(format!("Failed to create condition: {}", e)))?;

            // Find all list items
            if let Ok(list_items) = root.FindAll(TreeScope_Descendants, &list_item_condition) {
                let count = list_items.Length().unwrap_or(0);
                for i in 0..count {
                    if let Ok(item) = list_items.GetElement(i) {
                        if let Ok(name) = item.CurrentName() {
                            let name_str = name.to_string();
                            if Self::is_likely_participant_name(&name_str, &platform) {
                                participants.insert(name_str);
                            }
                        }
                    }
                }
            }

            // Also try text elements
            let text_condition = automation
                .CreatePropertyCondition(
                    UIA_ControlTypePropertyId,
                    &VARIANT::from(UIA_TextControlTypeId.0),
                )
                .map_err(|e| {
                    AppError::Detection(format!("Failed to create text condition: {}", e))
                })?;

            if let Ok(text_elements) = root.FindAll(TreeScope_Descendants, &text_condition) {
                let count = text_elements.Length().unwrap_or(0);
                for i in 0..count {
                    if let Ok(item) = text_elements.GetElement(i) {
                        if let Ok(name) = item.CurrentName() {
                            let name_str = name.to_string();
                            if Self::is_likely_participant_name(&name_str, &platform) {
                                participants.insert(name_str);
                            }
                        }
                    }
                }
            }

            // Walk the tree using ContentViewWalker
            if let Ok(walker) = automation.ContentViewWalker() {
                Self::walk_tree_for_participants(&walker, &root, &platform, &mut participants, 0);
            }
        }

        Ok(participants.into_iter().collect())
    }

    /// Recursively walks the UI tree to find participant names
    fn walk_tree_for_participants(
        walker: &IUIAutomationTreeWalker,
        element: &windows::Win32::UI::Accessibility::IUIAutomationElement,
        platform: &Platform,
        participants: &mut HashSet<String>,
        depth: u32,
    ) {
        // Limit recursion depth
        if depth > 15 {
            return;
        }

        unsafe {
            // Check current element
            if let Ok(name) = element.CurrentName() {
                let name_str = name.to_string();
                if Self::is_likely_participant_name(&name_str, platform) {
                    participants.insert(name_str);
                }
            }

            // Get first child
            if let Ok(child) = walker.GetFirstChildElement(element) {
                Self::walk_tree_for_participants(walker, &child, platform, participants, depth + 1);

                // Get siblings
                let mut current = child;
                while let Ok(sibling) = walker.GetNextSiblingElement(&current) {
                    Self::walk_tree_for_participants(
                        walker,
                        &sibling,
                        platform,
                        participants,
                        depth + 1,
                    );
                    current = sibling;
                }
            }
        }
    }

    /// Determines if a string is likely a participant name
    fn is_likely_participant_name(name: &str, platform: &Platform) -> bool {
        let name = name.trim();

        // Skip empty or very short names
        if name.len() < 2 {
            return false;
        }

        // Skip very long names
        if name.len() > 100 {
            return false;
        }

        // Skip common UI labels
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
            "lower hand",
            "pin",
            "spotlight",
            "remove",
            "admit",
            "waiting",
            "lobby",
            "host",
            "co-host",
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
            "you",
            "(you)",
            "me",
            "(me)",
            "presenter",
            "attendee",
            "guest",
        ];

        let name_lower = name.to_lowercase();
        for pattern in skip_patterns.iter() {
            if name_lower == *pattern {
                return false;
            }
        }

        // Platform-specific filtering
        match platform {
            Platform::Teams => {
                if name_lower.starts_with("microsoft")
                    || name_lower.contains("teams")
                    || name_lower.contains("new meeting")
                    || name_lower.contains("join with")
                {
                    return false;
                }
            }
            Platform::Zoom => {
                if name_lower.starts_with("zoom")
                    || name_lower.contains("breakout")
                    || name_lower.contains("polling")
                {
                    return false;
                }
            }
            Platform::Meet => {
                if name_lower.contains("google")
                    || name_lower.contains("meet")
                    || name_lower.contains("present")
                {
                    return false;
                }
            }
        }

        // Must have letters and reasonable number of spaces
        let has_letters = name.chars().any(|c| c.is_alphabetic());
        let reasonable_spaces = name.matches(' ').count() <= 5;

        has_letters && reasonable_spaces
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

impl Default for UiAutomationDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Callback function for EnumWindows
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = &mut *(lparam.0 as *mut Vec<(u64, String, String, u32)>);

    // Skip invisible windows
    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }

    // Get window title
    let mut title_buffer = [0u16; 512];
    let title_len = GetWindowTextW(hwnd, &mut title_buffer);
    if title_len == 0 {
        return BOOL(1);
    }
    let title = String::from_utf16_lossy(&title_buffer[..title_len as usize]);

    // Get class name
    let mut class_buffer = [0u16; 256];
    let class_len = GetClassNameW(hwnd, &mut class_buffer);
    let class_name = if class_len > 0 {
        String::from_utf16_lossy(&class_buffer[..class_len as usize])
    } else {
        String::new()
    };

    // Get process ID
    let mut process_id: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut process_id));

    // Check if it's a meeting window
    if UiAutomationDetector::identify_platform(&title, &class_name).is_some() {
        // Convert HWND to u64 for thread-safe transfer
        let hwnd_value = hwnd.0 as u64;
        windows.push((hwnd_value, title, class_name, process_id));
    }

    BOOL(1)
}

#[async_trait]
impl ParticipantDetectorPort for UiAutomationDetector {
    async fn list_active_meetings(&self) -> Result<Vec<DetectedMeeting>> {
        // Run window enumeration in a blocking task
        let windows = tokio::task::spawn_blocking(Self::enumerate_meeting_windows)
            .await
            .map_err(|e| AppError::Detection(format!("Task join error: {}", e)))??;

        let meetings: Vec<DetectedMeeting> = windows
            .into_iter()
            .filter_map(|(hwnd_value, title, class_name, pid)| {
                Self::identify_platform(&title, &class_name).map(|platform| DetectedMeeting {
                    platform,
                    window_title: Some(title),
                    process_id: pid,
                    window_handle: hwnd_value,
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
        let window_handle = meeting.window_handle;
        let platform = meeting.platform.clone();
        let include_self = config.include_self;

        // Run participant detection in a blocking task (COM is apartment-threaded)
        let participant_names = tokio::task::spawn_blocking(move || {
            Self::find_participants_blocking(window_handle, platform)
        })
        .await
        .map_err(|e| AppError::Detection(format!("Task join error: {}", e)))??;

        let mut warnings = Vec::new();
        if participant_names.is_empty() {
            warnings.push(
                "No participants detected. The participant panel may be closed or the UI structure may have changed.".to_string()
            );
        }

        // Convert names to DetectedParticipant structs
        let participants: Vec<DetectedParticipant> = participant_names
            .into_iter()
            .filter(|name| include_self || !Self::is_self_indicator(name))
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

        // Calculate confidence
        let confidence = if participants.is_empty() {
            0.0
        } else if participants.len() > 1 {
            0.8
        } else {
            0.5
        };

        Ok(DetectionResult {
            meeting: meeting.clone(),
            participants,
            method: DetectionMethod::UiAutomation,
            confidence,
            warnings,
        })
    }

    async fn auto_detect(&self, config: &DetectionConfig) -> Result<Option<DetectionResult>> {
        let meetings = self.list_active_meetings().await?;

        // Filter by target platform if specified
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
        DetectionMethod::UiAutomation
    }

    fn is_available(&self) -> bool {
        true
    }
}
