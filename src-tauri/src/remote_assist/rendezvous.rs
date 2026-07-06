//! 信令/ID 服务器通信模块
//!
//! 负责：
//! 1. 向信令服务器注册本机 ID
//! 2. 查询远程设备的网络地址
//! 3. 协调 P2P 打洞过程
//! 4. 保持在线心跳

use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::types::ServerConfig;

/// 信令服务器默认端口
const RENDEZVOUS_PORT: u16 = 21116;

/// 信令客户端
pub struct RendezvousClient {
    server_addr: String,
    device_id: String,
    connected: bool,
}

impl RendezvousClient {
    pub fn new(server: &ServerConfig, device_id: &str) -> Self {
        Self {
            server_addr: format!("{}:{}", server.host, RENDEZVOUS_PORT),
            device_id: device_id.to_string(),
            connected: false,
        }
    }

    /// 向信令服务器注册本机
    pub async fn register(&mut self) -> Result<(), String> {
        let mut stream = TcpStream::connect(&self.server_addr)
            .await
            .map_err(|e| format!("连接信令服务器失败: {}", e))?;

        // 发送注册消息
        let register_msg = serde_json::json!({
            "type": "register",
            "id": self.device_id,
            "version": "5.0.0",
            "platform": "windows",
        });
        let data = serde_json::to_vec(&register_msg).map_err(|e| e.to_string())?;

        let len = (data.len() as u32).to_be_bytes();
        stream.write_all(&len).await.map_err(|e| e.to_string())?;
        stream.write_all(&data).await.map_err(|e| e.to_string())?;

        self.connected = true;
        Ok(())
    }

    /// 查询远程设备地址
    pub async fn lookup_peer(&self, peer_id: &str) -> Result<PeerInfo, String> {
        let mut stream = TcpStream::connect(&self.server_addr)
            .await
            .map_err(|e| format!("连接信令服务器失败: {}", e))?;

        let lookup_msg = serde_json::json!({
            "type": "lookup",
            "target_id": peer_id,
            "from_id": self.device_id,
        });
        let data = serde_json::to_vec(&lookup_msg).map_err(|e| e.to_string())?;

        let len = (data.len() as u32).to_be_bytes();
        stream.write_all(&len).await.map_err(|e| e.to_string())?;
        stream.write_all(&data).await.map_err(|e| e.to_string())?;

        // 读取响应
        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| format!("读取响应失败: {}", e))?;
        let resp_len = u32::from_be_bytes(len_buf) as usize;

        let mut resp_buf = vec![0u8; resp_len.min(65536)];
        stream
            .read_exact(&mut resp_buf)
            .await
            .map_err(|e| format!("读取响应数据失败: {}", e))?;

        let resp: serde_json::Value =
            serde_json::from_slice(&resp_buf).map_err(|e| format!("解析响应失败: {}", e))?;

        let addr_str = resp
            .get("addr")
            .and_then(|v| v.as_str())
            .ok_or("响应中无地址信息")?;
        let addr: SocketAddr = addr_str
            .parse()
            .map_err(|e| format!("解析地址失败: {}", e))?;

        Ok(PeerInfo {
            id: peer_id.to_string(),
            addr,
            nat_type: resp
                .get("nat_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            online: true,
        })
    }

    /// 发送心跳保持在线
    pub async fn heartbeat(&self) -> Result<(), String> {
        if !self.connected {
            return Err("未连接到信令服务器".into());
        }
        // 心跳通过 UDP 发送以减少开销
        let socket = tokio::net::UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| e.to_string())?;

        let heartbeat = format!("HB:{}", self.device_id);
        socket
            .send_to(heartbeat.as_bytes(), &self.server_addr)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

/// 远程设备信息
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub id: String,
    pub addr: SocketAddr,
    pub nat_type: String,
    pub online: bool,
}
