use chrono::Local;
use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tauri::State;

pub struct DbState {
    pub conn: Arc<StdMutex<Connection>>,
}

impl DbState {
    pub fn new(app_dir: PathBuf) -> Self {
        fs::create_dir_all(&app_dir).expect("Failed to create app data dir");
        let database_path = app_dir.join("vrcdog.db");
        let mistaken_livehime_path = app_dir.join("livehime.db");
        if !database_path.exists() && mistaken_livehime_path.exists() {
            if let Err(error) = fs::copy(&mistaken_livehime_path, &database_path) {
                eprintln!("[VrcDog] Failed to migrate mistaken livehime.db to vrcdog.db: {error}");
            }
        }

        let conn = Connection::open(database_path).expect("Failed to open SQLite database");
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .expect("Failed to set busy timeout");

        // Apply SQLite Performance Optimizations
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -64000;
            ",
        )
        .expect("Failed to apply SQLite pragmas");

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS auth (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                cookie TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
            );
            CREATE TABLE IF NOT EXISTS friend_activity (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                display_name TEXT NOT NULL,
                status TEXT NOT NULL,
                location TEXT,
                recorded_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
                date_key TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS notes (
                user_id TEXT PRIMARY KEY,
                display_name TEXT,
                note TEXT,
                updated_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
            );
            CREATE TABLE IF NOT EXISTS status_presets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                status TEXT NOT NULL,
                status_description TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS favorite_worlds (
                world_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                image_url TEXT,
                added_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
            );
            CREATE TABLE IF NOT EXISTS favorite_avatars (
                avatar_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                image_url TEXT,
                author_id TEXT,
                author_name TEXT,
                added_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
            );
            CREATE TABLE IF NOT EXISTS friend_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                user_id TEXT,
                display_name TEXT,
                detail TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
            );
            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
            );
            CREATE TABLE IF NOT EXISTS friends (
                user_id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                status TEXT NOT NULL,
                location TEXT,
                friend_data TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
            );
            CREATE TABLE IF NOT EXISTS game_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                time TEXT NOT NULL,
                event_type TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
            );
            CREATE TABLE IF NOT EXISTS notifications (
                id TEXT PRIMARY KEY,
                type TEXT NOT NULL,
                sender_user_id TEXT,
                sender_username TEXT,
                receiver_user_id TEXT,
                message TEXT,
                details TEXT,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS api_cache (
                key TEXT PRIMARY KEY,
                data TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
            );
            CREATE INDEX IF NOT EXISTS idx_activity_user ON friend_activity(user_id);
            CREATE INDEX IF NOT EXISTS idx_activity_time ON friend_activity(recorded_at);
            CREATE INDEX IF NOT EXISTS idx_friend_log_time ON friend_log(created_at);
            CREATE INDEX IF NOT EXISTS idx_friend_log_type ON friend_log(event_type);
            CREATE INDEX IF NOT EXISTS idx_friends_status ON friends(status);
            CREATE INDEX IF NOT EXISTS idx_game_log_time ON game_log(time);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_game_log_unique ON game_log(time, event_type, content);
            CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at DESC);
            
            CREATE TABLE IF NOT EXISTS bili_tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bvid TEXT NOT NULL,
                cid INTEGER NOT NULL,
                format INTEGER NOT NULL,
                title TEXT NOT NULL,
                owner TEXT NOT NULL,
                cover TEXT NOT NULL,
                status TEXT NOT NULL,
                folder TEXT NOT NULL,
                duration INTEGER NOT NULL,
                download_type TEXT NOT NULL,
                create_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
            );
            ",
        )
        .expect("无法初始化数据库表");

        // Apply schema migrations gracefully for existing users
        let _ = conn.execute(
            "ALTER TABLE friend_activity ADD COLUMN date_key TEXT NOT NULL DEFAULT ''",
            [],
        );
        let _ = conn.execute("ALTER TABLE favorite_worlds ADD COLUMN image_url TEXT", []);
        // Rename column label to name for status_presets if old version existed
        let _ = conn.execute("ALTER TABLE status_presets RENAME COLUMN label TO name", []);
        let _ = conn.execute(
            "ALTER TABLE status_presets ADD COLUMN name TEXT NOT NULL DEFAULT ''",
            [],
        );
        let _ = conn.execute(
            "DELETE FROM notifications
             WHERE COALESCE(TRIM(message), '') = ''
               AND COALESCE(TRIM(details), '') IN ('', '{}', 'null')
               AND COALESCE(TRIM(sender_username), '') = ''
               AND type NOT IN ('friendRequest', 'invite', 'requestInvite', 'group.invite', 'group.request', 'friend-online', 'friend-offline')",
            [],
        );

        Self {
            conn: Arc::new(StdMutex::new(conn)),
        }
    }
}

