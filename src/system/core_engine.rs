// geometric_crypto/src/system/core_engine.rs
use crate::core::coordinates::Coordinate3D;
use crate::core::matrix::{GeometricMatrix, CryptoError};
use crate::core::cache::HierarchicalCoordinateCache;
use std::num::NonZeroUsize; // For LruCache capacity in HierarchicalCoordinateCache
use blake3;
use crate::security::prng::SecurePrng; // Import SecurePrng

#[derive(Debug, Clone, Copy)]
pub struct CoreEngineConfig {
    pub l2_cache_capacity: NonZeroUsize, // Changed from _bytes to actual capacity units
    pub use_cold_storage_for_cache: bool,
    // Any other core-specific configurations
}

impl Default for CoreEngineConfig { // Added default
    fn default() -> Self {
        Self {
            l2_cache_capacity: NonZeroUsize::new(10000).unwrap(), // Default L2 capacity
            use_cold_storage_for_cache: false,
        }
    }
}

// Removed Debug because GeometricMatrix no longer derives Debug.
pub struct CoreEngine {
    matrix: GeometricMatrix,
    cache: HierarchicalCoordinateCache, // HierarchicalCoordinateCache still derives Debug.
    seed: [u8; 32], // Store the seed used to initialize the matrix
}

impl CoreEngine {
    pub fn new(initial_seed: [u8; 32], config: CoreEngineConfig) -> Result<Self, CryptoError> {
        let mut matrix = GeometricMatrix::new(initial_seed);
        matrix.generate_bijective_mapping()?; // Ensure mapping is generated

        let cache = HierarchicalCoordinateCache::new(
            config.l2_cache_capacity,
            config.use_cold_storage_for_cache
        );
        
        Ok(Self { matrix, cache, seed: initial_seed })
    }

    pub fn set_seed(&mut self, seed: [u8; 32]) -> Result<(), CryptoError> {
        self.seed = seed;
        self.matrix = GeometricMatrix::new(seed);
        self.matrix.generate_bijective_mapping()?;
        // Optionally clear cache, or let it phase out old entries
        // self.cache = HierarchicalCoordinateCache::new(...); // Re-init cache if seed change invalidates it
        Ok(())
    }
    
    pub fn get_current_seed(&self) -> [u8;32] { // Added getter for seed
        self.seed
    }

    /// Converts a byte slice to a sequence of position-dependent coordinates.
    pub fn bytes_to_coordinates(&self, data: &[u8]) -> Vec<Coordinate3D> {
        // The cache here is for CoordinateMetadata, not direct coordinate caching.
        // If direct coordinate caching was intended, HierarchicalCoordinateCache would need adjustment.
        // For now, direct generation using matrix:
        if self.matrix.byte_to_coord_map.is_empty() {
             // This should not happen if new() or set_seed() guarantees mapping generation.
             // Consider returning Result<_, CryptoError> or specific error.
             panic!("CoreEngine's GeometricMatrix not initialized with mappings.");
        }
        data.iter()
            .enumerate()
            .map(|(position, &byte_value)| {
                self.matrix.coordinate_for_position(byte_value, position)
            })
            .collect()
    }

