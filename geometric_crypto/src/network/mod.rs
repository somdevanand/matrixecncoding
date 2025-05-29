// geometric_crypto/src/network/mod.rs
// Declare submodules first
pub mod layer;
pub mod serialization; // Should exist, if not, create placeholder
pub mod streaming;   // Placeholder for now
pub mod sync;        // Placeholder for now
pub mod protocol;    // Placeholder for now

use crate::security::IntegrityProof; // For NetworkError variant if needed

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Serialization Error: {0}")]
    SerializationError(String), // Can be from bincode or custom
    #[error("Deserialization Error: {0}")]
    DeserializationError(String),
    #[error("Invalid data format: {0}")]
    InvalidFormat(String),
    #[error("Operation not implemented yet: {0}")]
    NotImplemented(String),
    #[error("Proof mismatch during unpackaging")] // Example specific error
    ProofMismatch(IntegrityProof), // Assuming IntegrityProof itself is the data for comparison
    #[error("Version mismatch: expected {expected}, got {got}")]
    VersionMismatch { expected: u16, got: u16 }, // Corrected u116 to u16
}

// Re-export key public types
pub use layer::NetworkLayer;
pub use streaming::{StreamingFrame, frame_data, deframe_data};
pub use sync::{SeedRequest, SeedResponse, CheckpointData, create_seed_request, handle_seed_request, verify_seed_response, get_sync_checkpoint, request_retransmit};
pub use protocol::PROTOCOL_VERSION; // Added line
// pub use serialization::{serialize_operations, deserialize_operations}; // If these are still top-level functions
