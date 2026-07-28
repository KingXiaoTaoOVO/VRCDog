use super::{
    capture::{CaptureError, ScreenCapturer},
    transport,
    types::WireMessage,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage, RgbImage};
use std::{collections::HashMap, time::Duration};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

const MAX_FRAME_WIDTH: u32 = 1280;
const MAX_FRAME_HEIGHT: u32 = 720;
const FRAME_INTERVAL: Duration = Duration::from_millis(160);
const JPEG_QUALITY: u8 = 62;

lazy_static::lazy_static! {
    static ref CAPTURE_TASKS: Mutex<HashMap<String, CancellationToken>> =
        Mutex::new(HashMap::new());
}

pub async fn start_screen_share(session_id: String) -> Result<(), String> {
    let mut tasks = CAPTURE_TASKS.lock().await;
    if tasks.contains_key(&session_id) {
        return Ok(());
    }
    let cancellation = CancellationToken::new();
    tasks.insert(session_id.clone(), cancellation.clone());
    drop(tasks);

    let runtime = tokio::runtime::Handle::current();
    tokio::task::spawn_blocking(move || {
        let result = capture_loop(&runtime, &session_id, &cancellation);
        runtime.block_on(async {
            CAPTURE_TASKS.lock().await.remove(&session_id);
        });
        if result.is_err() {
            let _ = runtime.block_on(transport::send_wire(
                session_id,
                WireMessage::Disconnect {
                    reason: result.unwrap_err(),
                },
            ));
        }
    });
    Ok(())
}

pub async fn stop_screen_share(session_id: &str) {
    if let Some(cancellation) = CAPTURE_TASKS.lock().await.remove(session_id) {
        cancellation.cancel();
    }
}

fn capture_loop(
    runtime: &tokio::runtime::Handle,
    session_id: &str,
    cancellation: &CancellationToken,
) -> Result<(), String> {
    let mut capturer = ScreenCapturer::new()?;
    let (source_width, source_height) = capturer.dimensions();
    let mut sequence = 0u64;

    while !cancellation.is_cancelled() {
        let started = std::time::Instant::now();
        match capturer.capture_frame() {
            Ok(frame) => {
                sequence += 1;
                let jpeg = encode_frame(&frame, source_width, source_height)?;
                runtime.block_on(transport::send_wire(
                    session_id.to_string(),
                    WireMessage::Frame {
                        seq: sequence,
                        w: source_width,
                        h: source_height,
                        data: BASE64.encode(jpeg),
                        keyframe: true,
                    },
                ))?;
            }
            Err(CaptureError::WouldBlock) => {}
            Err(CaptureError::NotInitialized) => {
                return Err("Screen capture is not initialized".into())
            }
            Err(CaptureError::Failed(error)) => return Err(error),
        }
        let elapsed = started.elapsed();
        if elapsed < FRAME_INTERVAL {
            std::thread::sleep(FRAME_INTERVAL - elapsed);
        }
    }
    Ok(())
}

fn encode_frame(bgra: &[u8], source_width: u32, source_height: u32) -> Result<Vec<u8>, String> {
    let row_stride = bgra
        .len()
        .checked_div(source_height as usize)
        .ok_or_else(|| "Invalid captured frame height".to_string())?;
    if row_stride < source_width as usize * 4 {
        return Err("Captured frame has an invalid row stride".into());
    }

    let mut rgb = Vec::with_capacity((source_width * source_height * 3) as usize);
    for y in 0..source_height as usize {
        let row = &bgra[y * row_stride..y * row_stride + source_width as usize * 4];
        for pixel in row.chunks_exact(4) {
            rgb.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
        }
    }
    let image = RgbImage::from_raw(source_width, source_height, rgb)
        .ok_or_else(|| "Failed to build captured screen image".to_string())?;
    let scale = (MAX_FRAME_WIDTH as f64 / source_width as f64)
        .min(MAX_FRAME_HEIGHT as f64 / source_height as f64)
        .min(1.0);
    let width = (source_width as f64 * scale).round().max(1.0) as u32;
    let height = (source_height as f64 * scale).round().max(1.0) as u32;
    let image = if width != source_width || height != source_height {
        image::imageops::resize(&image, width, height, FilterType::Triangle)
    } else {
        image
    };

    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, JPEG_QUALITY)
        .encode_image(&DynamicImage::ImageRgb8(image))
        .map_err(|error| format!("Failed to encode screen frame: {error}"))?;
    Ok(jpeg)
}
