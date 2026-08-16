use serde::{Deserialize, Serialize};
use std::cmp::Ordering as CmpOrdering;
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Emitter;

const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CANVAS_DIMENSION: u32 = 768;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DrawingPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DrawingStroke {
    pub points: Vec<DrawingPoint>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DrawingConfig {
    pub mode: String,
    pub max_dimension: u32,
    pub threshold: u8,
    pub blur: f32,
    pub invert: bool,
    pub bridge_gaps: bool,
    pub prune_length: usize,
    pub min_stroke_length: usize,
    pub smooth_window: usize,
    pub simplify_epsilon: f32,
    pub merge_distance: f32,
    pub optimize_path: bool,
    pub sensitivity: f32,
    pub vertical_stretch: f32,
    pub max_step_px: f32,
    pub point_delay_ms: u64,
    pub lift_delay_ms: u64,
    pub start_delay_ms: u64,
    pub focus_vrchat: bool,
    pub hotkeys_enabled: bool,
    pub ai_model: String,
    pub contrast: f32,
    pub artifact_removal: f32,
    pub model_size: u32,
    pub lift_speed: f32,
}

impl Default for DrawingConfig {
    fn default() -> Self {
        Self {
            mode: "lineart".into(),
            max_dimension: 512,
            threshold: 150,
            blur: 0.8,
            invert: false,
            bridge_gaps: true,
            prune_length: 4,
            min_stroke_length: 7,
            smooth_window: 3,
            simplify_epsilon: 1.35,
            merge_distance: 3.0,
            optimize_path: true,
            sensitivity: 1.2,
            vertical_stretch: 1.0,
            max_step_px: 4.0,
            point_delay_ms: 28,
            lift_delay_ms: 45,
            start_delay_ms: 1500,
            focus_vrchat: true,
            hotkeys_enabled: true,
            ai_model: "image-to-line".into(),
            contrast: 1.0,
            artifact_removal: 0.6,
            model_size: 512,
            lift_speed: 1.0,
        }
    }
}

impl DrawingConfig {
    fn normalized(mut self) -> Self {
        if !matches!(self.mode.as_str(), "lineart" | "edges" | "dither" | "ai") {
            self.mode = "lineart".into();
        }
        if !matches!(self.ai_model.as_str(), "image-to-line" | "anime2sketch") {
            self.ai_model = "image-to-line".into();
        }
        self.max_dimension = self.max_dimension.clamp(128, MAX_CANVAS_DIMENSION);
        self.blur = finite_or(self.blur, 0.8).clamp(0.0, 8.0);
        self.prune_length = self.prune_length.min(24);
        self.min_stroke_length = self.min_stroke_length.clamp(2, 200);
        self.smooth_window = self.smooth_window.clamp(1, 15);
        self.simplify_epsilon = finite_or(self.simplify_epsilon, 1.35).clamp(0.0, 12.0);
        self.merge_distance = finite_or(self.merge_distance, 3.0).clamp(0.0, 24.0);
        self.sensitivity = finite_or(self.sensitivity, 1.2).clamp(0.1, 8.0);
        self.vertical_stretch = finite_or(self.vertical_stretch, 1.0).clamp(0.25, 3.0);
        self.max_step_px = finite_or(self.max_step_px, 4.0).clamp(1.0, 24.0);
        self.point_delay_ms = self.point_delay_ms.clamp(1, 250);
        self.lift_delay_ms = self.lift_delay_ms.clamp(1, 500);
        self.start_delay_ms = self.start_delay_ms.min(15_000);
        self.contrast = finite_or(self.contrast, 1.0).clamp(0.5, 3.0);
        self.artifact_removal = finite_or(self.artifact_removal, 0.6).clamp(0.0, 1.0);
        self.model_size = self.model_size.clamp(128, 1024);
        self.lift_speed = finite_or(self.lift_speed, 1.0).clamp(0.2, 3.0);
        self
    }
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreparedDrawing {
    pub source_path: String,
    pub width: u32,
    pub height: u32,
    pub strokes: Vec<DrawingStroke>,
    pub total_points: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct DrawingStatus {
    pub prepared: bool,
    pub running: bool,
    pub paused: bool,
    pub progress: f32,
    pub current_stroke: usize,
    pub total_strokes: usize,
    pub total_points: usize,
    pub source_path: String,
    pub last_event: String,
    pub last_error: String,
    pub hotkeys_enabled: bool,
    pub hotkeys_available: bool,
    pub last_hotkey: String,
    pub last_hotkey_at_ms: u64,
    pub stage: String,
}

impl Default for DrawingStatus {
    fn default() -> Self {
        Self {
            prepared: false,
            running: false,
            paused: false,
            progress: 0.0,
            current_stroke: 0,
            total_strokes: 0,
            total_points: 0,
            source_path: String::new(),
            last_event: String::new(),
            last_error: String::new(),
            hotkeys_enabled: true,
            hotkeys_available: cfg!(target_os = "windows"),
            last_hotkey: String::new(),
            last_hotkey_at_ms: 0,
            stage: String::new(),
        }
    }
}

struct DrawingRuntime {
    plan: Option<PreparedDrawing>,
    config: DrawingConfig,
    status: DrawingStatus,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct VrDrawingState {
    inner: Arc<Mutex<DrawingRuntime>>,
}

impl Default for VrDrawingState {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(DrawingRuntime {
                plan: None,
                config: DrawingConfig::default(),
                status: DrawingStatus::default(),
                stop: Arc::new(AtomicBool::new(false)),
                paused: Arc::new(AtomicBool::new(false)),
            })),
        }
    }
}

#[derive(Clone)]
struct DrawingContext {
    app: tauri::AppHandle,
    state: VrDrawingState,
}

static DRAWING_CONTEXT: OnceLock<DrawingContext> = OnceLock::new();
static HOTKEY_MONITOR_STARTED: AtomicBool = AtomicBool::new(false);

pub fn register_runtime(app: tauri::AppHandle, state: VrDrawingState) {
    let _ = DRAWING_CONTEXT.set(DrawingContext { app, state });
    start_hotkey_monitor();
}

#[tauri::command]
pub async fn vrdrawing_prepare(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrDrawingState>,
    source_path: String,
    config: DrawingConfig,
) -> Result<PreparedDrawing, String> {
    let path = validate_source_path(&source_path)?;
    let config = config.normalized();
    let source_path_for_task = path.clone();
    let config_for_task = config.clone();
    let app_for_task = app.clone();
    let state_for_task = state.inner().clone();
    let plan = tauri::async_runtime::spawn_blocking(move || {
        process_image(&app_for_task, &state_for_task, &source_path_for_task, &config_for_task)
    })
    .await
    .map_err(|error| format!("Drawing processor stopped unexpectedly: {error}"))??;

    {
        let mut runtime = state.inner.lock().map_err(|_| "Drawing state is unavailable")?;
        if runtime.status.running {
            return Err("Stop the current drawing before preparing another image".into());
        }
        runtime.config = config;
        runtime.plan = Some(plan.clone());
        runtime.status.prepared = true;
        runtime.status.progress = 0.0;
        runtime.status.current_stroke = 0;
        runtime.status.total_strokes = plan.strokes.len();
        runtime.status.total_points = plan.total_points;
        runtime.status.source_path = plan.source_path.clone();
        runtime.status.last_event = "Drawing plan ready".into();
        runtime.status.last_error.clear();
        runtime.status.hotkeys_enabled = runtime.config.hotkeys_enabled;
    }
    emit_status(&app, &state);
    Ok(plan)
}

#[tauri::command]
pub fn vrdrawing_get_plan(state: tauri::State<'_, VrDrawingState>) -> Result<Option<PreparedDrawing>, String> {
    Ok(state.inner.lock().map_err(|_| "Drawing state is unavailable")?.plan.clone())
}

#[tauri::command]
pub fn vrdrawing_get_status(state: tauri::State<'_, VrDrawingState>) -> Result<DrawingStatus, String> {
    status_snapshot(&state)
}

#[tauri::command]
pub fn vrdrawing_set_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrDrawingState>,
    config: DrawingConfig,
) -> Result<DrawingStatus, String> {
    {
        let mut runtime = state.inner.lock().map_err(|_| "Drawing state is unavailable")?;
        runtime.config = config.normalized();
        runtime.status.hotkeys_enabled = runtime.config.hotkeys_enabled;
    }
    emit_status(&app, &state);
    status_snapshot(&state)
}

