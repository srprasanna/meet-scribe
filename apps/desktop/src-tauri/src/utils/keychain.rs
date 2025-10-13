//! Secure API key storage using OS keychain
//!
//! This module provides secure storage for API keys using:
//! - Windows: Windows Credential Manager
//! - Linux: Secret Service (GNOME Keyring, KWallet)
//! - macOS: macOS Keychain (future support)

use crate::error::{AppError, Result};
use keyring::Entry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Keychain service name for Meet Scribe
const SERVICE_NAME: &str = "com.srprasanna.meet-scribe";

/// Trait for keychain operations - allows for mocking in tests
pub trait KeychainPort: Send + Sync {
    fn save_api_key(&self, service_type: &str, provider: &str, api_key: &str) -> Result<()>;
    fn get_api_key(&self, service_type: &str, provider: &str) -> Result<String>;
    fn delete_api_key(&self, service_type: &str, provider: &str) -> Result<()>;
    fn has_api_key(&self, service_type: &str, provider: &str) -> bool;
}

/// Keychain manager for secure API key storage using OS keychain
pub struct KeychainManager;

impl KeychainPort for KeychainManager {
    fn save_api_key(&self, service_type: &str, provider: &str, api_key: &str) -> Result<()> {
        let account = format!("{}_{}", service_type, provider);
        let entry = Entry::new(SERVICE_NAME, &account)
            .map_err(|e| AppError::KeychainError(e.to_string()))?;

        entry
            .set_password(api_key)
            .map_err(|e| AppError::KeychainError(format!("Failed to save API key: {}", e)))?;

        log::info!("API key saved for {}:{}", service_type, provider);
        Ok(())
    }

    fn get_api_key(&self, service_type: &str, provider: &str) -> Result<String> {
        let account = format!("{}_{}", service_type, provider);
        let entry = Entry::new(SERVICE_NAME, &account)
            .map_err(|e| AppError::KeychainError(e.to_string()))?;

        entry
            .get_password()
            .map_err(|e| AppError::KeychainError(format!("Failed to retrieve API key: {}", e)))
    }

    fn delete_api_key(&self, service_type: &str, provider: &str) -> Result<()> {
        let account = format!("{}_{}", service_type, provider);
        let entry = Entry::new(SERVICE_NAME, &account)
            .map_err(|e| AppError::KeychainError(e.to_string()))?;

        entry
            .delete_password()
            .map_err(|e| AppError::KeychainError(format!("Failed to delete API key: {}", e)))?;

        log::info!("API key deleted for {}:{}", service_type, provider);
        Ok(())
    }

    fn has_api_key(&self, service_type: &str, provider: &str) -> bool {
        self.get_api_key(service_type, provider).is_ok()
    }
}

impl KeychainManager {
    /// Creates a new KeychainManager instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for KeychainManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock keychain implementation for testing (in-memory storage)
#[derive(Clone, Default)]
pub struct MockKeychain {
    storage: Arc<Mutex<HashMap<String, String>>>,
}

impl MockKeychain {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl KeychainPort for MockKeychain {
    fn save_api_key(&self, service_type: &str, provider: &str, api_key: &str) -> Result<()> {
        let key = format!("{}_{}", service_type, provider);
        self.storage
            .lock()
            .unwrap()
            .insert(key, api_key.to_string());
        Ok(())
    }

    fn get_api_key(&self, service_type: &str, provider: &str) -> Result<String> {
        let key = format!("{}_{}", service_type, provider);
        self.storage
            .lock()
            .unwrap()
            .get(&key)
            .cloned()
            .ok_or_else(|| AppError::KeychainError(format!("API key not found for {}", key)))
    }

    fn delete_api_key(&self, service_type: &str, provider: &str) -> Result<()> {
        let key = format!("{}_{}", service_type, provider);
        self.storage.lock().unwrap().remove(&key);
        Ok(())
    }

    fn has_api_key(&self, service_type: &str, provider: &str) -> bool {
        let key = format!("{}_{}", service_type, provider);
        self.storage.lock().unwrap().contains_key(&key)
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
    #[ignore] // Requires OS keychain access - skip in CI
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
        assert!(
            retrieve_result.is_ok(),
            "Should retrieve API key successfully"
        );
        assert_eq!(
            retrieve_result.unwrap(),
            api_key,
            "Retrieved key should match saved key"
        );

        // Cleanup
        let _ = manager.delete_api_key(service_type, provider);
    }

