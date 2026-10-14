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
    pub fn new(app_dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&app_dir).map_err(|e| format!("Failed to create app data dir: {e}"))?;
        let database_path = app_dir.join("vrcdog.db");
        let mistaken_livehime_path = app_dir.join("livehime.db");
        if !database_path.exists() && mistaken_livehime_path.exists() {
            if let Err(error) = fs::copy(&mistaken_livehime_path, &database_path) {
                eprintln!("[VrcDog] Failed to migrate mistaken livehime.db to vrcdog.db: {error}");
            }
        }

        let conn = Connection::open(&database_path)
            .map_err(|e| format!("Failed to open SQLite database at {}: {e}", database_path.display()))?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| format!("Failed to set busy timeout: {e}"))?;

        // Apply SQLite Performance Optimizations
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -64000;
            PRAGMA temp_store = MEMORY;
            PRAGMA mmap_size = 268435456;
            ",
        )
        .map_err(|e| format!("Failed to apply SQLite pragmas: {e}"))?;

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
        .map_err(|e| format!("无法初始化数据库表: {e}"))?;

        // Apply schema migrations gracefully for existing users.
        // 只有「列/约束已经存在」这类幂等错误才允许忽略；其余错误必须暴露出来，
        // 否则库会停留在半迁移状态，后续 `SELECT 新列` 直接报错却无人知晓原因。
        apply_migration(&conn, "ALTER TABLE friend_activity ADD COLUMN date_key TEXT NOT NULL DEFAULT ''");
        apply_migration(&conn, "ALTER TABLE favorite_worlds ADD COLUMN image_url TEXT");
        // Rename column label to name for status_presets if old version existed
        apply_migration(&conn, "ALTER TABLE status_presets RENAME COLUMN label TO name");
        apply_migration(&conn, "ALTER TABLE status_presets ADD COLUMN name TEXT NOT NULL DEFAULT ''");
        let _ = conn.execute(
            "DELETE FROM notifications
             WHERE COALESCE(TRIM(message), '') = ''
               AND COALESCE(TRIM(details), '') IN ('', '{}', 'null')
               AND COALESCE(TRIM(sender_username), '') = ''
               AND type NOT IN ('friendRequest', 'invite', 'requestInvite', 'group.invite', 'group.request', 'friend-online', 'friend-offline')",
            [],
        );

        // 索引必须放在「建表 + 迁移」之后：
        // 之前它们和建表语句混在同一个批次里，而且排在 `CREATE TABLE bili_tasks` 前面，
        // 全新安装（空目录）第一启动就会在 `idx_bili_tasks_status` 上报
        // `no such table: main.bili_tasks`，整个批次中断、`DbState::new` 返回 Err、应用起不来。
        // 同理 `idx_activity_user_date` 引用 `date_key`，老库要等上面的 ALTER 迁移补完列才存在。
        // 索引只是性能优化，缺了不该让应用无法启动，因此这里记录错误但不中断。
        const INDEX_SQL: &str = "
            CREATE INDEX IF NOT EXISTS idx_activity_user ON friend_activity(user_id);
            CREATE INDEX IF NOT EXISTS idx_activity_time ON friend_activity(recorded_at);
            CREATE INDEX IF NOT EXISTS idx_activity_user_date ON friend_activity(user_id, date_key);
            CREATE INDEX IF NOT EXISTS idx_friend_log_time ON friend_log(created_at);
            CREATE INDEX IF NOT EXISTS idx_friend_log_type ON friend_log(event_type);
            CREATE INDEX IF NOT EXISTS idx_friend_log_user_id ON friend_log(user_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_friends_status ON friends(status);
            CREATE INDEX IF NOT EXISTS idx_game_log_time ON game_log(time);
            CREATE INDEX IF NOT EXISTS idx_game_log_event_type ON game_log(event_type, time DESC);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_game_log_unique ON game_log(time, event_type, content);
            CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_notifications_type ON notifications(type, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_bili_tasks_status ON bili_tasks(status);
            ";
        if let Err(e) = conn.execute_batch(INDEX_SQL) {
            eprintln!("[VrcDog] 部分索引创建失败（不影响启动，但查询可能变慢）: {e}");
            // 整批中断时前面的索引也不会生效，逐条重建一次，尽量把热点索引补齐。
            for statement in INDEX_SQL.split(';').map(str::trim).filter(|s| !s.is_empty()) {
                if let Err(inner) = conn.execute(statement, []) {
                    eprintln!("[VrcDog] 跳过索引语句: {inner}");
                }
            }
        }

        Ok(Self {
            conn: Arc::new(StdMutex::new(conn)),
        })
    }

    /// 把 WAL 里尚未落盘的帧写回主库文件并截断 `-wal`。
    ///
    /// 备份 / 还原 / 任何「直接拷贝 vrcdog.db 主文件」的操作之前都必须先调用它：
    /// WAL 模式下最近提交的数据位于 `vrcdog.db-wal`，只拷主文件会得到一个
    /// 「缺少最近写入」的静默残缺备份，还原时也会因主库与残留 `-wal` 不匹配而丢数据。
    pub fn checkpoint_wal(&self) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Database mutex poisoned: {e}"))?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|e| format!("WAL checkpoint failed: {e}"))?;
        Ok(())
    }

    /// 通过 SQLite Online Backup API 把外部数据库整库导入当前正在使用的连接。
    ///
    /// 不能用 `fs::copy` 直接覆盖在用的 `vrcdog.db`：那会留下与主库不匹配的
    /// `vrcdog.db-wal` / `vrcdog.db-shm`，重启后 SQLite 要么丢弃 WAL 内容、要么用
    /// 陈旧帧「复活」被覆盖前的数据；活连接的页缓存也可能把旧页回写覆盖新库。
    /// Backup API 在同一活连接内事务性地重写页面，WAL、页缓存与主文件始终一致，
    /// 并且源文件必须真是可读的 SQLite 库（顺带把「随便一个 .db 文件」挡在门外）。
    pub fn restore_from(&self, source: &std::path::Path) -> Result<(), String> {
        let mut to = self
            .conn
            .lock()
            .map_err(|e| format!("Database mutex poisoned: {e}"))?;
        let from = Connection::open_with_flags(source, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("Cannot open restore source as SQLite database: {e}"))?;
        let backup = rusqlite::backup::Backup::new(&from, &mut *to)
            .map_err(|e| format!("Restore init failed: {e}"))?;
        backup
            .run_to_completion(64, std::time::Duration::from_millis(10), None)
            .map_err(|e| format!("Restore failed: {e}"))?;
        Ok(())
    }
}

