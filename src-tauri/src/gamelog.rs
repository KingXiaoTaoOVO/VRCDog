use crate::AppResult;
use chrono::{Local, NaiveDateTime};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrackedPlayer {
    display_name: String,
    user_id: Option<String>,
}

pub struct LogReaderState {
    current_file: Mutex<Option<PathBuf>>,
    last_pos: Mutex<u64>,
    players: Mutex<HashMap<String, TrackedPlayer>>,
    last_timestamp: Mutex<Option<String>>,
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
            players: Mutex::new(HashMap::new()),
            last_timestamp: Mutex::new(None),
        }
    }
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct GameLogEvent {
    pub time: String,
    pub event_type: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedEvent {
    PlayerJoined(TrackedPlayer),
    PlayerLeft(TrackedPlayer),
    InstanceJoined(String),
    Authenticated(String),
    VideoPlayback(String),
    ApplicationQuit,
}

fn get_vrchat_log_dir() -> Option<PathBuf> {
    if let Some(mut path) = dirs::data_local_dir() {
        path.pop();
        path.push("LocalLow");
        path.push("VRChat");
        path.push("VRChat");
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
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !name.starts_with("output_log_") || !name.ends_with(".txt") {
                continue;
            }
            if let Ok(modified) = entry.metadata().and_then(|metadata| metadata.modified()) {
                if modified > latest_time {
                    latest_time = modified;
                    latest_file = Some(path);
                }
            }
        }
    }
    latest_file
}

fn parse_timestamp(line: &str) -> Option<String> {
    let timestamp = line.get(..19)?;
    NaiveDateTime::parse_from_str(timestamp, "%Y.%m.%d %H:%M:%S")
        .ok()
        .map(|_| timestamp.to_string())
}

fn sanitize_log_value(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>()
        .trim()
        .to_string()
}

fn valid_user_id(value: &str) -> bool {
    value.starts_with("usr_")
        && value.len() <= 64
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
}

fn parse_user_info(raw: &str) -> Option<TrackedPlayer> {
    let raw = sanitize_log_value(raw);
    if raw.is_empty() || raw.len() > 512 {
        return None;
    }

    let mut display_name = raw.as_str();
    let mut user_id = None;
    if raw.ends_with(')') {
        if let Some(id_start) = raw.rfind(" (usr_") {
            let candidate = &raw[id_start + 2..raw.len() - 1];
            if valid_user_id(candidate) {
                display_name = raw[..id_start].trim();
                user_id = Some(candidate.to_string());
            }
        }
    }

    let display_name = sanitize_log_value(display_name);
    if display_name.is_empty() && user_id.is_none() {
        return None;
    }
    Some(TrackedPlayer {
        display_name,
        user_id,
    })
}

fn parse_log_line(line: &str) -> Option<(String, ParsedEvent)> {
    let timestamp = parse_timestamp(line)?;
    let payload = line.split_once(" -  ")?.1.trim_start();

    for marker in [
        "[Behaviour] OnPlayerJoined ",
        "[NetworkManager] OnPlayerJoined ",
        "[Player Joined] ",
    ] {
        if let Some(raw_player) = payload.strip_prefix(marker) {
            let player = parse_user_info(raw_player)?;
            return Some((timestamp, ParsedEvent::PlayerJoined(player)));
        }
    }

    for marker in [
        "[Behaviour] OnPlayerLeft ",
        "[NetworkManager] OnPlayerLeft ",
        "[Player Left] ",
    ] {
        if let Some(raw_player) = payload.strip_prefix(marker) {
            let player = parse_user_info(raw_player)?;
            return Some((timestamp, ParsedEvent::PlayerLeft(player)));
        }
    }

    if let Some(offset) = payload.rfind("Entering Room: ") {
        let room = sanitize_log_value(&payload[offset + "Entering Room: ".len()..]);
        if !room.is_empty() {
            return Some((timestamp, ParsedEvent::InstanceJoined(room)));
        }
    }
    if let Some(offset) = payload.rfind("User Authenticated: ") {
        let user = sanitize_log_value(&payload[offset + "User Authenticated: ".len()..]);
        if !user.is_empty() {
            return Some((timestamp, ParsedEvent::Authenticated(user)));
        }
    }
    if let Some(offset) = payload.rfind("[Video Playback] Resolving URL: ") {
        let url = sanitize_log_value(&payload[offset + "[Video Playback] Resolving URL: ".len()..]);
        if !url.is_empty() {
            return Some((timestamp, ParsedEvent::VideoPlayback(url)));
        }
    }
    if payload.starts_with("VRCApplication: OnApplicationQuit at ")
        || payload.starts_with("VRCApplication: HandleApplicationQuit at ")
    {
        return Some((timestamp, ParsedEvent::ApplicationQuit));
    }

    None
}

