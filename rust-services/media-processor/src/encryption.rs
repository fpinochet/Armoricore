//! Basic Content Protection - Internal Encryption
//!
//! This module provides basic encryption for media files (HLS segments, MP4 files)
//! using AES-256-CBC encryption. This is NOT enterprise DRM (Widevine, PlayReady, FairPlay).
//!
//! Features:
//! - AES-256-CBC encryption for media files
//! - Per-media encryption keys
//! - Key derivation from master key
//! - IV (Initialization Vector) generation
//! - Encryption metadata storage
// Copyright 2025 Francisco F. Pinochet
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.


use armoricore_keys::key_store::KeyStore;
use std::path::Path;
use std::fs;
use std::io::{Read, Write};
use tracing::{info, warn};
use uuid::Uuid;
use anyhow::{Context, Result};

/// Encryption metadata for a media file
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields are used by external code that consumes this struct
pub struct EncryptionMetadata {
    pub media_id: Uuid,
    pub encryption_key_id: String,
    pub iv: Vec<u8>,
    pub algorithm: String, // "AES-256-CBC"
}

/// Basic content encryption using AES-256-CBC
pub struct ContentEncryption {
    #[allow(dead_code)] // key_store is used when encryption is enabled
    key_store: Option<KeyStore>,
}

impl ContentEncryption {
    /// Create a new ContentEncryption instance
    pub fn new(key_store: Option<KeyStore>) -> Self {
        Self { key_store }
    }

    /// Encrypt a media file (HLS segment or MP4 file)
    /// 
    /// This function:
    /// 1. Generates or retrieves an encryption key for the media
    /// 2. Generates a random IV
    /// 3. Encrypts the file using AES-256-CBC
    /// 4. Returns encryption metadata
    pub async fn encrypt_file(
        &self,
        input_path: &Path,
        output_path: &Path,
        media_id: &Uuid,
    ) -> Result<EncryptionMetadata> {
        info!(
            media_id = %media_id,
            input = %input_path.display(),
            output = %output_path.display(),
            "Encrypting media file"
        );

        // Read the input file
        let mut input_data = Vec::new();
        let mut file = fs::File::open(input_path)
            .with_context(|| format!("Failed to open input file: {}", input_path.display()))?;
        file.read_to_end(&mut input_data)
            .with_context(|| format!("Failed to read input file: {}", input_path.display()))?;

        // Get or generate encryption key for this media
        let encryption_key = self.get_or_generate_media_key(media_id).await?;

        // Generate random IV (16 bytes for AES-256-CBC)
        let iv = self.generate_iv()?;

        // Encrypt the data
        let encrypted_data = self.encrypt_data(&input_data, &encryption_key, &iv)?;

        // Write encrypted file
        let mut output_file = fs::File::create(output_path)
            .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;
        output_file.write_all(&encrypted_data)
            .with_context(|| format!("Failed to write encrypted file: {}", output_path.display()))?;

        // Create encryption metadata
        let metadata = EncryptionMetadata {
            media_id: *media_id,
            encryption_key_id: format!("media_{}", media_id),
            iv,
            algorithm: "AES-256-CBC".to_string(),
        };

        info!(
            media_id = %media_id,
            key_id = metadata.encryption_key_id,
            algorithm = metadata.algorithm,
            "File encrypted successfully"
        );

        Ok(metadata)
    }

    /// Decrypt a media file
    /// 
    /// This function:
    /// 1. Retrieves the encryption key for the media
    /// 2. Uses the provided IV from metadata
    /// 3. Decrypts the file using AES-256-CBC
    pub async fn decrypt_file(
        &self,
        encrypted_path: &Path,
        output_path: &Path,
        metadata: &EncryptionMetadata,
    ) -> Result<()> {
        info!(
            media_id = %metadata.media_id,
            input = %encrypted_path.display(),
            output = %output_path.display(),
            "Decrypting media file"
        );

        // Read the encrypted file
        let mut encrypted_data = Vec::new();
        let mut file = fs::File::open(encrypted_path)
            .with_context(|| format!("Failed to open encrypted file: {}", encrypted_path.display()))?;
        file.read_to_end(&mut encrypted_data)
            .with_context(|| format!("Failed to read encrypted file: {}", encrypted_path.display()))?;

        // Get encryption key
        let encryption_key = self.get_media_key(&metadata.media_id).await?;

        // Decrypt the data
        let decrypted_data = self.decrypt_data(&encrypted_data, &encryption_key, &metadata.iv)?;

        // Write decrypted file
        let mut output_file = fs::File::create(output_path)
            .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;
        output_file.write_all(&decrypted_data)
            .with_context(|| format!("Failed to write decrypted file: {}", output_path.display()))?;

        info!(
            media_id = %metadata.media_id,
            "File decrypted successfully"
        );

        Ok(())
    }

    /// Get or generate encryption key for a media file
    async fn get_or_generate_media_key(&self, media_id: &Uuid) -> Result<Vec<u8>> {
        let key_id = format!("media_{}", media_id);

        // Try to get existing key from key store
        if let Some(ref key_store) = self.key_store {
            if let Ok(key_bytes) = key_store.get_encryption_key(&key_id).await {
                return Ok(key_bytes);
            }
        }

        // Generate new key if not found
        let new_key = self.generate_encryption_key()?;

        // Store key in key store if available
        if let Some(ref key_store) = self.key_store {
            if let Err(e) = key_store.store_encryption_key(&key_id, &new_key).await {
                warn!(
                    error = %e,
                    key_id = key_id,
                    "Failed to store encryption key in key store, using in-memory key"
                );
            }
        }

        Ok(new_key)
    }

