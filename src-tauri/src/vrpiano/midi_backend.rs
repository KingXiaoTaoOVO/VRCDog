use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};

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

// ---------------------------------------------------------------------------
// Windows 原生 WinMM 实现：直接调用 winmm.dll 底层 API。
// 彻底解决 midir 在 Windows 上因 DRV_QUERYDEVICEINTERFACESIZE 返回 NOTSUPPORTED
// 而把 loopMIDI、teVirtualMIDI 等虚拟 MIDI 端口和软波表静默丢弃的致命缺陷。
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
mod winmm_ffi {
    use std::os::raw::c_uint;

    pub type MMRESULT = c_uint;
    pub type HMIDIOUT = *mut std::ffi::c_void;
    pub const MMSYSERR_NOERROR: MMRESULT = 0;

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct MIDIOUTCAPSW {
        pub w_mid: u16,
        pub w_pid: u16,
        pub v_driver_version: u32,
        pub sz_pname: [u16; 32],
        pub w_technology: u16,
        pub w_voices: u16,
        pub w_notes: u16,
        pub w_channel_mask: u16,
        pub dw_support: u32,
    }

    #[link(name = "winmm")]
    extern "system" {
        pub fn midiOutGetNumDevs() -> c_uint;
        pub fn midiOutGetDevCapsW(
            u_device_id: usize,
            lp_midi_out_caps: *mut MIDIOUTCAPSW,
            cb_midi_out_caps: c_uint,
        ) -> MMRESULT;
        pub fn midiOutOpen(
            lphmo: *mut HMIDIOUT,
            u_device_id: c_uint,
            dw_callback: usize,
            dw_callback_instance: usize,
            dw_flags: c_uint,
        ) -> MMRESULT;
        pub fn midiOutClose(hmo: HMIDIOUT) -> MMRESULT;
        pub fn midiOutShortMsg(hmo: HMIDIOUT, dw_msg: u32) -> MMRESULT;
        pub fn midiOutReset(hmo: HMIDIOUT) -> MMRESULT;
    }
}

#[cfg(target_os = "windows")]
struct WinmmConnection {
    hmo: winmm_ffi::HMIDIOUT,
}

#[cfg(target_os = "windows")]
unsafe impl Send for WinmmConnection {}
#[cfg(target_os = "windows")]
unsafe impl Sync for WinmmConnection {}

