// geometric_crypto/src/lib.rs

// Core module re-exports
pub mod core;
pub use core::coordinates::Coordinate3D;
pub use core::matrix::{GeometricMatrix, CryptoError};
pub use core::transforms::Axis;
pub use core::seed::derive_seed_from_key;
pub use core::cache::HierarchicalCoordinateCache;
pub use core::cache_structs::CoordinateMetadata;
pub use core::memory_optimizer::{MemoryOptimizer, MemoryProfile}; // Added MemoryProfile
#[cfg(feature = "parallel")]
pub use core::parallel_processor::ParallelProcessor;


// Compression module re-exports
pub mod compression;
pub use compression::{CompressionEngine, CompressionError, GeometricOperation};
// Potentially re-export supporting types from compression::operations if they are part of public API
// pub use compression::operations::{PatternId, InterpolationType, EncodingScheme, Matrix3x3, ...};


// Security module re-exports
pub mod security;
pub use security::{
    SecurityLayer, 
    SecurityError, 
    SecurityContext, 
    IntegrityProof, 
    KeyManager, 
    log_security_event
};
// pub use security::key_exchange::{KeyExchangeManager, EphemeralKeyPair}; // If these are for direct use


// Network module and its submodules
pub mod network;
pub use network::{
    NetworkLayer,
    NetworkError,
    PROTOCOL_VERSION,
    StreamingFrame,
    frame_data,
    deframe_data,
    SeedRequest,
    SeedResponse,
    CheckpointData,
    create_seed_request,
    handle_seed_request,
    verify_seed_response,
    get_sync_checkpoint,
    request_retransmit
};

// System module and its re-exports (NEWLY ADDED/UPDATED SECTION)
pub mod system;
pub use system::{
    GeometricCryptoSystem, 
    SystemConfiguration, 
    CoreEngine, CoreEngineConfig, // Also useful to have CoreEngine public from system
    CompressionLevel, SecurityLevel, // Enums for config
    GeometricCryptoError, // Main system error
};


#[cfg(test)]
mod tests {
    #[test]
    fn lib_works() { let result = 2 + 2; assert_eq!(result, 4); }

    #[test]
    fn test_public_api_access() {
        // Core API
        let _coord = crate::Coordinate3D::new(1,2,3);
        let _matrix_err: Result<(), crate::CryptoError> = Err(crate::CryptoError::MappingGenerationFailed("test".to_string()));
        let _mem_opt_config = crate::core::memory_optimizer::MemoryOptimizer::new(std::num::NonZeroUsize::new(1).unwrap(),0,0); // Path for non-reexported type
        let _mem_prof = crate::MemoryProfile::default();
        #[cfg(feature = "parallel")]
        let _par_proc = crate::ParallelProcessor::new(crate::core::parallel_processor::ParallelConfig { num_threads: None, coord_gen_chunk_size: 10 });


        // Compression API
        #[cfg(feature = "parallel")]
        let _comp_engine = crate::CompressionEngine::new(false, None);
        #[cfg(not(feature = "parallel"))]
        let _comp_engine = crate::CompressionEngine::new();
        let _comp_err: Result<(), crate::CompressionError> = Err(crate::CompressionError::NotImplemented);
        let _sample_op = crate::GeometricOperation::RegionFill {
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
        let _s_resp = crate::handle_seed_request(&_s_req);

        // System API (NEWLY ADDED CHECKS)
        let _sys_config = crate::SystemConfiguration::default();
        let _sys_err: Result<(), crate::GeometricCryptoError> = Err(crate::GeometricCryptoError::KeyNotSet);
        // For GeometricCryptoSystem::with_key, it might return an error.
        // let _system = crate::GeometricCryptoSystem::with_key([0u8;32]).expect("System creation failed");

        assert!(true); 
    }
}