    /// Converts a sequence of coordinates back to bytes.
    /// Uses the GeometricMatrix's reverse mapping after reversing the position-dependent transformation.
    pub fn coordinates_to_bytes(&self, coords: &[Coordinate3D]) -> Result<Vec<u8>, String> {
        if self.matrix.coord_to_byte_map.is_empty() {
             panic!("CoreEngine's GeometricMatrix not initialized with mappings for reverse lookup.");
        }
        let mut bytes = Vec::with_capacity(coords.len());
        // Iterate with index to get position
        for (position, coord) in coords.iter().enumerate() {
            // 1. Recalculate transformation seed based on matrix seed and position
            let mut hasher = blake3::Hasher::new(); // Assuming blake3 is available as in GeometricMatrix
            hasher.update(&self.seed);
            hasher.update(&position.to_le_bytes());
            let transformation_seed: [u8; 32] = hasher.finalize().into();

            // 2. Create a temporary PRNG for reversing the transformation
            let mut transform_prng = SecurePrng::new(transformation_seed);

            // 3. Generate the same offsets used in coordinate_for_position
            let offset_x = transform_prng.next_u8();
            let offset_y = transform_prng.next_u8();
            let offset_z = transform_prng.next_u8();

            let offset_coord = Coordinate3D::new(offset_x, offset_y, offset_z);

            // 4. Reverse the transformation to get the base coordinate
            //    Assuming Coordinate3D has a wrapping_sub or equivalent method.
            let base_coord = (*coord).subtract(offset_coord); // Corrected method name and call on value

            // 5. Look up the recovered base coordinate in the reverse map
            match self.matrix.get_byte_for_coord(&base_coord) { // Assumes get_byte_for_coord exists
                Some(byte_val) => bytes.push(byte_val),
                None => return Err(format!("Coordinate {:?} not found in reverse map.", coord)),
            }
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::core::matrix::CryptoError; // Not directly used in these specific tests

    const TEST_SEED: [u8; 32] = [1u8; 32];
    // Using a master key that will result in a null obfuscation key for testing purposes.
    // The derived key for "coord_obfuscation_v1" from this master key should be all zeros.
    const TEST_MASTER_KEY_FOR_NULL_OBFUSCATION: [u8; 32] = [0u8; 32];

    fn new_test_engine(seed: [u8; 32]) -> CoreEngine {
        let config = CoreEngineConfig::default();
        let mut engine_result = CoreEngine::new(seed, config);
        assert!(engine_result.is_ok());
        engine_result.unwrap()
    }

    #[test]
    fn test_core_engine_new_and_set_seed() {
        let config = CoreEngineConfig::default();
        let mut engine_result = CoreEngine::new(TEST_SEED, config);
        assert!(engine_result.is_ok());
        let mut engine = engine_result.unwrap();
        // byte_to_coord_map is pub(crate), so not directly accessible here for assert.
        // Successful creation implies mapping was generated if new() returns Ok.
        // We can test behaviorally via bytes_to_coordinates.

        let new_seed = [8u8; 32];
        let set_seed_result = engine.set_seed(new_seed);
        assert!(set_seed_result.is_ok());
        assert_eq!(engine.get_current_seed(), new_seed);
    }

    #[test]
    fn test_bytes_to_coordinates_and_back() {
        let config = CoreEngineConfig::default();
        // Use the master key that results in a null obfuscation key for testing the core mapping roundtrip.
        let engine = CoreEngine::new(TEST_MASTER_KEY_FOR_NULL_OBFUSCATION, config).unwrap();
        
        let data: Vec<u8> = b"hello".to_vec();
        let coords = engine.bytes_to_coordinates(&data);
        assert_eq!(coords.len(), data.len());
        
        if data.len() > 1 { 
             let l_indices: Vec<usize> = data.iter().enumerate().filter(|(_,&b)| b == b'l').map(|(i,_)| i).collect();
             if l_indices.len() > 1 {
                 assert_ne!(coords[l_indices[0]], coords[l_indices[1]], "Coords for same byte at different positions should differ");
             }
        }

        // This test relies on get_byte_for_coord being added to GeometricMatrix.
        // If that method is not yet present, this test might fail compilation or at runtime.
        let recovered_bytes_result = engine.coordinates_to_bytes(&coords);
        assert!(recovered_bytes_result.is_ok(), "coordinates_to_bytes failed: {:?}", recovered_bytes_result.err());
        assert_eq!(recovered_bytes_result.unwrap(), data);
    }
    
    #[test]
    fn test_coordinates_to_bytes_unknown_coord() {
         let config = CoreEngineConfig::default();
         let engine = CoreEngine::new(TEST_SEED, config).unwrap();
         let unknown_coord = Coordinate3D::new(254,253,252); // Using values less likely to conflict with actual map
         let result = engine.coordinates_to_bytes(&[unknown_coord]);
         assert!(result.is_err());
         assert!(result.err().unwrap().contains("not found in reverse map"));
    }
}
