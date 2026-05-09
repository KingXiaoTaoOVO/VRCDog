use std::sync::Mutex;
use serde::Serialize;
use tauri::State;
use crate::AppResult;

// VRCT Audio Capture & STT Skeleton
pub struct AudioCaptureState {
    pub is_recording: Mutex<bool>,
    pub selected_device: Mutex<String>,
}

impl Default for AudioCaptureState {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioCaptureState {
    pub fn new() -> Self {
        Self {
            is_recording: Mutex::new(false),
            selected_device: Mutex::new("Default".to_string()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
}

#[tauri::command]
pub async fn vrct_get_audio_devices() -> AppResult<Vec<AudioDevice>> {
    // Stub: Enumerate WASAPI devices (simulate for now)
    Ok(vec![
        AudioDevice { id: "default".into(), name: "Default Microphone".into() },
        AudioDevice { id: "vr_headset".into(), name: "HMD Microphone (Index/Quest)".into() },
    ])
}

#[tauri::command]
pub async fn vrct_start_stt_recording(state: State<'_, AudioCaptureState>) -> AppResult<()> {
    let mut rec = state.is_recording.lock().unwrap();
    *rec = true;
    println!("[VRCT] Microphone recording started. Audio routing to STT.");
    Ok(())
}

#[tauri::command]
pub async fn vrct_stop_stt_recording(state: State<'_, AudioCaptureState>) -> AppResult<()> {
    let mut rec = state.is_recording.lock().unwrap();
    *rec = false;
    println!("[VRCT] Microphone recording stopped.");
    Ok(())
}