    #[test]
    #[ignore] // Requires OS keychain access - skip in CI
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
    #[ignore] // Requires OS keychain access - skip in CI
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
    #[ignore] // Requires OS keychain access - skip in CI
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
    #[ignore] // Requires OS keychain access - skip in CI
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
    #[ignore] // Requires OS keychain access - skip in CI
    fn test_get_nonexistent_key() {
        let manager = KeychainManager::new();
        let result = manager.get_api_key("nonexistent_service", "nonexistent_provider");
        assert!(result.is_err(), "Should return error for nonexistent key");
    }

    // Tests using MockKeychain - can run in CI without OS keychain access

    #[test]
    fn test_mock_new() {
        let mock = MockKeychain::new();
        assert!(true, "MockKeychain should be created successfully");
    }

    #[test]
    fn test_mock_default() {
        let mock = MockKeychain::default();
        assert!(true, "MockKeychain default should work");
    }

    #[test]
    fn test_mock_save_and_retrieve_api_key() {
        let mock = MockKeychain::new();
        let service_type = "test_service";
        let provider = "test_provider";
        let api_key = "test_api_key_12345";

        // Save API key
        let save_result = mock.save_api_key(service_type, provider, api_key);
        assert!(save_result.is_ok(), "Should save API key successfully");

        // Retrieve API key
        let retrieve_result = mock.get_api_key(service_type, provider);
        assert!(
            retrieve_result.is_ok(),
            "Should retrieve API key successfully"
        );
        assert_eq!(
            retrieve_result.unwrap(),
            api_key,
            "Retrieved key should match saved key"
        );
    }

    #[test]
    fn test_mock_has_api_key() {
        let mock = MockKeychain::new();
        let service_type = "test_has";
        let provider = "test_provider_has";

        // Initially should not have key
        assert!(
            !mock.has_api_key(service_type, provider),
            "Should not have key initially"
        );

        // Save a key
        let _ = mock.save_api_key(service_type, provider, "test_key");

        // Now should have key
        assert!(
            mock.has_api_key(service_type, provider),
            "Should have key after saving"
        );
    }

    #[test]
    fn test_mock_delete_api_key() {
        let mock = MockKeychain::new();
        let service_type = "test_delete";
        let provider = "test_provider_delete";
        let api_key = "test_key_to_delete";

        // Save a key
        let _ = mock.save_api_key(service_type, provider, api_key);
        assert!(
            mock.has_api_key(service_type, provider),
            "Key should exist before deletion"
        );

        // Delete the key
        let delete_result = mock.delete_api_key(service_type, provider);
        assert!(delete_result.is_ok(), "Should delete key successfully");

        // Verify deletion
        assert!(
            !mock.has_api_key(service_type, provider),
            "Key should not exist after deletion"
        );
    }

    #[test]
    fn test_mock_overwrite_api_key() {
        let mock = MockKeychain::new();
        let service_type = "test_overwrite";
        let provider = "test_provider_overwrite";
        let old_key = "old_key_123";
        let new_key = "new_key_456";

        // Save initial key
        let _ = mock.save_api_key(service_type, provider, old_key);

        // Overwrite with new key
        let _ = mock.save_api_key(service_type, provider, new_key);

        // Verify new key is saved
        let retrieved = mock.get_api_key(service_type, provider).unwrap();
        assert_eq!(
            retrieved, new_key,
            "Should retrieve the new overwritten key"
        );
    }

    #[test]
    fn test_mock_multiple_providers() {
        let mock = MockKeychain::new();
        let service_type = "test_multi";
        let provider1 = "provider1";
        let provider2 = "provider2";
        let key1 = "key_for_provider1";
        let key2 = "key_for_provider2";

        // Save keys for different providers
        let _ = mock.save_api_key(service_type, provider1, key1);
        let _ = mock.save_api_key(service_type, provider2, key2);

        // Verify both keys exist independently
        assert_eq!(
            mock.get_api_key(service_type, provider1).unwrap(),
            key1,
            "Provider 1 key should match"
        );
        assert_eq!(
            mock.get_api_key(service_type, provider2).unwrap(),
            key2,
            "Provider 2 key should match"
        );
    }

    #[test]
    fn test_mock_get_nonexistent_key() {
        let mock = MockKeychain::new();
        let result = mock.get_api_key("nonexistent_service", "nonexistent_provider");
        assert!(result.is_err(), "Should return error for nonexistent key");
    }
}