#[tauri::command]
pub fn vrdrawing_start(app: tauri::AppHandle, state: tauri::State<'_, VrDrawingState>) -> Result<DrawingStatus, String> {
    start_drawing(&app, &state)?;
    status_snapshot(&state)
}

#[tauri::command]
pub fn vrdrawing_pause(app: tauri::AppHandle, state: tauri::State<'_, VrDrawingState>) -> Result<DrawingStatus, String> {
    set_paused(&app, &state, true)?;
    status_snapshot(&state)
}

#[tauri::command]
pub fn vrdrawing_resume(app: tauri::AppHandle, state: tauri::State<'_, VrDrawingState>) -> Result<DrawingStatus, String> {
    set_paused(&app, &state, false)?;
    status_snapshot(&state)
}

#[tauri::command]
pub fn vrdrawing_stop(app: tauri::AppHandle, state: tauri::State<'_, VrDrawingState>) -> Result<DrawingStatus, String> {
    stop_drawing(&app, &state)?;
    status_snapshot(&state)
}

pub fn handle_vr_action(action: &str) -> Result<(), String> {
    let context = DRAWING_CONTEXT.get().ok_or("Drawing runtime is not initialized")?;
    match action {
        "start" => start_drawing(&context.app, &context.state),
        "toggle_pause" => {
            let paused = context.state.inner.lock().map_err(|_| "Drawing state is unavailable")?.status.paused;
            set_paused(&context.app, &context.state, !paused)
        }
        "stop" => stop_drawing(&context.app, &context.state),
        _ => Err(format!("Unknown drawing action: {action}")),
    }
}

