// geometric_crypto/src/system/main_system.rs
use super::core_engine::{CoreEngine, CoreEngineConfig};
use crate::compression::{CompressionEngine, CompressionError as CompressionEngineError};
use crate::security::{SecurityLayer, SecurityError, SecurityContext}; 
use crate::core::matrix::CryptoError as CoreCryptoError;
use crate::network::NetworkError as NetworkLayerError; 
use crate::network::NetworkLayer;
use crate::network::PROTOCOL_VERSION; // Added
use crate::compression::operations::GeometricOperation; // Added for encrypt/decrypt type hints


use crate::core::parallel_processor::ParallelConfig; // Added

#[derive(Debug, Clone)] 
pub struct SystemConfiguration {
    pub core_config: CoreEngineConfig,
    pub compression_level: CompressionLevel, 
    pub security_level: SecurityLevel,
    pub parallel_pattern_analysis_enabled: bool, // New
    pub parallel_config: ParallelConfig,        // New, contains num_threads and coord_gen_chunk_size
}

impl Default for SystemConfiguration { 
    fn default() -> Self {
        Self {
            core_config: CoreEngineConfig::default(),
            compression_level: CompressionLevel::Balanced,
            security_level: SecurityLevel::Standard,
            parallel_pattern_analysis_enabled: cfg!(feature = "parallel"), // Enable if feature is compiled
            parallel_config: ParallelConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)] 
pub enum CompressionLevel {
    Fast,
    Balanced,
    Maximum,
}
impl Default for CompressionLevel { fn default() -> Self { Self::Balanced } }


#[derive(Debug, Clone, Copy, PartialEq, Eq)] 
pub enum SecurityLevel {
    Basic,
    Standard,
    High,
    Maximum,
}
impl Default for SecurityLevel { fn default() -> Self { Self::Standard } }


#[derive(Debug, thiserror::Error)]
pub enum GeometricCryptoError {
    #[error("Core Engine Error: {0}")]
    CoreError(#[from] CoreCryptoError), 
    #[error("Security Layer Error: {0}")]
    SecurityError(#[from] SecurityError), 
    #[error("Compression Engine Error: {0}")]
    CompressionError(#[from] CompressionEngineError),
    #[error("Decompression Engine Error: {0}")]
    DecompressionError(#[from] crate::compression::engine::DecompressionError), // Added this line
    #[error("Network Layer Error: {0}")]
    NetworkError(#[from] NetworkLayerError),
    #[error("Key not set for operation")]
    KeyNotSet,
    #[error("Initialization failed: {0}")]
    InitializationError(String),
}


use crate::core::memory_optimizer::MemoryOptimizer; // Added
use std::num::NonZeroUsize; // Added for MemoryOptimizer::new

// Removed Debug because CoreEngine no longer derives Debug.
pub struct GeometricCryptoSystem {
    core_engine: CoreEngine,
    compression_engine: CompressionEngine, 
    security_layer: SecurityLayer,
    network_layer: NetworkLayer,
    memory_optimizer: MemoryOptimizer, // New field
    config: SystemConfiguration,
}

impl GeometricCryptoSystem {
    pub fn new(config: SystemConfiguration, initial_key: [u8; 32]) -> Result<Self, GeometricCryptoError> {
        let core_engine = CoreEngine::new(initial_key, config.core_config)?;
        
        #[cfg(feature = "parallel")]
        let compression_engine = CompressionEngine::new(
            config.parallel_pattern_analysis_enabled,
            Some(config.parallel_config) // Pass the ParallelConfig from SystemConfiguration
        );
        #[cfg(not(feature = "parallel"))]
        let compression_engine = CompressionEngine::new(); // Original constructor if "parallel" feature is off

        let mut security_layer = SecurityLayer::new();
        security_layer.set_master_key(initial_key)?; 
        let network_layer = NetworkLayer::new();
        
        // Initialize MemoryOptimizer with default/configurable values
        // TODO: Make these capacities configurable via SystemConfiguration or a new OptimizerConfig
        let op_cache_capacity = NonZeroUsize::new(1000).ok_or_else(|| GeometricCryptoError::InitializationError("Invalid op_cache_capacity".to_string()))?;
        let buffer_capacity = 1024 * 1024; // 1MB
        let coord_pool_capacity = 2048;    // Pool for 2048 Coordinate3D objects
        let memory_optimizer = MemoryOptimizer::new(op_cache_capacity, buffer_capacity, coord_pool_capacity);

        Ok(Self {
            core_engine,
            compression_engine,
            security_layer,
            network_layer,
            memory_optimizer, // Initialize new field
            config,
        })
    }
    
    pub fn with_key(initial_key: [u8; 32]) -> Result<Self, GeometricCryptoError> {
        Self::new(SystemConfiguration::default(), initial_key)
    }

    pub fn set_key(&mut self, key: [u8; 32]) -> Result<(), GeometricCryptoError> {
        self.core_engine.set_seed(key)?;
        self.security_layer.set_master_key(key)?;
        Ok(())
    }
    
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, GeometricCryptoError> {
        if !self.security_layer.key_manager.is_key_set() {
            return Err(GeometricCryptoError::KeyNotSet);
        }

        let coords = self.core_engine.bytes_to_coordinates(data);
        if coords.is_empty() && !data.is_empty() { 
            return Err(GeometricCryptoError::CoreError(CoreCryptoError::MappingGenerationFailed("Coordinate generation resulted in empty set for non-empty data".to_string())));
        }

        let mut operations: Vec<GeometricOperation> = self.compression_engine.compress_coordinate_sequence(&coords)?;
        
        let security_context = SecurityContext::default(); 
        let proof = self.security_layer.protect_data(&mut operations, &security_context)?;
        
        let packaged_data = self.network_layer.package_data(
            PROTOCOL_VERSION,
            &operations, 
            Some(&proof)
        )?;

        Ok(packaged_data)
    }

    pub fn decrypt(&mut self, encrypted_payload: &[u8]) -> Result<Vec<u8>, GeometricCryptoError> { // Changed to &mut self
        if !self.security_layer.key_manager.is_key_set() {
            return Err(GeometricCryptoError::KeyNotSet);
        }

        let (version, mut operations, proof_opt) = self.network_layer.unpackage_data(encrypted_payload)?;

        if version != PROTOCOL_VERSION {
            return Err(GeometricCryptoError::NetworkError(
                crate::network::NetworkError::VersionMismatch { expected: PROTOCOL_VERSION, got: version }
            ));
        }

        let proof = proof_opt.ok_or_else(|| GeometricCryptoError::NetworkError(
            crate::network::NetworkError::InvalidFormat("Integrity proof missing from payload".to_string())
        ))?;
        
        let security_context = SecurityContext::default(); 
        self.security_layer.unprotect_data(&mut operations, &proof, &security_context)?;
        
        // Pass mutable reference to memory_optimizer
        let coords = self.compression_engine.decompress_operations(&operations, &mut self.memory_optimizer)?;
        
        let bytes = self.core_engine.coordinates_to_bytes(&coords)
            .map_err(|e| GeometricCryptoError::CoreError(CoreCryptoError::MappingGenerationFailed(format!("Coord to bytes failed: {}",e))))?;

        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY: [u8; 32] = [1u8; 32];

    #[test]
    fn test_geometric_crypto_system_new() {
        let config = SystemConfiguration::default();
        let system_result = GeometricCryptoSystem::new(config, TEST_KEY);
        assert!(system_result.is_ok());
        let system = system_result.unwrap();
        assert_eq!(system.core_engine.get_current_seed(), TEST_KEY);
        assert!(system.security_layer.key_manager.is_key_set());
    }

    #[test]
    fn test_geometric_crypto_system_with_key() {
        let system_result = GeometricCryptoSystem::with_key(TEST_KEY);
        assert!(system_result.is_ok());
    }

    #[test]
    fn test_set_key() {
        let config = SystemConfiguration::default();
        let mut system = GeometricCryptoSystem::new(config, TEST_KEY).unwrap();
        
        let new_key = [2u8; 32];
        let set_key_result = system.set_key(new_key);
        assert!(set_key_result.is_ok());
        assert_eq!(system.core_engine.get_current_seed(), new_key);
        assert!(system.security_layer.key_manager.is_key_set());
    }

    #[test]
    fn test_encrypt_decrypt_basic_roundtrip() {
        let config = SystemConfiguration::default();
        let key = [65u8; 32]; // 'A'
        let mut system = GeometricCryptoSystem::new(config, key).expect("System creation failed"); // Made system mutable

        let data_str = "Hello, Geometric World!";
        let data = data_str.as_bytes();

        let encrypted_result = system.encrypt(data);
        assert!(encrypted_result.is_ok(), "Encryption failed: {:?}", encrypted_result.err());
        let encrypted_data = encrypted_result.unwrap();
        assert!(!encrypted_data.is_empty());

        let decrypted_result = system.decrypt(&encrypted_data);
        assert!(decrypted_result.is_ok(), "Decryption failed: {:?}", decrypted_result.err());
        let decrypted_data = decrypted_result.unwrap();
        
        assert_eq!(decrypted_data, data, "Decrypted data does not match original");
        assert_eq!(String::from_utf8_lossy(&decrypted_data), data_str);
    }
    
    #[test]
    fn test_encrypt_decrypt_empty_data() {
        let config = SystemConfiguration::default();
        let key = [66u8; 32]; // 'B'
        let mut system = GeometricCryptoSystem::new(config, key).expect("System creation failed"); // Made system mutable

        let data: Vec<u8> = Vec::new();
        let encrypted_result = system.encrypt(&data); // encrypt still takes &self
        assert!(encrypted_result.is_ok(), "Encryption of empty data failed: {:?}", encrypted_result.err());
        let encrypted_data = encrypted_result.unwrap();
        assert!(!encrypted_data.is_empty(), "Encrypted empty data should not be empty (due to headers/structure)");

        let decrypted_result = system.decrypt(&encrypted_data);
        assert!(decrypted_result.is_ok(), "Decryption of empty data failed: {:?}", decrypted_result.err());
        assert!(decrypted_result.unwrap().is_empty(), "Decrypted empty data should be empty");
    }
}