fn player_key(player: &TrackedPlayer) -> String {
    player
        .user_id
        .clone()
        .unwrap_or_else(|| format!("name:{}", player.display_name.to_lowercase()))
}

fn player_content(player: &TrackedPlayer) -> String {
    match (&player.display_name, &player.user_id) {
        (name, Some(user_id)) if !name.is_empty() => format!("{name} ({user_id})"),
        (_, Some(user_id)) => user_id.clone(),
        (name, None) => name.clone(),
    }
}

fn player_event(timestamp: &str, event_type: &str, player: &TrackedPlayer) -> GameLogEvent {
    GameLogEvent {
        time: timestamp.to_string(),
        event_type: event_type.to_string(),
        content: player_content(player),
        display_name: (!player.display_name.is_empty()).then(|| player.display_name.clone()),
        user_id: player.user_id.clone(),
    }
}

fn plain_event(timestamp: &str, event_type: &str, content: String) -> GameLogEvent {
    GameLogEvent {
        time: timestamp.to_string(),
        event_type: event_type.to_string(),
        content,
        display_name: None,
        user_id: None,
    }
}

fn push_limited(events: &mut VecDeque<GameLogEvent>, event: GameLogEvent, limit: usize) {
    if limit == 0 {
        return;
    }
    if events.len() == limit {
        events.pop_front();
    }
    events.push_back(event);
}

fn drain_players(
    players: &mut HashMap<String, TrackedPlayer>,
    timestamp: &str,
    events: &mut VecDeque<GameLogEvent>,
    limit: usize,
) {
    let mut remaining = players
        .drain()
        .map(|(_, player)| player)
        .collect::<Vec<_>>();
    remaining.sort_by(|left, right| left.display_name.cmp(&right.display_name));
    for player in remaining {
        push_limited(
            events,
            player_event(timestamp, "Player Left", &player),
            limit,
        );
    }
}

fn remove_tracked_player(
    players: &mut HashMap<String, TrackedPlayer>,
    player: &TrackedPlayer,
) -> Option<TrackedPlayer> {
    let key = player_key(player);
    if let Some(tracked) = players.remove(&key) {
        return Some(tracked);
    }

    let normalized_name = player.display_name.to_lowercase();
    let matching_keys = players
        .iter()
        .filter(|(_, tracked)| {
            tracked.display_name.to_lowercase() == normalized_name
                && (player.user_id.is_none() || tracked.user_id.is_none())
        })
        .map(|(key, _)| key.clone())
        .take(2)
        .collect::<Vec<_>>();
    if matching_keys.len() == 1 {
        players.remove(&matching_keys[0])
    } else {
        None
    }
}

