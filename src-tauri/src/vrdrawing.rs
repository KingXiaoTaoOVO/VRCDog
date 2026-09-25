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
    /// Extra wait after `mouse_left(true)` before the first move step. Lets the canvas
    /// commit the pen-down state so the first point is not lost or offset by a half-step.
    pub pen_settle_ms: u64,
    /// Optional explicit canvas size in screen pixels. When > 0 the engine computes an
    /// automatic scale = canvas_size_px / max(plan.width, plan.height) so the image is
    /// mapped 1:1 to the real VRChat drawing canvas. 0 keeps the legacy sensitivity-only
    /// behavior for backwards compatibility.
    pub canvas_size_px: u32,
    /// When true, the engine attempts a lightweight 2-opt pass after nearest-neighbour
    /// ordering. Roughly halves aerial pen travel on complex images; cheap relative to the
    /// overall drawing time.
    pub two_opt_path: bool,
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
            prune_length: 3,
            min_stroke_length: 6,
            smooth_window: 7,
            simplify_epsilon: 1.6,
            merge_distance: 6.0,
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
            pen_settle_ms: 15,
            canvas_size_px: 0,
            two_opt_path: true,
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
        self.pen_settle_ms = self.pen_settle_ms.min(300);
        self.canvas_size_px = self.canvas_size_px.min(8192);
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

