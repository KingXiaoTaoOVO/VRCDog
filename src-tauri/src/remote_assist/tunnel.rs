//! 加密隧道 — X25519 密钥交换 + ChaCha20-Poly1305 对称加密
//!
//! 所有远程桌面数据通过此隧道传输，确保端到端加密。

use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use std::sync::atomic::{AtomicU64, Ordering};
use x25519_dalek::{EphemeralSecret, PublicKey};

/// 加密隧道上下文
pub struct SecureTunnel {
    cipher: ChaCha20Poly1305,
    nonce_counter: AtomicU64,
}

impl SecureTunnel {
    /// 从共享密钥创建隧道
    pub fn from_shared_secret(shared: &[u8; 32]) -> Self {
        let cipher = ChaCha20Poly1305::new(shared.into());
        Self {
            cipher,
            nonce_counter: AtomicU64::new(0),
        }
    }

    /// 生成密钥对 (用于 X25519 密钥交换)
    pub fn generate_keypair() -> (EphemeralSecret, PublicKey) {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        (secret, public)
    }

    /// 执行密钥交换，返回共享密钥
    pub fn key_exchange(our_secret: EphemeralSecret, their_public: &PublicKey) -> [u8; 32] {
        let shared = our_secret.diffie_hellman(their_public);
        *shared.as_bytes()
    }

    /// 加密数据
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let counter = self.nonce_counter.fetch_add(1, Ordering::SeqCst);
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..12].copy_from_slice(&counter.to_le_bytes());
        let nonce = Nonce::from(nonce_bytes);

        self.cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| format!("Encrypt failed: {}", e))
    }

    /// 解密数据
    pub fn decrypt(&self, ciphertext: &[u8], nonce_counter: u64) -> Result<Vec<u8>, String> {
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..12].copy_from_slice(&nonce_counter.to_le_bytes());
        let nonce = Nonce::from(nonce_bytes);

        self.cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|e| format!("Decrypt failed: {}", e))
    }
}
