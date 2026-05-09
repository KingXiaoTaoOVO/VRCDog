# 0001. Bilidown Native Rust Integration

## Context
We need to port `bilidown` (a Bilibili DASH video parser and downloader written in Go) into the `VrcDog` Tauri application. The original application relied on a local Go HTTP server with an isolated SQLite DB and client-side polling.

## Decision
We will completely discard the Go backend and rewrite the core downloading, tracking, and merging logic in Rust.
1. **No Go Sidecar**: We will implement API calls using `reqwest` and track progress using Rust futures and asynchronous events.
2. **Event-Driven Progress**: Instead of polling `/api/getActiveTask` every second, the Rust backend will push global `bili_task_progress` events to Vue via Tauri `app_handle.emit`.
3. **Database Unification**: Tasks will be stored in a new `bili_tasks` table inside `vrcdog.db` instead of a standalone `bilidown.db`.
4. **Environment Dependency**: FFmpeg will not be bundled. We assume the user has `ffmpeg` in their system `PATH`. If missing, the download task will immediately fail with a descriptive UI error.

## Consequences
- **Pros**: Reduced payload size (no Go binary), lower memory overhead, no port collisions, unified UI/UX, and instant real-time progress feedback.
- **Cons**: We must maintain a complex concurrency logic (semaphore locks for downloads and merges) and shell-out process parsing (reading FFmpeg stdout) natively in Rust. Users without `ffmpeg` installed will be unable to merge audio/video tracks.