// ========== Favorite Worlds ==========
#[derive(Serialize, Deserialize, Debug)]
pub struct FavoriteWorldRecord {
    pub world_id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub added_at: String,
}

#[tauri::command]
pub fn db_add_favorite_world(
    state: State<'_, DbState>,
    world_id: String,
    name: String,
    image_url: Option<String>,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO favorite_worlds (world_id, name, image_url, added_at) VALUES (?1, ?2, ?3, datetime('now','localtime'))",
        params![world_id, name, image_url],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn db_get_favorite_worlds(
    state: State<'_, DbState>,
) -> Result<Vec<FavoriteWorldRecord>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT world_id, name, image_url, added_at FROM favorite_worlds ORDER BY added_at DESC")
        .map_err(|e| e.to_string())?;
    let results = stmt
        .query_map([], |row| {
            Ok(FavoriteWorldRecord {
                world_id: row.get(0)?,
                name: row.get(1)?,
                image_url: row.get(2)?,
                added_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(results)
}

#[tauri::command]
pub fn db_remove_favorite_world(state: State<'_, DbState>, world_id: String) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM favorite_worlds WHERE world_id = ?1",
        params![world_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ========== Auth ==========
#[tauri::command]
pub fn db_save_auth(state: State<'_, DbState>, cookie: String) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO auth (id, cookie, updated_at) VALUES (1, ?1, datetime('now','localtime'))",
        params![cookie],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn db_get_auth(state: State<'_, DbState>) -> Result<Option<String>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT cookie FROM auth WHERE id = 1")
        .map_err(|e| e.to_string())?;
    let result = stmt.query_row([], |row| row.get(0)).ok();
    Ok(result)
}

#[tauri::command]
pub fn db_clear_auth(state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM auth WHERE id = 1", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ========== Friend Activity ==========
#[tauri::command]
pub fn db_record_activity(
    state: State<'_, DbState>,
    user_id: String,
    display_name: String,
    status: String,
    location: Option<String>,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let date_key = Local::now().format("%Y-%m-%d").to_string();
    conn.execute(
        "INSERT INTO friend_activity (user_id, display_name, status, location, date_key) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![user_id, display_name, status, location, date_key],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct OnlineFriendRecord {
    pub id: String,
    pub displayName: String,
    pub status: String,
    pub location: Option<String>,
}

#[tauri::command]
pub fn db_batch_record_friends(
    state: State<'_, DbState>,
    friends_json: String,
) -> Result<u32, String> {
    let friends: Vec<OnlineFriendRecord> =
        serde_json::from_str(&friends_json).map_err(|e| e.to_string())?;
    let mut conn = state.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let date_key = Local::now().format("%Y-%m-%d").to_string();
    let mut count = 0u32;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO friend_activity (user_id, display_name, status, location, date_key) VALUES (?1, ?2, ?3, ?4, ?5)"
        ).map_err(|e| e.to_string())?;
        for f in &friends {
            let res = stmt.execute(params![f.id, f.displayName, f.status, f.location, date_key]);
            if res.is_ok() {
                count += 1;
            }
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}

#[tauri::command]
pub fn db_get_heatmap(state: State<'_, DbState>) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    // SQLite strftime('%w', recorded_at) returns '0' (Sun) to '6' (Sat)
    // strftime('%H', recorded_at) returns '00' to '23'
    let mut stmt = conn
        .prepare("SELECT CAST(strftime('%w', recorded_at) AS INTEGER) as day_of_week, CAST(strftime('%H', recorded_at) AS INTEGER) as hour_of_day, COUNT(*) as activity_count FROM friend_activity GROUP BY day_of_week, hour_of_day")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let mut map = serde_json::Map::new();
            // Shift day so Monday = 0, Sunday = 6
            let raw_day: i64 = row.get(0)?;
            let mut day = raw_day - 1;
            if day < 0 {
                day = 6;
            }

            map.insert("day".to_string(), serde_json::Value::Number(day.into()));
            map.insert(
                "hour".to_string(),
                serde_json::Value::Number(row.get::<_, i64>(1)?.into()),
            );
            map.insert(
                "count".to_string(),
                serde_json::Value::Number(row.get::<_, i64>(2)?.into()),
            );
            Ok(serde_json::Value::Object(map))
        })
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for val in rows.flatten() {
        results.push(val);
    }
    Ok(results)
}

#[tauri::command]
pub fn db_get_heatmap_details(
    state: State<'_, DbState>,
    day: i64,
    hour: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    // Reverse the day shift: if day is 6 (Sun), raw_day is 0. Else raw_day is day + 1.
    let raw_day = if day == 6 { 0 } else { day + 1 };
    let raw_day_str = raw_day.to_string();
    let hour_str = format!("{:02}", hour);

    let mut stmt = conn
        .prepare("SELECT display_name, COUNT(*) as c FROM friend_activity WHERE strftime('%w', recorded_at) = ?1 AND strftime('%H', recorded_at) = ?2 GROUP BY display_name ORDER BY c DESC LIMIT 10")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![raw_day_str, hour_str], |row| {
            let mut map = serde_json::Map::new();
            map.insert(
                "displayName".to_string(),
                serde_json::Value::String(row.get(0)?),
            );
            map.insert(
                "count".to_string(),
                serde_json::Value::Number(row.get::<_, i64>(1)?.into()),
            );
            Ok(serde_json::Value::Object(map))
        })
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for val in rows.flatten() {
        results.push(val);
    }
    Ok(results)
}

// ========== Friend Logs ==========
#[tauri::command]
pub fn db_add_friend_log(
    state: State<'_, DbState>,
    event_type: String,
    user_id: Option<String>,
    display_name: Option<String>,
    detail: Option<String>,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let mut final_display_name = display_name;
    if let Some(uid) = &user_id {
        let needs_cached_name = final_display_name
            .as_deref()
            .map(str::trim)
            .is_none_or(|name| name.is_empty() || name.eq_ignore_ascii_case("unknown") || name == uid);
        if needs_cached_name {
            if let Ok(Some(cached_name)) = conn
                .query_row(
                    "SELECT display_name FROM friends WHERE user_id = ?1",
                    params![uid],
                    |row| row.get::<_, String>(0),
                )
                .optional()
            {
                final_display_name = Some(cached_name);
            }
        }
    }

    conn.execute(
        "INSERT INTO friend_log (event_type, user_id, display_name, detail) VALUES (?1, ?2, ?3, ?4)",
        params![event_type, user_id, final_display_name, detail],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FriendLogRecord {
    pub id: i64,
    pub event_type: String,
    pub user_id: Option<String>,
    pub display_name: Option<String>,
    pub detail: Option<String>,
    pub created_at: String,
}

#[tauri::command]
pub fn db_get_friend_logs(
    state: State<'_, DbState>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<FriendLogRecord>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, event_type, user_id, display_name, detail, created_at FROM friend_log ORDER BY id DESC LIMIT ?1 OFFSET ?2")
        .map_err(|e| e.to_string())?;
    let lim = limit.unwrap_or(200);
    let off = offset.unwrap_or(0);

    let logs = stmt
        .query_map(params![lim, off], |row| {
            Ok(FriendLogRecord {
                id: row.get(0)?,
                event_type: row.get(1)?,
                user_id: row.get(2)?,
                display_name: row.get(3)?,
                detail: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(logs)
}

// ========== Notes ==========
#[derive(Serialize, Deserialize, Debug)]
pub struct NoteRecord {
    pub user_id: String,
    pub display_name: Option<String>,
    pub note: String,
    pub updated_at: String,
}

#[tauri::command]
pub fn db_get_all_notes(state: State<'_, DbState>) -> Result<Vec<NoteRecord>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT user_id, display_name, note, updated_at FROM notes ORDER BY updated_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let notes = stmt
        .query_map([], |row| {
            Ok(NoteRecord {
                user_id: row.get(0)?,
                display_name: row.get(1)?,
                note: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(notes)
}

#[tauri::command]
pub fn db_save_note(
    state: State<'_, DbState>,
    user_id: String,
    display_name: Option<String>,
    note: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO notes (user_id, display_name, note, updated_at) VALUES (?1, ?2, ?3, datetime('now','localtime'))",
        params![user_id, display_name, note],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn db_get_note(
    state: State<'_, DbState>,
    user_id: String,
) -> Result<Option<NoteRecord>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT user_id, display_name, note, updated_at FROM notes WHERE user_id = ?1")
        .map_err(|e| e.to_string())?;
    let res = stmt
        .query_row(params![user_id], |row| {
            Ok(NoteRecord {
                user_id: row.get(0)?,
                display_name: row.get(1)?,
                note: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })
        .ok();
    Ok(res)
}

// ========== Status Presets ==========
#[derive(Serialize, Deserialize, Debug)]
pub struct StatusPreset {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub status_description: String,
}

#[tauri::command]
pub fn db_get_status_presets(state: State<'_, DbState>) -> Result<Vec<StatusPreset>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, status, status_description FROM status_presets")
        .map_err(|e| e.to_string())?;
    let presets = stmt
        .query_map([], |row| {
            Ok(StatusPreset {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
                status_description: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(presets)
}

#[tauri::command]
pub fn db_save_status_preset(
    state: State<'_, DbState>,
    name: String,
    status: String,
    status_description: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO status_presets (name, status, status_description) VALUES (?1, ?2, ?3)",
        params![name, status, status_description],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn db_delete_status_preset(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM status_presets WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ========== Export ==========
#[tauri::command]
pub fn db_export_all(state: State<'_, DbState>) -> Result<serde_json::Value, String> {
    let mut result = serde_json::Map::new();

    if let Ok(notes) = db_get_all_notes(state.clone()) {
        if let Ok(v) = serde_json::to_value(notes) {
            result.insert("notes".to_string(), v);
        }
    }
    if let Ok(logs) = db_get_friend_logs(state.clone(), Some(10000), Some(0)) {
        if let Ok(v) = serde_json::to_value(logs) {
            result.insert("friend_logs".to_string(), v);
        }
    }
    if let Ok(presets) = db_get_status_presets(state.clone()) {
        if let Ok(v) = serde_json::to_value(presets) {
            result.insert("status_presets".to_string(), v);
        }
    }

    Ok(serde_json::Value::Object(result))
}

// ========== App Settings ==========
#[tauri::command]
pub fn db_save_setting(
    state: State<'_, DbState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, datetime('now','localtime'))",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn db_get_setting(state: State<'_, DbState>, key: String) -> Result<Option<String>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT value FROM app_settings WHERE key = ?1")
        .map_err(|e| e.to_string())?;
    let result: Option<String> = stmt.query_row(params![key], |row| row.get(0)).ok();
    Ok(result)
}

#[tauri::command]
pub fn db_get_all_settings(state: State<'_, DbState>) -> Result<serde_json::Value, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT key, value FROM app_settings")
        .map_err(|e| e.to_string())?;
    let mut map = serde_json::Map::new();
    let rows = stmt
        .query_map([], |row| {
            let k: String = row.get(0)?;
            let v: String = row.get(1)?;
            Ok((k, v))
        })
        .map_err(|e| e.to_string())?;
    for (k, v) in rows.flatten() {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&v) {
            map.insert(k, parsed);
        } else {
            map.insert(k, serde_json::Value::String(v));
        }
    }
    Ok(serde_json::Value::Object(map))
}

// ========== Friends Caching ==========
#[tauri::command]
pub fn db_save_friend(
    state: State<'_, DbState>,
    user_id: String,
    display_name: String,
    status: String,
    location: Option<String>,
    friend_data: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO friends (user_id, display_name, status, location, friend_data, updated_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, datetime('now','localtime'))",
        params![user_id, display_name, status, location, friend_data],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn db_batch_save_friends(
    state: State<'_, DbState>,
    friends_json: String,
) -> Result<u32, String> {
    let friends: Vec<serde_json::Value> =
        serde_json::from_str(&friends_json).map_err(|e| e.to_string())?;
    let mut conn = state.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let mut count = 0u32;
    {
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO friends (user_id, display_name, status, location, friend_data, updated_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now','localtime'))"
        ).map_err(|e| e.to_string())?;

        for f in &friends {
            let user_id = f["id"].as_str().unwrap_or("");
            let display_name = f["displayName"].as_str().unwrap_or("");
            let status = f["status"].as_str().unwrap_or("offline");
            let location = f["location"].as_str();
            let friend_data = serde_json::to_string(f).unwrap_or_default();

            if !user_id.is_empty() {
                let res = stmt.execute(params![
                    user_id,
                    display_name,
                    status,
                    location,
                    friend_data
                ]);
                if res.is_ok() {
                    count += 1;
                }
            }
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}

#[tauri::command]
pub fn db_get_friends(state: State<'_, DbState>) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT friend_data FROM friends ORDER BY display_name COLLATE NOCASE ASC")
        .map_err(|e| e.to_string())?;

    let results = stmt
        .query_map([], |row| {
            let json_str: String = row.get(0)?;
            Ok(json_str)
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .filter_map(|s| serde_json::from_str(&s).ok())
        .collect();

    Ok(results)
}

#[tauri::command]
pub fn db_remove_friend(state: State<'_, DbState>, user_id: String) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM friends WHERE user_id = ?1", params![user_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ========== Game Logs ==========
#[derive(Serialize, Deserialize, Debug)]
pub struct GameLogRecord {
    pub time: String,
    pub event_type: String,
    pub content: String,
}

#[tauri::command]
pub fn db_save_game_logs(state: State<'_, DbState>, logs_json: String) -> Result<u32, String> {
    let logs: Vec<GameLogRecord> = serde_json::from_str(&logs_json).map_err(|e| e.to_string())?;
    let mut conn = state.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let mut count = 0u32;
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO game_log (time, event_type, content) 
             SELECT ?1, ?2, ?3
             WHERE NOT EXISTS (
                 SELECT 1 FROM game_log WHERE time = ?1 AND event_type = ?2 AND content = ?3
             )",
            )
            .map_err(|e| e.to_string())?;
        for log in &logs {
            let res = stmt.execute(params![log.time, log.event_type, log.content]);
            if let Ok(affected) = res {
                if affected > 0 {
                    count += 1;
                }
            }
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}

#[tauri::command]
pub fn db_get_game_logs(
    state: State<'_, DbState>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<GameLogRecord>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let lim = limit.unwrap_or(200);
    let off = offset.unwrap_or(0);

    let mut stmt = conn
        .prepare(
            "SELECT time, event_type, content FROM game_log ORDER BY time DESC, id DESC LIMIT ?1 OFFSET ?2",
        )
        .map_err(|e| e.to_string())?;

    let results = stmt
        .query_map(params![lim, off], |row| {
            Ok(GameLogRecord {
                time: row.get(0)?,
                event_type: row.get(1)?,
                content: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(results)
}

// ========== Notifications ==========
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct NotificationRecord {
    pub id: String,
    pub r#type: String, // Use r#type because type is a reserved keyword in Rust
    pub senderUserId: Option<String>,
    pub senderUsername: Option<String>,
    pub receiverUserId: Option<String>,
    pub message: String,
    pub details: String,
    pub created_at: String,
}

#[tauri::command]
pub fn db_batch_save_notifications(
    state: State<'_, DbState>,
    notifications_json: String,
) -> Result<u32, String> {
    let notifications: Vec<NotificationRecord> =
        serde_json::from_str(&notifications_json).map_err(|e| e.to_string())?;
    let mut conn = state.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let mut count = 0u32;
    {
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO notifications (id, type, sender_user_id, sender_username, receiver_user_id, message, details, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
        ).map_err(|e| e.to_string())?;
        for n in &notifications {
            let res = stmt.execute(params![
                n.id,
                n.r#type,
                n.senderUserId,
                n.senderUsername,
                n.receiverUserId,
                n.message,
                n.details,
                n.created_at
            ]);
            if res.is_ok() {
                count += 1;
            }
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}

#[tauri::command]
pub fn db_save_notification(
    state: State<'_, DbState>,
    notification_json: String,
) -> Result<(), String> {
    let n: NotificationRecord =
        serde_json::from_str(&notification_json).map_err(|e| e.to_string())?;
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO notifications (id, type, sender_user_id, sender_username, receiver_user_id, message, details, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            n.id, n.r#type, n.senderUserId, n.senderUsername, n.receiverUserId, n.message, n.details, n.created_at
        ]
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn db_get_notifications(
    state: State<'_, DbState>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let lim = limit.unwrap_or(200);
    let off = offset.unwrap_or(0);

    let mut stmt = conn.prepare("SELECT id, type, sender_user_id, sender_username, receiver_user_id, message, details, created_at FROM notifications ORDER BY created_at DESC LIMIT ?1 OFFSET ?2")
        .map_err(|e| e.to_string())?;

    let results = stmt
        .query_map(params![lim, off], |row| {
            let mut map = serde_json::Map::new();
            map.insert("id".to_string(), serde_json::Value::String(row.get(0)?));
            map.insert("type".to_string(), serde_json::Value::String(row.get(1)?));
            if let Ok(Some(s)) = row.get::<_, Option<String>>(2) {
                map.insert("senderUserId".to_string(), serde_json::Value::String(s));
            }
            if let Ok(Some(s)) = row.get::<_, Option<String>>(3) {
                map.insert("senderUsername".to_string(), serde_json::Value::String(s));
            }
            if let Ok(Some(s)) = row.get::<_, Option<String>>(4) {
                map.insert("receiverUserId".to_string(), serde_json::Value::String(s));
            }
            map.insert(
                "message".to_string(),
                serde_json::Value::String(row.get(5)?),
            );
            if let Ok(details) =
                serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(6)?)
            {
                map.insert("details".to_string(), details);
            } else {
                map.insert(
                    "details".to_string(),
                    serde_json::Value::String(row.get(6)?),
                );
            }
            map.insert(
                "created_at".to_string(),
                serde_json::Value::String(row.get(7)?),
            );
            Ok(serde_json::Value::Object(map))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(results)
}

#[tauri::command]
pub fn db_delete_notification(state: State<'_, DbState>, id: String) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM notifications WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ========== Favorite Avatars ==========

#[tauri::command]
pub fn db_add_favorite_avatar(
    state: State<'_, DbState>,
    avatar_id: String,
    name: String,
    image_url: String,
    author_id: String,
    author_name: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO favorite_avatars (avatar_id, name, image_url, author_id, author_name) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![avatar_id, name, image_url, author_id, author_name],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn db_remove_favorite_avatar(
    state: State<'_, DbState>,
    avatar_id: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM favorite_avatars WHERE avatar_id = ?1",
        params![avatar_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn db_get_favorite_avatars(
    state: State<'_, DbState>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT avatar_id, name, image_url, author_id, author_name, added_at FROM favorite_avatars ORDER BY added_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let mut map = serde_json::Map::new();
            map.insert(
                "avatar_id".to_string(),
                serde_json::Value::String(row.get(0)?),
            );
            map.insert("name".to_string(), serde_json::Value::String(row.get(1)?));
            map.insert(
                "image_url".to_string(),
                serde_json::Value::String(row.get(2)?),
            );
            map.insert(
                "author_id".to_string(),
                serde_json::Value::String(row.get(3)?),
            );
            map.insert(
                "author_name".to_string(),
                serde_json::Value::String(row.get(4)?),
            );
            map.insert(
                "added_at".to_string(),
                serde_json::Value::String(row.get(5)?),
            );
            Ok(serde_json::Value::Object(map))
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for val in rows.flatten() {
        result.push(val);
    }
    Ok(result)
}

// ========== Bilibili Tasks ==========
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct BiliTaskRecord {
    pub id: i64,
    pub bvid: String,
    pub cid: i64,
    pub format: i64,
    pub title: String,
    pub owner: String,
    pub cover: String,
    pub status: String,
    pub folder: String,
    pub duration: i64,
    pub download_type: String,
    pub create_at: String,
}

#[tauri::command]
pub fn db_bili_add_task(
    state: tauri::State<'_, DbState>,
    bvid: String,
    cid: i64,
    format: i64,
    title: String,
    owner: String,
    cover: String,
    status: String,
    folder: String,
    duration: i64,
    download_type: String,
) -> Result<i64, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO bili_tasks (bvid, cid, format, title, owner, cover, status, folder, duration, download_type)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![bvid, cid, format, title, owner, cover, status, folder, duration, download_type],
    ).map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    Ok(id)
}

#[tauri::command]
pub fn db_bili_get_tasks(state: tauri::State<'_, DbState>) -> Result<Vec<BiliTaskRecord>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, bvid, cid, format, title, owner, cover, status, folder, duration, download_type, create_at FROM bili_tasks ORDER BY id DESC")
        .map_err(|e| e.to_string())?;

    let tasks = stmt
        .query_map([], |row| {
            Ok(BiliTaskRecord {
                id: row.get(0)?,
                bvid: row.get(1)?,
                cid: row.get(2)?,
                format: row.get(3)?,
                title: row.get(4)?,
                owner: row.get(5)?,
                cover: row.get(6)?,
                status: row.get(7)?,
                folder: row.get(8)?,
                duration: row.get(9)?,
                download_type: row.get(10)?,
                create_at: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(tasks)
}

#[tauri::command]
pub fn db_bili_update_task_status(
    state: tauri::State<'_, DbState>,
    id: i64,
    status: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE bili_tasks SET status = ?1 WHERE id = ?2",
        rusqlite::params![status, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn db_bili_delete_task(state: tauri::State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM bili_tasks WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ========== API Cache ==========
#[tauri::command]
pub fn db_save_api_cache(
    state: tauri::State<'_, DbState>,
    key: String,
    data: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO api_cache (key, data, updated_at) VALUES (?1, ?2, datetime('now','localtime'))",
        params![key, data],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn db_get_api_cache(
    state: tauri::State<'_, DbState>,
    key: String,
) -> Result<Option<String>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT data FROM api_cache WHERE key = ?1")
        .map_err(|e| e.to_string())?;
    let result = stmt.query_row([key], |row| row.get(0)).ok();
    Ok(result)
}

#[tauri::command]
pub fn db_clear_game_logs(state: tauri::State<'_, DbState>) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM game_log", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn db_clear_friend_logs(state: tauri::State<'_, DbState>) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM friend_log", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}
