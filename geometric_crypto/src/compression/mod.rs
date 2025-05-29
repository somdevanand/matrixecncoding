// geometric_crypto/src/compression/mod.rs

// Declare submodules within the compression module
pub mod engine;
pub mod operations;
pub mod pattern_analyzer;
pub mod optimizer;
pub mod encoding;
pub mod clustering;

// Optional: Re-export key public types for easier access from outside the compression module
// e.g., crate::compression::CompressionEngine instead of crate::compression::engine::CompressionEngine
pub use engine::{CompressionEngine, CompressionError};
pub use operations::GeometricOperation;
// Re-export other types like PatternAnalyzer, PatternCandidate, etc., if desired.
// For now, only re-exporting the main engine, error, and operation enum.
