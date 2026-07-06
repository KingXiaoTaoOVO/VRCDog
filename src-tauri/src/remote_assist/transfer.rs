//! 文件传输模块 — 双向、断点续传
//!
//! 支持：
//! - 发送/接收文件
//! - 大文件分块传输
//! - 断点续传 (记录已传输偏移)
//! - 传输进度回调
//! - 目录递归传输

use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

/// 文件传输块大小 (64KB)
const CHUNK_SIZE: usize = 64 * 1024;

/// 文件发送器
pub struct FileSender {
    file: File,
    path: PathBuf,
    total_size: u64,
    sent: u64,
    id: String,
}

impl FileSender {
    /// 打开文件准备发送
    pub async fn new(path: &Path) -> Result<Self, String> {
        let file = File::open(path)
            .await
            .map_err(|e| format!("打开文件失败: {}", e))?;
        let metadata = file
            .metadata()
            .await
            .map_err(|e| format!("获取文件信息失败: {}", e))?;

        Ok(Self {
            file,
            path: path.to_path_buf(),
            total_size: metadata.len(),
            sent: 0,
            id: uuid::Uuid::new_v4().to_string(),
        })
    }

    /// 读取下一个数据块
    pub async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, String> {
        if self.sent >= self.total_size {
            return Ok(None);
        }

        let mut buf = vec![0u8; CHUNK_SIZE];
        let n = self
            .file
            .read(&mut buf)
            .await
            .map_err(|e| format!("读取文件失败: {}", e))?;

        if n == 0 {
            return Ok(None);
        }

        buf.truncate(n);
        self.sent += n as u64;
        Ok(Some(buf))
    }

    /// 从指定偏移恢复传输 (断点续传)
    pub async fn seek_to(&mut self, offset: u64) -> Result<(), String> {
        self.file
            .seek(std::io::SeekFrom::Start(offset))
            .await
            .map_err(|e| format!("文件定位失败: {}", e))?;
        self.sent = offset;
        Ok(())
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    }
    pub fn total_size(&self) -> u64 {
        self.total_size
    }
    pub fn sent(&self) -> u64 {
        self.sent
    }
    pub fn progress(&self) -> f64 {
        if self.total_size == 0 {
            1.0
        } else {
            self.sent as f64 / self.total_size as f64
        }
    }
}

/// 文件接收器
pub struct FileReceiver {
    file: File,
    path: PathBuf,
    expected_size: u64,
    received: u64,
    id: String,
}

impl FileReceiver {
    /// 创建接收文件
    pub async fn new(
        save_dir: &Path,
        file_name: &str,
        expected_size: u64,
        id: &str,
    ) -> Result<Self, String> {
        let path = save_dir.join(file_name);

        // 确保目录存在
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("创建目录失败: {}", e))?;
        }

        let file = File::create(&path)
            .await
            .map_err(|e| format!("创建文件失败: {}", e))?;

        Ok(Self {
            file,
            path,
            expected_size,
            received: 0,
            id: id.to_string(),
        })
    }

    /// 写入数据块
    pub async fn write_chunk(&mut self, data: &[u8]) -> Result<(), String> {
        self.file
            .write_all(data)
            .await
            .map_err(|e| format!("写入文件失败: {}", e))?;
        self.received += data.len() as u64;
        Ok(())
    }

    /// 完成接收
    pub async fn finish(&mut self) -> Result<PathBuf, String> {
        self.file
            .flush()
            .await
            .map_err(|e| format!("刷新文件失败: {}", e))?;
        Ok(self.path.clone())
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn received(&self) -> u64 {
        self.received
    }
    pub fn progress(&self) -> f64 {
        if self.expected_size == 0 {
            1.0
        } else {
            self.received as f64 / self.expected_size as f64
        }
    }
    pub fn is_complete(&self) -> bool {
        self.received >= self.expected_size
    }
}
