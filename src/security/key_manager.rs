// geometric_crypto/src/security/key_manager.rs
use blake3;
use super::SecurityError; // From security/mod.rs

#[derive(Debug, Clone, Default)] // Added Default
pub struct KeyManager {
    master_key: Option<[u8; 32]>,
}

impl KeyManager {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_master_key(&mut self, key: [u8; 32]) {
        self.master_key = Some(key);
    }

    pub fn is_key_set(&self) -> bool {
        self.master_key.is_some()
    }

    /// Derives a key for a specific purpose using Blake3 keyed hash or HKDF-like expansion.
    /// The output length is fixed by Blake3 hash size (32 bytes) if directly using hash output,
    /// or can be specified if using an XOF or HKDF. For simplicity, fixed 32-byte output for now.
    pub fn get_derived_key(
        &self,
        purpose: &[u8], // e.g., b"obfuscation_coord_key", b"integrity_mac_key"
        // output_len: usize, // To make it more like HKDF, but blake3::derive_key is simpler for fixed size
    ) -> Result<[u8; 32], SecurityError> {
        let master_key = self.master_key.as_ref().ok_or_else(|| {
            SecurityError::KeyExchangeError("Master key not set in KeyManager".to_string())
        })?;

        // Use blake3::derive_key for domain separation.
        // The context string should be unique and fixed for the application.
        // For deriving multiple keys, the 'purpose' can be part of the context
        // or used to further process the derived key if more advanced KDF is needed.
        // For this, let's use the 'purpose' as the context string for blake3::derive_key.
        // It expects a string, so convert purpose bytes if necessary, or ensure purpose is string.
        // For simplicity, let's assume purpose is a valid UTF-8 string for context.
        let context_str = std::str::from_utf8(purpose).map_err(|_| SecurityError::KeyExchangeError("Purpose for key derivation is not valid UTF-8".to_string()))?;
        
        let mut derived_key = [0u8; 32];
        blake3::derive_key(context_str, master_key, &mut derived_key);
        Ok(derived_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MASTER_KEY: [u8; 32] = [42u8; 32];

    #[test]
    fn test_key_manager_new_and_set_key() {
        let mut km = KeyManager::new();
        assert!(!km.is_key_set());
        km.set_master_key(TEST_MASTER_KEY);
        assert!(km.is_key_set());
    }

    #[test]
    fn test_get_derived_key_no_master_key() {
        let km = KeyManager::new();
        let result = km.get_derived_key(b"test_purpose");
        assert!(result.is_err());
        match result.err().unwrap() {
            SecurityError::KeyExchangeError(msg) => assert!(msg.contains("Master key not set")),
            _ => panic!("Unexpected error type"),
        }
    }

    #[test]
    fn test_get_derived_key_success() {
        let mut km = KeyManager::new();
        km.set_master_key(TEST_MASTER_KEY);
        
        let key1_result = km.get_derived_key(b"purpose1");
        assert!(key1_result.is_ok());
        let key1 = key1_result.unwrap();
        assert_eq!(key1.len(), 32);

        // Check determinism: Same purpose, same key
        let key1_again_result = km.get_derived_key(b"purpose1");
        assert!(key1_again_result.is_ok());
        assert_eq!(key1, key1_again_result.unwrap());

        // Different purpose, different key
        let key2_result = km.get_derived_key(b"purpose2");
        assert!(key2_result.is_ok());
        let key2 = key2_result.unwrap();
        assert_eq!(key2.len(), 32);
        assert_ne!(key1, key2, "Keys derived with different purposes should be different");
        
        // Empty purpose string (valid context for blake3::derive_key)
        let key_empty_purpose_result = km.get_derived_key(b"");
        assert!(key_empty_purpose_result.is_ok());
        assert_ne!(key1, key_empty_purpose_result.unwrap(), "Key from empty purpose should differ");
    }
    
    #[test]
    fn test_get_derived_key_invalid_purpose_utf8() {
        let mut km = KeyManager::new();
        km.set_master_key(TEST_MASTER_KEY);
        let invalid_purpose = &[0xC3, 0x28]; // Invalid UTF-8 sequence
        let result = km.get_derived_key(invalid_purpose);
        assert!(result.is_err());
         match result.err().unwrap() {
            SecurityError::KeyExchangeError(msg) => assert!(msg.contains("not valid UTF-8")),
            _ => panic!("Unexpected error type for invalid UTF-8 purpose"),
        }
    }
}
