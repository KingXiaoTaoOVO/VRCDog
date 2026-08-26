use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use midir::{MidiOutput, MidiOutputConnection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BleMidiDevice {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiDevice {
    pub id: String,
    pub name: String,
    pub kind: MidiDeviceKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MidiDeviceKind {
    Usb,
    Ble,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiOutputState {
    pub connected: bool,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub kind: Option<MidiDeviceKind>,
    pub messages_sent: u64,
    pub last_error: Option<String>,
}

impl Default for MidiOutputState {
    fn default() -> Self {
        Self {
            connected: false,
            device_id: None,
            device_name: None,
            kind: None,
            messages_sent: 0,
            last_error: None,
        }
    }
}

pub struct MidiOutputBackend {
    connection: Option<Arc<Mutex<MidiOutputConnection>>>,
    ble_connection: Option<Arc<Mutex<()>>>,
    state: Arc<Mutex<MidiOutputState>>,
    stop_flag: Arc<AtomicBool>,
}

impl MidiOutputBackend {
    pub fn new() -> Self {
        Self {
            connection: None,
            ble_connection: None,
            state: Arc::new(Mutex::new(MidiOutputState::default())),
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn state(&self) -> Arc<Mutex<MidiOutputState>> {
        self.state.clone()
    }

    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        self.stop_flag.clone()
    }

    pub fn list_usb_devices() -> Vec<MidiDevice> {
        let mut devices = Vec::new();
        if let Ok(midi_out) = MidiOutput::new("VRCDog MIDI Output") {
            for (index, port) in midi_out.ports().iter().enumerate() {
                if let Ok(name) = midi_out.port_name(port) {
                    devices.push(MidiDevice {
                        id: format!("usb://{}", index),
                        name,
                        kind: MidiDeviceKind::Usb,
                    });
                }
            }
        }
        devices
    }

    pub fn list_ble_devices() -> Vec<MidiDevice> {
        #[cfg(target_os = "windows")]
        {
            let _ = btleplug::platform::Manager::new();
        }
        Vec::new()
    }

    pub fn connect_usb(&mut self, device_id: &str) -> Result<(), String> {
        self.disconnect();
        let devices = Self::list_usb_devices();
        let device = devices.iter().find(|d| d.id == device_id).ok_or_else(|| {
            format!("MIDI device not found: {device_id}")
        })?;

        let midi_out = MidiOutput::new("VRCDog MIDI Output")
            .map_err(|e| format!("Failed to initialize MIDI output: {e}"))?;

        let index_str = device_id.strip_prefix("usb://").ok_or_else(|| "Invalid device ID".to_string())?;
        let index: usize = index_str.parse().map_err(|_| "Invalid device index".to_string())?;
        let ports = midi_out.ports();
        let port = ports.get(index).ok_or_else(|| "Device port not available".to_string())?;

        let mut connection = midi_out
            .connect(port, "vrpiano-output")
            .map_err(|e| format!("Failed to connect to MIDI device: {e}"))?;

        // Send MIDI active sensing / reset to verify connection
        connection.send(&[0xFE]).ok();

        let conn = Arc::new(Mutex::new(connection));
        self.connection = Some(conn.clone());

        let mut state = self.state.lock().unwrap();
        state.connected = true;
        state.device_id = Some(device_id.to_string());
        state.device_name = Some(device.name.clone());
        state.kind = Some(MidiDeviceKind::Usb);
        state.messages_sent = 0;
        state.last_error = None;

        Ok(())
    }

    pub fn connect_ble(&mut self, device_id: &str) -> Result<(), String> {
        self.disconnect();
        let _ = device_id;
        Err("BLE MIDI output is not yet implemented. Please use USB MIDI.".to_string())
    }

    pub fn disconnect(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        self.connection = None;

        let mut state = self.state.lock().unwrap();
        state.connected = false;
        state.device_id = None;
        state.device_name = None;
        state.kind = None;
        state.last_error = None;

        self.stop_flag.store(false, Ordering::SeqCst);
    }

    pub fn send_note_on(&self, note: u8, velocity: u8, channel: u8) -> Result<(), String> {
        let conn = self.connection.as_ref().ok_or_else(|| "No MIDI connection".to_string())?;
        let mut conn = conn.lock().unwrap();
        let status: u8 = 0x90 | (channel & 0x0F);
        conn.send(&[status, note & 0x7F, velocity & 0x7F])
            .map_err(|e| format!("Failed to send note on: {e}"))?;

        let mut state = self.state.lock().unwrap();
        state.messages_sent += 1;
        Ok(())
    }

    pub fn send_note_off(&self, note: u8, channel: u8) -> Result<(), String> {
        let conn = self.connection.as_ref().ok_or_else(|| "No MIDI connection".to_string())?;
        let mut conn = conn.lock().unwrap();
        let status: u8 = 0x80 | (channel & 0x0F);
        conn.send(&[status, note & 0x7F, 0x00])
            .map_err(|e| format!("Failed to send note off: {e}"))?;

        let mut state = self.state.lock().unwrap();
        state.messages_sent += 1;
        Ok(())
    }

    pub fn send_control_change(&self, channel: u8, cc: u8, value: u8) -> Result<(), String> {
        let conn = self.connection.as_ref().ok_or_else(|| "No MIDI connection".to_string())?;
        let mut conn = conn.lock().unwrap();
        let status: u8 = 0xB0 | (channel & 0x0F);
        conn.send(&[status, cc & 0x7F, value & 0x7F])
            .map_err(|e| format!("Failed to send control change: {e}"))?;

        let mut state = self.state.lock().unwrap();
        state.messages_sent += 1;
        Ok(())
    }

    pub fn send_all_notes_off(&self, channel: u8) -> Result<(), String> {
        self.send_control_change(channel, 123, 0)
    }

    pub fn send_reset_all_controllers(&self, channel: u8) -> Result<(), String> {
        self.send_control_change(channel, 121, 0)
    }

    pub fn send_all_sound_off(&self, channel: u8) -> Result<(), String> {
        self.send_control_change(channel, 120, 0)
    }

    pub fn send_panic(&self) -> Result<(), String> {
        for ch in 0..16u8 {
            let _ = self.send_all_notes_off(ch);
            let _ = self.send_reset_all_controllers(ch);
            let _ = self.send_all_sound_off(ch);
        }
        // Send explicit note offs for common notes
        for note in 0..128u8 {
            let _ = self.send_note_off(note, 0);
        }
        Ok(())
    }

    pub fn send_program_change(&self, channel: u8, program: u8) -> Result<(), String> {
        let conn = self.connection.as_ref().ok_or_else(|| "No MIDI connection".to_string())?;
        let mut conn = conn.lock().unwrap();
        let status: u8 = 0xC0 | (channel & 0x0F);
        conn.send(&[status, program & 0x7F])
            .map_err(|e| format!("Failed to send program change: {e}"))?;
        Ok(())
    }
}

impl Default for MidiOutputBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Precise sleep that can be interrupted by a stop flag.
/// This is inspired by aps-notecast's sleep_unscaled_interruptible.
pub fn sleep_precise_interruptible(duration: Duration, stop: &AtomicBool, paused: &AtomicBool) {
    let start = Instant::now();
    while start.elapsed() < duration {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        while paused.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(10));
            if stop.load(Ordering::SeqCst) {
                return;
            }
        }
        thread::sleep(Duration::from_millis(1));
    }
}

/// Sleep with playback speed scaling.
pub fn sleep_scaled_interruptible(
    base_ms: u64,
    stop: &AtomicBool,
    paused: &AtomicBool,
    speed: f64,
) {
    if speed <= 0.0 {
        return;
    }
    let scaled = Duration::from_millis((base_ms as f64 / speed) as u64);
    sleep_precise_interruptible(scaled, stop, paused);
}
