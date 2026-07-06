//! 中继服务器模块 — P2P 失败时的回退方案
//!
//! 当两端都是对称 NAT 或打洞失败时，通过中继服务器转发数据。
//! 中继服务器只转发加密数据，无法解密内容。

use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// 中继服务器默认端口
const RELAY_PORT: u16 = 21117;

/// 中继连接
pub struct RelayConnection {
    stream: TcpStream,
    session_id: String,
}

impl RelayConnection {
    /// 连接到中继服务器并请求转发会话
    pub async fn connect(relay_host: &str, our_id: &str, peer_id: &str) -> Result<Self, String> {
        let addr = format!("{}:{}", relay_host, RELAY_PORT);
        let mut stream = tokio::time::timeout(Duration::from_secs(10), TcpStream::connect(&addr))
            .await
            .map_err(|_| "连接中继服务器超时".to_string())?
            .map_err(|e| format!("连接中继服务器失败: {}", e))?;

        // 发送中继请求
        let session_id = uuid::Uuid::new_v4().to_string();
        let relay_req = serde_json::json!({
            "type": "relay_request",
            "from": our_id,
            "to": peer_id,
            "session": &session_id,
        });
        let data = serde_json::to_vec(&relay_req).map_err(|e| e.to_string())?;

        let len = (data.len() as u32).to_be_bytes();
        stream.write_all(&len).await.map_err(|e| e.to_string())?;
        stream.write_all(&data).await.map_err(|e| e.to_string())?;

        // 等待中继确认
        let mut len_buf = [0u8; 4];
        tokio::time::timeout(Duration::from_secs(10), stream.read_exact(&mut len_buf))
            .await
            .map_err(|_| "等待中继确认超时")?
            .map_err(|e| format!("读取中继响应失败: {}", e))?;

        let resp_len = u32::from_be_bytes(len_buf) as usize;
        let mut resp_buf = vec![0u8; resp_len.min(4096)];
        stream
            .read_exact(&mut resp_buf)
            .await
            .map_err(|e| format!("读取中继数据失败: {}", e))?;

        Ok(Self { stream, session_id })
    }

    /// 通过中继发送数据
    pub async fn send(&mut self, data: &[u8]) -> Result<(), String> {
        let len = (data.len() as u32).to_be_bytes();
        self.stream
            .write_all(&len)
            .await
            .map_err(|e| format!("中继发送失败: {}", e))?;
        self.stream
            .write_all(data)
            .await
            .map_err(|e| format!("中继发送数据失败: {}", e))?;
        Ok(())
    }

    /// 从中继接收数据
    pub async fn recv(&mut self) -> Result<Vec<u8>, String> {
        let mut len_buf = [0u8; 4];
        self.stream
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| format!("中继接收失败: {}", e))?;
        let data_len = u32::from_be_bytes(len_buf) as usize;

        if data_len > 16 * 1024 * 1024 {
            return Err("数据包过大".into());
        }

        let mut data = vec![0u8; data_len];
        self.stream
            .read_exact(&mut data)
            .await
            .map_err(|e| format!("中继接收数据失败: {}", e))?;
        Ok(data)
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}