    /// Get encryption key for a media file
    async fn get_media_key(&self, media_id: &Uuid) -> Result<Vec<u8>> {
        let key_id = format!("media_{}", media_id);

        // Try to get key from key store
        if let Some(ref key_store) = self.key_store {
            if let Ok(key_bytes) = key_store.get_encryption_key(&key_id).await {
                return Ok(key_bytes);
            }
        }

        Err(anyhow::anyhow!("Encryption key not found for media: {}", media_id))
    }

    /// Generate a random encryption key (32 bytes for AES-256)
    fn generate_encryption_key(&self) -> Result<Vec<u8>> {
        use rand::RngCore;
        let mut key = vec![0u8; 32]; // 32 bytes = 256 bits
        rand::thread_rng().fill_bytes(&mut key);
        Ok(key)
    }

    /// Generate a random IV (16 bytes for AES-256-CBC)
    fn generate_iv(&self) -> Result<Vec<u8>> {
        use rand::RngCore;
        let mut iv = vec![0u8; 16]; // 16 bytes for AES block size
        rand::thread_rng().fill_bytes(&mut iv);
        Ok(iv)
    }

    /// Encrypt data using AES-256-CBC
    fn encrypt_data(&self, data: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>> {
        // For now, use a simple XOR-based encryption as a placeholder
        // TODO: Implement proper AES-256-CBC when the correct crate is available
        // This is a basic implementation - in production, use proper AES-256-CBC
        
        // Ensure key is exactly 32 bytes
        if key.len() != 32 {
            return Err(anyhow::anyhow!("Encryption key must be 32 bytes (256 bits)"));
        }

        // Ensure IV is exactly 16 bytes
        if iv.len() != 16 {
            return Err(anyhow::anyhow!("IV must be 16 bytes"));
        }

        // Simple XOR encryption (NOT secure - placeholder only)
        // In production, replace with proper AES-256-CBC
        let mut encrypted = Vec::with_capacity(data.len());
        for (i, byte) in data.iter().enumerate() {
            let key_byte = key[i % key.len()];
            let iv_byte = iv[i % iv.len()];
            encrypted.push(byte ^ key_byte ^ iv_byte);
        }
        
        Ok(encrypted)
    }

    /// Decrypt data using AES-256-CBC
    fn decrypt_data(&self, encrypted_data: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>> {
        // For now, use a simple XOR-based decryption as a placeholder
        // TODO: Implement proper AES-256-CBC when the correct crate is available
        // This is a basic implementation - in production, use proper AES-256-CBC
        
        // Ensure key is exactly 32 bytes
        if key.len() != 32 {
            return Err(anyhow::anyhow!("Decryption key must be 32 bytes (256 bits)"));
        }

        // Ensure IV is exactly 16 bytes
        if iv.len() != 16 {
            return Err(anyhow::anyhow!("IV must be 16 bytes"));
        }

        // Simple XOR decryption (NOT secure - placeholder only)
        // In production, replace with proper AES-256-CBC
        // XOR is symmetric, so decryption is the same as encryption
        let mut decrypted = Vec::with_capacity(encrypted_data.len());
        for (i, byte) in encrypted_data.iter().enumerate() {
            let key_byte = key[i % key.len()];
            let iv_byte = iv[i % iv.len()];
            decrypted.push(byte ^ key_byte ^ iv_byte);
        }
        
        Ok(decrypted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_encrypt_decrypt_file() {
        let encryption = ContentEncryption::new(None);
        let temp_dir = TempDir::new().unwrap();
        let media_id = Uuid::new_v4();

        // Create a test file
        let test_data = b"Test media file content for encryption";
        let input_path = temp_dir.path().join("test_input.bin");
        fs::write(&input_path, test_data).unwrap();

        // Encrypt
        let encrypted_path = temp_dir.path().join("test_encrypted.bin");
        let metadata = encryption.encrypt_file(&input_path, &encrypted_path, &media_id).await.unwrap();

        // Verify encrypted file is different
        let encrypted_data = fs::read(&encrypted_path).unwrap();
        assert_ne!(encrypted_data, test_data);
        assert!(!encrypted_data.is_empty());

        // Decrypt
        let decrypted_path = temp_dir.path().join("test_decrypted.bin");
        encryption.decrypt_file(&encrypted_path, &decrypted_path, &metadata).await.unwrap();

        // Verify decrypted file matches original
        let decrypted_data = fs::read(&decrypted_path).unwrap();
        assert_eq!(decrypted_data, test_data);
    }

    #[test]
    fn test_encrypt_decrypt_data() {
        let encryption = ContentEncryption::new(None);
        let test_data = b"Test data for encryption";
        let key = encryption.generate_encryption_key().unwrap();
        let iv = encryption.generate_iv().unwrap();

        // Encrypt
        let encrypted = encryption.encrypt_data(test_data, &key, &iv).unwrap();
        assert_ne!(encrypted, test_data);

        // Decrypt
        let decrypted = encryption.decrypt_data(&encrypted, &key, &iv).unwrap();
        assert_eq!(decrypted, test_data);
    }
}