pub fn vr_status_lines() -> (String, String) {
    let Some(context) = DRAWING_CONTEXT.get() else {
        return ("未初始化".into(), "请先打开绘画工具".into());
    };
    let Ok(runtime) = context.state.inner.lock() else {
        return ("状态不可用".into(), String::new());
    };
    let state = if runtime.status.running {
        if runtime.status.paused { "已暂停" } else { "绘制中" }
    } else if runtime.status.prepared {
        "已就绪"
    } else {
        "未载入图片"
    };
    (
        format!("{} · {:.0}%", state, runtime.status.progress * 100.0),
        format!("笔画 {}/{}", runtime.status.current_stroke, runtime.status.total_strokes),
    )
}

fn validate_source_path(raw: &str) -> Result<std::path::PathBuf, String> {
    let path = std::path::PathBuf::from(raw.trim());
    if !path.is_file() {
        return Err("Please select a valid image file".into());
    }
    let size = std::fs::metadata(&path).map_err(|error| format!("Unable to inspect image: {error}"))?.len();
    if size > MAX_SOURCE_BYTES {
        return Err("The selected image is larger than 64 MB".into());
    }
    let extension = path.extension().and_then(|value| value.to_str()).unwrap_or_default().to_ascii_lowercase();
    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif") {
        return Err("Supported image formats: PNG, JPG, WEBP, BMP, GIF".into());
    }
    path.canonicalize().map_err(|error| format!("Unable to resolve image path: {error}"))
}

fn report_stage(state: &VrDrawingState, app: &tauri::AppHandle, stage: &str, progress: f32) {
    if let Ok(mut runtime) = state.inner.lock() {
        runtime.status.stage = stage.to_string();
        runtime.status.progress = progress;
        runtime.status.last_event = format!("Processing: {stage}");
    }
    emit_status(app, state);
}

fn process_image(app: &tauri::AppHandle, state: &VrDrawingState, path: &Path, config: &DrawingConfig) -> Result<PreparedDrawing, String> {
    report_stage(state, app, "decode", 0.05);
    let image = image::open(path).map_err(|error| format!("Unable to decode image: {error}"))?;
    let dimension = if config.mode == "ai" { config.model_size } else { config.max_dimension };
    let resized = image.resize(dimension, dimension, image::imageops::FilterType::Triangle);
    let mut gray = resized.to_luma8();
    if config.blur > 0.01 {
        gray = image::imageops::blur(&gray, config.blur);
    }
    let (width, height) = gray.dimensions();
    if width < 3 || height < 3 {
        return Err("The image is too small to create a drawing".into());
    }
    let raw = gray.into_raw();
    report_stage(state, app, "binarize", 0.3);
    let mut binary = match config.mode.as_str() {
        "edges" => sobel_edges(&raw, width as usize, height as usize, config.threshold, config.invert),
        "dither" => floyd_steinberg(&raw, width as usize, height as usize, config.threshold, config.invert),
        "ai" => ai_line_art(&raw, width as usize, height as usize, config),
        _ => threshold_image(&raw, config.threshold, config.invert),
    };
    if config.bridge_gaps {
        binary = erode(&dilate(&binary, width as usize, height as usize), width as usize, height as usize);
    }
    report_stage(state, app, "skeletonize", 0.6);
    skeletonize(&mut binary, width as usize, height as usize);
    if config.prune_length > 0 {
        prune_short_branches(&mut binary, width as usize, height as usize, config.prune_length);
    }
    report_stage(state, app, "extract", 0.8);
    let mut strokes = extract_strokes(&binary, width as usize, height as usize, config.min_stroke_length);
    for stroke in &mut strokes {
        if config.smooth_window > 1 {
            stroke.points = smooth_points(&stroke.points, config.smooth_window);
        }
        if config.simplify_epsilon > 0.0 {
            stroke.points = simplify_points(&stroke.points, config.simplify_epsilon);
        }
    }
    strokes.retain(|stroke| stroke.points.len() >= 2);
    if config.merge_distance > 0.0 && strokes.len() <= 2500 {
        strokes = merge_nearby_strokes(strokes, config.merge_distance);
    }
    report_stage(state, app, "optimize", 0.95);
    if config.optimize_path {
        strokes = order_strokes(strokes);
    }
    let total_points = strokes.iter().map(|stroke| stroke.points.len()).sum();
    if strokes.is_empty() || total_points < 2 {
        return Err("No drawable lines were found. Adjust threshold, contrast or invert the image".into());
    }
    let plan = PreparedDrawing {
        source_path: path.to_string_lossy().to_string(),
        width,
        height,
        strokes,
        total_points,
    };
    report_stage(state, app, "ready", 1.0);
    Ok(plan)
}

fn sobel_magnitude(raw: &[u8], width: usize, height: usize) -> Vec<f32> {
    let mut result = vec![0.0f32; raw.len()];
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let idx = |dx: isize, dy: isize| raw[((y as isize + dy) as usize) * width + (x as isize + dx) as usize] as f32;
            let gx = -idx(-1, -1) + idx(1, -1) - 2.0 * idx(-1, 0) + 2.0 * idx(1, 0) - idx(-1, 1) + idx(1, 1);
            let gy = -idx(-1, -1) - 2.0 * idx(0, -1) - idx(1, -1) + idx(-1, 1) + 2.0 * idx(0, 1) + idx(1, 1);
            result[y * width + x] = gx.hypot(gy);
        }
    }
    result
}