fn apply_parsed_event(
    timestamp: &str,
    parsed: ParsedEvent,
    players: &mut HashMap<String, TrackedPlayer>,
    events: &mut VecDeque<GameLogEvent>,
    limit: usize,
    reset_output_on_instance: bool,
) {
    match parsed {
        ParsedEvent::PlayerJoined(player) => {
            let key = player_key(&player);
            if players.insert(key, player.clone()).is_some() {
                return;
            }
            push_limited(
                events,
                player_event(timestamp, "Player Joined", &player),
                limit,
            );
        }
        ParsedEvent::PlayerLeft(player) => {
            // Ignore unmatched leave events to prevent duplicate or spoofed notifications.
            if let Some(tracked) = remove_tracked_player(players, &player) {
                let resolved = if player.display_name.is_empty() {
                    tracked
                } else {
                    player
                };
                push_limited(
                    events,
                    player_event(timestamp, "Player Left", &resolved),
                    limit,
                );
            }
        }
        ParsedEvent::InstanceJoined(room) => {
            drain_players(players, timestamp, events, limit);
            if reset_output_on_instance {
                events.clear();
            }
            push_limited(
                events,
                plain_event(timestamp, "Instance Joined", room),
                limit,
            );
        }
        ParsedEvent::Authenticated(user) => {
            push_limited(events, plain_event(timestamp, "Authenticated", user), limit);
        }
        ParsedEvent::VideoPlayback(url) => {
            push_limited(events, plain_event(timestamp, "Video Playback", url), limit);
        }
        ParsedEvent::ApplicationQuit => {
            drain_players(players, timestamp, events, limit);
            push_limited(
                events,
                plain_event(timestamp, "Application Quit", "VRChat".to_string()),
                limit,
            );
        }
    }
}

fn process_complete_lines(
    path: &Path,
    start_pos: u64,
    players: &mut HashMap<String, TrackedPlayer>,
    events: &mut VecDeque<GameLogEvent>,
    limit: usize,
    last_timestamp: &mut Option<String>,
    reset_output_on_instance: bool,
) -> Result<u64, String> {
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    file.seek(SeekFrom::Start(start_pos))
        .map_err(|error| error.to_string())?;
    let mut reader = BufReader::new(file);
    let mut committed_pos = start_pos;

    loop {
        let mut bytes = Vec::new();
        let read = reader
            .read_until(b'\n', &mut bytes)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        if bytes.last() != Some(&b'\n') {
            break;
        }
        committed_pos += read as u64;
        while matches!(bytes.last(), Some(b'\n' | b'\r')) {
            bytes.pop();
        }
        let line = String::from_utf8_lossy(&bytes);
        if let Some((timestamp, parsed)) = parse_log_line(&line) {
            *last_timestamp = Some(timestamp.clone());
            apply_parsed_event(
                &timestamp,
                parsed,
                players,
                events,
                limit,
                reset_output_on_instance,
            );
        }
    }

    Ok(committed_pos)
}

fn newest_first(events: VecDeque<GameLogEvent>) -> Vec<GameLogEvent> {
    events.into_iter().rev().collect()
}

#[tauri::command]
pub async fn vrc_get_latest_gamelogs(
    state: State<'_, LogReaderState>,
    max_lines: Option<usize>,
    finalize_session: Option<bool>,
) -> AppResult<Vec<GameLogEvent>> {
    let Some(file_path) = get_latest_log_file() else {
        return Ok(vec![]);
    };
    let limit = max_lines.unwrap_or(100_000).clamp(1, 250_000);
    let mut current_file = state
        .current_file
        .lock()
        .map_err(|error| error.to_string())?;
    let mut last_pos = state.last_pos.lock().map_err(|error| error.to_string())?;
    let mut players = state.players.lock().map_err(|error| error.to_string())?;
    let mut last_timestamp = state
        .last_timestamp
        .lock()
        .map_err(|error| error.to_string())?;
    let mut events = VecDeque::with_capacity(limit.min(4096));

    if current_file.as_ref() != Some(&file_path) {
        if let Some(previous_file) = current_file.as_ref() {
            if previous_file.exists() {
                if let Ok(position) = process_complete_lines(
                    previous_file,
                    *last_pos,
                    &mut players,
                    &mut events,
                    limit,
                    &mut last_timestamp,
                    false,
                ) {
                    *last_pos = position;
                }
            }
            let finish_time = last_timestamp
                .clone()
                .unwrap_or_else(|| Local::now().format("%Y.%m.%d %H:%M:%S").to_string());
            drain_players(&mut players, &finish_time, &mut events, limit);
        }
        *current_file = Some(file_path.clone());
        *last_pos = 0;
        players.clear();
        *last_timestamp = None;
    }

    let file_len = fs::metadata(&file_path)
        .map_err(|error| error.to_string())?
        .len();
    if file_len < *last_pos {
        *last_pos = 0;
        players.clear();
        *last_timestamp = None;
    }

    *last_pos = process_complete_lines(
        &file_path,
        *last_pos,
        &mut players,
        &mut events,
        limit,
        &mut last_timestamp,
        false,
    )?;

    if finalize_session.unwrap_or(false) && !players.is_empty() {
        let finish_time = last_timestamp
            .clone()
            .unwrap_or_else(|| Local::now().format("%Y.%m.%d %H:%M:%S").to_string());
        drain_players(&mut players, &finish_time, &mut events, limit);
    }

    Ok(newest_first(events))
}

