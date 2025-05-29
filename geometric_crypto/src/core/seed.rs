use blake3::Hasher;

pub fn derive_seed_from_key(key: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(key);
    let hash = hasher.finalize();
    *hash.as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*; // Imports derive_seed_from_key

    #[test]
    fn test_seed_derivation_length() {
        let key = b"test_key";
        let seed = derive_seed_from_key(key);
        assert_eq!(seed.len(), 32, "Seed length should be 32 bytes");
    }

    #[test]
    fn test_seed_derivation_determinism() {
        let key = b"my_secret_key";
        let seed1 = derive_seed_from_key(key);
        let seed2 = derive_seed_from_key(key);
        assert_eq!(seed1, seed2, "Seeds derived from the same key should be identical");
    }

    #[test]
    fn test_seed_derivation_uniqueness() {
        let key1 = b"key_one";
        let seed1 = derive_seed_from_key(key1);

        let key2 = b"key_two";
        let seed2 = derive_seed_from_key(key2);

        assert_ne!(seed1, seed2, "Seeds derived from different keys should be different");
    }
    
    #[test]
    fn test_seed_derivation_empty_key() {
        let key: &[u8] = &[];
        let seed = derive_seed_from_key(key);
        assert_eq!(seed.len(), 32, "Seed from empty key should be 32 bytes");
        // We can't assert a specific value without knowing blake3's output for empty string,
        // but we can ensure it runs and produces a 32-byte array.
        // For example, another call with empty key should produce the same seed:
        let seed2 = derive_seed_from_key(key);
        assert_eq!(seed, seed2, "Seed from empty key should be deterministic");
    }

    #[test]
    fn test_seed_derivation_long_key() {
        let long_key = b"this_is_a_very_long_key_that_is_definitely_more_than_32_bytes_long_for_testing_purposes";
        assert!(long_key.len() > 32, "Test key should be longer than 32 bytes");
        let seed = derive_seed_from_key(long_key);
        assert_eq!(seed.len(), 32, "Seed from long key should be 32 bytes");

        // Also check determinism for long keys
        let seed2 = derive_seed_from_key(long_key);
        assert_eq!(seed, seed2, "Seed from the same long key should be deterministic");
    }
}
