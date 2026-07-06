//! # VrcDog Remote Assist — 原生远程桌面模块
//!
//! 完全原生实现的远程桌面功能，核心原理参考开源远程桌面协议。
//! 无任何第三方远程桌面品牌依赖，完全集成于 VrcDog 应用内。
//!
//! ## 架构
//! - `capture`: 屏幕捕获 (DXGI Desktop Duplication / GDI fallback)
//! - `input`: 键盘鼠标输入模拟 (Windows SendInput API)
//! - `tunnel`: 加密隧道 (X25519 + ChaCha20-Poly1305)
//! - `nat`: NAT 穿透 (STUN + UDP/TCP 打洞)
//! - `relay`: 中继服务器连接 (当 P2P 失败时回退)
//! - `rendezvous`: 信令/ID 服务器通信 (设备注册与发现)
//! - `transfer`: 文件传输 (双向、断点续传)
//! - `codec`: 视频编码/解码 (LZ4 帧压缩)
//! - `commands`: Tauri 命令接口

pub mod capture;
pub mod codec;
pub mod commands;
pub mod input;
pub mod nat;
pub mod relay;
pub mod rendezvous;
pub mod transfer;
pub mod tunnel;
pub mod types;

// Re-export Tauri commands for registration in lib.rs
pub use commands::*;