/// Builds the binary ink mask for the selected mode.
/// Builds the binary ink mask for the selected mode.
fn build_ink_mask(raw: &[u8], width: usize, height: usize, config: &DrawingConfig) -> Vec<u8> {
    // `threshold` doubles as a sensitivity control: higher means fewer, stronger
    // lines. The 150 default maps to a scale of 1.0.
    let scale = (config.threshold as f32 / 150.0).clamp(0.3, 3.0);
    if config.mode == "dither" {
        return floyd_steinberg(raw, width, height, config.threshold, config.invert);
    }
    if config.mode == "edges" {
        return canny_edges(raw, width, height, scale, 0.4);
    }
    let stats = image_stats(raw);
    let pre = edge_preserving(raw, width, height, &stats);
    if config.mode == "ai" {
        // Strongest texture suppression: the DoG line map alone.
        let contrasted: Vec<u8> = pre.iter().map(|value| {
            let adjusted = (*value as f32 - 128.0) * config.contrast + 128.0;
            adjusted.clamp(0.0, 255.0) as u8
        }).collect();
        return line_map(&contrasted, width, height);
    }
    // `lineart` auto-routes on the image itself.
    if is_line_art(raw, width, height) {
        // A drawing already IS a set of thin dark strokes: thresholding keeps it
        // exactly, and an edge detector would only add a second, offset line.
        return adaptive_ink(&pre, width, height, (width.min(height) / 24).max(6), 16.0, config.invert);
    }
    // Photo / shaded illustration. XDoG gives clean coherent lines and suppresses
    // the surface texture and shading gradients that a threshold would ink in;
    // Canny adds back the fine interior detail (eyes, lettering, vents) that XDoG
    // alone drops. Their union beats either one on its own.
    let mut mask = line_map(&pre, width, height);
    let detail = canny_edges(&pre, width, height, scale, 0.4);
    for (slot, value) in mask.iter_mut().zip(detail.iter()) {
        *slot |= *value;
    }
    let outline = silhouette(&pre, width, height, stats.bright_ratio, 6);
    for (slot, value) in mask.iter_mut().zip(outline.iter()) {
        *slot |= *value;
    }
    mask
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
    let (w, h) = (width as usize, height as usize);
    let mut binary = build_ink_mask(&raw, w, h, config);
    if config.bridge_gaps {
        // Plus-shaped closing: bridges one-pixel gaps without growing lines
        // diagonally and gluing neighbours together.
        binary = erode_plus(&dilate_plus(&binary, w, h), w, h);
    }
    despeckle(&mut binary, w, h, config.artifact_removal);
    report_stage(state, app, "skeletonize", 0.6);
    let mut strokes = if config.mode == "dither" {
        // Dithering means dots, not centre lines: thinning would erase them all.
        dots(&binary, w, h)
    } else {
        skeletonize(&mut binary, w, h);
        if config.prune_length > 0 {
            prune_spurs(&mut binary, w, h, config.prune_length);
        }
        report_stage(state, app, "extract", 0.8);
        extract_strokes(&binary, w, h, config.min_stroke_length)
    };
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
        // The gap tolerance scales with resolution so the result is the same at
        // any processing size.
        let resolution = (w.max(h) as f32 / 512.0).max(0.5);
        strokes = merge_nearby_strokes(strokes, config.merge_distance * resolution);
    }
    report_stage(state, app, "optimize", 0.95);
    if config.optimize_path {
        strokes = order_strokes(strokes);
    } else if config.two_opt_path {
        // Even without nearest-neighbour ordering, a 2-opt pass over the raw extract order
        // can noticeably reduce aerial travel on complex images.
        strokes = two_opt_pass(strokes, 2);
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

/// Drops tiny connected components. The limit scales with `artifact_removal` but
/// stays small by default: the old 3..31 px range erased eyes, text and other
/// genuine detail along with the noise.
/// Image-level statistics used to auto-pick processing parameters.
struct ImageStats {
    bright_ratio: f32,
}

fn image_stats(raw: &[u8]) -> ImageStats {
    let mut histogram = [0usize; 256];
    for &value in raw {
        histogram[value as usize] += 1;
    }
    let total = raw.len().max(1) as f32;
    let bright: usize = histogram[240..].iter().sum();
    ImageStats { bright_ratio: bright as f32 / total }
}

/// Edge-preserving smoothing with parameters derived from the image.
///
/// Removes sensor noise and surface texture (fabric, reflections, skin) without
/// softening real edges. This is what lets the detectors below ignore photo
/// texture while keeping the object structure -- the single biggest reason the
/// old output was full of lines that are not in the picture.
fn edge_preserving(raw: &[u8], width: usize, height: usize, stats: &ImageStats) -> Vec<u8> {
    let scale = (width.max(height) as f32 / 512.0).max(0.5);
    let radius = ((3.5 * scale).round() as usize).clamp(2, 5);
    let sigma_color = (14.0 + (1.0 - stats.bright_ratio) * 45.0).clamp(12.0, 70.0);
    let sigma_space = (6.0 * scale).max(2.0);
    bilateral_filter(raw, width, height, radius, sigma_color, sigma_space)
}

fn bilateral_filter(raw: &[u8], width: usize, height: usize, radius: usize, sigma_color: f32, sigma_space: f32) -> Vec<u8> {
    let size = radius * 2 + 1;
    let mut spatial = vec![0.0f32; size * size];
    for dy in 0..size {
        for dx in 0..size {
            let x = dx as f32 - radius as f32;
            let y = dy as f32 - radius as f32;
            spatial[dy * size + dx] = (-(x * x + y * y) / (2.0 * sigma_space * sigma_space)).exp();
        }
    }
    // Colour weight lookup: every possible 0..255 difference, so no exp() in the loop.
    let mut colour = [0.0f32; 511];
    for (index, value) in colour.iter_mut().enumerate() {
        let diff = index as f32 - 255.0;
        *value = (-(diff * diff) / (2.0 * sigma_color * sigma_color)).exp();
    }
    let mut out = vec![0u8; raw.len()];
    for y in 0..height {
        for x in 0..width {
            let center = raw[y * width + x];
            let mut sum = 0.0f32;
            let mut weight = 0.0f32;
            for dy in 0..size {
                let ny = y as isize + dy as isize - radius as isize;
                if ny < 0 || ny >= height as isize { continue; }
                for dx in 0..size {
                    let nx = x as isize + dx as isize - radius as isize;
                    if nx < 0 || nx >= width as isize { continue; }
                    let value = raw[ny as usize * width + nx as usize];
                    let w = spatial[dy * size + dx] * colour[(value as i32 - center as i32 + 255) as usize];
                    sum += value as f32 * w;
                    weight += w;
                }
            }
            out[y * width + x] = if weight > 0.0 { (sum / weight).round().clamp(0.0, 255.0) as u8 } else { center };
        }
    }
    out
}

/// Separable Gaussian blur on a float buffer.
fn gaussian_blur_f32(src: &[f32], width: usize, height: usize, sigma: f32) -> Vec<f32> {
    if sigma <= 0.01 { return src.to_vec(); }
    let radius = ((sigma * 3.0).ceil() as usize).max(1);
    let mut kernel = vec![0.0f32; radius * 2 + 1];
    let mut total = 0.0f32;
    for (index, value) in kernel.iter_mut().enumerate() {
        let x = index as f32 - radius as f32;
        *value = (-(x * x) / (2.0 * sigma * sigma)).exp();
        total += *value;
    }
    for value in kernel.iter_mut() { *value /= total; }

    let mut horizontal = vec![0.0f32; src.len()];
    for y in 0..height {
        for x in 0..width {
            let mut acc = 0.0f32;
            for (k, &weight) in kernel.iter().enumerate() {
                let sx = (x as isize + k as isize - radius as isize).clamp(0, width as isize - 1) as usize;
                acc += src[y * width + sx] * weight;
            }
            horizontal[y * width + x] = acc;
        }
    }
    let mut out = vec![0.0f32; src.len()];
    for y in 0..height {
        for x in 0..width {
            let mut acc = 0.0f32;
            for (k, &weight) in kernel.iter().enumerate() {
                let sy = (y as isize + k as isize - radius as isize).clamp(0, height as isize - 1) as usize;
                acc += horizontal[sy * width + x] * weight;
            }
            out[y * width + x] = acc;
        }
    }
    out
}

/// Difference of Gaussians of the normalised image; negative at dark lines.
fn dog_response(pre: &[u8], width: usize, height: usize, sigma: f32, k: f32) -> Vec<f32> {
    let normalised: Vec<f32> = pre.iter().map(|&value| value as f32 / 255.0).collect();
    let narrow = gaussian_blur_f32(&normalised, width, height, sigma);
    let wide = gaussian_blur_f32(&normalised, width, height, sigma * k);
    narrow.iter().zip(wide.iter()).map(|(a, b)| a - b).collect()
}

/// XDoG: ink where the DoG response falls below `-epsilon`.
///
/// `epsilon` is derived from the response distribution so that a fixed fraction
/// of the image becomes ink. That single rule is what makes the operator adapt
/// to any exposure, contrast or subject instead of needing a hand-tuned level --
/// and it is why the same constants work on a white-background product shot and
/// on a busy low-contrast illustration.
fn dog_ink(response: &[f32], target: f32) -> Vec<u8> {
    const BINS: usize = 1024;
    let mut histogram = [0usize; BINS];
    for &value in response {
        let magnitude = (-value).max(0.0).min(0.999);
        histogram[(magnitude * BINS as f32) as usize] += 1;
    }
    let wanted = target * response.len().max(1) as f32;
    let mut accumulated = 0.0f32;
    let mut epsilon = 1.0f32;
    for index in (0..BINS).rev() {
        accumulated += histogram[index] as f32;
        if accumulated >= wanted {
            epsilon = index as f32 / BINS as f32;
            break;
        }
    }
    response.iter().map(|&value| u8::from(value < -epsilon)).collect()
}

/// Multi-scale XDoG line map.
///
/// The large scale is deliberately more sensitive: it is the one carrying a
/// smooth object silhouette, and at that frequency there is no noise to amplify.
fn line_map(pre: &[u8], width: usize, height: usize) -> Vec<u8> {
    const SCALES: [(f32, f32); 3] = [(0.8, 0.040), (1.3, 0.055), (2.2, 0.080)];
    let mut out = vec![0u8; pre.len()];
    for (sigma, target) in SCALES {
        let ink = dog_ink(&dog_response(pre, width, height, sigma, 1.6), target);
        for (slot, value) in out.iter_mut().zip(ink.iter()) {
            *slot |= *value;
        }
    }
    out
}

/// Object outline for pictures sitting on a light background.
///
/// A photo's silhouette against white is a low-frequency, high-contrast boundary
/// that XDoG only responds to intermittently; without this the contour comes out
/// dotted.
fn silhouette(pre: &[u8], width: usize, height: usize, bright_ratio: f32, min_area: usize) -> Vec<u8> {
    if bright_ratio < 0.30 { return vec![0u8; pre.len()]; }
    let radius = (width.min(height) / 24).max(6);
    let mut closed = adaptive_ink(pre, width, height, radius, 12.0, false);
    // Two plus-shaped iterations approximate the 5x5 ellipse closing the
    // prototype uses; the 8-connected version would over-merge.
    for _ in 0..2 { closed = dilate_plus(&closed, width, height); }
    for _ in 0..2 { closed = erode_plus(&closed, width, height); }

    let mut visited = vec![false; closed.len()];
    let mut keep = vec![0u8; closed.len()];
    let minimum = (min_area * 6).max(64);
    for start in 0..closed.len() {
        if closed[start] == 0 || visited[start] { continue; }
        let mut queue = vec![start];
        let mut component = Vec::new();
        visited[start] = true;
        while let Some(index) = queue.pop() {
            component.push(index);
            for neighbor in neighbors(&closed, width, height, index % width, index / width) {
                if !visited[neighbor] {
                    visited[neighbor] = true;
                    queue.push(neighbor);
                }
            }
        }
        if component.len() >= minimum {
            for index in component { keep[index] = 1; }
        }
    }
    let eroded = erode(&keep, width, height);
    keep.iter().zip(eroded.iter()).map(|(&outer, &inner)| u8::from(outer != 0 && inner == 0)).collect()
}

/// A drawing is a mostly-light canvas covered in THIN dark strokes. A photo or a
/// shaded illustration fails the thinness test, which routes it to the edge path.
fn is_line_art(raw: &[u8], width: usize, height: usize) -> bool {
    let radius = (width.min(height) / 24).max(6);
    let ink = adaptive_ink(raw, width, height, radius, 16.0, false);
    let inked = ink.iter().filter(|&&value| value != 0).count();
    if inked == 0 { return false; }
    let ratio = inked as f32 / ink.len().max(1) as f32;
    if ratio > 0.14 { return false; }
    let dist = distance_transform(&ink, width, height);
    let mut thickness: Vec<f32> = ink.iter().enumerate()
        .filter(|(_, &value)| value != 0)
        .map(|(index, _)| dist[index] * 2.0)
        .collect();
    thickness.sort_by(|a, b| a.partial_cmp(b).unwrap_or(CmpOrdering::Equal));
    let p90 = thickness[((thickness.len() - 1) as f32 * 0.90) as usize];
    p90 <= 4.5
}
fn despeckle(data: &mut [u8], width: usize, height: usize, artifact_removal: f32) {
    let strength = artifact_removal.clamp(0.0, 1.0);
    if strength <= 0.0 { return; }
    // Scaled with resolution so the result is the same at any processing size.
    // Kept deliberately low: a larger floor deletes the short fragments that make
    // up eyes, lettering and other genuine detail.
    let scale = (width.max(height) as f32 / 512.0).max(0.5);
    let minimum = ((1.0 + (1.0 - strength) * 12.0) * scale * scale).round().max(1.0) as usize;
    let mut visited = vec![false; data.len()];
    for start in 0..data.len() {
        if data[start] == 0 || visited[start] { continue; }
        let mut queue = vec![start];
        let mut component = Vec::new();
        visited[start] = true;
        while let Some(index) = queue.pop() {
            component.push(index);
            for neighbor in neighbors(data, width, height, index % width, index / width) {
                if !visited[neighbor] {
                    visited[neighbor] = true;
                    queue.push(neighbor);
                }
            }
        }
        if component.len() < minimum {
            for index in component { data[index] = 0; }
        }
    }
}

/// Sobel gradient pair. Kept separate from the magnitude because the edge
/// detector needs the direction to run non-maximum suppression.
fn sobel_gradients(raw: &[u8], width: usize, height: usize) -> (Vec<f32>, Vec<f32>) {
    let mut gx_out = vec![0.0f32; raw.len()];
    let mut gy_out = vec![0.0f32; raw.len()];
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let at = |dx: isize, dy: isize| raw[((y as isize + dy) as usize) * width + (x as isize + dx) as usize] as f32;
            gx_out[y * width + x] = -at(-1, -1) + at(1, -1) - 2.0 * at(-1, 0) + 2.0 * at(1, 0) - at(-1, 1) + at(1, 1);
            gy_out[y * width + x] = -at(-1, -1) - 2.0 * at(0, -1) - at(1, -1) + at(-1, 1) + 2.0 * at(0, 1) + at(1, 1);
        }
    }
    (gx_out, gy_out)
}