/// 收藏世界 upsert：冲突时保留原始 `added_at`，并且只在导入源真的给了图片时才覆盖图片。
const UPSERT_FAVORITE_WORLD: &str = "
    INSERT INTO favorite_worlds (world_id, name, image_url, added_at)
    VALUES (?1, ?2, ?3, COALESCE(
        (SELECT added_at FROM favorite_worlds WHERE world_id = ?1),
        datetime('now','localtime')))
    ON CONFLICT(world_id) DO UPDATE SET
        name = excluded.name,
        image_url = COALESCE(excluded.image_url, favorite_worlds.image_url)";

/// 收藏头像 upsert（手动收藏路径，字段均为非空字符串）。
const UPSERT_FAVORITE_AVATAR: &str = "
    INSERT INTO favorite_avatars (avatar_id, name, image_url, author_id, author_name)
    VALUES (?1, ?2, ?3, ?4, ?5)
    ON CONFLICT(avatar_id) DO UPDATE SET
        name = excluded.name,
        image_url = excluded.image_url,
        author_id = excluded.author_id,
        author_name = excluded.author_name";

/// 收藏头像 upsert（导入路径：允许缺失字段，用已有值兜底，并保留原始收藏时间）。
const UPSERT_IMPORTED_AVATAR: &str = "
    INSERT INTO favorite_avatars (avatar_id, name, image_url, author_id, author_name, added_at)
    VALUES (?1, ?2, ?3, ?4, ?5, COALESCE(
        (SELECT added_at FROM favorite_avatars WHERE avatar_id = ?1),
        datetime('now','localtime')))
    ON CONFLICT(avatar_id) DO UPDATE SET
        name = excluded.name,
        image_url = COALESCE(excluded.image_url, favorite_avatars.image_url),
        author_id = COALESCE(excluded.author_id, favorite_avatars.author_id),
        author_name = COALESCE(excluded.author_name, favorite_avatars.author_name)";

