//! 视频编解码模块 — LZ4 帧压缩 + 增量编码
//!
//! 使用 LZ4 快速压缩算法对屏幕帧进行压缩传输。
//! 支持增量帧（只传输变化区域）以降低带宽。

use lz4_flex::{compress_prepend_size, decompress_size_prepended};

/// 帧编码器
pub struct FrameEncoder {
    prev_frame: Option<Vec<u8>>,
    _width: u32,
    _height: u32,
    frame_seq: u64,
}

impl FrameEncoder {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            prev_frame: None,
            _width: width,
            _height: height,
            frame_seq: 0,
        }
    }

    /// 编码一帧 (LZ4 压缩)
    /// 返回 (序列号, 压缩数据)
    pub fn encode(&mut self, raw_bgra: &[u8]) -> (u64, Vec<u8>) {
        self.frame_seq += 1;

        // 如果有前一帧，计算差异帧 (XOR delta)
        let data_to_compress = if let Some(ref prev) = self.prev_frame {
            if prev.len() == raw_bgra.len() {
                // XOR delta encoding - 只编码变化部分
                let delta: Vec<u8> = raw_bgra
                    .iter()
                    .zip(prev.iter())
                    .map(|(a, b)| a ^ b)
                    .collect();
                delta
            } else {
                raw_bgra.to_vec()
            }
        } else {
            raw_bgra.to_vec()
        };

        // 保存当前帧作为下一次的参考
        self.prev_frame = Some(raw_bgra.to_vec());

        // LZ4 压缩 (delta 帧压缩率极高，因为大部分是0)
        let compressed = compress_prepend_size(&data_to_compress);
        (self.frame_seq, compressed)
    }

    /// 强制关键帧 (完整帧，不做 delta)
    pub fn encode_keyframe(&mut self, raw_bgra: &[u8]) -> (u64, Vec<u8>) {
        self.frame_seq += 1;
        self.prev_frame = Some(raw_bgra.to_vec());
        let compressed = compress_prepend_size(raw_bgra);
        (self.frame_seq, compressed)
    }

    pub fn seq(&self) -> u64 {
        self.frame_seq
    }
}

/// 帧解码器
/// 帧解码器
pub struct FrameDecoder {
    prev_frame: Option<Vec<u8>>,
    _width: u32,
    _height: u32,
}

impl FrameDecoder {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            prev_frame: None,
            _width: width,
            _height: height,
        }
    }

    /// 解码一帧
    pub fn decode(&mut self, compressed: &[u8], is_keyframe: bool) -> Result<Vec<u8>, String> {
        let decompressed =
            decompress_size_prepended(compressed).map_err(|e| format!("LZ4 解压失败: {}", e))?;

        if is_keyframe || self.prev_frame.is_none() {
            self.prev_frame = Some(decompressed.clone());
            Ok(decompressed)
        } else {
            // 应用 XOR delta
            let prev = self.prev_frame.as_ref().unwrap();
            if prev.len() != decompressed.len() {
                // 尺寸不匹配，当作关键帧处理
                self.prev_frame = Some(decompressed.clone());
                return Ok(decompressed);
            }
            let frame: Vec<u8> = prev
                .iter()
                .zip(decompressed.iter())
                .map(|(a, b)| a ^ b)
                .collect();
            self.prev_frame = Some(frame.clone());
            Ok(frame)
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self._width, self._height)
    }
}
