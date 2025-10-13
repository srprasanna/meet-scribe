//! Secure API key storage using OS keychain
//!
//! This module provides secure storage for API keys using:
//! - Windows: Windows Credential Manager
//! - Linux: Secret Service (GNOME Keyring, KWallet)
//! - macOS: macOS Keychain (future support)

use crate::error::{AppError, Result};
use keyring::Entry;

/// Keychain service name for Meet Scribe
const SERVICE_NAME: &str = "com.srprasanna.meet-scribe";

/// Keychain manager for secure API key storage
pub struct KeychainManager;

impl KeychainManager {
    /// Creates a new KeychainManager instance
    pub fn new() -> Self {
        Self
    }

    /// Saves an API key to the OS keychain
    ///
    /// # Arguments
    /// * `service_type` - Type of service (e.g., "asr", "llm")
    /// * `provider` - Provider name (e.g., "deepgram", "anthropic")
    /// * `api_key` - The API key to store
    ///
    /// # Example
    /// ```
    /// let manager = KeychainManager::new();
    /// manager.save_api_key("asr", "deepgram", "sk-...")?;
    /// ```
    pub fn save_api_key(
        &self,
        service_type: &str,
        provider: &str,
        api_key: &str,
    ) -> Result<()> {
        let account = format!("{}_{}", service_type, provider);
        let entry = Entry::new(SERVICE_NAME, &account)
            .map_err(|e| AppError::KeychainError(e.to_string()))?;

        entry
            .set_password(api_key)
            .map_err(|e| AppError::KeychainError(format!("Failed to save API key: {}", e)))?;

        log::info!("API key saved for {}:{}", service_type, provider);
        Ok(())
    }

    /// Retrieves an API key from the OS keychain
    ///
    /// # Arguments
    /// * `service_type` - Type of service (e.g., "asr", "llm")
    /// * `provider` - Provider name (e.g., "deepgram", "anthropic")
    ///
    /// # Returns
    /// The API key if found, or an error if not found or access denied
    ///
    /// # Example
    /// ```
    /// let manager = KeychainManager::new();
    /// let api_key = manager.get_api_key("asr", "deepgram")?;
    /// ```
    pub fn get_api_key(&self, service_type: &str, provider: &str) -> Result<String> {
        let account = format!("{}_{}", service_type, provider);
        let entry = Entry::new(SERVICE_NAME, &account)
            .map_err(|e| AppError::KeychainError(e.to_string()))?;

        entry
            .get_password()
            .map_err(|e| AppError::KeychainError(format!("Failed to retrieve API key: {}", e)))
    }

    /// Deletes an API key from the OS keychain
    ///
    /// # Arguments
    /// * `service_type` - Type of service (e.g., "asr", "llm")
    /// * `provider` - Provider name (e.g., "deepgram", "anthropic")
    ///
    /// # Example
    /// ```
    /// let manager = KeychainManager::new();
    /// manager.delete_api_key("asr", "deepgram")?;
    /// ```
    pub fn delete_api_key(&self, service_type: &str, provider: &str) -> Result<()> {
        let account = format!("{}_{}", service_type, provider);
        let entry = Entry::new(SERVICE_NAME, &account)
            .map_err(|e| AppError::KeychainError(e.to_string()))?;

        entry
            .delete_password()
            .map_err(|e| AppError::KeychainError(format!("Failed to delete API key: {}", e)))?;

        log::info!("API key deleted for {}:{}", service_type, provider);
        Ok(())
    }

    /// Checks if an API key exists in the keychain
    ///
    /// # Arguments
    /// * `service_type` - Type of service (e.g., "asr", "llm")
    /// * `provider` - Provider name (e.g., "deepgram", "anthropic")
    ///
    /// # Returns
    /// `true` if the key exists, `false` otherwise
    pub fn has_api_key(&self, service_type: &str, provider: &str) -> bool {
        self.get_api_key(service_type, provider).is_ok()
    }
}

