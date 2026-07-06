//! 屏幕捕获模块 — 使用 DXGI Desktop Duplication API
//!
//! 高性能屏幕捕获，支持硬件加速，低延迟。
//! 回退方案：GDI BitBlt (兼容性更好但性能较低)

use scrap::{Capturer, Display};

/// 屏幕捕获器
pub struct ScreenCapturer {
    capturer: Option<Capturer>,
    width: u32,
    height: u32,
}

impl ScreenCapturer {
    /// 创建新的屏幕捕获器 (主显示器)
    pub fn new() -> Result<Self, String> {
        let display = Display::primary().map_err(|e| format!("无法获取主显示器: {}", e))?;
        let width = display.width() as u32;
        let height = display.height() as u32;
        let capturer = Capturer::new(display).map_err(|e| format!("无法创建捕获器: {}", e))?;

        Ok(Self {
            capturer: Some(capturer),
            width,
            height,
        })
    }

    /// 获取屏幕尺寸
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// 捕获一帧 (返回 BGRA 原始像素数据)
    pub fn capture_frame(&mut self) -> Result<Vec<u8>, CaptureError> {
        let capturer = self.capturer.as_mut().ok_or(CaptureError::NotInitialized)?;

        match capturer.frame() {
            Ok(frame) => {
                let data = frame.to_vec();
                Ok(data)
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                Err(CaptureError::WouldBlock)
            }
            Err(e) => Err(CaptureError::Failed(format!("{}", e))),
        }
    }

    /// 列出所有可用显示器
    pub fn list_displays() -> Result<Vec<DisplayInfo>, String> {
        let displays = Display::all().map_err(|e| format!("无法枚举显示器: {}", e))?;

        Ok(displays
            .iter()
            .enumerate()
            .map(|(i, d)| DisplayInfo {
                index: i as u32,
                width: d.width() as u32,
                height: d.height() as u32,
                is_primary: i == 0,
            })
            .collect())
    }
}

/// 显示器信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct DisplayInfo {
    pub index: u32,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

/// 捕获错误
#[derive(Debug)]
pub enum CaptureError {
    NotInitialized,
    WouldBlock,
    Failed(String),
}
