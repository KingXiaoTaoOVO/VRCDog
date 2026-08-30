    use crate::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{mpsc, Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConflict {
    pub hotkey: String,
    pub reason: String,
}

fn normalize(value: &str) -> String { value.trim().to_ascii_lowercase().replace(' ', "") }

#[derive(Debug, Clone, Deserialize)]
pub struct TranslationHotkeyConfig { pub id: u32, pub hotkey: String }

#[cfg(windows)]
static HOTKEY_STOP: OnceLock<Mutex<Option<mpsc::Sender<()>>>> = OnceLock::new();

fn parse_hotkey(value: &str) -> Option<(u32, u32)> {
    let mut modifiers = 0u32;
    let mut key = None;
    for part in normalize(value).split('+') {
        match part {
            "ctrl" | "control" => modifiers |= 0x0002,
            "alt" => modifiers |= 0x0001,
            "shift" => modifiers |= 0x0004,
            "win" | "windows" => modifiers |= 0x0008,
            value if value.starts_with('f') && value[1..].parse::<u32>().is_ok() => key = value[1..].parse::<u32>().ok().filter(|number| (1..=12).contains(number)).map(|number| 0x70 + number - 1),
            single if single.len() == 1 => key = single.as_bytes().first().copied().map(|byte| byte.to_ascii_uppercase() as u32),
            _ => {}
        }
    }
    key.map(|virtual_key| (modifiers, virtual_key))
}

#[tauri::command]
pub fn translation_check_hotkeys(hotkeys: Vec<String>) -> AppResult<Vec<HotkeyConflict>> {
    let mut seen = HashSet::new();
    let mut conflicts = Vec::new();
    for raw in hotkeys {
        let hotkey = normalize(&raw);
        if hotkey.is_empty() { continue; }
        if !seen.insert(hotkey.clone()) { conflicts.push(HotkeyConflict { hotkey, reason: "与另一个翻译动作重复".into() }); continue; }
        if ["ctrl+c", "ctrl+v", "ctrl+x", "alt+f4", "win+l"].contains(&hotkey.as_str()) {
            conflicts.push(HotkeyConflict { hotkey, reason: "占用 Windows 或常用编辑快捷键".into() });
        }
    }
    Ok(conflicts)
}

#[tauri::command]
pub fn translation_apply_hotkeys(app: AppHandle, configs: Vec<TranslationHotkeyConfig>) -> AppResult<Vec<HotkeyConflict>> {
    let conflicts = translation_check_hotkeys(configs.iter().map(|item| item.hotkey.clone()).collect())?;
    if !conflicts.is_empty() { return Ok(conflicts); }
    #[cfg(windows)] {
        let stop_slot = HOTKEY_STOP.get_or_init(|| Mutex::new(None));
        if let Ok(mut slot) = stop_slot.lock() {
            if let Some(sender) = slot.take() { let _ = sender.send(()); }
            let (stop_tx, stop_rx) = mpsc::channel::<()>();
            let parsed = configs.into_iter().filter_map(|item| parse_hotkey(&item.hotkey).map(|(modifiers, key)| (item.id, modifiers, key))).collect::<Vec<_>>();
            if parsed.is_empty() { return Ok(Vec::new()); }
            let thread_app = app.clone();
            std::thread::Builder::new().name("vrcdog-translation-hotkeys".into()).spawn(move || {
                use windows::Win32::Foundation::HWND;
                use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS};
                use windows::Win32::UI::WindowsAndMessaging::{PeekMessageW, MSG, PM_REMOVE, WM_HOTKEY};
                let mut registered = Vec::new();
                for (id, modifiers, key) in &parsed {
                    if unsafe { RegisterHotKey(HWND::default(), *id as i32, HOT_KEY_MODIFIERS(*modifiers), *key) }.is_ok() { registered.push(*id); }
                }
                let mut message = MSG::default();
                loop {
                    if stop_rx.try_recv().is_ok() { break; }
                    while unsafe { PeekMessageW(&mut message, HWND::default(), 0, 0, PM_REMOVE) }.as_bool() {
                        if message.message == WM_HOTKEY { let _ = thread_app.emit("translation-hotkey", serde_json::json!({ "id": message.wParam.0 })); }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                for id in registered { unsafe { let _ = UnregisterHotKey(HWND::default(), id as i32); } }
            }).map_err(|error| AppError::from(format!("无法启动翻译快捷键: {error}")))?;
            *slot = Some(stop_tx);
        }
    }
    #[cfg(not(windows))] let _ = app;
    Ok(Vec::new())
}
