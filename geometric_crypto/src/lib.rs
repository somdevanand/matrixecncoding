// geometric_crypto/src/lib.rs

// Core module re-exports (confirm these are sufficient)
pub mod core;
pub use core::coordinates::Coordinate3D;
pub use core::matrix::{GeometricMatrix, CryptoError};
pub use core::transforms::Axis;
pub use core::seed::derive_seed_from_key;
pub use core::cache::HierarchicalCoordinateCache;
pub use core::cache_structs::CoordinateMetadata;

// Compression module re-exports (confirm these are sufficient)
pub mod compression;
pub use compression::{CompressionEngine, CompressionError, GeometricOperation};
// Potentially re-export supporting types from compression::operations if they are part of public API
// pub use compression::operations::{PatternId, InterpolationType, EncodingScheme, Matrix3x3, ...};


// Security module re-exports (confirm these are sufficient)
pub mod security;
pub use security::{SecurityLayer, SecurityError, SecurityContext, IntegrityProof, KeyManager, log_security_event};
// pub use security::key_exchange::{KeyExchangeManager, EphemeralKeyPair}; // If these are for direct use


// Network module and its submodules
pub mod network;
// Update re-exports for network module:
pub use network::{
    NetworkLayer,
    NetworkError,
    PROTOCOL_VERSION,
    StreamingFrame,       // From network::streaming
    frame_data,           // From network::streaming
    deframe_data,         // From network::streaming
    SeedRequest,          // From network::sync
    SeedResponse,         // From network::sync
    CheckpointData,       // From network::sync
    create_seed_request,  // From network::sync
    handle_seed_request,  // From network::sync
    verify_seed_response, // From network::sync
    get_sync_checkpoint,  // From network::sync
    request_retransmit    // From network::sync
};
// The individual serialization functions in network::serialization are not typically
// re-exported if NetworkLayer methods are the primary interface.

#[cfg(test)]
mod tests {
    // use super::*; // Already here from previous setup.

    #[test]
    fn lib_works() { let result = 2 + 2; assert_eq!(result, 4); }


    // Update test_public_api_access to include new network types
    #[test]
    fn test_public_api_access() {
        // Core API
        let _coord = crate::Coordinate3D::new(1,2,3);
        let _matrix_err: Result<(), crate::CryptoError> = Err(crate::CryptoError::MappingGenerationFailed("test".to_string()));
        
        // Compression API
        let _comp_engine = crate::CompressionEngine::new();
        let _comp_err: Result<(), crate::CompressionError> = Err(crate::CompressionError::NotImplemented);
        let _sample_op = crate::GeometricOperation::RegionFill { // Requires pub use for GeometricOperation
            start: _coord, end: _coord, fill_byte: 0, compression_ratio: 1.0
        };


        // Security API
        let _sec_layer = crate::SecurityLayer::new();
        let _sec_err: Result<(), crate::SecurityError> = Err(crate::SecurityError::NotImplemented("test".to_string()));
        let _sec_context = crate::SecurityContext::default();
        let _proof = crate::IntegrityProof::default();
        let _km = crate::KeyManager::new();
        crate::log_security_event("API_TEST", "Security types accessible");


        // Network API
        let _net_layer = crate::NetworkLayer::new();
        let _net_err: Result<(), crate::NetworkError> = Err(crate::NetworkError::NotImplemented("test".to_string()));
        assert_eq!(crate::PROTOCOL_VERSION, 1);
        let _frame = crate::StreamingFrame::Header{total_payload_size:0, num_chunks:0};
        let _s_req = crate::SeedRequest::default();
        // Call a function too
        let _s_resp = crate::handle_seed_request(&_s_req);


        assert!(true); 
    }
}
