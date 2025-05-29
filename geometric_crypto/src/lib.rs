// geometric_crypto/src/lib.rs

// Core module and its submodules
pub mod core;
// Re-export key core types for easier access at crate level
pub use core::coordinates::Coordinate3D;
pub use core::matrix::{GeometricMatrix, CryptoError}; // CryptoError from core::matrix
pub use core::transforms::Axis;
pub use core::seed::derive_seed_from_key;
pub use core::cache::HierarchicalCoordinateCache;
pub use core::cache_structs::CoordinateMetadata;


// Compression module and its submodules
pub mod compression;
// Re-export key compression types
pub use compression::{CompressionEngine, CompressionError, GeometricOperation};


// Security module and its submodules
pub mod security;
// Re-export key security types
pub use security::{SecurityLayer, SecurityError, SecurityContext, IntegrityProof};
// KeyManager is currently a simple default struct within security::layer,
// if it becomes more significant, it could be re-exported too.
// CoordinateObfuscator, IntegrityVerifier are internal to SecurityLayer for now.


// Network module and its submodules
pub mod network;
// Re-export key network types
pub use network::{NetworkLayer, NetworkError};
// Potentially re-export serialization functions if they are meant to be public API
// pub use network::serialization::{serialize_operations, deserialize_operations};


#[cfg(test)]
mod tests {
    // use super::*; // Already here from previous setup.

    #[test]
    fn lib_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

    // Example test to check if a type from a sub-sub-module is accessible
    // This depends on the pub declarations within those modules as well.
    #[test]
    fn test_public_api_access() {
        // Test core API access
        let _coord = crate::Coordinate3D::new(1,2,3); // Using re-exported path
        let _matrix_err: Result<(), crate::CryptoError> = Err(crate::CryptoError::MappingGenerationFailed("test".to_string()));
        
        // Test compression API access
        let _comp_engine = crate::CompressionEngine::new();
        let _comp_err: Result<(), crate::CompressionError> = Err(crate::CompressionError::NotImplemented);

        // Test security API access
        // The following lines will cause compile errors if the types are not defined or not pub.
        // Assuming SecurityLayer::new(), SecurityError::NotImplemented, SecurityContext (unit/default), IntegrityProof::default() exist.
        // These types are not yet defined in the project, so these lines would fail.
        // For the purpose of testing lib.rs structure, we comment them out if they cause failure due to missing types.
        // However, the task is to *add* these re-exports, assuming the types *will* exist.
        // So, the test should reflect the desired state.
        // If the types are not created yet in their respective modules (e.g. security::layer),
        // then this test will only pass once those are implemented.
        // For now, let's assume the types exist for the sake of setting up lib.rs.
        
        // Placeholder for actual types if not yet fully implemented, to make test compile:
        // This is a common strategy: define minimal stubs for types so API surface tests can pass.
        // For this task, we assume the types are defined in their modules as per the pub use statements.
        
        let _sec_layer = crate::security::SecurityLayer::new(); // Path corrected to use module path
        let _sec_err: Result<(), crate::security::SecurityError> = Err(crate::security::SecurityError::NotImplemented("test".to_string()));
        let _sec_context = crate::security::SecurityContext; 
        let _proof = crate::security::IntegrityProof::default();


        // Test network API access
        let _net_layer = crate::network::NetworkLayer::new(); // Path corrected
        let _net_err: Result<(), crate::network::NetworkError> = Err(crate::network::NetworkError::NotImplemented("test".to_string()));

        assert!(true); // If all above compile, API paths are likely okay.
    }
}
