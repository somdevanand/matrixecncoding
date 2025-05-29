use crate::core::coordinates::Coordinate3D;
use crate::security::prng::SecurePrng; // Assuming prng.rs is in security module, adjust path if needed
use std::collections::{HashMap, HashSet}; // Added HashSet for uniqueness check

// Define a basic CryptoError enum for now. This might be expanded later.
#[derive(Debug)]
pub enum CryptoError {
    MappingGenerationFailed(String),
    // Other errors can be added here
}

pub struct GeometricMatrix {
    seed: [u8; 32],
    generator: SecurePrng,
    pub byte_to_coord_map: Vec<Coordinate3D>, // Made public for easier testing initially
    pub coord_to_byte_map: HashMap<Coordinate3D, u8>, // Made public for easier testing
}

impl GeometricMatrix {
    pub fn new(initial_seed: [u8; 32]) -> Self {
        Self {
            seed: initial_seed,
            generator: SecurePrng::new(initial_seed),
            byte_to_coord_map: Vec::with_capacity(256),
            coord_to_byte_map: HashMap::with_capacity(256),
        }
    }

    // Generates a bijective mapping from u8 (0-255) to unique Coordinate3D.
    pub fn generate_bijective_mapping(&mut self) -> Result<(), CryptoError> {
        self.byte_to_coord_map.clear();
        self.coord_to_byte_map.clear();
        let mut used_coordinates = HashSet::new();

        for byte_val_u16 in 0..256 {
            let byte_val = byte_val_u16 as u8;
            let mut attempts = 0;
            loop {
                if attempts > 10000 { // Safeguard against potential infinite loops
                    return Err(CryptoError::MappingGenerationFailed(
                        format!("Failed to find unique coordinate for byte {} after many attempts", byte_val)
                    ));
                }

                let coord = Coordinate3D::new(
                    self.generator.next_u8(),
                    self.generator.next_u8(),
                    self.generator.next_u8(),
                );

                if used_coordinates.insert(coord) {
                    self.byte_to_coord_map.push(coord);
                    self.coord_to_byte_map.insert(coord, byte_val);
                    break; // Found a unique coordinate
                }
                attempts += 1;
            }
        }

        if self.byte_to_coord_map.len() == 256 && self.coord_to_byte_map.len() == 256 {
            Ok(())
        } else {
            // This case should ideally not be reached if the loop logic is correct
            Err(CryptoError::MappingGenerationFailed(
                "Mapping generation did not result in 256 unique mappings.".to_string()
            ))
        }
    }
}

// Placeholder for module structure - this will be handled by lib.rs or main.rs later
// mod crate::core::coordinates;
// mod crate::security::prng;

#[cfg(test)]
mod tests {
    use super::*; // Imports GeometricMatrix, Coordinate3D, CryptoError
    use std::collections::HashSet;

    // It's good practice to define test seeds as constants.
    // Using simple, non-random seeds for testing makes tests reproducible.
    const TEST_SEED_1: [u8; 32] = [1u8; 32];
    const TEST_SEED_2: [u8; 32] = [2u8; 32];

    #[test]
    fn test_generate_bijective_mapping_completeness() {
        let mut matrix = GeometricMatrix::new(TEST_SEED_1);
        let result = matrix.generate_bijective_mapping();
        assert!(result.is_ok(), "Mapping generation should succeed. Error: {:?}", result.err());

        assert_eq!(matrix.byte_to_coord_map.len(), 256, "byte_to_coord_map should have 256 entries");
        assert_eq!(matrix.coord_to_byte_map.len(), 256, "coord_to_byte_map should have 256 entries");
    }

    #[test]
    fn test_generate_bijective_mapping_uniqueness() {
        let mut matrix = GeometricMatrix::new(TEST_SEED_1);
        matrix.generate_bijective_mapping().expect("Mapping generation failed for uniqueness test");

        let mut unique_coords = HashSet::new();
        for coord in &matrix.byte_to_coord_map {
            unique_coords.insert(*coord); // Coordinate3D implements Copy, so *coord is fine
        }
        assert_eq!(unique_coords.len(), 256, "All 256 coordinates in byte_to_coord_map should be unique");
        
        // Also check the keys in coord_to_byte_map for uniqueness (which HashMap guarantees by design)
        assert_eq!(matrix.coord_to_byte_map.keys().len(), 256, "coord_to_byte_map should have 256 unique coordinate keys");
    }

