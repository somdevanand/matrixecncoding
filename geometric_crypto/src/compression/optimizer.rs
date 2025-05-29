use crate::core::coordinates::Coordinate3D;
use crate::compression::operations::{GeometricOperation, Matrix3x3}; // Assuming fill_byte change is in operations.rs
use crate::compression::pattern_analyzer::PatternCandidate;
use crate::compression::engine::CompressionError;
use std::collections::HashSet; // For managing covered coordinates/indices

#[derive(Debug, Default, Clone)]
pub struct OperationOptimizer;

impl OperationOptimizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Optimizes operations using an enhanced greedy approach.
    pub fn optimize_operations(
        &self,
        coords: &[Coordinate3D], // Original coordinates needed to get base_coordinate for patterns
        patterns: &[PatternCandidate],
        clusters: &[Vec<Coordinate3D>],
    ) -> Result<Vec<GeometricOperation>, CompressionError> {
        let mut operations: Vec<GeometricOperation> = Vec::new();
        // For simplicity, assume coords is small enough that a Vec<bool> is fine.
        // For larger inputs, a more complex data structure might be needed.
        let mut covered_indices: HashSet<usize> = HashSet::new(); 
                                                // Stores indices from the original `coords` slice

        // 1. Prioritize patterns
        // Sort patterns by compression_potential (descending)
        let mut sorted_patterns = patterns.to_vec(); // Clone to sort
        sorted_patterns.sort_by(|a, b| b.compression_potential.partial_cmp(&a.compression_potential).unwrap_or(std::cmp::Ordering::Equal));

        for pattern_candidate in sorted_patterns {
            if pattern_candidate.coordinates.is_empty() || pattern_candidate.compression_potential <= 0.0 {
                continue;
            }

            // Try to apply this pattern multiple times if its occurrences don't overlap significantly
            // This basic version will just try to apply it based on its first_occurrence_index if not covered.
            // A full solution would search for all non-overlapping occurrences.
            if let Some(start_idx) = pattern_candidate.first_occurrence_index {
                let pattern_len = pattern_candidate.coordinates.len();
                let mut current_op_covers_new_indices = false;
                // Check if *any* part of this specific instance would cover new ground
                for i in 0..pattern_len {
                     if start_idx + i >= coords.len() { // Boundary check
                        current_op_covers_new_indices = false; // Invalid pattern instance
                        break;
                    }
                    if !covered_indices.contains(&(start_idx + i)) {
                        current_op_covers_new_indices = true;
                        break;
                    }
                }

                if current_op_covers_new_indices {
                    // Check if this specific occurrence is (mostly) uncovered
                    let mut pattern_instance_indices_to_cover = Vec::new();
                    let mut can_apply = true; // Assume true until proven otherwise
                    for i in 0..pattern_len {
                        // This boundary check is redundant due to the one above if start_idx + i doesn't change
                        // but keeping for safety if logic evolves.
                        if start_idx + i >= coords.len() { 
                            can_apply = false; break;
                        }
                        if covered_indices.contains(&(start_idx + i)) {
                            // Overlap logic: for now, if any part of the first occurrence is covered, skip.
                            // A more complex logic could allow partial overlaps or find other occurrences.
                            // For this simplified greedy version, we only consider the first reported occurrence.
                            // If this first occurrence is blocked, we don't use this pattern candidate.
                            can_apply = false; break;
                        }
                        pattern_instance_indices_to_cover.push(start_idx+i);
                    }

                    if can_apply {
                        operations.push(GeometricOperation::PatternReference {
                            base_coordinate: coords[start_idx], // Use actual coord from original sequence
                            pattern_id: 0, // Placeholder ID
                            transformation_matrix: Matrix3x3::identity(),
                        });
                        for idx_to_cover in pattern_instance_indices_to_cover {
                            covered_indices.insert(idx_to_cover);
                        }
                    }
                }
            }
        }

        // 2. Process clusters for RegionFill (basic version)
        for cluster_coords in clusters {
            if cluster_coords.is_empty() {
                continue;
            }

            // Check if a significant portion of the cluster is uncovered.
            // For simplicity, if any point in the cluster is uncovered, try to process.
            // A more robust check: is a majority of points in cluster_coords (by index) uncovered?
            // This part needs revision: clusters are Vec<Coordinate3D>, not Vec<usize>.
            // We need a way to map cluster_coords back to their original indices to use `covered_indices`.
            // For now, let's assume clusters are processed independently and we don't check covered_indices for them.
            // This is a simplification and will be improved if coordinate mapping is available.
            // The following logic is a placeholder and does not interact with `covered_indices` for clusters.

            if cluster_coords.len() >= 2 { // Need at least 2 points to define a region's start/end meaningfully
                let mut min_x = u8::MAX; let mut max_x = u8::MIN;
                let mut min_y = u8::MAX; let mut max_y = u8::MIN;
                let mut min_z = u8::MAX; let mut max_z = u8::MIN;

                for coord in cluster_coords {
                    min_x = min_x.min(coord.x); max_x = max_x.max(coord.x);
                    min_y = min_y.min(coord.y); max_y = max_y.max(coord.y);
                    min_z = min_z.min(coord.z); max_z = max_z.max(coord.z);
                }
                
                // Heuristic: if the bounding box is not excessively large compared to num points.
                // For now, always try to create a RegionFill if cluster is not empty.
                operations.push(GeometricOperation::RegionFill {
                    start: Coordinate3D::new(min_x, min_y, min_z),
                    end: Coordinate3D::new(max_x, max_y, max_z),
                    fill_byte: 0, // Placeholder fill_byte, assuming fill_byte was added to RegionFill
                    compression_ratio: cluster_coords.len() as f32 / 10.0, // Placeholder
                });
                // Mark coordinates within this bounding box as covered (complex, skip for now)
            }
        }
        Ok(operations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coordinates::Coordinate3D;
    use crate::compression::pattern_analyzer::PatternCandidate;
    // use crate::compression::operations::GeometricOperation; // Not needed if only matching variants

    #[test]
    fn test_optimize_operations_enhanced_greedy() {
        let optimizer = OperationOptimizer::new();
        
        let coords = vec![ // Define coords for context
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), // P1, indices 0,1,2
            Coordinate3D::new(10,0,0), // Separator, index 3
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), // P1 again, indices 4,5,6
        ];

        // Case 1: Prioritize best pattern
        let pattern1 = PatternCandidate {
            coordinates: vec![coords[0], coords[1], coords[2]], // (1,0,0), (2,0,0), (3,0,0)
            frequency: 2, spatial_density: 0.3, compression_potential: 20.0, first_occurrence_index: Some(0)
        };
        let pattern2 = PatternCandidate {
            coordinates: vec![coords[0], coords[1]], // (1,0,0), (2,0,0)
            frequency: 2, spatial_density: 0.5, compression_potential: 10.0, first_occurrence_index: Some(0)
        };
        let patterns = vec![pattern2.clone(), pattern1.clone()]; // P1 is better, P2 is also there
        let clusters: Vec<Vec<Coordinate3D>> = Vec::new();

        let result = optimizer.optimize_operations(&coords, &patterns, &clusters);
        assert!(result.is_ok());
        let ops = result.unwrap();
        assert_eq!(ops.len(), 1, "Should pick the best pattern (P1)");
        match &ops[0] {
            GeometricOperation::PatternReference { base_coordinate, .. } => {
                assert_eq!(*base_coordinate, coords[0]); // Base of P1
            }
            _ => panic!("Expected PatternReference for best pattern"),
        }

        // Case 2: Pattern covers indices, then cluster for remaining
        // This test focuses on cluster processing independently as pattern coverage of clusters is not yet implemented.
        let patterns_empty: Vec<PatternCandidate> = Vec::new();
        let cluster1_coords = vec![Coordinate3D::new(10,0,0), Coordinate3D::new(11,0,0), Coordinate3D::new(10,1,0)];
        let clusters_one = vec![cluster1_coords.clone()];
        
        let result_cluster = optimizer.optimize_operations(&coords, &patterns_empty, &clusters_one);
        assert!(result_cluster.is_ok());
        let ops_cluster = result_cluster.unwrap();
        assert_eq!(ops_cluster.len(), 1, "Should create RegionFill for cluster");
        match &ops_cluster[0] {
            GeometricOperation::RegionFill { start, end, fill_byte, .. } => {
                assert_eq!(*start, Coordinate3D::new(10,0,0));
                assert_eq!(*end, Coordinate3D::new(11,1,0));
                assert_eq!(*fill_byte, 0); // Placeholder
            }
            _ => panic!("Expected RegionFill for cluster"),
        }
        
        // Case 3: Pattern is chosen, second occurrence of pattern is blocked by the first.
        // Create two patterns. P1 is better. P1_second_occurrence starts at index 4.
        // If P1 (first_occurrence_index: Some(0)) is chosen, indices 0,1,2 are covered.
        // Then P1_second_occurrence should not be chosen.
        let p1_first = PatternCandidate {
             coordinates: vec![coords[0], coords[1], coords[2]], 
             frequency: 2, spatial_density: 0.3, compression_potential: 20.0, first_occurrence_index: Some(0)
        };
        let p1_second = PatternCandidate { // Same pattern, different potential start, less potential to ensure it's not chosen first
             coordinates: vec![coords[4], coords[5], coords[6]], 
             frequency: 2, spatial_density: 0.3, compression_potential: 19.0, first_occurrence_index: Some(4)
        };
        let patterns_overlapping = vec![p1_first.clone(), p1_second.clone()];
        let result_overlapping = optimizer.optimize_operations(&coords, &patterns_overlapping, &clusters);
        assert!(result_overlapping.is_ok());
        let ops_overlapping = result_overlapping.unwrap();
        assert_eq!(ops_overlapping.len(), 1, "Should only pick the first occurrence of P1 as second is now covered");
         match &ops_overlapping[0] {
            GeometricOperation::PatternReference { base_coordinate, .. } => {
                 assert_eq!(*base_coordinate, coords[0]); // Base of P1 (first occurrence)
            }
            _ => panic!("Expected PatternReference for P1's first occurrence"),
        }


        // Case 4: No beneficial patterns, no clusters
        let result_all_empty = optimizer.optimize_operations(&coords, &patterns_empty, &Vec::new());
        assert!(result_all_empty.is_ok());
        assert!(result_all_empty.unwrap().is_empty(), "Should be empty if no patterns and no clusters");
    }
}