impl Default for KeychainManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_keychain_manager() {
        let manager = KeychainManager::new();
        assert!(true, "KeychainManager should be created successfully");
    }

    #[test]
    fn test_default_keychain_manager() {
        let manager = KeychainManager::default();
        assert!(true, "KeychainManager default should work");
    }

    #[test]
    fn test_save_and_retrieve_api_key() {
        let manager = KeychainManager::new();
        let service_type = "test_service";
        let provider = "test_provider";
        let api_key = "test_api_key_12345";

        // Save API key
        let save_result = manager.save_api_key(service_type, provider, api_key);
        assert!(save_result.is_ok(), "Should save API key successfully");

        // Retrieve API key
        let retrieve_result = manager.get_api_key(service_type, provider);
        assert!(retrieve_result.is_ok(), "Should retrieve API key successfully");
        assert_eq!(
            retrieve_result.unwrap(),
            api_key,
            "Retrieved key should match saved key"
        );

        // Cleanup
        let _ = manager.delete_api_key(service_type, provider);
    }

    #[test]
    fn test_has_api_key() {
        let manager = KeychainManager::new();
        let service_type = "test_has";
        let provider = "test_provider_has";

        // Initially should not have key
        assert!(
            !manager.has_api_key(service_type, provider),
            "Should not have key initially"
        );

        // Save a key
        let _ = manager.save_api_key(service_type, provider, "test_key");

        // Now should have key
        assert!(
            manager.has_api_key(service_type, provider),
            "Should have key after saving"
        );

        // Cleanup
        let _ = manager.delete_api_key(service_type, provider);
    }

    #[test]
    fn test_delete_api_key() {
        let manager = KeychainManager::new();
        let service_type = "test_delete";
        let provider = "test_provider_delete";
        let api_key = "test_key_to_delete";

        // Save a key
        let _ = manager.save_api_key(service_type, provider, api_key);
        assert!(
            manager.has_api_key(service_type, provider),
            "Key should exist before deletion"
        );

        // Delete the key
        let delete_result = manager.delete_api_key(service_type, provider);
        assert!(delete_result.is_ok(), "Should delete key successfully");

        // Verify deletion
        assert!(
            !manager.has_api_key(service_type, provider),
            "Key should not exist after deletion"
        );
    }

    #[test]
    fn test_overwrite_api_key() {
        let manager = KeychainManager::new();
        let service_type = "test_overwrite";
        let provider = "test_provider_overwrite";
        let old_key = "old_key_123";
        let new_key = "new_key_456";

        // Save initial key
        let _ = manager.save_api_key(service_type, provider, old_key);

        // Overwrite with new key
        let _ = manager.save_api_key(service_type, provider, new_key);

        // Verify new key is saved
        let retrieved = manager.get_api_key(service_type, provider).unwrap();
        assert_eq!(
            retrieved, new_key,
            "Should retrieve the new overwritten key"
        );

        // Cleanup
        let _ = manager.delete_api_key(service_type, provider);
    }

    #[test]
    fn test_multiple_providers() {
        let manager = KeychainManager::new();
        let service_type = "test_multi";
        let provider1 = "provider1";
        let provider2 = "provider2";
        let key1 = "key_for_provider1";
        let key2 = "key_for_provider2";

        // Save keys for different providers
        let _ = manager.save_api_key(service_type, provider1, key1);
        let _ = manager.save_api_key(service_type, provider2, key2);

        // Verify both keys exist independently
        assert_eq!(
            manager.get_api_key(service_type, provider1).unwrap(),
            key1,
            "Provider 1 key should match"
        );
        assert_eq!(
            manager.get_api_key(service_type, provider2).unwrap(),
            key2,
            "Provider 2 key should match"
        );

        // Cleanup
        let _ = manager.delete_api_key(service_type, provider1);
        let _ = manager.delete_api_key(service_type, provider2);
    }

    #[test]
    fn test_get_nonexistent_key() {
        let manager = KeychainManager::new();
        let result = manager.get_api_key("nonexistent_service", "nonexistent_provider");
        assert!(result.is_err(), "Should return error for nonexistent key");
    }
}
