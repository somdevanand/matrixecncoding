// geometric_crypto/src/system/mod.rs
pub mod core_engine;
pub mod main_system; // Added line for the new file

pub use core_engine::{CoreEngine, CoreEngineConfig};
// Re-export types from main_system.rs
pub use main_system::{
    GeometricCryptoSystem, 
    SystemConfiguration, 
    CompressionLevel, 
    SecurityLevel,
    GeometricCryptoError, // Main error type
};