/// Otsu's method over a 256-bin histogram.
fn adaptive_ink(raw: &[u8], width: usize, height: usize, radius: usize, offset: f32, invert: bool) -> Vec<u8> {
    let stride = width + 1;
    let mut integral = vec![0u64; stride * (height + 1)];
    for y in 0..height {
        let mut row_sum = 0u64;
        for x in 0..width {
            row_sum += raw[y * width + x] as u64;
            integral[(y + 1) * stride + x + 1] = integral[y * stride + x + 1] + row_sum;
        }
    }
    let mut out = vec![0u8; raw.len()];
    for y in 0..height {
        let y0 = y.saturating_sub(radius);
        let y1 = (y + radius + 1).min(height);
        for x in 0..width {
            let x0 = x.saturating_sub(radius);
            let x1 = (x + radius + 1).min(width);
            let area = ((x1 - x0) * (y1 - y0)) as f32;
            let sum = integral[y1 * stride + x1] + integral[y0 * stride + x0]
                - integral[y0 * stride + x1] - integral[y1 * stride + x0];
            let mean = sum as f32 / area;
            let value = raw[y * width + x] as f32;
            let ink = if invert { value > mean + offset } else { value < mean - offset };
            out[y * width + x] = u8::from(ink);
        }
    }
    out
}