#[tauri::command]
pub async fn vrc_get_gamelog_snapshot(max_lines: Option<usize>) -> AppResult<Vec<GameLogEvent>> {
    let Some(file_path) = get_latest_log_file() else {
        return Ok(vec![]);
    };
    let limit = max_lines.unwrap_or(20_000).clamp(1, 250_000);
    let mut players = HashMap::new();
    let mut events = VecDeque::with_capacity(limit.min(4096));
    let mut last_timestamp = None;
    process_complete_lines(
        &file_path,
        0,
        &mut players,
        &mut events,
        limit,
        &mut last_timestamp,
        true,
    )?;
    Ok(newest_first(events))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parses_player_identity_from_current_vrchat_format() {
        let line = "2026.08.15 01:03:07 Debug      -  [Behaviour] OnPlayerLeft 火锅涮冰块 ffbc (usr_b9cd9d50-cac5-43c2-91f0-7637426effbc)";
        let parsed = parse_log_line(line);
        assert_eq!(
            parsed,
            Some((
                "2026.08.15 01:03:07".to_string(),
                ParsedEvent::PlayerLeft(TrackedPlayer {
                    display_name: "火锅涮冰块 ffbc".to_string(),
                    user_id: Some("usr_b9cd9d50-cac5-43c2-91f0-7637426effbc".to_string()),
                })
            ))
        );
    }

    #[test]
    fn ignores_internal_player_left_room_and_debug_lines() {
        assert!(
            parse_log_line("2026.08.15 01:03:07 Debug      -  [Behaviour] OnPlayerLeftRoom")
                .is_none()
        );
        assert!(parse_log_line(
            "2026.08.15 01:03:07 Debug      -  [Behaviour] OnPlayerLeft:Unnamed"
        )
        .is_none());
        assert!(parse_log_line(
            "2026.08.15 01:03:07 Debug      -  Downloaded text: [Behaviour] OnPlayerLeft Spoofed (usr_spoofed)"
        )
        .is_none());
    }

    #[test]
    fn quit_synthesizes_only_players_without_a_real_leave() {
        let mut players = HashMap::new();
        let mut events = VecDeque::new();
        let alice = TrackedPlayer {
            display_name: "Alice".to_string(),
            user_id: Some("usr_alice".to_string()),
        };
        let bob = TrackedPlayer {
            display_name: "Bob".to_string(),
            user_id: Some("usr_bob".to_string()),
        };
        apply_parsed_event(
            "2026.08.15 01:00:00",
            ParsedEvent::PlayerJoined(alice.clone()),
            &mut players,
            &mut events,
            20,
            false,
        );
        apply_parsed_event(
            "2026.08.15 01:00:01",
            ParsedEvent::PlayerJoined(bob.clone()),
            &mut players,
            &mut events,
            20,
            false,
        );
        apply_parsed_event(
            "2026.08.15 01:01:00",
            ParsedEvent::PlayerLeft(alice),
            &mut players,
            &mut events,
            20,
            false,
        );
        apply_parsed_event(
            "2026.08.15 01:02:00",
            ParsedEvent::ApplicationQuit,
            &mut players,
            &mut events,
            20,
            false,
        );

        let left = events
            .iter()
            .filter(|event| event.event_type == "Player Left")
            .map(|event| event.content.clone())
            .collect::<Vec<_>>();
        assert_eq!(left, vec!["Alice (usr_alice)", "Bob (usr_bob)"]);
        assert!(players.is_empty());
    }

    #[test]
    fn duplicate_player_left_is_ignored() {
        let mut players = HashMap::new();
        let mut events = VecDeque::new();
        let player = TrackedPlayer {
            display_name: "Alice".to_string(),
            user_id: Some("usr_alice".to_string()),
        };
        apply_parsed_event(
            "2026.08.15 01:00:00",
            ParsedEvent::PlayerJoined(player.clone()),
            &mut players,
            &mut events,
            20,
            false,
        );
        apply_parsed_event(
            "2026.08.15 01:00:01",
            ParsedEvent::PlayerLeft(player.clone()),
            &mut players,
            &mut events,
            20,
            false,
        );
        apply_parsed_event(
            "2026.08.15 01:00:02",
            ParsedEvent::PlayerLeft(player),
            &mut players,
            &mut events,
            20,
            false,
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| event.event_type == "Player Left")
                .count(),
            1
        );
    }

    #[test]
    fn duplicate_join_and_ambiguous_legacy_leave_are_ignored() {
        let mut players = HashMap::new();
        let mut events = VecDeque::new();
        for user_id in ["usr_alice_1", "usr_alice_2"] {
            let player = TrackedPlayer {
                display_name: "Alice".to_string(),
                user_id: Some(user_id.to_string()),
            };
            apply_parsed_event(
                "2026.08.15 01:00:00",
                ParsedEvent::PlayerJoined(player.clone()),
                &mut players,
                &mut events,
                20,
                false,
            );
            apply_parsed_event(
                "2026.08.15 01:00:01",
                ParsedEvent::PlayerJoined(player),
                &mut players,
                &mut events,
                20,
                false,
            );
        }

        apply_parsed_event(
            "2026.08.15 01:01:00",
            ParsedEvent::PlayerLeft(TrackedPlayer {
                display_name: "Alice".to_string(),
                user_id: None,
            }),
            &mut players,
            &mut events,
            20,
            false,
        );

        assert_eq!(players.len(), 2);
        assert_eq!(
            events
                .iter()
                .filter(|event| event.event_type == "Player Joined")
                .count(),
            2
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| event.event_type == "Player Left")
                .count(),
            0
        );
    }

    #[test]
    fn incomplete_line_is_retried_instead_of_losing_the_player_name() {
        let path = std::env::temp_dir().join(format!(
            "vrcdog-gamelog-partial-{}-{}.txt",
            std::process::id(),
            Local::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let joined =
            "2026.08.15 01:00:00 Debug      -  [Behaviour] OnPlayerJoined Alice (usr_alice)\n";
        let partial = "2026.08.15 01:01:00 Debug      -  [Behaviour] OnPlayerLeft Al";
        fs::write(&path, format!("{joined}{partial}")).expect("write partial log");

        let mut players = HashMap::new();
        let mut events = VecDeque::new();
        let mut last_timestamp = None;
        let first_pos = process_complete_lines(
            &path,
            0,
            &mut players,
            &mut events,
            20,
            &mut last_timestamp,
            false,
        )
        .expect("read partial log");
        assert_eq!(first_pos, joined.len() as u64);
        assert_eq!(players.len(), 1);

        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open partial log");
        file.write_all(b"ice (usr_alice)\n")
            .expect("complete partial log");
        process_complete_lines(
            &path,
            first_pos,
            &mut players,
            &mut events,
            20,
            &mut last_timestamp,
            false,
        )
        .expect("read completed log");

        let leave = events
            .iter()
            .find(|event| event.event_type == "Player Left")
            .expect("player leave event");
        assert_eq!(leave.content, "Alice (usr_alice)");
        assert!(players.is_empty());
        let _ = fs::remove_file(path);
    }
}