fn otsu_threshold(data: &[u8]) -> u8 {
    let mut histogram = [0usize; 256];
    for &value in data {
        histogram[value as usize] += 1;
    }
    let total = data.len() as f32;
    if total == 0.0 {
        return 128;
    }
    let mut sum = 0.0f32;
    for (index, &count) in histogram.iter().enumerate() {
        sum += index as f32 * count as f32;
    }
    let mut sum_background = 0.0f32;
    let mut weight_background = 0.0f32;
    let mut max_variance = 0.0f32;
    let mut threshold = 127u8;
    for index in 0..256 {
        weight_background += histogram[index] as f32;
        if weight_background == 0.0 {
            continue;
        }
        let weight_foreground = total - weight_background;
        if weight_foreground == 0.0 {
            break;
        }
        sum_background += index as f32 * histogram[index] as f32;
        let mean_background = sum_background / weight_background;
        let mean_foreground = (sum - sum_background) / weight_foreground;
        let between = weight_background * weight_foreground * (mean_background - mean_foreground) * (mean_background - mean_foreground);
        if between > max_variance {
            max_variance = between;
            threshold = index as u8;
        }
    }
    threshold
}

/// CPU approximation of a learned line-art model (image-to-line / Anime2Sketch):
/// contrast stretch + adaptive (Otsu) edge detection + morphological opening to suppress
/// scan artifacts. Replace this branch with a real ONNX/PyTorch inference call to use an
/// actual model; the surrounding pipeline (skeletonize / extract / optimize / draw) is unchanged.
fn ai_line_art(raw: &[u8], width: usize, height: usize, config: &DrawingConfig) -> Vec<u8> {
    let contrasted: Vec<u8> = raw.iter().map(|value| {
        let adjusted = (*value as f32 - 128.0) * config.contrast + 128.0;
        adjusted.clamp(0.0, 255.0) as u8
    }).collect();
    let threshold = otsu_threshold(&contrasted);
    let magnitude = sobel_magnitude(&contrasted, width, height);
    let limit = if config.ai_model == "anime2sketch" {
        (threshold as f32 / 255.0 * 540.0).max(18.0)
    } else {
        (threshold as f32 / 255.0 * 720.0).max(24.0)
    };
    let mut edges = vec![0u8; magnitude.len()];
    for (index, value) in magnitude.iter().enumerate() {
        edges[index] = u8::from(*value >= limit);
    }
    if config.invert {
        for pixel in edges.iter_mut() {
            *pixel = u8::from(*pixel == 0);
        }
    }
    let iterations = (config.artifact_removal * 3.0).round().clamp(0.0, 4.0) as usize;
    let mut cleaned = edges;
    for _ in 0..iterations {
        cleaned = erode(&cleaned, width, height);
    }
    for _ in 0..iterations {
        cleaned = dilate(&cleaned, width, height);
    }
    cleaned
}

fn threshold_image(raw: &[u8], threshold: u8, invert: bool) -> Vec<u8> {
    raw.iter().map(|value| u8::from(if invert { *value > threshold } else { *value < threshold })).collect()
}

fn sobel_edges(raw: &[u8], width: usize, height: usize, threshold: u8, invert: bool) -> Vec<u8> {
    let mut result = vec![0; raw.len()];
    let limit = (threshold as f32 / 255.0 * 720.0).max(24.0);
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let idx = |dx: isize, dy: isize| raw[((y as isize + dy) as usize) * width + (x as isize + dx) as usize] as f32;
            let gx = -idx(-1, -1) + idx(1, -1) - 2.0 * idx(-1, 0) + 2.0 * idx(1, 0) - idx(-1, 1) + idx(1, 1);
            let gy = -idx(-1, -1) - 2.0 * idx(0, -1) - idx(1, -1) + idx(-1, 1) + 2.0 * idx(0, 1) + idx(1, 1);
            let edge = gx.hypot(gy) >= limit;
            result[y * width + x] = u8::from(if invert { !edge } else { edge });
        }
    }
    result
}

fn floyd_steinberg(raw: &[u8], width: usize, height: usize, threshold: u8, invert: bool) -> Vec<u8> {
    let mut work: Vec<f32> = raw.iter().map(|value| *value as f32).collect();
    let mut result = vec![0; raw.len()];
    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            let white = work[index] >= threshold as f32;
            let next = if white { 255.0 } else { 0.0 };
            let mark = if invert { white } else { !white };
            result[index] = u8::from(mark);
            let error = work[index] - next;
            for (dx, dy, weight) in [(1isize, 0isize, 7.0), (-1, 1, 3.0), (0, 1, 5.0), (1, 1, 1.0)] {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if nx >= 0 && nx < width as isize && ny < height as isize {
                    let target = ny as usize * width + nx as usize;
                    work[target] = (work[target] + error * weight / 16.0).clamp(0.0, 255.0);
                }
            }
        }
    }
    result
}

fn dilate(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut result = data.to_vec();
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            if data[y * width + x] == 0 && neighbors(data, width, height, x, y).iter().any(|&index| data[index] != 0) {
                result[y * width + x] = 1;
            }
        }
    }
    result
}

