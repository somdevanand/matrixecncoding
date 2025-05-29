// geometric_crypto/src/core/mod.rs
pub mod coordinates;
pub mod matrix;
pub mod seed;
pub mod transforms;
pub mod cache_structs;
pub mod cache; 
pub mod memory_optimizer;
#[cfg(feature = "parallel")] // Added cfg attribute
pub mod parallel_processor; // Added line

pub use memory_optimizer::{MemoryOptimizer, MemoryProfile};
#[cfg(feature = "parallel")] // Added cfg attribute
pub use parallel_processor::ParallelProcessor; // Added line
