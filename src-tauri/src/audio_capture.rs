use crate::AppResult;
use serde::Serialize;
use std::sync::Mutex;
use tauri::State;

// VrcDog translation audio capture state used by the Tauri commands and Python WASAPI bridge.
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
    Ok(vec![
        AudioDevice {
            id: "default".into(),
            name: "Default Windows audio input".into(),
        },
        AudioDevice {
            id: "wasapi_loopback".into(),
            name: "Default Windows playback loopback".into(),
        },
    ])
}

#[tauri::command]
pub async fn vrct_start_stt_recording(state: State<'_, AudioCaptureState>) -> AppResult<()> {
    let mut rec = state.is_recording.lock().unwrap();
    *rec = true;
    println!("[VrcDog] Microphone recording started. Audio routing to STT.");
    Ok(())
}

#[tauri::command]
pub async fn vrct_stop_stt_recording(state: State<'_, AudioCaptureState>) -> AppResult<()> {
    let mut rec = state.is_recording.lock().unwrap();
    *rec = false;
    println!("[VrcDog] Microphone recording stopped.");
    Ok(())
}