fn erode(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut result = data.to_vec();
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            if data[y * width + x] != 0 && neighbors(data, width, height, x, y).iter().any(|&index| data[index] == 0) {
                result[y * width + x] = 0;
            }
        }
    }
    result
}

fn neighbors(data: &[u8], width: usize, height: usize, x: usize, y: usize) -> Vec<usize> {
    let mut result = Vec::with_capacity(8);
    for dy in -1isize..=1 {
        for dx in -1isize..=1 {
            if dx == 0 && dy == 0 { continue; }
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if nx >= 0 && ny >= 0 && nx < width as isize && ny < height as isize {
                let index = ny as usize * width + nx as usize;
                if data[index] != 0 { result.push(index); }
            }
        }
    }
    result
}

fn skeletonize(data: &mut [u8], width: usize, height: usize) {
    let mut changed = true;
    let mut remove = Vec::new();
    while changed {
        changed = false;
        for phase in 0..2 {
            for y in 1..height - 1 {
                for x in 1..width - 1 {
                    let index = y * width + x;
                    if data[index] == 0 { continue; }
                    let p = [data[(y - 1) * width + x], data[(y - 1) * width + x + 1], data[y * width + x + 1], data[(y + 1) * width + x + 1], data[(y + 1) * width + x], data[(y + 1) * width + x - 1], data[y * width + x - 1], data[(y - 1) * width + x - 1]];
                    let transitions = (0..8).filter(|&i| p[i] == 0 && p[(i + 1) % 8] != 0).count();
                    let count: u8 = p.iter().sum();
                    let removable = transitions == 1 && (2..=6).contains(&count) && if phase == 0 {
                        p[0] * p[2] * p[4] == 0 && p[2] * p[4] * p[6] == 0
                    } else {
                        p[0] * p[2] * p[6] == 0 && p[0] * p[4] * p[6] == 0
                    };
                    if removable { remove.push(index); }
                }
            }
            if !remove.is_empty() {
                changed = true;
                for index in remove.drain(..) { data[index] = 0; }
            }
        }
    }
}

fn prune_short_branches(data: &mut [u8], width: usize, height: usize, min_length: usize) {
    for _ in 0..min_length {
        let endpoints: Vec<usize> = (0..data.len()).filter(|&index| {
            if data[index] == 0 { return false; }
            neighbors(data, width, height, index % width, index / width).len() <= 1
        }).collect();
        if endpoints.is_empty() { break; }
        for index in endpoints { data[index] = 0; }
    }
}

fn extract_strokes(data: &[u8], width: usize, height: usize, min_length: usize) -> Vec<DrawingStroke> {
    let mut visited = vec![false; data.len()];
    let mut starts: Vec<usize> = (0..data.len()).filter(|&index| data[index] != 0 && neighbors(data, width, height, index % width, index / width).len() == 1).collect();
    let endpoint_set: HashSet<usize> = starts.iter().copied().collect();
    starts.extend((0..data.len()).filter(|&index| data[index] != 0 && !endpoint_set.contains(&index)));
    let mut strokes = Vec::new();
    for start in starts {
        if visited[start] || data[start] == 0 { continue; }
        let mut points = Vec::new();
        let mut current = start;
        let mut previous = None;
        loop {
            if visited[current] { break; }
            visited[current] = true;
            points.push(DrawingPoint { x: (current % width) as f32, y: (current / width) as f32 });
            let candidates: Vec<usize> = neighbors(data, width, height, current % width, current / width).into_iter().filter(|index| !visited[*index]).collect();
            if candidates.is_empty() { break; }
            let next = if let Some(previous) = previous {
                let px = (current % width) as f32 - (previous % width) as f32;
                let py = (current / width) as f32 - (previous / width) as f32;
                *candidates.iter().max_by(|a, b| {
                    let score = |index: usize| {
                        let dx = (index % width) as f32 - (current % width) as f32;
                        let dy = (index / width) as f32 - (current / width) as f32;
                        (px * dx + py * dy) / (px.hypot(py) * dx.hypot(dy)).max(0.001)
                    };
                    score(**a).partial_cmp(&score(**b)).unwrap_or(CmpOrdering::Equal)
                }).unwrap()
            } else { candidates[0] };
            previous = Some(current);
            current = next;
        }
        if points.len() >= min_length { strokes.push(DrawingStroke { points }); }
    }
    strokes
}

fn smooth_points(points: &[DrawingPoint], window: usize) -> Vec<DrawingPoint> {
    if points.len() <= 2 || window <= 1 { return points.to_vec(); }
    let radius = window / 2;
    points.iter().enumerate().map(|(index, _)| {
        if index == 0 || index + 1 == points.len() { return points[index].clone(); }
        let start = index.saturating_sub(radius);
        let end = (index + radius + 1).min(points.len());
        let count = (end - start) as f32;
        DrawingPoint {
            x: points[start..end].iter().map(|point| point.x).sum::<f32>() / count,
            y: points[start..end].iter().map(|point| point.y).sum::<f32>() / count,
        }
    }).collect()
}

