//! 加密隧道 — X25519 密钥交换 + ChaCha20-Poly1305 对称加密
//!
//! 所有远程桌面数据通过此隧道传输，确保端到端加密。

use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use std::sync::atomic::{AtomicU64, Ordering};
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};

/// 加密隧道上下文
pub struct SecureTunnel {
    cipher: ChaCha20Poly1305,
    nonce_counter: AtomicU64,
    send_nonce_prefix: u32,
    receive_nonce_prefix: u32,
}

impl SecureTunnel {
    /// 从共享密钥创建隧道
    pub fn from_shared_secret(shared: &[u8; 32]) -> Self {
        Self::from_shared_secret_with_prefixes(shared, 0, 0)
    }

    pub fn from_shared_secret_with_prefixes(
        shared: &[u8; 32],
        send_nonce_prefix: u32,
        receive_nonce_prefix: u32,
    ) -> Self {
        let cipher = ChaCha20Poly1305::new(shared.into());
        Self {
            cipher,
            nonce_counter: AtomicU64::new(0),
            send_nonce_prefix,
            receive_nonce_prefix,
        }
    }

    /// 生成密钥对 (用于 X25519 密钥交换)
    pub fn generate_keypair() -> (EphemeralSecret, PublicKey) {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        (secret, public)
    }

    pub fn generate_static_keypair() -> (StaticSecret, PublicKey) {
        let secret = StaticSecret::random_from_rng(OsRng);
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
        self.encrypt_packet(plaintext)
            .map(|(_, ciphertext)| ciphertext)
    }

    pub fn encrypt_packet(&self, plaintext: &[u8]) -> Result<(u64, Vec<u8>), String> {
        let counter = self.nonce_counter.fetch_add(1, Ordering::SeqCst);
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[0..4].copy_from_slice(&self.send_nonce_prefix.to_le_bytes());
        nonce_bytes[4..12].copy_from_slice(&counter.to_le_bytes());
        let nonce = Nonce::from(nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| format!("Encrypt failed: {}", e))?;
        Ok((counter, ciphertext))
    }

    /// 解密数据
    pub fn decrypt(&self, ciphertext: &[u8], nonce_counter: u64) -> Result<Vec<u8>, String> {
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[0..4].copy_from_slice(&self.receive_nonce_prefix.to_le_bytes());
        nonce_bytes[4..12].copy_from_slice(&nonce_counter.to_le_bytes());
        let nonce = Nonce::from(nonce_bytes);

        self.cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|e| format!("Decrypt failed: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::SecureTunnel;

    #[test]
    fn directional_tunnels_encrypt_both_directions() {
        let shared = [42u8; 32];
        let left = SecureTunnel::from_shared_secret_with_prefixes(&shared, 0, 1);
        let right = SecureTunnel::from_shared_secret_with_prefixes(&shared, 1, 0);

        let (left_nonce, left_ciphertext) = left.encrypt_packet(b"left to right").unwrap();
        assert_eq!(
            right.decrypt(&left_ciphertext, left_nonce).unwrap(),
            b"left to right"
        );

        let (right_nonce, right_ciphertext) = right.encrypt_packet(b"right to left").unwrap();
        assert_eq!(
            left.decrypt(&right_ciphertext, right_nonce).unwrap(),
            b"right to left"
        );
        assert_ne!(left_ciphertext, right_ciphertext);
    }

    #[test]
    fn modified_ciphertext_is_rejected() {
        let shared = [7u8; 32];
        let sender = SecureTunnel::from_shared_secret_with_prefixes(&shared, 0, 1);
        let receiver = SecureTunnel::from_shared_secret_with_prefixes(&shared, 1, 0);
        let (nonce, mut ciphertext) = sender.encrypt_packet(b"authenticated").unwrap();
        ciphertext[0] ^= 1;
        assert!(receiver.decrypt(&ciphertext, nonce).is_err());
    }
}
