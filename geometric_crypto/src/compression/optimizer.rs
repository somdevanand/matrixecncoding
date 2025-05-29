use crate::core::coordinates::Coordinate3D;
use crate::compression::operations::{GeometricOperation, Matrix3x3};
use crate::compression::pattern_analyzer::PatternCandidate;
use crate::compression::engine::CompressionError;

#[derive(Debug, Default, Clone)]
pub struct OperationOptimizer;

impl OperationOptimizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Optimizes operations using a greedy approach based on patterns.
    /// This version only considers the best pattern.
    pub fn optimize_operations(
        &self,
        patterns: &[PatternCandidate],
        _clusters: &[Vec<Vec<Coordinate3D>>] // _clusters not used in this basic greedy version
    ) -> Result<Vec<GeometricOperation>, CompressionError> {
        if patterns.is_empty() {
            return Ok(Vec::new());
        }

        // Find the pattern with the highest compression_potential.
        // Note: f32 comparison in max_by can be tricky if NaN is possible, but should be fine here.
        let best_pattern = patterns.iter()
            .filter(|p| !p.coordinates.is_empty() && p.compression_potential > 0.0) // Ensure pattern is valid and beneficial
            .max_by(|a, b| a.compression_potential.partial_cmp(&b.compression_potential).unwrap_or(std::cmp::Ordering::Equal));

        if let Some(pattern_to_use) = best_pattern {
            // Use the first coordinate of the pattern sequence as its base_coordinate for the reference.
            // A more sophisticated approach might store a canonical base or allow selection.
            let base_coord_for_ref = pattern_to_use.coordinates[0]; 
            
            // A placeholder for pattern_id. In a real system, patterns would be cataloged and given unique IDs.
            // Here, we could use its index in the input `patterns` array, or a hash, or just 0.
            // For simplicity, using a fixed ID.
            let pattern_id_for_ref = 0; // Placeholder ID

            let pattern_ref_op = GeometricOperation::PatternReference {
                base_coordinate: base_coord_for_ref,
                pattern_id: pattern_id_for_ref, 
                transformation_matrix: Matrix3x3::identity(), // No transformation in this basic stub
            };
            Ok(vec![pattern_ref_op])
        } else {
            // No beneficial patterns found
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coordinates::Coordinate3D;
    use crate::compression::pattern_analyzer::PatternCandidate;

    #[test]
    fn test_optimize_operations_greedy_sync() {
        let optimizer = OperationOptimizer::new();
        let clusters: Vec<Vec<Coordinate3D>> = Vec::new(); // Not used by this stub

        // Test case 1: No patterns
        let no_patterns: Vec<PatternCandidate> = Vec::new();
        let result_no_patterns = optimizer.optimize_operations(&no_patterns, &clusters);
        assert!(result_no_patterns.is_ok());
        assert!(result_no_patterns.unwrap().is_empty());

        // Test case 2: One pattern
        let pattern1_coords = vec![Coordinate3D::new(1,2,3), Coordinate3D::new(4,5,6)];
        let patterns_one = vec![PatternCandidate {
            coordinates: pattern1_coords.clone(),
            frequency: 2, spatial_density: 0.5, compression_potential: 10.0, first_occurrence_index: Some(0)
        }];
        let result_one_pattern = optimizer.optimize_operations(&patterns_one, &clusters);
        assert!(result_one_pattern.is_ok());
        let ops_one = result_one_pattern.unwrap();
        assert_eq!(ops_one.len(), 1);
        match &ops_one[0] {
            GeometricOperation::PatternReference { base_coordinate, pattern_id, .. } => {
                assert_eq!(*base_coordinate, pattern1_coords[0]);
                assert_eq!(*pattern_id, 0); // Placeholder ID
            }
            _ => panic!("Expected PatternReference"),
        }

        // Test case 3: Multiple patterns, chooses best potential
        let pattern2_coords = vec![Coordinate3D::new(7,8,9)];
        let patterns_multi = vec![
            PatternCandidate { // Lower potential
                coordinates: pattern1_coords.clone(),
                frequency: 2, spatial_density: 0.5, compression_potential: 10.0, first_occurrence_index: Some(0)
            },
            PatternCandidate { // Higher potential
                coordinates: pattern2_coords.clone(),
                frequency: 3, spatial_density: 1.0, compression_potential: 20.0, first_occurrence_index: Some(10)
            },
        ];
        let result_multi_pattern = optimizer.optimize_operations(&patterns_multi, &clusters);
        assert!(result_multi_pattern.is_ok());
        let ops_multi = result_multi_pattern.unwrap();
        assert_eq!(ops_multi.len(), 1); // Greedy: picks one best
        match &ops_multi[0] {
            GeometricOperation::PatternReference { base_coordinate, .. } => {
                assert_eq!(*base_coordinate, pattern2_coords[0]); // Base of the higher potential pattern
            }
            _ => panic!("Expected PatternReference"),
        }
        
        // Test case 4: Pattern with zero or negative potential (should be ignored)
        let patterns_low_potential = vec![PatternCandidate {
            coordinates: pattern1_coords.clone(),
            frequency: 1, spatial_density: 0.5, compression_potential: 0.0, first_occurrence_index: Some(0)
        }];
        let result_low_potential = optimizer.optimize_operations(&patterns_low_potential, &clusters);
        assert!(result_low_potential.is_ok());
        assert!(result_low_potential.unwrap().is_empty());
        
        // Test case 5: Pattern with empty coordinates (should be ignored)
        let patterns_empty_coords = vec![PatternCandidate {
            coordinates: vec![],
            frequency: 2, spatial_density: 0.0, compression_potential: 10.0, first_occurrence_index: Some(0)
        }];
        let result_empty_coords = optimizer.optimize_operations(&patterns_empty_coords, &clusters);
        assert!(result_empty_coords.is_ok());
        assert!(result_empty_coords.unwrap().is_empty());
    }
}