fn simplify_points(points: &[DrawingPoint], epsilon: f32) -> Vec<DrawingPoint> {
    if points.len() <= 2 { return points.to_vec(); }
    let start = &points[0];
    let end = &points[points.len() - 1];
    let mut max_distance = 0.0;
    let mut max_index = 0;
    for (index, point) in points.iter().enumerate().take(points.len() - 1).skip(1) {
        let distance = point_line_distance(point, start, end);
        if distance > max_distance { max_distance = distance; max_index = index; }
    }
    if max_distance <= epsilon { return vec![start.clone(), end.clone()]; }
    let mut left = simplify_points(&points[..=max_index], epsilon);
    let right = simplify_points(&points[max_index..], epsilon);
    left.pop();
    left.extend(right);
    left
}

fn point_line_distance(point: &DrawingPoint, start: &DrawingPoint, end: &DrawingPoint) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    if dx.abs() + dy.abs() < f32::EPSILON { return (point.x - start.x).hypot(point.y - start.y); }
    ((dy * point.x - dx * point.y + end.x * start.y - end.y * start.x).abs()) / dx.hypot(dy)
}

fn point_distance(a: &DrawingPoint, b: &DrawingPoint) -> f32 { (a.x - b.x).hypot(a.y - b.y) }

fn merge_nearby_strokes(mut strokes: Vec<DrawingStroke>, distance: f32) -> Vec<DrawingStroke> {
    let mut changed = true;
    while changed {
        changed = false;
        'outer: for left in 0..strokes.len() {
            for right in left + 1..strokes.len() {
                let a_start = strokes[left].points.first().unwrap();
                let a_end = strokes[left].points.last().unwrap();
                let b_start = strokes[right].points.first().unwrap();
                let b_end = strokes[right].points.last().unwrap();
                let options = [(point_distance(a_end, b_start), false, false), (point_distance(a_end, b_end), false, true), (point_distance(a_start, b_end), true, false), (point_distance(a_start, b_start), true, true)];
                let Some((_, reverse_a, reverse_b)) = options.into_iter().filter(|option| option.0 <= distance).min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(CmpOrdering::Equal)) else { continue; };
                let mut b = strokes.remove(right).points;
                if reverse_a { strokes[left].points.reverse(); }
                if reverse_b { b.reverse(); }
                strokes[left].points.extend(b);
                changed = true;
                break 'outer;
            }
        }
    }
    strokes
}

fn order_strokes(mut strokes: Vec<DrawingStroke>) -> Vec<DrawingStroke> {
    if strokes.len() <= 1 { return strokes; }
    let first = strokes.iter().enumerate().min_by(|(_, a), (_, b)| {
        let da = a.points[0].x.hypot(a.points[0].y);
        let db = b.points[0].x.hypot(b.points[0].y);
        da.partial_cmp(&db).unwrap_or(CmpOrdering::Equal)
    }).map(|(index, _)| index).unwrap_or(0);
    let mut ordered = vec![strokes.remove(first)];
    while !strokes.is_empty() {
        let current = ordered.last().unwrap().points.last().unwrap();
        let mut best = (f32::MAX, 0usize, false);
        for (index, stroke) in strokes.iter().enumerate() {
            for (distance, reverse) in [(point_distance(current, stroke.points.first().unwrap()), false), (point_distance(current, stroke.points.last().unwrap()), true)] {
                if distance < best.0 { best = (distance, index, reverse); }
            }
        }
        let mut next = strokes.remove(best.1);
        if best.2 { next.points.reverse(); }
        ordered.push(next);
    }
    ordered
}

fn start_drawing(app: &tauri::AppHandle, state: &VrDrawingState) -> Result<(), String> {
    let (plan, config, stop, paused) = {
        let mut runtime = state.inner.lock().map_err(|_| "Drawing state is unavailable")?;
        if runtime.status.running { return Err("A drawing is already running".into()); }
        let plan = runtime.plan.clone().ok_or("Prepare an image before drawing")?;
        runtime.stop = Arc::new(AtomicBool::new(false));
        runtime.paused = Arc::new(AtomicBool::new(false));
        runtime.status.running = true;
        runtime.status.paused = false;
        runtime.status.progress = 0.0;
        runtime.status.current_stroke = 0;
        runtime.status.last_event = "Drawing countdown started".into();
        runtime.status.last_error.clear();
        (plan, runtime.config.clone(), runtime.stop.clone(), runtime.paused.clone())
    };
    emit_status(app, state);
    let app = app.clone();
    let state = state.clone();
    thread::spawn(move || run_drawing(app, state, plan, config, stop, paused));
    Ok(())
}

fn set_paused(app: &tauri::AppHandle, state: &VrDrawingState, value: bool) -> Result<(), String> {
    {
        let mut runtime = state.inner.lock().map_err(|_| "Drawing state is unavailable")?;
        if !runtime.status.running { return Err("No drawing is currently running".into()); }
        runtime.paused.store(value, Ordering::SeqCst);
        runtime.status.paused = value;
        runtime.status.last_event = if value { "Drawing paused" } else { "Drawing resumed" }.into();
    }
    if value { mouse_left(false); }
    emit_status(app, state);
    Ok(())
}

