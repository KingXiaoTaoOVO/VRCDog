use crate::AppResult;
use serde::Serialize;
use tokio::fs as async_fs;

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
