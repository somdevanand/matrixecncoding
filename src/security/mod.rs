// geometric_crypto/src/security/mod.rs

// Declare submodules first (order can matter for `super::` resolution if used in these files)
pub mod obfuscation;
pub mod integrity;
pub mod key_exchange;
pub mod layer;
pub mod key_manager; 
pub mod audit; // Added line

// Common error type for the security module
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Operation not implemented yet: {0}")]
    NotImplemented(String),
    #[error("Integrity check failed")]
    IntegrityCheckFailed,
    #[error("Key exchange failed: {0}")]
    KeyExchangeError(String),
    #[error("Obfuscation error: {0}")]
    ObfuscationError(String),
    // Add other specific error variants as needed
}

// Common security context placeholder
// It can be expanded later to include keys, nonces, algorithm choices, etc.
#[derive(Debug, Clone, Default)] // Added Default
pub struct SecurityContext; 
// Example of adding a field:
// pub struct SecurityContext {
//    pub key_material: Option<[u8; 32]>,
// }

// Common structure for integrity proofs
#[derive(Debug, Default, Clone, PartialEq, Eq)] // Added PartialEq, Eq
pub struct IntegrityProof(pub Vec<u8>); // A simple proof based on a byte vector (e.g., a hash)


// Re-export key public types for easier access from outside the security module,
// e.g., crate::security::SecurityLayer instead of crate::security::layer::SecurityLayer.
pub use layer::SecurityLayer;
pub use key_manager::KeyManager; 
pub use audit::log_security_event; // Added line
// CoordinateObfuscator and IntegrityVerifier are typically used internally by SecurityLayer,
// but can be re-exported if direct access is desired. For now, keep them internal to SecurityLayer's usage.
// pub use obfuscation::CoordinateObfuscator;
// pub use integrity::IntegrityVerifier;