fn stop_drawing(app: &tauri::AppHandle, state: &VrDrawingState) -> Result<(), String> {
    {
        let mut runtime = state.inner.lock().map_err(|_| "Drawing state is unavailable")?;
        runtime.stop.store(true, Ordering::SeqCst);
        runtime.paused.store(false, Ordering::SeqCst);
        runtime.status.running = false;
        runtime.status.paused = false;
        runtime.status.last_event = "Drawing stopped".into();
    }
    mouse_left(false);
    emit_status(app, state);
    Ok(())
}

fn run_drawing(app: tauri::AppHandle, state: VrDrawingState, plan: PreparedDrawing, config: DrawingConfig, stop: Arc<AtomicBool>, paused: Arc<AtomicBool>) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), String> {
        if config.focus_vrchat { focus_vrchat_window()?; }
        interruptible_sleep(config.start_delay_ms, &stop, &paused);
        if stop.load(Ordering::SeqCst) { return Ok(()); }
        let (start_x, start_y) = cursor_position()?;
        let center_x = plan.width as f32 / 2.0;
        let center_y = plan.height as f32 / 2.0;
        let mut current_x = center_x;
        let mut current_y = center_y;
        let mut error_x = 0.0f32;
        let mut error_y = 0.0f32;
        for (index, stroke) in plan.strokes.iter().enumerate() {
            if stop.load(Ordering::SeqCst) { break; }
            wait_while_paused(&stop, &paused);
            if stop.load(Ordering::SeqCst) { break; }
            mouse_left(false);
            let first = &stroke.points[0];
            move_planar(&mut current_x, &mut current_y, first, &config, true, &stop, &paused, &mut error_x, &mut error_y);
            if stop.load(Ordering::SeqCst) { break; }
            mouse_left(true);
            for point in stroke.points.iter().skip(1) {
                move_planar(&mut current_x, &mut current_y, point, &config, false, &stop, &paused, &mut error_x, &mut error_y);
                if stop.load(Ordering::SeqCst) { break; }
            }
            mouse_left(false);
            interruptible_sleep(config.lift_delay_ms, &stop, &paused);
            update_status(&state, |status| {
                status.current_stroke = index + 1;
                status.progress = (index + 1) as f32 / plan.strokes.len().max(1) as f32;
                status.last_event = "Drawing in progress".into();
            });
            emit_status(&app, &state);
        }
        let target_dx = start_x - cursor_position()?.0;
        let target_dy = start_y - cursor_position()?.1;
        mouse_move(target_dx, target_dy);
        Ok(())
    }));
    mouse_left(false);
    let error = match result { Ok(Ok(())) => None, Ok(Err(error)) => Some(error), Err(_) => Some("Drawing worker crashed unexpectedly".into()) };
    update_status(&state, |status| {
        status.running = false;
        status.paused = false;
        if let Some(error) = error {
            status.last_error = error;
            status.last_event = "Drawing failed".into();
        } else if stop.load(Ordering::SeqCst) {
            status.last_event = "Drawing stopped".into();
        } else {
            status.progress = 1.0;
            status.current_stroke = status.total_strokes;
            status.last_event = "Drawing completed".into();
        }
    });
    emit_status(&app, &state);
}

#[allow(clippy::too_many_arguments)]
fn move_planar(current_x: &mut f32, current_y: &mut f32, target: &DrawingPoint, config: &DrawingConfig, pen_up: bool, stop: &AtomicBool, paused: &AtomicBool, error_x: &mut f32, error_y: &mut f32) {
    let pen_speed = if pen_up { config.lift_speed } else { 1.0 };
    let delta_x = (target.x - *current_x) * config.sensitivity * pen_speed;
    let delta_y = (target.y - *current_y) * config.sensitivity * config.vertical_stretch * pen_speed;
    let distance = delta_x.hypot(delta_y);
    let steps = (distance / config.max_step_px).ceil().max(1.0) as usize;
    for _ in 0..steps {
        wait_while_paused(stop, paused);
        if stop.load(Ordering::SeqCst) { break; }
        let ideal_x = delta_x / steps as f32 + *error_x;
        let ideal_y = delta_y / steps as f32 + *error_y;
        let dx = ideal_x.round() as i32;
        let dy = ideal_y.round() as i32;
        *error_x = ideal_x - dx as f32;
        *error_y = ideal_y - dy as f32;
        mouse_move(dx, dy);
        if !pen_up { thread::sleep(Duration::from_millis(config.point_delay_ms)); }
    }
    *current_x = target.x;
    *current_y = target.y;
}

fn wait_while_paused(stop: &AtomicBool, paused: &AtomicBool) {
    while paused.load(Ordering::SeqCst) && !stop.load(Ordering::SeqCst) {
        mouse_left(false);
        thread::sleep(Duration::from_millis(25));
    }
}

fn interruptible_sleep(duration_ms: u64, stop: &AtomicBool, paused: &AtomicBool) {
    let mut remaining = duration_ms;
    while remaining > 0 && !stop.load(Ordering::SeqCst) {
        wait_while_paused(stop, paused);
        let chunk = remaining.min(25);
        thread::sleep(Duration::from_millis(chunk));
        remaining -= chunk;
    }
}

