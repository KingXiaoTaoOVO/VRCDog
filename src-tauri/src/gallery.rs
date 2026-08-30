use crate::AppResult;
use serde::Serialize;
use tokio::fs as async_fs;
use tokio::time::{sleep, Duration};

#[derive(Serialize, Clone)]
pub struct VrcImage {
    pub name: String,
    pub path: String,
    pub created_at: u64,
    pub size: u64,
}

#[tauri::command]
pub async fn gallery_get_images(
    limit: Option<usize>,
    offset: Option<usize>,
) -> AppResult<Vec<VrcImage>> {
    let mut images = Vec::new();
    if let Some(mut pic_dir) = dirs::picture_dir() {
        pic_dir.push("VRChat");
        if pic_dir.exists() {
            // 需要读取所有的按月份划分的文件夹 (例如 2023-10, 2023-11)
            if let Ok(mut entries) = async_fs::read_dir(&pic_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_dir() {
                        // VRChat saves screenshots in subfolders (e.g. 2023-10)
                        if let Ok(mut sub_entries) = async_fs::read_dir(&path).await {
                            while let Ok(Some(sub_entry)) = sub_entries.next_entry().await {
                                process_image_file(sub_entry, &mut images).await;
                            }
                        }
                    } else {
                        // Some images might be in the root VRChat folder
                        process_image_file(entry, &mut images).await;
                    }
                }
            }
        }
    }

    // 按时间倒序
    images.sort_by_key(|b| std::cmp::Reverse(b.created_at));

    let offset_val = offset.unwrap_or(0);
    let limit_val = limit.unwrap_or(100);

    if offset_val >= images.len() {
        return Ok(Vec::new());
    }

    let end = std::cmp::min(offset_val + limit_val, images.len());
    let paginated_images = images[offset_val..end].to_vec();

    Ok(paginated_images)
}

async fn process_image_file(entry: tokio::fs::DirEntry, images: &mut Vec<VrcImage>) {
    let path = entry.path();
    if path.is_file() {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if ext_str == "png" || ext_str == "jpg" || ext_str == "jpeg" {
                if let Ok(metadata) = entry.metadata().await {
                    let created_at = metadata
                        .created()
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();

                    images.push(VrcImage {
                        name: entry.file_name().to_string_lossy().into_owned(),
                        path: path.to_string_lossy().into_owned(),
                        created_at,
                        size: metadata.len(),
                    });
                }
            }
        }
    }
}

#[tauri::command]
pub async fn gallery_delete_image(path: String) -> AppResult<()> {
    // Basic security check to ensure it's a PNG and it's in the Pictures folder
    if path.ends_with(".png") {
        let _ = async_fs::remove_file(path).await;
    }
    Ok(())
}

fn screenshot_roots() -> Vec<std::path::PathBuf> {
    let mut roots = Vec::new();
    if let Some(mut root) = dirs::picture_dir() {
        root.push("VRChat");
        roots.push(root);
    }
    roots
}

async fn collect_recent_images(root: &std::path::Path, output: &mut Vec<(u64, String)>) {
    let Ok(mut entries) = async_fs::read_dir(root).await else { return; };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_dir() {
            Box::pin(collect_recent_images(&path, output)).await;
            continue;
        }
        let supported = path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| {
            matches!(ext.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp")
        });
        if !supported { continue; }
        let Ok(metadata) = entry.metadata().await else { continue; };
        let modified = metadata.modified().ok().and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |duration| duration.as_millis() as u64);
        output.push((modified, path.to_string_lossy().into_owned()));
    }
}

/// Wait for the next VRChat screenshot, matching VRCLS's armed photo mode.
#[tauri::command]
pub async fn gallery_wait_for_new_image(timeout_seconds: Option<u64>) -> AppResult<String> {
    let timeout = timeout_seconds.unwrap_or(120).clamp(5, 600);
    let mut baseline = Vec::new();
    for root in screenshot_roots() { collect_recent_images(&root, &mut baseline).await; }
    let baseline_latest = baseline.iter().map(|(modified, _)| *modified).max().unwrap_or(0);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout);
    loop {
        let mut images = Vec::new();
        for root in screenshot_roots() { collect_recent_images(&root, &mut images).await; }
        if let Some((_, path)) = images.into_iter().filter(|(modified, _)| *modified > baseline_latest).max_by_key(|(modified, _)| *modified) {
            return Ok(path);
        }
        if tokio::time::Instant::now() >= deadline { return Err("等待 VRChat 新截图超时".into()); }
        sleep(Duration::from_millis(500)).await;
    }
}