    #[test]
    fn test_generate_bijective_mapping_bijection_property() {
        let mut matrix = GeometricMatrix::new(TEST_SEED_1);
        matrix.generate_bijective_mapping().expect("Mapping generation failed for bijection test");

        for i in 0..=255 {
            let byte_val = i as u8;
            
            // Test forward mapping (byte_to_coord_map)
            assert!(byte_val as usize  < matrix.byte_to_coord_map.len(), "Index out of bounds for byte_val: {}", byte_val);
            let coord = matrix.byte_to_coord_map[byte_val as usize];
            
            // Test reverse mapping (coord_to_byte_map)
            match matrix.coord_to_byte_map.get(&coord) {
                Some(retrieved_byte) => {
                    assert_eq!(*retrieved_byte, byte_val, "Bijection property f^-1(f(x)) = x failed. For byte {}, expected {}, got {}", byte_val, byte_val, *retrieved_byte);
                },
                None => {
                    panic!("Coordinate {:?} derived from byte {} not found in coord_to_byte_map.", coord, byte_val);
                }
            }
        }
    }

    #[test]
    fn test_generate_bijective_mapping_within_bounds() {
        let mut matrix = GeometricMatrix::new(TEST_SEED_1);
        matrix.generate_bijective_mapping().expect("Mapping generation failed for bounds test");

        for coord in &matrix.byte_to_coord_map {
            // For u8 fields, x, y, z are always within [0, 255].
            // This test is more of a sanity check that the types are indeed u8
            // and no unexpected numeric conversion or overflow happened if the generation logic were more complex.
            // With current Coordinate3D::new(u8,u8,u8), this is inherently true.
            let _x = coord.x; // Will not compile if x is not u8 or accessible
            let _y = coord.y;
            let _z = coord.z;
            // No explicit assert needed as the u8 type itself enforces the bounds.
            // If they were, e.g., i16 and we expected them to be in u8 range, then asserts would be crucial.
        }
        // To make the test meaningful, we can assert that all map values are within u8 range
        for &byte_val in matrix.coord_to_byte_map.values() {
             let _b_u8: u8 = byte_val; // this assignment checks if it's a u8
        }
    }
    
    #[test]
    fn test_generate_bijective_mapping_determinism() {
        let mut matrix1 = GeometricMatrix::new(TEST_SEED_1);
        let mut matrix2 = GeometricMatrix::new(TEST_SEED_1); // Same seed

        let res1 = matrix1.generate_bijective_mapping();
        assert!(res1.is_ok(), "Matrix1 mapping generation failed: {:?}", res1.err());
        let res2 = matrix2.generate_bijective_mapping();
        assert!(res2.is_ok(), "Matrix2 mapping generation failed: {:?}", res2.err());

        assert_eq!(matrix1.byte_to_coord_map, matrix2.byte_to_coord_map, "byte_to_coord_map should be deterministic for the same seed");
        assert_eq!(matrix1.coord_to_byte_map, matrix2.coord_to_byte_map, "coord_to_byte_map should be deterministic for the same seed");
    }

    #[test]
    fn test_generate_bijective_mapping_uniqueness_different_seeds() {
        let mut matrix1 = GeometricMatrix::new(TEST_SEED_1);
        let mut matrix2 = GeometricMatrix::new(TEST_SEED_2); // Different seed

        let res1 = matrix1.generate_bijective_mapping();
        assert!(res1.is_ok(), "Matrix1 mapping generation failed: {:?}", res1.err());
        let res2 = matrix2.generate_bijective_mapping();
        assert!(res2.is_ok(), "Matrix2 mapping generation failed: {:?}", res2.err());
        
        // With cryptographic PRNGs like ChaCha20, the probability of collision for different seeds
        // producing the exact same full mapping of 256 coordinates is astronomically low.
        assert_ne!(matrix1.byte_to_coord_map, matrix2.byte_to_coord_map, "byte_to_coord_map should be different for different seeds");
        assert_ne!(matrix1.coord_to_byte_map, matrix2.coord_to_byte_map, "coord_to_byte_map should be different for different seeds");
    }
}
