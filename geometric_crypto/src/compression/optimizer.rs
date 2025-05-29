use crate::core::coordinates::Coordinate3D;
use crate::compression::operations::{GeometricOperation, Matrix3x3}; // Added Matrix3x3
use crate::compression::pattern_analyzer::PatternCandidate;
use crate::compression::engine::CompressionError;

#[derive(Debug, Default, Clone)]
pub struct OperationOptimizer;

impl OperationOptimizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Optimizes a sequence of identified patterns and spatial clusters into
    /// a compact set of GeometricOperations. (Synchronous Stub)
    pub fn optimize_operations(
        &self,
        patterns: &[PatternCandidate],
        _clusters: &[Vec<Coordinate3D>] // _clusters is not used in this basic stub
    ) -> Result<Vec<GeometricOperation>, CompressionError> {
        if patterns.is_empty() {
            // If no patterns, maybe process clusters or return empty. For now, empty.
            return Ok(Vec::new());
        }

        // Basic stub: Convert the first pattern found into a PatternReference operation.
        // A real optimizer would consider all patterns, clusters, and their interplay.
        if let Some(first_pattern) = patterns.first() {
            if !first_pattern.coordinates.is_empty() {
                let pattern_ref_op = GeometricOperation::PatternReference {
                    base_coordinate: first_pattern.coordinates[0], // Use first coord of pattern as base
                    pattern_id: 0, // Placeholder PatternId
                    transformation_matrix: Matrix3x3::identity(), // No transformation for stub
                };
                return Ok(vec![pattern_ref_op]);
            }
        }
        Ok(Vec::new()) // Return empty if first pattern is empty or no patterns
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coordinates::Coordinate3D;
    use crate::compression::pattern_analyzer::PatternCandidate;
    // Removed unused GeometricOperation import from here, as it's used in main code

    // Updated test for optimize_operations (non-async)
    #[test]
    fn test_optimize_operations_stub_sync() {
        let optimizer = OperationOptimizer::new();
        let mut patterns: Vec<PatternCandidate> = Vec::new();
        let clusters: Vec<Vec<Coordinate3D>> = Vec::new();

        // Test case 1: No patterns
        let result_no_patterns = optimizer.optimize_operations(&patterns, &clusters);
        assert!(result_no_patterns.is_ok());
        assert!(result_no_patterns.unwrap().is_empty(), "Should return empty ops for no patterns");

        // Test case 2: With a simple pattern
        patterns.push(PatternCandidate {
            coordinates: vec![Coordinate3D::new(1,2,3), Coordinate3D::new(4,5,6)],
            frequency: 2,
            spatial_density: 0.5,
            compression_potential: 2.0,
            first_occurrence_index: Some(0),
        });
        let result_with_pattern = optimizer.optimize_operations(&patterns, &clusters);
        assert!(result_with_pattern.is_ok());
        let ops = result_with_pattern.unwrap();
        assert_eq!(ops.len(), 1, "Should produce one operation for one pattern");
        match &ops[0] {
            GeometricOperation::PatternReference { base_coordinate, pattern_id, .. } => {
                assert_eq!(*base_coordinate, Coordinate3D::new(1,2,3));
                assert_eq!(*pattern_id, 0); // Placeholder ID
            }
            _ => panic!("Expected PatternReference operation"),
        }
        
        // Test case 3: Pattern with empty coordinates (should result in empty ops)
        patterns = vec![PatternCandidate {
            coordinates: vec![],
            frequency: 2,
            spatial_density: 0.0,
            compression_potential: 0.0,
            first_occurrence_index: Some(0),
        }];
        let result_empty_coord_pattern = optimizer.optimize_operations(&patterns, &clusters);
        assert!(result_empty_coord_pattern.is_ok());
        assert!(result_empty_coord_pattern.unwrap().is_empty(), "Should return empty ops for pattern with no coords");
    }
}