#[cfg(target_os = "windows")]
impl Drop for WinmmConnection {
    fn drop(&mut self) {
        if !self.hmo.is_null() {
            unsafe {
                winmm_ffi::midiOutReset(self.hmo);
                winmm_ffi::midiOutClose(self.hmo);
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub struct MidiOutputBackend {
    connection: Option<Arc<Mutex<WinmmConnection>>>,
    state: Arc<Mutex<MidiOutputState>>,
    messages_sent: Arc<AtomicU64>,
}

#[cfg(target_os = "windows")]
impl MidiOutputBackend {
    pub fn new() -> Self {
        Self {
            connection: None,
            state: Arc::new(Mutex::new(MidiOutputState::default())),
            messages_sent: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn state(&self) -> Arc<Mutex<MidiOutputState>> {
        if let Ok(mut state) = self.state.try_lock() {
            state.messages_sent = self.messages_sent.load(Ordering::Relaxed);
        }
        self.state.clone()
    }

    pub fn list_usb_devices() -> Vec<MidiDevice> {
        let mut devices = Vec::new();
        unsafe {
            let count = winmm_ffi::midiOutGetNumDevs();
            for i in 0..count {
                let mut caps = std::mem::zeroed::<winmm_ffi::MIDIOUTCAPSW>();
                let res = winmm_ffi::midiOutGetDevCapsW(
                    i as usize,
                    &mut caps,
                    std::mem::size_of::<winmm_ffi::MIDIOUTCAPSW>() as u32,
                );
                if res == winmm_ffi::MMSYSERR_NOERROR {
                    let len = caps
                        .sz_pname
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(caps.sz_pname.len());
                    let name = String::from_utf16_lossy(&caps.sz_pname[..len]).trim().to_string();
                    if !name.is_empty() {
                        devices.push(MidiDevice {
                            id: format!("usb://{}", i),
                            name,
                            kind: MidiDeviceKind::Usb,
                        });
                    }
                }
            }
        }
        devices
    }

    pub fn connect_usb(&mut self, device_id: &str) -> Result<(), String> {
        self.disconnect();
        let devices = Self::list_usb_devices();
        if devices.is_empty() {
            return Err("未检测到任何系统 MIDI 输出设备。请先启动 loopMIDI 并添加虚拟端口。".to_string());
        }

        // 1. 尝试按 exact ID 匹配（例如 "usb://0"）
        // 2. 尝试按名称匹配（例如用户传入设备名称 "loopMIDI Port"）
        // 3. 容错回退：寻找任何 loopMIDI / virtual / loopback / vrc 虚拟端口
        // 4. 容错回退：取首个可用端口
        let matched_device = devices
            .iter()
            .find(|d| d.id == device_id)
            .cloned()
            .or_else(|| {
                let id_lower = device_id.to_lowercase();
                devices.iter().find(|d| {
                    let name_lower = d.name.to_lowercase();
                    name_lower == id_lower || id_lower.contains(&name_lower) || name_lower.contains(&id_lower)
                }).cloned()
            })
            .or_else(|| {
                devices.iter().find(|d| {
                    let name_lower = d.name.to_lowercase();
                    name_lower.contains("loopmidi")
                        || name_lower.contains("virtual")
                        || name_lower.contains("loopback")
                        || name_lower.contains("vrc")
                }).cloned()
            })
            .or_else(|| devices.first().cloned())
            .ok_or_else(|| format!("指定的 MIDI 设备未找到: {device_id}"))?;

        let index_str = matched_device
            .id
            .strip_prefix("usb://")
            .ok_or_else(|| "Invalid device ID format".to_string())?;
        let index: u32 = index_str
            .parse()
            .map_err(|_| "Invalid device index".to_string())?;

        let mut hmo: winmm_ffi::HMIDIOUT = std::ptr::null_mut();
        let res = unsafe { winmm_ffi::midiOutOpen(&mut hmo, index, 0, 0, 0) };
        if res != winmm_ffi::MMSYSERR_NOERROR || hmo.is_null() {
            return Err(format!(
                "打开 MIDI 设备 {} 失败 (WinMM 错误码: {})",
                matched_device.name, res
            ));
        }

        // 发送 Active Sensing (0xFE) 激活设备
        unsafe {
            winmm_ffi::midiOutShortMsg(hmo, 0xFE);
        }

        let conn = Arc::new(Mutex::new(WinmmConnection { hmo }));
        self.connection = Some(conn);

        let mut state = self.state.lock().unwrap();
        state.connected = true;
        state.device_id = Some(matched_device.id.clone());
        state.device_name = Some(matched_device.name.clone());
        state.kind = Some(MidiDeviceKind::Usb);
        state.messages_sent = 0;
        state.last_error = None;
        self.messages_sent.store(0, Ordering::Relaxed);

        Ok(())
    }

    pub fn disconnect(&mut self) {
        if let Some(conn) = self.connection.take() {
            if let Ok(c) = conn.lock() {
                if !c.hmo.is_null() {
                    unsafe {
                        winmm_ffi::midiOutReset(c.hmo);
                    }
                }
            }
        }

        let mut state = self.state.lock().unwrap();
        state.connected = false;
        state.device_id = None;
        state.device_name = None;
        state.kind = None;
        state.last_error = None;
        self.messages_sent.store(0, Ordering::Relaxed);
    }

    pub fn send_note_on(&self, note: u8, velocity: u8, channel: u8) -> Result<(), String> {
        let conn = self.connection.as_ref().ok_or_else(|| "No MIDI connection".to_string())?;
        let conn = conn.lock().unwrap();
        let status = (0x90 | (channel & 0x0F)) as u32;
        let msg = status | (((note & 0x7F) as u32) << 8) | (((velocity & 0x7F) as u32) << 16);
        let res = unsafe { winmm_ffi::midiOutShortMsg(conn.hmo, msg) };
        if res != winmm_ffi::MMSYSERR_NOERROR {
            return Err(format!("WinMM midiOutShortMsg failed: {res}"));
        }
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn send_note_off(&self, note: u8, channel: u8) -> Result<(), String> {
        let conn = self.connection.as_ref().ok_or_else(|| "No MIDI connection".to_string())?;
        let conn = conn.lock().unwrap();
        let status = (0x80 | (channel & 0x0F)) as u32;
        let msg = status | (((note & 0x7F) as u32) << 8);
        let res = unsafe { winmm_ffi::midiOutShortMsg(conn.hmo, msg) };
        if res != winmm_ffi::MMSYSERR_NOERROR {
            return Err(format!("WinMM midiOutShortMsg failed: {res}"));
        }
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn send_control_change(&self, channel: u8, cc: u8, value: u8) -> Result<(), String> {
        let conn = self.connection.as_ref().ok_or_else(|| "No MIDI connection".to_string())?;
        let conn = conn.lock().unwrap();
        let status = (0xB0 | (channel & 0x0F)) as u32;
        let msg = status | (((cc & 0x7F) as u32) << 8) | (((value & 0x7F) as u32) << 16);
        let res = unsafe { winmm_ffi::midiOutShortMsg(conn.hmo, msg) };
        if res != winmm_ffi::MMSYSERR_NOERROR {
            return Err(format!("WinMM midiOutShortMsg failed: {res}"));
        }
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn send_all_notes_off(&self, channel: u8) -> Result<(), String> {
        self.send_control_change(channel, 123, 0)
    }

    #[allow(dead_code)]
    pub fn send_reset_all_controllers(&self, channel: u8) -> Result<(), String> {
        self.send_control_change(channel, 121, 0)
    }

    #[allow(dead_code)]
    pub fn send_all_sound_off(&self, channel: u8) -> Result<(), String> {
        self.send_control_change(channel, 120, 0)
    }

    #[allow(dead_code)]
    pub fn send_panic(&self) {
        if let Some(conn) = self.connection.as_ref() {
            if let Ok(c) = conn.lock() {
                if !c.hmo.is_null() {
                    unsafe {
                        winmm_ffi::midiOutReset(c.hmo);
                    }
                }
            }
        }
        for channel in 0..16u8 {
            let _ = self.send_control_change(channel, 64, 0);
            let _ = self.send_control_change(channel, 66, 0);
            let _ = self.send_control_change(channel, 67, 0);
            let _ = self.send_all_notes_off(channel);
            let _ = self.send_reset_all_controllers(channel);
            let _ = self.send_all_sound_off(channel);
        }
    }
}

// ---------------------------------------------------------------------------
// 非 Windows 平台（macOS / Linux）保持使用 midir 抽象
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
pub struct MidiOutputBackend {
    connection: Option<Arc<Mutex<midir::MidiOutputConnection>>>,
    state: Arc<Mutex<MidiOutputState>>,
    messages_sent: Arc<AtomicU64>,
}

#[cfg(not(target_os = "windows"))]
impl MidiOutputBackend {
    pub fn new() -> Self {
        Self {
            connection: None,
            state: Arc::new(Mutex::new(MidiOutputState::default())),
            messages_sent: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn state(&self) -> Arc<Mutex<MidiOutputState>> {
        if let Ok(mut state) = self.state.try_lock() {
            state.messages_sent = self.messages_sent.load(Ordering::Relaxed);
        }
        self.state.clone()
    }

    pub fn list_usb_devices() -> Vec<MidiDevice> {
        let mut devices = Vec::new();
        if let Ok(midi_out) = midir::MidiOutput::new("VRCDog MIDI Output") {
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

    pub fn connect_usb(&mut self, device_id: &str) -> Result<(), String> {
        self.disconnect();
        let devices = Self::list_usb_devices();
        let device = devices.iter().find(|d| d.id == device_id).ok_or_else(|| {
            format!("MIDI device not found: {device_id}")
        })?;

        let midi_out = midir::MidiOutput::new("VRCDog MIDI Output")
            .map_err(|e| format!("Failed to initialize MIDI output: {e}"))?;

        let index_str = device_id.strip_prefix("usb://").ok_or_else(|| "Invalid device ID".to_string())?;
        let index: usize = index_str.parse().map_err(|_| "Invalid device index".to_string())?;
        let ports = midi_out.ports();
        let port = ports.get(index).ok_or_else(|| "Device port not available".to_string())?;

        let mut connection = midi_out
            .connect(port, "vrpiano-output")
            .map_err(|e| format!("Failed to connect to MIDI device: {e}"))?;

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
        self.messages_sent.store(0, Ordering::Relaxed);

        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.connection = None;

        let mut state = self.state.lock().unwrap();
        state.connected = false;
        state.device_id = None;
        state.device_name = None;
        state.kind = None;
        state.last_error = None;
        self.messages_sent.store(0, Ordering::Relaxed);
    }

    pub fn send_note_on(&self, note: u8, velocity: u8, channel: u8) -> Result<(), String> {
        let conn = self.connection.as_ref().ok_or_else(|| "No MIDI connection".to_string())?;
        let mut conn = conn.lock().unwrap();
        let status: u8 = 0x90 | (channel & 0x0F);
        conn.send(&[status, note & 0x7F, velocity & 0x7F])
            .map_err(|e| format!("Failed to send note on: {e}"))?;

        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn send_note_off(&self, note: u8, channel: u8) -> Result<(), String> {
        let conn = self.connection.as_ref().ok_or_else(|| "No MIDI connection".to_string())?;
        let mut conn = conn.lock().unwrap();
        let status: u8 = 0x80 | (channel & 0x0F);
        conn.send(&[status, note & 0x7F, 0x00])
            .map_err(|e| format!("Failed to send note off: {e}"))?;

        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn send_control_change(&self, channel: u8, cc: u8, value: u8) -> Result<(), String> {
        let conn = self.connection.as_ref().ok_or_else(|| "No MIDI connection".to_string())?;
        let mut conn = conn.lock().unwrap();
        let status: u8 = 0xB0 | (channel & 0x0F);
        conn.send(&[status, cc & 0x7F, value & 0x7F])
            .map_err(|e| format!("Failed to send control change: {e}"))?;

        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn send_all_notes_off(&self, channel: u8) -> Result<(), String> {
        self.send_control_change(channel, 123, 0)
    }

    #[allow(dead_code)]
    pub fn send_reset_all_controllers(&self, channel: u8) -> Result<(), String> {
        self.send_control_change(channel, 121, 0)
    }

    #[allow(dead_code)]
    pub fn send_all_sound_off(&self, channel: u8) -> Result<(), String> {
        self.send_control_change(channel, 120, 0)
    }

    #[allow(dead_code)]
    pub fn send_panic(&self) {
        for channel in 0..16u8 {
            let _ = self.send_control_change(channel, 64, 0);
            let _ = self.send_control_change(channel, 66, 0);
            let _ = self.send_control_change(channel, 67, 0);
            let _ = self.send_all_notes_off(channel);
            let _ = self.send_reset_all_controllers(channel);
            let _ = self.send_all_sound_off(channel);
        }
    }
}

impl Default for MidiOutputBackend {
    fn default() -> Self {
        Self::new()
    }
}