fn status_snapshot(state: &VrDrawingState) -> Result<DrawingStatus, String> {
    let runtime = state.inner.lock().map_err(|_| "Drawing state is unavailable")?;
    let mut status = runtime.status.clone();
    status.paused = runtime.paused.load(Ordering::SeqCst) && status.running;
    Ok(status)
}

fn update_status(state: &VrDrawingState, update: impl FnOnce(&mut DrawingStatus)) {
    if let Ok(mut runtime) = state.inner.lock() { update(&mut runtime.status); }
}

fn emit_status(app: &tauri::AppHandle, state: &VrDrawingState) {
    if let Ok(status) = status_snapshot(state) { let _ = app.emit("vrdrawing_status", status); }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

#[cfg(target_os = "windows")]
fn start_hotkey_monitor() {
    if HOTKEY_MONITOR_STARTED.swap(true, Ordering::SeqCst) { return; }
    thread::spawn(|| {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        let keys = [(0x78i32, "F9", "start"), (0x79, "F10", "stop"), (0x7A, "F11", "toggle_pause")];
        let mut previous = HashSet::new();
        loop {
            if let Some(context) = DRAWING_CONTEXT.get() {
                let enabled = context.state.inner.lock().map(|runtime| runtime.config.hotkeys_enabled).unwrap_or(false);
                if enabled {
                    for (key, label, action) in keys {
                        let down = unsafe { GetAsyncKeyState(key) } < 0;
                        if down && !previous.contains(&key) {
                            let _ = handle_vr_action(action);
                            update_status(&context.state, |status| {
                                status.last_hotkey = label.into();
                                status.last_hotkey_at_ms = now_ms();
                            });
                            emit_status(&context.app, &context.state);
                        }
                        if down { previous.insert(key); } else { previous.remove(&key); }
                    }
                } else { previous.clear(); }
            }
            thread::sleep(Duration::from_millis(35));
        }
    });
}

#[cfg(not(target_os = "windows"))]
fn start_hotkey_monitor() {}

#[cfg(target_os = "windows")]
fn mouse_move(dx: i32, dy: i32) {
    if dx == 0 && dy == 0 { return; }
    use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEINPUT};
    let input = INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi: MOUSEINPUT { dx, dy, dwFlags: MOUSEEVENTF_MOVE, ..Default::default() } } };
    unsafe { let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32); }
}

#[cfg(not(target_os = "windows"))]
fn mouse_move(_dx: i32, _dy: i32) {}

#[cfg(target_os = "windows")]
fn mouse_left(down: bool) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT};
    let input = INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi: MOUSEINPUT { dwFlags: if down { MOUSEEVENTF_LEFTDOWN } else { MOUSEEVENTF_LEFTUP }, ..Default::default() } } };
    unsafe { let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32); }
}

#[cfg(not(target_os = "windows"))]
fn mouse_left(_down: bool) {}

#[cfg(target_os = "windows")]
fn cursor_position() -> Result<(i32, i32), String> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point).map_err(|error| format!("Unable to read cursor position: {error}"))?; }
    Ok((point.x, point.y))
}

#[cfg(not(target_os = "windows"))]
fn cursor_position() -> Result<(i32, i32), String> { Err("Automatic drawing is currently supported on Windows".into()) }

#[cfg(target_os = "windows")]
fn focus_vrchat_window() -> Result<(), String> {
    use windows::core::w;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE};
    let window = unsafe { FindWindowW(None, w!("VRChat")) }.map_err(|_| "VRChat window was not found")?;
    unsafe {
        if IsIconic(window).as_bool() { let _ = ShowWindow(window, SW_RESTORE); }
        if !SetForegroundWindow(window).as_bool() { return Err("VRChat could not be focused. Focus it manually and try again".into()); }
    }
    thread::sleep(Duration::from_millis(350));
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn focus_vrchat_window() -> Result<(), String> { Err("Automatic drawing is currently supported on Windows".into()) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simplifies_a_straight_line_to_its_endpoints() {
        let points = (0..10).map(|x| DrawingPoint { x: x as f32, y: 2.0 }).collect::<Vec<_>>();
        let result = simplify_points(&points, 0.5);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].x, 0.0);
        assert_eq!(result[1].x, 9.0);
    }

    #[test]
    fn path_order_can_reverse_the_nearest_stroke() {
        let strokes = vec![
            DrawingStroke { points: vec![DrawingPoint { x: 0.0, y: 0.0 }, DrawingPoint { x: 10.0, y: 0.0 }] },
            DrawingStroke { points: vec![DrawingPoint { x: 30.0, y: 0.0 }, DrawingPoint { x: 12.0, y: 0.0 }] },
        ];
        let ordered = order_strokes(strokes);
        assert_eq!(ordered[1].points[0].x, 12.0);
    }

    #[test]
    fn normalizes_unsafe_drawing_values() {
        let config = DrawingConfig { sensitivity: f32::NAN, max_dimension: 4000, point_delay_ms: 0, ..Default::default() }.normalized();
        assert_eq!(config.sensitivity, 1.2);
        assert_eq!(config.max_dimension, MAX_CANVAS_DIMENSION);
        assert_eq!(config.point_delay_ms, 1);
    }
}
