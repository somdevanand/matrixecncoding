use crate::core::coordinates::Coordinate3D;
use super::operations::GeometricOperation; // Use super:: to access sibling module
use super::pattern_analyzer::PatternAnalyzer;
use super::optimizer::OperationOptimizer;
use super::encoding::GeometricEncoder;
// Remove placeholder struct definitions for PatternAnalyzer, etc. if they were here.

#[derive(Debug, thiserror::Error)]
pub enum CompressionError {
    #[error("Operation not implemented yet: {0}")]
    NotImplementedDetails(String), // Added a variant to provide more context

    #[error("An underlying component failed: {0}")]
    ComponentError(String), // For errors from sub-components

    #[error("No solution found by optimizer")]
    OptimizationFailed,
    
    #[error("NotImplemented")] // Keep the generic one if preferred for stubs
    NotImplemented,
}

#[derive(Debug, Clone)] // Added Clone
pub struct CompressionEngine {
    pattern_analyzer: PatternAnalyzer,
    operation_optimizer: OperationOptimizer,
    geometric_encoder: GeometricEncoder,
}

impl CompressionEngine {
    pub fn new() -> Self {
        Self {
            pattern_analyzer: PatternAnalyzer::new(),
            operation_optimizer: OperationOptimizer::new(),
            geometric_encoder: GeometricEncoder::new(),
        }
    }

    /// Multi-pass compression analysis (now fully synchronous)
    pub fn compress_coordinate_sequence(
        &self,
        coords: &[Coordinate3D]
    ) -> Result<Vec<u8>, CompressionError> { // Return type changed to Vec<u8>
        let min_window_size = 4;
        let max_window_size = 32;
        
        let patterns = self.pattern_analyzer.find_patterns(coords, min_window_size, max_window_size)
            .map_err(|e| CompressionError::ComponentError(format!("Pattern analysis (find_patterns) failed: {}", e)))?;
        
        // Pass example values for epsilon_sq and min_points
        let clusters = self.pattern_analyzer.spatial_clustering(coords, 3, 2) 
            .map_err(|e| CompressionError::ComponentError(format!("Spatial clustering failed: {}", e)))?;
        
        let optimized_ops = self.operation_optimizer.optimize_operations(coords, &patterns, &clusters)
            .map_err(|e| CompressionError::ComponentError(format!("Operation optimization failed: {}", e)))?;
        
        // final_ops is now Vec<u8>
        let final_encoded_bytes = self.geometric_encoder.encode_operations(optimized_ops)
            .map_err(|e| CompressionError::ComponentError(format!("Geometric encoding failed: {}", e)))?;

        Ok(final_encoded_bytes) // Return the Vec<u8>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coordinates::Coordinate3D;
    use crate::compression::operations::GeometricOperation; // For deserialization in test

    #[test]
    fn test_compression_engine_new() {
        let _engine = CompressionEngine::new();
        // Basic check
    }

    #[test]
    fn test_compress_coordinate_sequence_sync_flow() {
        let engine = CompressionEngine::new();
        
        // Scenario 1: Empty input coordinates
        let empty_coords: Vec<Coordinate3D> = Vec::new();
        let result_empty = engine.compress_coordinate_sequence(&empty_coords);
        assert!(result_empty.is_ok(), "Empty input should result in Ok");
        // find_patterns -> Ok(vec![])
        // spatial_clustering -> Ok(vec![])
        // optimize_operations -> Ok(vec![])
        // encode_operations -> Ok(vec![]) // which is an empty Vec<u8>
        assert!(result_empty.unwrap().is_empty(), "Expected empty Vec<u8> for empty input");
        
        // Scenario 3: Coords that form a pattern (from previous test, Scenario 2 omitted for brevity based on instructions)
        // This will now produce a non-empty Vec<u8>
        let patterned_coords = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), Coordinate3D::new(4,0,0),
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), Coordinate3D::new(4,0,0),
        ];
        let result_patterned = engine.compress_coordinate_sequence(&patterned_coords);
        assert!(result_patterned.is_ok(), "Patterned input should result in Ok: {:?}", result_patterned.err());
        let encoded_bytes_patterned = result_patterned.unwrap();
        assert!(!encoded_bytes_patterned.is_empty(), "Expected non-empty Vec<u8> for patterned input");
        
        // Verify by deserializing (optional, but good)
        let deserialized_ops: Vec<GeometricOperation> = bincode::deserialize(&encoded_bytes_patterned)
            .expect("Bincode deserialization failed for verification in test");
        assert_eq!(deserialized_ops.len(), 1, "Expected one operation after deserialization"); 
        match &deserialized_ops[0] {
            GeometricOperation::PatternReference { base_coordinate, pattern_id, .. } => {
                assert_eq!(*base_coordinate, Coordinate3D::new(1,0,0));
                assert_eq!(*pattern_id, 0);
            }
            _ => panic!("Expected PatternReference operation for patterned input after deserialization"),
        }
    }
}
