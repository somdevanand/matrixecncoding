use chacha20::ChaCha20Rng;
use chacha20::rand_core::{SeedableRng, RngCore};

pub struct SecurePrng {
    rng: ChaCha20Rng,
}

impl SecurePrng {
    /// Creates a new PRNG instance seeded with the given 32-byte seed.
    pub fn new(seed: [u8; 32]) -> Self {
        // ChaCha20Rng::from_seed expects a [u8; 32] seed.
        // The stream number can be set to 0 for simplicity here.
        Self {
            rng: ChaCha20Rng::from_seed(seed),
        }
    }

    /// Fills the destination byte slice with random data.
    pub fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.rng.fill_bytes(dest);
    }

    /// Generates a u64 random number.
    pub fn next_u64(&mut self) -> u64 {
        self.rng.next_u64()
    }

    /// Generates a u8 random number.
    pub fn next_u8(&mut self) -> u8 {
        // You can obtain a u8 by taking a portion of u32 or u64,
        // or by filling a single-byte slice.
        let mut buffer = [0u8; 1];
        self.rng.fill_bytes(&mut buffer);
        buffer[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Imports SecurePrng

    const TEST_SEED_1: [u8; 32] = [1u8; 32];
    const TEST_SEED_2: [u8; 32] = [2u8; 32];

    #[test]
    fn test_prng_creation() {
        let _prng = SecurePrng::new(TEST_SEED_1);
        // If new() panics, this test will fail.
    }

    #[test]
    fn test_prng_fill_bytes_deterministic() {
        let mut prng1 = SecurePrng::new(TEST_SEED_1);
        let mut prng2 = SecurePrng::new(TEST_SEED_1);

        let mut buffer1 = [0u8; 64];
        let mut buffer2 = [0u8; 64];

        prng1.fill_bytes(&mut buffer1);
        prng2.fill_bytes(&mut buffer2);

        assert_eq!(buffer1, buffer2, "PRNG output should be deterministic for the same seed");
    }

    #[test]
    fn test_prng_fill_bytes_unique_seeds() {
        let mut prng1 = SecurePrng::new(TEST_SEED_1);
        let mut prng2 = SecurePrng::new(TEST_SEED_2);

        let mut buffer1 = [0u8; 64];
        let mut buffer2 = [0u8; 64];

        prng1.fill_bytes(&mut buffer1);
        prng2.fill_bytes(&mut buffer2);

        assert_ne!(buffer1, buffer2, "PRNG output should be different for different seeds");
    }

    #[test]
    fn test_prng_fill_bytes_successive_calls() {
        let mut prng = SecurePrng::new(TEST_SEED_1);
        let mut buffer1 = [0u8; 64];
        let mut buffer2 = [0u8; 64];

        prng.fill_bytes(&mut buffer1);
        prng.fill_bytes(&mut buffer2);

        assert_ne!(buffer1, buffer2, "Successive calls to fill_bytes should produce different output");
    }
    
    #[test]
    fn test_prng_next_u64_deterministic() {
        let mut prng1 = SecurePrng::new(TEST_SEED_1);
        let mut prng2 = SecurePrng::new(TEST_SEED_1);
        assert_eq!(prng1.next_u64(), prng2.next_u64());
        assert_eq!(prng1.next_u64(), prng2.next_u64());
    }

    #[test]
    fn test_prng_next_u64_unique_seeds() {
        let mut prng1 = SecurePrng::new(TEST_SEED_1);
        let mut prng2 = SecurePrng::new(TEST_SEED_2);
        assert_ne!(prng1.next_u64(), prng2.next_u64());
    }

    #[test]
    fn test_prng_next_u8_deterministic() {
        let mut prng1 = SecurePrng::new(TEST_SEED_1);
        let mut prng2 = SecurePrng::new(TEST_SEED_1);
        assert_eq!(prng1.next_u8(), prng2.next_u8());
        assert_eq!(prng1.next_u8(), prng2.next_u8());
    }

    #[test]
    fn test_prng_next_u8_unique_seeds() {
        let mut prng1 = SecurePrng::new(TEST_SEED_1);
        let mut prng2 = SecurePrng::new(TEST_SEED_2);
        assert_ne!(prng1.next_u8(), prng2.next_u8());
    }

    #[test]
    fn test_prng_mixed_calls_deterministic() {
        let mut prng1 = SecurePrng::new(TEST_SEED_1);
        let mut prng2 = SecurePrng::new(TEST_SEED_1);

        let mut buffer1_1 = [0u8; 10];
        let mut buffer2_1 = [0u8; 10];
        let mut buffer1_2 = [0u8; 5];
        let mut buffer2_2 = [0u8; 5];

        assert_eq!(prng1.next_u8(), prng2.next_u8(), "Mixed call: next_u8 deterministic");
        
        prng1.fill_bytes(&mut buffer1_1);
        prng2.fill_bytes(&mut buffer2_1);
        assert_eq!(buffer1_1, buffer2_1, "Mixed call: fill_bytes(10) deterministic");

        assert_eq!(prng1.next_u64(), prng2.next_u64(), "Mixed call: next_u64 deterministic");

        prng1.fill_bytes(&mut buffer1_2);
        prng2.fill_bytes(&mut buffer2_2);
        assert_eq!(buffer1_2, buffer2_2, "Mixed call: fill_bytes(5) deterministic");
        
        assert_eq!(prng1.next_u8(), prng2.next_u8(), "Mixed call: subsequent next_u8 deterministic");
    }
}