/// Canny-style edge detection: Sobel gradients, non-maximum suppression, then
/// hysteresis. Unlike the previous `sobel_edges` this yields a clean one-pixel
/// edge map, so nothing downstream has to guess the stroke width -- and no
/// morphological opening can wipe the edges out.
fn canny_edges(raw: &[u8], width: usize, height: usize, threshold_scale: f32, hysteresis: f32) -> Vec<u8> {
    if width < 3 || height < 3 { return vec![0u8; raw.len()]; }
    let (gx, gy) = sobel_gradients(raw, width, height);
    let magnitude: Vec<f32> = gx.iter().zip(gy.iter()).map(|(x, y)| x.hypot(*y)).collect();
    let suppressed = non_maximum_suppression(&magnitude, &gx, &gy, width, height);

    let mut histogram = [0usize; 256];
    for &value in &suppressed {
        if value > 0.0 {
            histogram[(value.min(255.0)) as usize] += 1;
        }
    }
    // Percentile rather than Otsu. On a photo the gradient histogram is dominated
    // by texture, so Otsu sets the bar far above the contrast of real structure
    // and most of the picture is discarded.
    let kept: usize = histogram.iter().sum();
    let wanted = (kept as f32 * 0.78) as usize;
    let mut accumulated = 0usize;
    let mut level = 0usize;
    for (index, &count) in histogram.iter().enumerate() {
        accumulated += count;
        if accumulated >= wanted {
            level = index;
            break;
        }
    }
    let high = (level as f32 * threshold_scale).max(8.0);
    let low = (high * hysteresis).max(3.0);

    let mut result = vec![0u8; raw.len()];
    let mut queue: Vec<usize> = Vec::new();
    for (index, &value) in suppressed.iter().enumerate() {
        if value >= high {
            result[index] = 1;
            queue.push(index);
        }
    }
    // Hysteresis: keep weak edges that are 8-connected to a strong edge.
    while let Some(index) = queue.pop() {
        let x = index % width;
        let y = index / width;
        for dy in -1isize..=1 {
            for dx in -1isize..=1 {
                if dx == 0 && dy == 0 { continue; }
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize { continue; }
                let neighbor = ny as usize * width + nx as usize;
                if result[neighbor] == 0 && suppressed[neighbor] >= low {
                    result[neighbor] = 1;
                    queue.push(neighbor);
                }
            }
        }
    }
    result
}

/// Thins a gradient magnitude map down to its local maxima along the gradient
/// direction, quantised into the four 45-degree sectors.
fn non_maximum_suppression(magnitude: &[f32], gx: &[f32], gy: &[f32], width: usize, height: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; magnitude.len()];
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let index = y * width + x;
            let value = magnitude[index];
            if value <= 0.0 { continue; }
            let ax = gx[index].abs();
            let ay = gy[index].abs();
            let (a, b) = if ax >= ay * 2.4142 {
                (magnitude[index - 1], magnitude[index + 1])
            } else if ax >= ay * 0.4142 {
                if gx[index] * gy[index] > 0.0 {
                    (magnitude[index - width - 1], magnitude[index + width + 1])
                } else {
                    (magnitude[index - width + 1], magnitude[index + width - 1])
                }
            } else {
                (magnitude[index - width], magnitude[index + width])
            };
            if value >= a && value >= b { out[index] = value; }
        }
    }
    out
}

/// Chamfer (3-4) distance to the nearest background pixel.
fn distance_transform(mask: &[u8], width: usize, height: usize) -> Vec<f32> {
    const DIAGONAL: f32 = 1.414_213_6;
    const FAR: f32 = 1.0e9;
    let mut dist: Vec<f32> = mask.iter().map(|&value| if value == 0 { 0.0 } else { FAR }).collect();
    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            if dist[index] == 0.0 { continue; }
            let mut best = dist[index];
            if y > 0 {
                best = best.min(dist[index - width] + 1.0);
                if x > 0 { best = best.min(dist[index - width - 1] + DIAGONAL); }
                if x + 1 < width { best = best.min(dist[index - width + 1] + DIAGONAL); }
            }
            if x > 0 { best = best.min(dist[index - 1] + 1.0); }
            dist[index] = best;
        }
    }
    for y in (0..height).rev() {
        for x in (0..width).rev() {
            let index = y * width + x;
            if dist[index] == 0.0 { continue; }
            let mut best = dist[index];
            if y + 1 < height {
                best = best.min(dist[index + width] + 1.0);
                if x > 0 { best = best.min(dist[index + width - 1] + DIAGONAL); }
                if x + 1 < width { best = best.min(dist[index + width + 1] + DIAGONAL); }
            }
            if x + 1 < width { best = best.min(dist[index + 1] + 1.0); }
            dist[index] = best;
        }
    }
    dist
}

fn dots(data: &[u8], width: usize, height: usize) -> Vec<DrawingStroke> {
    const MAX_DOTS: usize = 4096;
    let mut visited = vec![false; data.len()];
    let mut centers: Vec<(f32, f32)> = Vec::new();
    for start in 0..data.len() {
        if data[start] == 0 || visited[start] { continue; }
        let mut queue = vec![start];
        let mut count = 0usize;
        let mut sum_x = 0usize;
        let mut sum_y = 0usize;
        visited[start] = true;
        while let Some(index) = queue.pop() {
            count += 1;
            sum_x += index % width;
            sum_y += index / width;
            for neighbor in neighbors(data, width, height, index % width, index / width) {
                if !visited[neighbor] {
                    visited[neighbor] = true;
                    queue.push(neighbor);
                }
            }
        }
        if count > 0 && count <= 24 {
            centers.push((sum_x as f32 / count as f32, sum_y as f32 / count as f32));
        }
    }
    if centers.len() > MAX_DOTS {
        let step = centers.len() as f32 / MAX_DOTS as f32;
        centers = (0..MAX_DOTS)
            .map(|index| centers[((index as f32 * step) as usize).min(centers.len() - 1)])
            .collect();
    }
    centers.into_iter().map(|(x, y)| DrawingStroke {
        points: vec![DrawingPoint { x, y }, DrawingPoint { x: x + 0.75, y }],
    }).collect()
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

/// 4-connected dilation (plus-shaped structuring element).
///
/// `dilate`/`erode` above use the 8-connected neighbourhood, i.e. a 3x3 SQUARE.
/// Applied to a one-pixel-wide edge map that also grows lines diagonally, which
/// merges any two lines that are two pixels apart -- and on a photo that is most
/// of them. Gap bridging wants the plus shape, which is what the reference tools
/// use as a 3x3 ellipse.
fn dilate_plus(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut result = data.to_vec();
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let index = y * width + x;
            if data[index] == 0
                && (data[index - 1] != 0 || data[index + 1] != 0 || data[index - width] != 0 || data[index + width] != 0)
            {
                result[index] = 1;
            }
        }
    }
    result
}

