//! 共享数据类型定义

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

lazy_static::lazy_static! {
    pub static ref REMOTE_STATE: Arc<RwLock<RemoteAssistState>> =
        Arc::new(RwLock::new(RemoteAssistState::default()));
}

/// 服务器类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerType {
    /// 官方公共服务器（源码默认的信令/中继节点）
    Official,
    /// VrcDog 服务端（用户自己部署的后台，同时充当远程协助中继）
    VrcDogBackend,
}

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub relay: String,
    pub api: String,
    pub key: String,
    pub is_official: bool,
    pub label: String,
    /// 服务器类型：Official = 公共节点, VrcDogBackend = VrcDog 服务端
    pub server_type: ServerType,
}

/// 本机设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub password: String,
    pub hostname: String,
    pub platform: String,
    pub nat_type: String,
    pub online: bool,
}

/// 连接会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSession {
    pub session_id: String,
    pub peer_id: String,
    pub peer_name: String,
    pub started_at: String,
    pub conn_type: String, // "p2p" | "relay"
    pub latency_ms: u32,
    pub status: String, // "connecting"|"connected"|"disconnected"
}

/// 文件传输条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferItem {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub transferred: u64,
    pub direction: String, // "up" | "down"
    pub status: String,
}

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub from: String, // "local" | "remote"
    pub text: String,
    pub time: String,
}

/// 全局状态
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemoteAssistState {
    pub device: Option<DeviceInfo>,
    pub server: Option<ServerConfig>,
    pub sessions: Vec<ConnectionSession>,
    pub transfers: Vec<FileTransferItem>,
    pub messages: Vec<ChatMessage>,
    pub service_on: bool,
    pub accepting: bool,
}

/// 帧数据 (屏幕捕获后压缩)
#[derive(Debug, Clone)]
pub struct FrameData {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // LZ4 compressed BGRA
    pub timestamp: u64,
}

/// 输入事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InputEvent {
    MouseMove { x: i32, y: i32 },
    MouseDown { button: u8 },
    MouseUp { button: u8 },
    MouseWheel { delta: i32 },
    KeyDown { code: u32 },
    KeyUp { code: u32 },
}

/// 协议消息 (通过加密隧道传输)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "c")]
pub enum WireMessage {
    /// 认证请求
    Auth {
        id: String,
        password: String,
    },
    /// 认证响应
    AuthResult {
        ok: bool,
        reason: String,
    },
    /// 屏幕帧 (二进制，不走JSON)
    Frame {
        seq: u64,
        w: u32,
        h: u32,
        len: u32,
    },
    /// 输入事件
    Input(InputEvent),
    /// 聊天
    Chat {
        text: String,
    },
    /// 文件传输控制
    FileStart {
        id: String,
        name: String,
        size: u64,
    },
    FileData {
        id: String,
        offset: u64,
        len: u32,
    },
    FileEnd {
        id: String,
    },
    FileAccept {
        id: String,
    },
    FileReject {
        id: String,
        reason: String,
    },
    /// 剪贴板
    Clipboard {
        content: String,
    },
    /// 心跳
    Ping,
    Pong,
    /// 断开
    Disconnect {
        reason: String,
    },
}
