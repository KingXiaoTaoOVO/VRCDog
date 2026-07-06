//! NAT 穿透模块 — STUN 探测 + UDP/TCP 打洞
//!
//! 实现 P2P 直连的核心：
//! 1. STUN 探测获取公网地址和 NAT 类型
//! 2. UDP 打洞 (对称 NAT 除外)
//! 3. TCP 打洞 (同时连接技术)
//! 4. 失败时回退到中继服务器

use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;
use tokio::net::{TcpStream, UdpSocket as TokioUdp};

/// NAT 类型
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub enum NatType {
    /// 无 NAT (公网 IP)
    Open,
    /// 完全锥形 NAT (最容易穿透)
    FullCone,
    /// 受限锥形 NAT
    RestrictedCone,
    /// 端口受限锥形 NAT
    PortRestricted,
    /// 对称 NAT (最难穿透，通常需要中继)
    Symmetric,
    /// 未知
    Unknown,
}

impl NatType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NatType::Open => "open",
            NatType::FullCone => "full_cone",
            NatType::RestrictedCone => "restricted_cone",
            NatType::PortRestricted => "port_restricted",
            NatType::Symmetric => "symmetric",
            NatType::Unknown => "unknown",
        }
    }

    /// 是否可以进行 P2P 打洞
    pub fn can_hole_punch(&self) -> bool {
        !matches!(self, NatType::Symmetric | NatType::Unknown)
    }
}

/// STUN 探测结果
#[derive(Debug, Clone)]
pub struct StunResult {
    pub nat_type: NatType,
    pub public_addr: Option<SocketAddr>,
    pub local_addr: SocketAddr,
}

/// 执行 STUN 探测
pub fn detect_nat() -> StunResult {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => {
            return StunResult {
                nat_type: NatType::Unknown,
                public_addr: None,
                local_addr: "0.0.0.0:0".parse().unwrap(),
            }
        }
    };

    socket.set_read_timeout(Some(Duration::from_secs(3))).ok();
    let local_addr = socket.local_addr().unwrap_or("0.0.0.0:0".parse().unwrap());

    // 尝试连接 STUN 服务器获取公网地址
    let stun_servers = [
        "stun.l.google.com:19302",
        "stun1.l.google.com:19302",
        "stun.stunprotocol.org:3478",
    ];

    for server in &stun_servers {
        if let Ok(_) = socket.connect(*server) {
            // 简化的 STUN 绑定请求
            let binding_request = build_stun_binding_request();
            if socket.send(&binding_request).is_ok() {
                let mut buf = [0u8; 256];
                if let Ok(n) = socket.recv(&mut buf) {
                    if let Some(addr) = parse_stun_response(&buf[..n]) {
                        let nat_type = if addr.port() == local_addr.port() {
                            NatType::FullCone
                        } else {
                            NatType::PortRestricted
                        };
                        return StunResult {
                            nat_type,
                            public_addr: Some(addr),
                            local_addr,
                        };
                    }
                }
            }
        }
    }

    StunResult {
        nat_type: NatType::Unknown,
        public_addr: None,
        local_addr,
    }
}

/// UDP 打洞
pub async fn udp_hole_punch(
    local_port: u16,
    peer_addr: SocketAddr,
    timeout_ms: u64,
) -> Result<TokioUdp, String> {
    let socket = TokioUdp::bind(format!("0.0.0.0:{}", local_port))
        .await
        .map_err(|e| format!("绑定 UDP 失败: {}", e))?;

    // 发送打洞包 (多次尝试)
    let punch_data = b"VRCDOG_PUNCH";
    for _ in 0..5 {
        let _ = socket.send_to(punch_data, peer_addr).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // 等待对方的打洞包
    let mut buf = [0u8; 64];
    match tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        socket.recv_from(&mut buf),
    )
    .await
    {
        Ok(Ok((_, addr))) if addr == peer_addr => Ok(socket),
        _ => Err("UDP 打洞超时".into()),
    }
}

/// TCP 打洞 (同时连接)
pub async fn tcp_hole_punch(peer_addr: SocketAddr, timeout_ms: u64) -> Result<TcpStream, String> {
    match tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        TcpStream::connect(peer_addr),
    )
    .await
    {
        Ok(Ok(stream)) => Ok(stream),
        Ok(Err(e)) => Err(format!("TCP 连接失败: {}", e)),
        Err(_) => Err("TCP 打洞超时".into()),
    }
}

// ─── STUN 协议实现 (RFC 5389 简化版) ────────────────────────────────────────

fn build_stun_binding_request() -> Vec<u8> {
    let mut msg = Vec::with_capacity(20);
    // Message Type: Binding Request (0x0001)
    msg.extend_from_slice(&[0x00, 0x01]);
    // Message Length: 0
    msg.extend_from_slice(&[0x00, 0x00]);
    // Magic Cookie: 0x2112A442
    msg.extend_from_slice(&[0x21, 0x12, 0xA4, 0x42]);
    // Transaction ID: 12 random bytes
    let tx_id: [u8; 12] = rand_bytes();
    msg.extend_from_slice(&tx_id);
    msg
}

fn parse_stun_response(data: &[u8]) -> Option<SocketAddr> {
    if data.len() < 20 {
        return None;
    }
    // Check it's a Binding Response (0x0101)
    if data[0] != 0x01 || data[1] != 0x01 {
        return None;
    }

    let msg_len = u16::from_be_bytes([data[2], data[3]]) as usize;
    let attrs = &data[20..20 + msg_len.min(data.len() - 20)];

    // Parse attributes looking for XOR-MAPPED-ADDRESS (0x0020)
    // or MAPPED-ADDRESS (0x0001)
    let mut offset = 0;
    while offset + 4 <= attrs.len() {
        let attr_type = u16::from_be_bytes([attrs[offset], attrs[offset + 1]]);
        let attr_len = u16::from_be_bytes([attrs[offset + 2], attrs[offset + 3]]) as usize;
        let attr_data = &attrs[offset + 4..offset + 4 + attr_len.min(attrs.len() - offset - 4)];

        if attr_type == 0x0020 && attr_data.len() >= 8 {
            // XOR-MAPPED-ADDRESS
            let port = u16::from_be_bytes([attr_data[2], attr_data[3]]) ^ 0x2112;
            let ip = [
                attr_data[4] ^ 0x21,
                attr_data[5] ^ 0x12,
                attr_data[6] ^ 0xA4,
                attr_data[7] ^ 0x42,
            ];
            let addr = SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3])),
                port,
            );
            return Some(addr);
        } else if attr_type == 0x0001 && attr_data.len() >= 8 {
            // MAPPED-ADDRESS
            let port = u16::from_be_bytes([attr_data[2], attr_data[3]]);
            let ip = [attr_data[4], attr_data[5], attr_data[6], attr_data[7]];
            let addr = SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3])),
                port,
            );
            return Some(addr);
        }

        // Align to 4 bytes
        offset += 4 + ((attr_len + 3) & !3);
    }

    None
}

fn rand_bytes<const N: usize>() -> [u8; N] {
    use std::time::SystemTime;
    let mut bytes = [0u8; N];
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut state = seed;
    for b in bytes.iter_mut() {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *b = (state >> 33) as u8;
    }
    bytes
}