fn erode_plus(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut result = data.to_vec();
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let index = y * width + x;
            if data[index] != 0
                && (data[index - 1] == 0 || data[index + 1] == 0 || data[index - width] == 0 || data[index + width] == 0)
            {
                result[index] = 0;
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

/// Removes short dead-end branches (spurs) from a skeleton.
///
/// The previous implementation deleted every endpoint pixel `prune_length`
/// times, which also shortened every legitimate stroke by that many pixels at
/// both ends and could erase genuinely short strokes outright. This version only
/// deletes a branch when it runs from an endpoint into a junction within
/// `min_length` steps.
fn prune_spurs(data: &mut [u8], width: usize, height: usize, min_length: usize) {
    if min_length == 0 { return; }
    for _ in 0..4 {
        let mut branches: Vec<Vec<usize>> = Vec::new();
        let mut endpoints: Vec<usize> = (0..data.len())
            .filter(|&index| data[index] != 0 && neighbors(data, width, height, index % width, index / width).len() == 1)
            .collect();
        endpoints.sort_unstable();
        for start in endpoints {
            if data[start] == 0 { continue; }
            let mut branch = vec![start];
            let mut previous = usize::MAX;
            let mut current = start;
            let mut hit_junction = false;
            while branch.len() <= min_length {
                let candidates: Vec<usize> = neighbors(data, width, height, current % width, current / width)
                    .into_iter().filter(|index| *index != previous).collect();
                if candidates.is_empty() { break; }
                if candidates.len() > 1 { hit_junction = true; break; }
                previous = current;
                current = candidates[0];
                branch.push(current);
                if neighbors(data, width, height, current % width, current / width).len() > 2 {
                    hit_junction = true;
                    break;
                }
            }
            if hit_junction && branch.len() <= min_length {
                branch.pop();
                if !branch.is_empty() { branches.push(branch); }
            }
        }
        if branches.is_empty() { break; }
        for branch in branches {
            for index in branch { data[index] = 0; }
        }
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
                let in_len = px.hypot(py).max(0.001);
                *candidates.iter().max_by(|a, b| {
                    let score = |index: usize| -> f32 {
                        let dx = (index % width) as f32 - (current % width) as f32;
                        let dy = (index / width) as f32 - (current / width) as f32;
                        let out_len = dx.hypot(dy).max(0.001);
                        // cosine similarity with the incoming direction
                        let cosine = (px * dx + py * dy) / (in_len * out_len);
                        // look-ahead: average direction of this candidate's unvisited
                        // neighbours. If continuing through this candidate also keeps the
                        // stroke straight, prefer it over a sharp turn.
                        let ahead = neighbors(data, width, height, index % width, index / width)
                            .into_iter()
                            .filter(|n| !visited[*n])
                            .collect::<Vec<_>>();
                        if ahead.is_empty() { return cosine; }
                        let mut ax = 0.0f32;
                        let mut ay = 0.0f32;
                        for n in &ahead {
                            ax += (*n % width) as f32 - (index % width) as f32;
                            ay += (*n / width) as f32 - (index / width) as f32;
                        }
                        let ncount = ahead.len() as f32;
                        // Composite direction: the average of the chosen step and the
                        // average of the following steps. If this composite is collinear
                        // with the incoming direction, the candidate continues smoothly.
                        let cx = dx + ax / ncount;
                        let cy = dy + ay / ncount;
                        let c_len = cx.hypot(cy).max(0.001);
                        let continuity = (px * cx + py * cy) / (in_len * c_len);
                        // 0.4 immediate cosine + 0.6 look-ahead continuity: a small turn
                        // is acceptable if the next segment continues the original heading.
                        cosine * 0.4 + continuity * 0.6
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

/// Moving average along the stroke.
///
/// The traced skeleton is an 8-connected pixel staircase, so without this every
/// outline renders as a shaky potato. A radius-3 window is what turns it back
/// into a smooth arc while keeping genuine corners.
fn smooth_points(points: &[DrawingPoint], window: usize) -> Vec<DrawingPoint> {
    if window <= 1 || points.len() <= 2 { return points.to_vec(); }
    let radius = (window / 2).max(1);
    let count = points.len();
    let closed = count > 3 && point_distance(&points[0], &points[count - 1]) < 2.0;
    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        if !closed && (index < radius || index + radius >= count) {
            out.push(points[index].clone());
            continue;
        }
        let mut sum_x = 0.0f32;
        let mut sum_y = 0.0f32;
        let mut total = 0.0f32;
        for offset in -(radius as isize)..=(radius as isize) {
            let target = if closed {
                (index as isize + offset).rem_euclid(count as isize) as usize
            } else {
                (index as isize + offset).clamp(0, count as isize - 1) as usize
            };
            sum_x += points[target].x;
            sum_y += points[target].y;
            total += 1.0;
        }
        out.push(DrawingPoint { x: sum_x / total, y: sum_y / total });
    }
    out
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
    // 防御：丢弃空笔画，避免后续 points.first()/last() 在空集合上 unwrap panic
    strokes.retain(|s| !s.points.is_empty());
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
                let Some((_, reverse_a, reverse_b)) = options.into_iter()
                    .filter(|option| option.0 <= distance)
                    .filter(|(_, reverse_a, reverse_b)| stroke_join_is_smooth(&strokes[left], &strokes[right], *reverse_a, *reverse_b))
                    .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(CmpOrdering::Equal)) else { continue; };
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

/// Outgoing tangent at one end of a stroke, averaged over up to `SPAN` points so
/// pixel-level noise cannot flip the direction.
fn endpoint_tangent(points: &[DrawingPoint], at_end: bool) -> Option<(f32, f32)> {
    const SPAN: usize = 6;
    let count = points.len();
    if count < 2 { return None; }
    let span = (count - 1).min(SPAN);
    let (dx, dy) = if at_end {
        (points[count - 1].x - points[count - 1 - span].x, points[count - 1].y - points[count - 1 - span].y)
    } else {
        (points[span].x - points[0].x, points[span].y - points[0].y)
    };
    let length = dx.hypot(dy);
    if length < f32::EPSILON { return None; }
    Some((dx / length, dy / length))
}

/// True when joining `left` to `right` continues both strokes in the same
/// direction: the outgoing heading of the left end, the heading the right stroke
/// leaves its start with, and the actual gap direction must all agree.
fn stroke_join_is_smooth(left: &DrawingStroke, right: &DrawingStroke, reverse_left: bool, reverse_right: bool) -> bool {
    const MIN_COSINE: f32 = 0.82;
    let left_end = if reverse_left { &left.points[0] } else { left.points.last().unwrap() };
    let right_start = if reverse_right { right.points.last().unwrap() } else { &right.points[0] };
    let gap_x = right_start.x - left_end.x;
    let gap_y = right_start.y - left_end.y;
    let gap_length = gap_x.hypot(gap_y);
    if gap_length < 1e-4 { return true; }
    let gap = (gap_x / gap_length, gap_y / gap_length);
    let Some(left_dir) = endpoint_tangent(&left.points, !reverse_left) else { return true };
    let Some(right_dir) = endpoint_tangent(&right.points, reverse_right) else { return true };
    left_dir.0 * gap.0 + left_dir.1 * gap.1 >= MIN_COSINE
        && right_dir.0 * gap.0 + right_dir.1 * gap.1 >= MIN_COSINE
}
fn order_strokes(strokes: Vec<DrawingStroke>) -> Vec<DrawingStroke> {
    let mut strokes: Vec<DrawingStroke> = strokes.into_iter().filter(|s| !s.points.is_empty()).collect();
    if strokes.len() <= 1 { return strokes; }
    // Nearest-neighbour routing is quadratic; skip it for dither-scale counts.
    if strokes.len() > 8000 { return strokes; }
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
    // Two-opt improvement: cap at 3 passes; empirically enough for image-sized stroke
    // counts and bounded so we never spend more than a few ms on routing.
    two_opt_pass(ordered, 3)
}

/// Reverses segments of the tour when doing so reduces the total endpoint-to-endpoint
/// pen travel. Operates in place; `max_passes` caps iterations to keep the cost bounded.
fn two_opt_pass(mut tour: Vec<DrawingStroke>, max_passes: usize) -> Vec<DrawingStroke> {
    tour.retain(|s| !s.points.is_empty());
    // Bounded so a dither-scale stroke count can never turn this into a stall.
    if tour.len() < 4 || tour.len() > 1500 { return tour; }
    for _ in 0..max_passes.max(1) {
        let mut improved = false;
        let n = tour.len();
        for i in 1..n - 1 {
            let prev_end = tour[i - 1].points.last().unwrap().clone();
            for j in (i + 1)..n {
                let cur_first = tour[i].points.first().unwrap().clone();
                let cur_last = tour[j].points.last().unwrap().clone();
                let next_first = if j + 1 < n { Some(tour[j + 1].points.first().unwrap().clone()) } else { None };
                // After reversing the segment [i..=j] AND each stroke's points inside it,
                // the new boundary is: prev -> (old j's last), (old i's first) -> next.
                let new_first = tour[j].points.last().unwrap().clone();
                let new_last = tour[i].points.first().unwrap().clone();
                let mut before = point_distance(&prev_end, &cur_first);
                let mut after = point_distance(&prev_end, &new_first);
                if let Some(next) = next_first {
                    before += point_distance(&cur_last, &next);
                    after += point_distance(&new_last, &next);
                }
                if after + 1e-3 < before {
                    for k in i..=j {
                        tour[k].points.reverse();
                    }
                    tour[i..=j].reverse();
                    improved = true;
                }
            }
        }
        if !improved { break; }
    }
    tour
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
        // Optional explicit canvas size: when set, derive scale so 1 image pixel maps to
        // (canvas_size_px / plan.dim) screen pixels. The user still controls the fine-tune
        // via `sensitivity` and `vertical_stretch`. canvas_size_px == 0 keeps the legacy
        // sensitivity-only behaviour.
        let (scale_x, scale_y) = if config.canvas_size_px > 0 && plan.width > 0 && plan.height > 0 {
            let sx = config.canvas_size_px as f32 / plan.width as f32;
            let sy = config.canvas_size_px as f32 / plan.height as f32;
            (sx, sy)
        } else { (1.0, 1.0) };
        for (index, stroke) in plan.strokes.iter().enumerate() {
            if stop.load(Ordering::SeqCst) { break; }
            wait_while_paused(&stop, &paused);
            if stop.load(Ordering::SeqCst) { break; }
            mouse_left(false);
            interruptible_sleep(config.lift_delay_ms.min(500), &stop, &paused);
            let first = &stroke.points[0];
            move_planar(&mut current_x, &mut current_y, first, &config, true, &stop, &paused, &mut error_x, &mut error_y, scale_x, scale_y);
            if stop.load(Ordering::SeqCst) { break; }
            mouse_left(true);
            // Pen-down settle: give the canvas a few ms to register the click so the first
            // actual stroke point isn't lost or offset by a half-pixel.
            if config.pen_settle_ms > 0 {
                interruptible_sleep(config.pen_settle_ms, &stop, &paused);
            }
            if stop.load(Ordering::SeqCst) { break; }
            for point in stroke.points.iter().skip(1) {
                move_planar(&mut current_x, &mut current_y, point, &config, false, &stop, &paused, &mut error_x, &mut error_y, scale_x, scale_y);
                if stop.load(Ordering::SeqCst) { break; }
            }
            mouse_left(false);
            interruptible_sleep(config.lift_delay_ms, &stop, &paused);
            error_x = 0.0;
            error_y = 0.0;
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
fn move_planar(current_x: &mut f32, current_y: &mut f32, target: &DrawingPoint, config: &DrawingConfig, pen_up: bool, stop: &AtomicBool, paused: &AtomicBool, error_x: &mut f32, error_y: &mut f32, scale_x: f32, scale_y: f32) {
    let delta_x = (target.x - *current_x) * scale_x * config.sensitivity;
    let delta_y = (target.y - *current_y) * scale_y * config.sensitivity * config.vertical_stretch;
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
        let delay = if pen_up {
            (config.point_delay_ms as f32 / config.lift_speed).max(1.0) as u64
        } else {
            config.point_delay_ms
        };
        thread::sleep(Duration::from_millis(delay));
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
    fn nearby_strokes_with_a_sharp_join_remain_separate() {
        let strokes = vec![
            DrawingStroke { points: vec![DrawingPoint { x: 0.0, y: 0.0 }, DrawingPoint { x: 10.0, y: 0.0 }] },
            DrawingStroke { points: vec![DrawingPoint { x: 10.5, y: 0.0 }, DrawingPoint { x: 10.5, y: 10.0 }] },
        ];
        assert_eq!(merge_nearby_strokes(strokes, 2.0).len(), 2);
    }

    /// Bresenham helper used only by the sample renderer below.
    fn draw_line(canvas: &mut image::RgbImage, x0: f32, y0: f32, x1: f32, y1: f32) {
        let ink = image::Rgb([84u8, 36, 15]);
        let (mut x0, mut y0) = (x0.round() as i32, y0.round() as i32);
        let (x1, y1) = (x1.round() as i32, y1.round() as i32);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        loop {
            if x0 >= 0 && y0 >= 0 && (x0 as u32) < canvas.width() && (y0 as u32) < canvas.height() {
                canvas.put_pixel(x0 as u32, y0 as u32, ink);
            }
            if x0 == x1 && y0 == y1 { break; }
            let doubled = 2 * error;
            if doubled >= dy { error += dy; x0 += sx; }
            if doubled <= dx { error += dx; y0 += sy; }
        }
    }

    /// Runs the real pipeline over the bundled sample images and writes the
    /// resulting line art to `src-tauri/target/lineart-check/` so the output can
    /// be inspected visually. Run with `--nocapture` to see the stroke counts.
    #[test]
    fn renders_bundled_samples_to_png() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("src").join("assets");
        let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join("lineart-check");
        std::fs::create_dir_all(&out_dir).expect("create output directory");
        let config = DrawingConfig::default();
        let mut produced = 0usize;
        for name in ["helmet.jpeg", "dog.jpg", "main.png", "unknown.jpeg", "mono.jpeg"] {
            let path = root.join(name);
            if !path.is_file() { continue; }
            let Ok(image) = image::open(&path) else { continue };
            let resized = image.resize(512, 512, image::imageops::FilterType::Triangle);
            let gray = image::imageops::blur(&resized.to_luma8(), config.blur);
            let (width, height) = gray.dimensions();
            let (w, h) = (width as usize, height as usize);

            let raw = gray.into_raw();
            let mut mask = build_ink_mask(&raw, w, h, &config);
            let before = mask.iter().filter(|&&value| value != 0).count();
            mask = erode_plus(&dilate_plus(&mask, w, h), w, h);
            let after_close = mask.iter().filter(|&&value| value != 0).count();
            despeckle(&mut mask, w, h, config.artifact_removal);
            let after_despeckle = mask.iter().filter(|&&value| value != 0).count();
            let mask_view = image::GrayImage::from_raw(w as u32, h as u32, mask.iter().map(|&value| if value == 0 { 255u8 } else { 0u8 }).collect())
                .expect("mask view");
            mask_view.save(out_dir.join(format!("{}.mask.png", name.replace('.', "_")))).expect("save mask");
            println!("  DIAG {name}: lineart={} ink raw={:.2}% closed={:.2}% despeckled={:.2}%",
                is_line_art(&raw, w, h),
                100.0 * before as f32 / raw.len() as f32,
                100.0 * after_close as f32 / raw.len() as f32,
                100.0 * after_despeckle as f32 / raw.len() as f32);
            skeletonize(&mut mask, w, h);
            let after_skeleton = mask.iter().filter(|&&value| value != 0).count();
            println!("  DIAG {name}: skeleton px={after_skeleton}");
            prune_spurs(&mut mask, w, h, config.prune_length);
            let after_prune = mask.iter().filter(|&&value| value != 0).count();
            println!("  DIAG {name}: after prune px={after_prune}");
            let mut strokes = extract_strokes(&mask, w, h, config.min_stroke_length);
            // Mirror process_image's post-processing so the PNG is the real preview.
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
            strokes = order_strokes(strokes);
            let points: usize = strokes.iter().map(|stroke| stroke.points.len()).sum();

            assert!(strokes.len() > 20, "{name}: expected a rich line drawing, got {} strokes", strokes.len());
            assert!(points > 200, "{name}: expected a detailed path, got {points} points");

            let scale = 2u32;
            let mut canvas = image::RgbImage::from_pixel(w as u32 * scale, h as u32 * scale, image::Rgb([255, 253, 247]));
            for stroke in &strokes {
                for pair in stroke.points.windows(2) {
                    draw_line(&mut canvas, pair[0].x * scale as f32, pair[0].y * scale as f32, pair[1].x * scale as f32, pair[1].y * scale as f32);
                }
            }
            let target = out_dir.join(format!("{}.png", name.replace('.', "_")));
            canvas.save(&target).expect("save preview");
            println!("{name}: {} strokes, {points} points -> {}", strokes.len(), target.display());
            produced += 1;
        }
        assert!(produced > 0, "no bundled sample images were found");
    }

    #[test]
    fn adaptive_threshold_keeps_a_dark_line_on_a_bright_background() {
        let (width, height) = (64usize, 64usize);
        let mut raw = vec![220u8; width * height];
        for x in 0..width {
            raw[32 * width + x] = 30;
        }
        let ink = adaptive_ink(&raw, width, height, 8, 20.0, false);
        assert_eq!(ink[32 * width + 32], 1, "the dark line must be ink");
        assert_eq!(ink[5 * width + 5], 0, "the bright background must stay clear");
    }

    #[test]
    fn canny_returns_a_thin_edge_for_a_step() {
        let (width, height) = (48usize, 48usize);
        let mut raw = vec![240u8; width * height];
        for y in 0..height {
            for x in 24..width {
                raw[y * width + x] = 20;
            }
        }
        let edges = canny_edges(&raw, width, height, 1.0, 0.4);
        let total: usize = edges.iter().filter(|&&value| value != 0).count();
        assert!(total > 0, "a hard step must produce edges");
        // A one-pixel-wide vertical step must not bloom into a thick band.
        assert!(total < height * 6, "edge map should stay thin, got {total} pixels");
    }


    #[test]
    fn prune_spurs_keeps_a_long_stroke_intact() {
        let (width, height) = (32usize, 32usize);
        let mut mask = vec![0u8; width * height];
        for x in 0..24 {
            mask[16 * width + x] = 1;
        }
        // A two-pixel spur branching upwards off the main line.
        mask[15 * width + 10] = 1;
        mask[14 * width + 10] = 1;
        prune_spurs(&mut mask, width, height, 4);
        assert_eq!(mask[16 * width + 0], 1, "the far end of the main stroke must survive");
        assert_eq!(mask[16 * width + 23], 1, "the near end of the main stroke must survive");
        assert_eq!(mask[14 * width + 10], 0, "the short spur must be removed");
    }

    #[test]
    fn dither_emits_one_stroke_per_dot() {
        let (width, height) = (16usize, 16usize);
        let mut mask = vec![0u8; width * height];
        for &(x, y) in &[(2usize, 2usize), (6, 6), (10, 10)] {
            mask[y * width + x] = 1;
        }
        let strokes = dots(&mask, width, height);
        assert_eq!(strokes.len(), 3);
        assert!(strokes.iter().all(|stroke| stroke.points.len() >= 2));
    }

    #[test]
    fn image_type_routing_separates_a_drawing_from_a_photo() {
        let (width, height) = (96usize, 96usize);
        // thin dark strokes on a light canvas -> a drawing
        let mut drawing = vec![235u8; width * height];
        for x in 0..width {
            drawing[48 * width + x] = 20;
        }
        for y in 0..height {
            drawing[y * width + 48] = 20;
        }
        assert!(is_line_art(&drawing, width, height), "a thin-stroke image is line art");
        // a large filled dark region -> not a drawing
        let mut photo = vec![235u8; width * height];
        for y in 20..76 {
            for x in 20..76 {
                photo[y * width + x] = 40;
            }
        }
        assert!(!is_line_art(&photo, width, height), "a filled blob must not be treated as line art");
    }

    #[test]
    fn bilateral_filter_keeps_an_edge_but_smooths_noise() {
        let (width, height) = (64usize, 64usize);
        let mut raw = vec![220u8; width * height];
        for y in 0..height {
            for x in 32..width {
                raw[y * width + x] = 40;
            }
        }
        // sprinkle single-pixel noise on the bright half
        for (x, y) in [(4usize, 4usize), (10, 20), (18, 40), (6, 50)] {
            raw[y * width + x] = 160;
        }
        let out = bilateral_filter(&raw, width, height, 3, 30.0, 4.0);
        assert!(out[32 * width + 32] < 90, "the edge must survive");
        assert!(out[4 * width + 4] > 195, "isolated noise must be smoothed away");
    }

    #[test]
    fn dog_ink_hits_its_target_ink_fraction() {
        let (width, height) = (64usize, 64usize);
        let mut raw = vec![230u8; width * height];
        for y in 8..56 {
            for x in 8..56 {
                raw[y * width + x] = 90;
            }
        }
        let response = dog_response(&raw, width, height, 1.3, 1.6);
        let ink = dog_ink(&response, 0.05);
        let ratio = ink.iter().filter(|&&value| value != 0).count() as f32 / ink.len() as f32;
        assert!(ratio > 0.005, "the square boundary must register as ink, got {ratio}");
        assert!(ratio < 0.20, "the target must keep ink sparse, got {ratio}");
    }

    #[test]
    fn smoothing_removes_skeleton_tremor() {
        // A staircase, exactly what tracing an 8-connected skeleton produces for a
        // smooth diagonal. Jaggedness is the sum of second differences, which is
        // what a moving average is supposed to shrink.
        let points: Vec<DrawingPoint> = (0..60)
            .map(|i| DrawingPoint { x: i as f32, y: (i as f32 * 0.5).round() })
            .collect();
        let smoothed = smooth_points(&points, 7);
        let jaggedness = |p: &[DrawingPoint]| -> f32 {
            (1..p.len() - 1).map(|i| (p[i + 1].y - 2.0 * p[i].y + p[i - 1].y).abs()).sum()
        };
        let before = jaggedness(&points);
        let after = jaggedness(&smoothed);
        assert!(before > 1.0, "the fixture must actually be jagged, got {before}");
        assert!(after < before * 0.35, "smoothing must remove most of the tremor: {before} -> {after}");
    }

    #[test]
    fn normalizes_unsafe_drawing_values() {
        let config = DrawingConfig { sensitivity: f32::NAN, max_dimension: 4000, point_delay_ms: 0, ..Default::default() }.normalized();
        assert_eq!(config.sensitivity, 1.2);
        assert_eq!(config.max_dimension, MAX_CANVAS_DIMENSION);
        assert_eq!(config.point_delay_ms, 1);
    }
}
