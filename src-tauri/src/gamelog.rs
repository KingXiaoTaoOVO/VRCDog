use std::fs;
use std::path::PathBuf;
use std::io::{BufReader, BufRead, Seek, SeekFrom};
use std::sync::Mutex;
use serde::Serialize;
use tauri::State;
use crate::AppResult;

pub struct LogReaderState {
    pub current_file: Mutex<Option<PathBuf>>,
    pub last_pos: Mutex<u64>,
}

impl Default for LogReaderState {
    fn default() -> Self {
        Self::new()
    }
}

impl LogReaderState {
    pub fn new() -> Self {
        Self {
            current_file: Mutex::new(None),
            last_pos: Mutex::new(0),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct GameLogEvent {
    pub time: String,
    pub event_type: String,
    pub content: String,
}

fn get_vrchat_log_dir() -> Option<PathBuf> {
    if let Some(mut path) = dirs::data_local_dir() {
        path.pop(); // Go up from Local to AppData
        path.push("LocalLow");
        path.push("VRChat");
        path.push("vrchat");
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn get_latest_log_file() -> Option<PathBuf> {
    let dir = get_vrchat_log_dir()?;
    let mut latest_file = None;
    let mut latest_time = std::time::UNIX_EPOCH;

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("output_log") && name.ends_with(".txt") {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                if modified > latest_time {
                                    latest_time = modified;
                                    latest_file = Some(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    latest_file
}

#[tauri::command]
pub async fn vrc_get_latest_gamelogs(
    state: State<'_, LogReaderState>,
    max_lines: Option<usize>
) -> AppResult<Vec<GameLogEvent>> {
    let file_path = match get_latest_log_file() {
        Some(p) => p,
        None => return Ok(vec![]),
    };

    let mut current_file_guard = state.current_file.lock().unwrap();
    let mut last_pos_guard = state.last_pos.lock().unwrap();

    if current_file_guard.as_ref() != Some(&file_path) {
        *current_file_guard = Some(file_path.clone());
        *last_pos_guard = 0;
    }

    let mut file = fs::File::open(&file_path).map_err(|e| e.to_string())?;
    
    // Check if file shrank (e.g. truncated)
    if let Ok(metadata) = file.metadata() {
        if metadata.len() < *last_pos_guard {
            *last_pos_guard = 0;
        }
    }

    file.seek(SeekFrom::Start(*last_pos_guard)).map_err(|e| e.to_string())?;
    
    let reader = BufReader::new(file);
    let limit = max_lines.unwrap_or(2000);
    let mut all_lines: std::collections::VecDeque<String> = std::collections::VecDeque::with_capacity(limit);

    for line in reader.lines().map_while(Result::ok) {
        if all_lines.len() == limit {
            all_lines.pop_front();
        }
        all_lines.push_back(line);
    }

    let mut events = Vec::new();
    
    for line in all_lines {
        if line.len() < 20 { continue; }
        let time_str = if line.is_char_boundary(19) {
            &line[0..19]
        } else {
            ""
        };
        
        if let Some(idx) = line.find("OnPlayerJoined ") {
            let name = line[idx + 15..].trim();
            events.push(GameLogEvent {
                time: time_str.to_string(),
                event_type: "Player Joined".to_string(),
                content: name.to_string(),
            });
        } else if let Some(idx) = line.find("[Player Joined]") {
            let name = line[idx + 15..].trim();
            events.push(GameLogEvent {
                time: time_str.to_string(),
                event_type: "Player Joined".to_string(),
                content: name.to_string(),
            });
        } else if let Some(idx) = line.find("OnPlayerLeft ") {
            let name = line[idx + 13..].trim();
            events.push(GameLogEvent {
                time: time_str.to_string(),
                event_type: "Player Left".to_string(),
                content: name.to_string(),
            });
        } else if let Some(idx) = line.find("[Player Left]") {
            let name = line[idx + 13..].trim();
            events.push(GameLogEvent {
                time: time_str.to_string(),
                event_type: "Player Left".to_string(),
                content: name.to_string(),
            });
        } else if let Some(idx) = line.find("Entering Room: ") {
            let name = line[idx + 15..].trim();
            events.push(GameLogEvent {
                time: time_str.to_string(),
                event_type: "Instance Joined".to_string(),
                content: name.to_string(),
            });
        } else if let Some(idx) = line.find("User Authenticated: ") {
            let name = line[idx + 20..].trim();
            events.push(GameLogEvent {
                time: time_str.to_string(),
                event_type: "Authenticated".to_string(),
                content: name.to_string(),
            });
        } else if let Some(idx) = line.find("[Video Playback] Resolving URL: ") {
            let name = line[idx + 32..].trim();
            events.push(GameLogEvent {
                time: time_str.to_string(),
                event_type: "Video Playback".to_string(),
                content: name.to_string(),
            });
        }
    }
    
    if let Ok(metadata) = fs::metadata(&file_path) {
        *last_pos_guard = metadata.len();
    }

    // Reverse so newest is first
    events.reverse();
    Ok(events)
}