/// 备注 upsert：导入源没有 displayName 时保留本地已有显示名。
const UPSERT_IMPORTED_NOTE: &str = "
    INSERT INTO notes (user_id, display_name, note, updated_at)
    VALUES (?1, ?2, ?3, datetime('now','localtime'))
    ON CONFLICT(user_id) DO UPDATE SET
        display_name = COALESCE(NULLIF(TRIM(excluded.display_name), ''), notes.display_name),
        note = excluded.note,
        updated_at = excluded.updated_at";

/// 判断一条迁移语句的失败是否属于「已经应用过」的幂等错误。
fn is_benign_migration_error(message: &str) -> bool {
    let msg = message.to_ascii_lowercase();
    msg.contains("duplicate column name")
        || msg.contains("no such column")
        || msg.contains("already in use")
        || msg.contains("already exists")
}

/// 执行一条幂等迁移语句：成功或「已应用」都返回 true；真正的失败会被记录并返回 false。
fn apply_migration(conn: &Connection, sql: &str) -> bool {
    match conn.execute(sql, []) {
        Ok(_) => true,
        Err(e) => {
            if is_benign_migration_error(&e.to_string()) {
                true
            } else {
                eprintln!("[VrcDog] schema migration failed: {e} :: {sql}");
                false
            }
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
    // 保留原始收藏时间（同 `db_add_favorite_avatar`）：重复收藏不应把条目顶到列表最前。
    conn.execute(UPSERT_FAVORITE_WORLD, params![world_id, name, image_url])
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
            // 不能把 SQL 错误咽掉只少计一个数：否则「今日活动」会静默缺行，
            // 界面显示成功而数据库里什么都没有。出错就回滚整批并把原因报给前端。
            if let Err(e) = res {
                return Err(format!("批量写入好友活动记录失败（第 {} 条）: {}", count + 1, e));
            }
            count += 1;
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

/// Batch-save many settings in a single transaction. Replaces the previous
/// per-key `db_save_setting` loop that issued 100+ separate IPC calls.
#[tauri::command]
pub fn db_save_settings(
    state: State<'_, DbState>,
    settings: Vec<(String, String)>,
) -> Result<(), String> {
    let mut conn = state.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for (key, value) in &settings {
        tx.execute(
            "INSERT OR REPLACE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, datetime('now','localtime'))",
            params![key, value],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
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
                // 同 `db_batch_record_friends`：SQL 错误必须上报，不能静默丢行。
                if let Err(e) = res {
                    return Err(format!("批量保存好友缓存失败（第 {} 条）: {}", count + 1, e));
                }
                count += 1;
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
            // 同 `db_batch_record_friends`：SQL 错误必须上报，不能静默丢通知。
            if let Err(e) = res {
                return Err(format!("批量保存通知失败（第 {} 条）: {}", count + 1, e));
            }
            count += 1;
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
    // 用 upsert 而不是 `INSERT OR REPLACE`：REPLACE 是「删了重插」，未列出的
    // `added_at` 会退回列默认值变成当前时间。收藏列表按 `added_at DESC` 排序，
    // 于是重新收藏一个头像会把它跳到列表最前，并丢掉原始收藏时间。
    conn.execute(
        UPSERT_FAVORITE_AVATAR,
        params![avatar_id, name, image_url, author_id, author_name],
    )
    .map_err(|e| e.to_string())?;
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

// ========== VRCX Interoperability & Fine-grained Export/Import ==========
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VrcxExportData {
    pub version: String,
    pub exported_at: String,
    pub generator: String,
    pub notes: Vec<NoteRecord>,
    pub favorite_worlds: Vec<FavoriteWorldRecord>,
    pub favorite_avatars: Vec<serde_json::Value>,
    pub total_notes: usize,
    pub total_worlds: usize,
    pub total_avatars: usize,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VrcxImportResult {
    pub imported_notes: usize,
    pub imported_worlds: usize,
    pub imported_avatars: usize,
}

#[tauri::command]
pub fn db_export_vrcx_data(state: tauri::State<'_, DbState>) -> Result<VrcxExportData, String> {
    let notes = db_get_all_notes(state.clone())?;
    let favorite_worlds = db_get_favorite_worlds(state.clone())?;
    let favorite_avatars = db_get_favorite_avatars(state)?;

    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    Ok(VrcxExportData {
        version: "5.7.18".to_string(),
        exported_at: now_str,
        generator: "VRCDog".to_string(),
        total_notes: notes.len(),
        total_worlds: favorite_worlds.len(),
        total_avatars: favorite_avatars.len(),
        notes,
        favorite_worlds,
        favorite_avatars,
    })
}

#[tauri::command]
pub fn db_import_vrcx_data(state: tauri::State<'_, DbState>, json_data: String) -> Result<VrcxImportResult, String> {
    let value: serde_json::Value = serde_json::from_str(&json_data)
        .map_err(|e| format!("JSON 格式解析失败: {e}"))?;

    let mut conn = state.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut imported_notes = 0;
    let mut imported_worlds = 0;
    let mut imported_avatars = 0;

    // 1. Process notes: support either { "notes": [...] } or direct array [...] or VRCX { "friends": [...] }
    let notes_candidates = value.get("notes")
        .or_else(|| value.get("friends"))
        .and_then(|v| v.as_array())
        .or_else(|| value.as_array());

    if let Some(list) = notes_candidates {
        let mut note_stmt = tx.prepare(UPSERT_IMPORTED_NOTE).map_err(|e| e.to_string())?;

        for item in list {
            let user_id = item.get("userId")
                .or_else(|| item.get("user_id"))
                .or_else(|| item.get("id"))
                .and_then(|v| v.as_str());
            let display_name = item.get("displayName")
                .or_else(|| item.get("display_name"))
                .or_else(|| item.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let note = item.get("note")
                .or_else(|| item.get("memo"))
                .and_then(|v| v.as_str());

            if let (Some(uid), Some(n)) = (user_id, note) {
                if !uid.trim().is_empty() && !n.trim().is_empty() {
                    // 原来 `let _ = execute(..)` 把错误全部吞掉却照样 `+1`，
                    // 于是导入失败也会汇报「成功导入 N 条备注」，用户完全看不出来。
                    note_stmt
                        .execute(params![uid.trim(), display_name.trim(), n.trim()])
                        .map_err(|e| format!("导入备注 {} 失败: {}", uid.trim(), e))?;
                    imported_notes += 1;
                }
            }
        }
    }

    // 2. Process favorite worlds
    let worlds_candidates = value.get("favoriteWorlds")
        .or_else(|| value.get("favorite_worlds"))
        .or_else(|| value.get("worlds"))
        .and_then(|v| v.as_array());

    if let Some(list) = worlds_candidates {
        let mut world_stmt = tx.prepare(UPSERT_FAVORITE_WORLD).map_err(|e| e.to_string())?;

        for item in list {
            let world_id = item.get("worldId")
                .or_else(|| item.get("world_id"))
                .or_else(|| item.get("id"))
                .and_then(|v| v.as_str());
            let name = item.get("name")
                .or_else(|| item.get("worldName"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown World");
            let image_url = item.get("imageUrl")
                .or_else(|| item.get("image_url"))
                .or_else(|| item.get("thumbnailImageUrl"))
                .and_then(|v| v.as_str());

            if let Some(wid) = world_id {
                if !wid.trim().is_empty() {
                    world_stmt
                        .execute(params![wid.trim(), name.trim(), image_url])
                        .map_err(|e| format!("导入收藏世界 {} 失败: {}", wid.trim(), e))?;
                    imported_worlds += 1;
                }
            }
        }
    }

    // 3. Process favorite avatars
    let avatars_candidates = value.get("favoriteAvatars")
        .or_else(|| value.get("favorite_avatars"))
        .or_else(|| value.get("avatars"))
        .and_then(|v| v.as_array());

    if let Some(list) = avatars_candidates {
        let mut avatar_stmt = tx.prepare(UPSERT_IMPORTED_AVATAR).map_err(|e| e.to_string())?;

        for item in list {
            let avatar_id = item.get("avatarId")
                .or_else(|| item.get("avatar_id"))
                .or_else(|| item.get("id"))
                .and_then(|v| v.as_str());
            let name = item.get("name")
                .or_else(|| item.get("avatarName"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Avatar");
            let image_url = item.get("imageUrl")
                .or_else(|| item.get("image_url"))
                .or_else(|| item.get("thumbnailImageUrl"))
                .and_then(|v| v.as_str());
            let author_id = item.get("authorId")
                .or_else(|| item.get("author_id"))
                .and_then(|v| v.as_str());
            let author_name = item.get("authorName")
                .or_else(|| item.get("author_name"))
                .and_then(|v| v.as_str());

            if let Some(aid) = avatar_id {
                if !aid.trim().is_empty() {
                    avatar_stmt
                        .execute(params![aid.trim(), name.trim(), image_url, author_id, author_name])
                        .map_err(|e| format!("导入收藏头像 {} 失败: {}", aid.trim(), e))?;
                    imported_avatars += 1;
                }
            }
        }
    }

    tx.commit().map_err(|e| e.to_string())?;

    Ok(VrcxImportResult {
        imported_notes,
        imported_worlds,
        imported_avatars,
    })
}


#[cfg(test)]
mod sql_tests {
    use super::*;
    use std::fs;

    /// 用真实的 `DbState::new` 建库，这样测试跑的是生产环境的建表 + 迁移路径。
    fn temp_db(label: &str) -> (DbState, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "vrcdog-db-{}-{}",
            label,
            uuid::Uuid::new_v4().simple()
        ));
        let state = DbState::new(dir.clone()).expect("open test database");
        (state, dir)
    }

    fn cleanup(dir: &PathBuf) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn re_favorite_world_keeps_added_at_and_existing_image() {
        let (db, dir) = temp_db("fav-world");
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO favorite_worlds (world_id, name, image_url, added_at)
             VALUES ('w1', 'First', 'https://img/1.png', '2020-01-01 00:00:00')",
            [],
        )
        .unwrap();
        // 再次收藏：源没给图片，名字变了
        conn.execute(UPSERT_FAVORITE_WORLD, params!("w1", "Renamed", None::<&str>))
            .unwrap();
        let (name, image, added): (String, Option<String>, String) = conn
            .query_row(
                "SELECT name, image_url, added_at FROM favorite_worlds WHERE world_id = 'w1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        drop(conn);
        assert_eq!(name, "Renamed");
        assert_eq!(
            image.as_deref(),
            Some("https://img/1.png"),
            "导入源缺少图片时不能清空本地已有缩略图"
        );
        assert_eq!(added, "2020-01-01 00:00:00", "重复收藏不能重置收藏时间");
        cleanup(&dir);
    }

    #[test]
    fn imported_avatar_keeps_metadata_and_favorite_time() {
        let (db, dir) = temp_db("fav-avatar");
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO favorite_avatars (avatar_id, name, image_url, author_id, author_name, added_at)
             VALUES ('a1', 'Cool', 'https://img/a.png', 'u_author', 'Author', '2021-06-01 12:00:00')",
            [],
        )
        .unwrap();
        conn.execute(
            UPSERT_IMPORTED_AVATAR,
            params!("a1", "Renamed", None::<&str>, None::<&str>, None::<&str>),
        )
        .unwrap();
        let (image, author, added): (Option<String>, Option<String>, String) = conn
            .query_row(
                "SELECT image_url, author_name, added_at FROM favorite_avatars WHERE avatar_id = 'a1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();

        // 手动收藏路径（字段全非空）同样必须保留 added_at
        conn.execute(
            UPSERT_FAVORITE_AVATAR,
            params!("a1", "Again", "https://img/b.png", "u2", "Other"),
        )
        .unwrap();
        let added_after_manual: String = conn
            .query_row(
                "SELECT added_at FROM favorite_avatars WHERE avatar_id = 'a1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        drop(conn);
        assert_eq!(image.as_deref(), Some("https://img/a.png"));
        assert_eq!(author.as_deref(), Some("Author"));
        assert_eq!(added, "2021-06-01 12:00:00");
        assert_eq!(added_after_manual, "2021-06-01 12:00:00");
        cleanup(&dir);
    }

    #[test]
    fn imported_note_without_display_name_keeps_local_display_name() {
        let (db, dir) = temp_db("note");
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO notes (user_id, display_name, note) VALUES ('u1', 'Alice', 'old')",
            [],
        )
        .unwrap();
        // 导入文件只有 id + 备注，displayName 缺失 -> 空串
        conn.execute(UPSERT_IMPORTED_NOTE, params!("u1", "", "new note"))
            .unwrap();
        let (name, note): (String, String) = conn
            .query_row(
                "SELECT display_name, note FROM notes WHERE user_id = 'u1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        drop(conn);
        assert_eq!(name, "Alice", "导入缺少 displayName 时不能清空本地显示名");
        assert_eq!(note, "new note", "备注本身应当被导入值覆盖");
        cleanup(&dir);
    }

    #[test]
    fn wal_checkpoint_truncates_side_file_so_plain_copy_is_complete() {
        let (db, dir) = temp_db("wal");
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO notes (user_id, display_name, note) VALUES ('u9', 'Nin', 'payload')",
                [],
            )
            .unwrap();
        }
        // 未 checkpoint 时，刚写入的行只存在于 vrcdog.db-wal；
        // 备份只 fs::copy 主文件就会得到一个静默缺数据的备份。
        db.checkpoint_wal().unwrap();
        let wal = dir.join("vrcdog.db-wal");
        let wal_len = fs::metadata(&wal).map(|m| m.len()).unwrap_or(0);
        assert_eq!(wal_len, 0, "TRUNCATE checkpoint 后 -wal 必须为空");
        let main = fs::read(dir.join("vrcdog.db")).unwrap();
        assert!(
            main.windows(7).any(|w| w == b"payload"),
            "checkpoint 后主库文件必须自带最新写入"
        );
        cleanup(&dir);
    }

    #[test]
    fn restore_from_replaces_live_database_instead_of_merging() {
        let (source, src_dir) = temp_db("restore-src");
        let (target, tgt_dir) = temp_db("restore-dst");
        {
            let conn = source.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO notes (user_id, display_name, note) VALUES ('u1', 'Ann', 'hello')",
                [],
            )
            .unwrap();
        }
        source.checkpoint_wal().unwrap();
        {
            let conn = target.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO notes (user_id, display_name, note) VALUES ('u2', 'Old', 'stale')",
                [],
            )
            .unwrap();
        }
        target.checkpoint_wal().unwrap();

        target
            .restore_from(&src_dir.join("vrcdog.db"))
            .expect("restore should succeed");

        let conn = target.conn.lock().unwrap();
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0))
            .unwrap();
        let restored: String = conn
            .query_row(
                "SELECT note FROM notes WHERE user_id = 'u1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let leftover: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM notes WHERE user_id = 'u2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        drop(conn);
        assert_eq!(total, 1, "还原必须是整库替换，不能和旧数据混合");
        assert_eq!(restored, "hello");
        assert_eq!(leftover, 0);
        cleanup(&src_dir);
        cleanup(&tgt_dir);
    }

    #[test]
    fn restore_rejects_a_file_that_is_not_a_database() {
        let (target, tgt_dir) = temp_db("restore-bad");
        let junk = std::env::temp_dir().join(format!(
            "vrcdog-junk-{}.db",
            uuid::Uuid::new_v4().simple()
        ));
        fs::write(&junk, b"this is definitely not a sqlite database file").unwrap();
        assert!(
            target.restore_from(&junk).is_err(),
            "随便一个 .db 后缀的文件必须被拒绝，而不是覆盖在用库上"
        );
        let _ = fs::remove_file(&junk);
        drop(target);
        cleanup(&tgt_dir);
    }

    #[test]
    fn migration_errors_are_classified() {
        assert!(is_benign_migration_error("Duplicate column name: name"));
        assert!(is_benign_migration_error("no such column: label"));
        assert!(is_benign_migration_error("Column label is already in use"));
        assert!(!is_benign_migration_error("database is locked"));
        assert!(!is_benign_migration_error("disk I/O error"));
    }

    #[test]
    fn apply_migration_is_idempotent_but_still_reports_real_failures() {
        let (db, dir) = temp_db("migration");
        let conn = db.conn.lock().unwrap();
        let sql = "ALTER TABLE favorite_worlds ADD COLUMN image_url TEXT";
        assert!(apply_migration(&conn, sql));
        assert!(
            apply_migration(&conn, sql),
            "重复执行同一条迁移必须被当作幂等成功"
        );
        assert!(
            !apply_migration(&conn, "ALTER TABLE no_such_table_xyz ADD COLUMN c TEXT"),
            "真正失败的迁移语句不能被当成「已应用」悄悄跳过"
        );
        drop(conn);
        cleanup(&dir);
    }

    /// 全新安装（空数据目录）必须能完整建库。
    /// 回归 f190405 引入的阻塞性缺陷：`idx_bili_tasks_status` 排在
    /// `CREATE TABLE bili_tasks` 之前，导致首次启动直接失败。
    #[test]
    fn fresh_install_creates_every_table_and_index() {
        let (db, dir) = temp_db("fresh");
        let conn = db.conn.lock().unwrap();
        for table in [
            "auth",
            "friend_activity",
            "notes",
            "status_presets",
            "favorite_worlds",
            "favorite_avatars",
            "friend_log",
            "app_settings",
            "friends",
            "game_log",
            "notifications",
            "api_cache",
            "bili_tasks",
        ] {
            let exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "全新安装必须建出表 {table}");
        }
        for index in [
            "idx_activity_user_date",
            "idx_game_log_unique",
            "idx_bili_tasks_status",
        ] {
            let exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
                    [index],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "索引 {index} 必须存在");
        }
        drop(conn);
        cleanup(&dir);
    }

    /// 老库升级路径：库里已经有一张缺 `date_key` 列的 `friend_activity`，
    /// 索引建在迁移之前会因「no such column: date_key」中断整个初始化。
    #[test]
    fn upgrading_an_old_database_succeeds_and_backfills_columns() {
        let dir = std::env::temp_dir().join(format!(
            "vrcdog-db-old-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&dir).unwrap();
        {
            let legacy = Connection::open(dir.join("vrcdog.db")).unwrap();
            // v5.6 之前的老表结构：没有 date_key
            legacy
                .execute_batch(
                    "CREATE TABLE friend_activity (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        user_id TEXT NOT NULL,
                        display_name TEXT NOT NULL,
                        status TEXT NOT NULL,
                        location TEXT,
                        recorded_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
                     );
                     INSERT INTO friend_activity (user_id, display_name, status)
                        VALUES ('u1', 'Legacy', 'online');",
                )
                .unwrap();
        }

        let db = DbState::new(dir.clone()).expect("老库升级必须成功");
        let conn = db.conn.lock().unwrap();
        let date_key: String = conn
            .query_row(
                "SELECT COALESCE(date_key, '<null>') FROM friend_activity LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(date_key, "", "迁移必须补上 date_key 且保留原有行");
        let index_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_activity_user_date'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(index_exists, 1, "迁移后热点索引必须建起来");
        drop(conn);
        cleanup(&dir);
    }
}
