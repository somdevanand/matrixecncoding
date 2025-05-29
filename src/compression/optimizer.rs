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

        // 2. Process clusters for RegionFill (basic version) - This logic will be replaced by iterating through coords.
        // The `clusters` input might be used by a more advanced RegionFill or other ops later.
        // For now, the primary logic will iterate `coords` using `covered_indices`.

        let mut i = 0;
        while i < coords.len() {
            if covered_indices.contains(&i) {
                i += 1;
                continue;
            }

            let coord_start = coords[i];
            let mut current_op_indices = Vec::new();

            // Try to detect a RegionFill (line along X, then Y, then Z for simplicity)
            // Minimum line length for RegionFill (e.g., 3)
            const MIN_LINE_LEN: usize = 3;

            // Attempt X-axis line
            let mut x_line_coords = vec![coord_start];
            current_op_indices.push(i);
            for j in 1..(coords.len() - i) {
                if covered_indices.contains(&(i + j)) { break; }
                let next_expected = Coordinate3D::new(coord_start.x.wrapping_add(j as u8), coord_start.y, coord_start.z);
                if coords[i+j] == next_expected {
                    x_line_coords.push(coords[i+j]);
                    current_op_indices.push(i+j);
                } else {
                    break;
                }
            }
            if x_line_coords.len() >= MIN_LINE_LEN {
                operations.push(GeometricOperation::RegionFill {
                    start: coord_start,
                    end: *x_line_coords.last().unwrap(),
                    fill_byte: 0, // Placeholder
                    compression_ratio: x_line_coords.len() as f32, // Simple ratio
                });
                for idx in &current_op_indices { covered_indices.insert(*idx); }
                i += x_line_coords.len();
                continue;
            }
            current_op_indices.clear(); // Reset for next attempt

            // Attempt Y-axis line (if X-line failed)
            let mut y_line_coords = vec![coord_start];
            current_op_indices.push(i);
            for j in 1..(coords.len() - i) {
                 if covered_indices.contains(&(i + j)) { break; }
                let next_expected = Coordinate3D::new(coord_start.x, coord_start.y.wrapping_add(j as u8), coord_start.z);
                // This check is flawed: coords[i+j] might not be the (j)-th element in a Y-line if data is sparse.
                // For simple contiguous check in original array:
                // This requires that coords[i], coords[i+1], coords[i+2] form the Y-line.
                // The current pattern analyzer and optimizer structure assumes patterns/ops replace contiguous segments of the original array.
                // Let's stick to that for now.
                if coords[i+j] == next_expected {
                    y_line_coords.push(coords[i+j]);
                    current_op_indices.push(i+j);
                } else {
                    break;
                }
            }
            if y_line_coords.len() >= MIN_LINE_LEN {
                 operations.push(GeometricOperation::RegionFill {
                    start: coord_start,
                    end: *y_line_coords.last().unwrap(),
                    fill_byte: 0, // Placeholder
                    compression_ratio: y_line_coords.len() as f32,
                });
                for idx in &current_op_indices { covered_indices.insert(*idx); }
                i += y_line_coords.len();
                continue;
            }
            current_op_indices.clear();


            // Attempt Z-axis line (if X and Y-lines failed)
            // Note: This is a simplified 1D line check along Z.
            // A true 3D RegionFill detection would be more complex.
            let mut z_line_coords = vec![coord_start];
            current_op_indices.push(i); // Start with current point
            for j in 1..(coords.len() - i) {
                if covered_indices.contains(&(i + j)) { break; }
                let next_expected = Coordinate3D::new(coord_start.x, coord_start.y, coord_start.z.wrapping_add(j as u8));
                if coords[i+j] == next_expected {
                    z_line_coords.push(coords[i+j]);
                    current_op_indices.push(i+j);
                } else {
                    break;
                }
            }
            if z_line_coords.len() >= MIN_LINE_LEN {
                 operations.push(GeometricOperation::RegionFill {
                    start: coord_start,
                    end: *z_line_coords.last().unwrap(),
                    fill_byte: 0, // Placeholder
                    compression_ratio: z_line_coords.len() as f32,
                });
                for idx_to_cover in current_op_indices { covered_indices.insert(idx_to_cover); }
                i += z_line_coords.len();
                // current_op_indices.clear(); // Not strictly needed here due to continue
                continue; 
            }
            current_op_indices.clear(); // Clear if Z-line check fails

            // Fallback: PathTrace. Collect subsequent points as long as they don't start a new line.
            current_op_indices.clear();
            let mut path_trace_waypoints = vec![coord_start];
            current_op_indices.push(i);

            for k in 1..(coords.len() - i) {
                let lookahead_idx = i + k;
                if covered_indices.contains(&lookahead_idx) {
                    break; 
                }

                // Check if coords[lookahead_idx] could start a new line of MIN_LINE_LEN.
                // If it can, the current PathTrace should end *before* this point.
                let mut can_start_new_line_at_lookahead = false;
                if lookahead_idx + MIN_LINE_LEN <= coords.len() {
                    // Check X-line from lookahead_idx
                    let mut potential_x_line = true;
                    for l in 0..MIN_LINE_LEN {
                        // A point in a potential new line must not be covered, unless it's the very first point (l=0, which is lookahead_idx)
                        if covered_indices.contains(&(lookahead_idx + l)) && l > 0 { potential_x_line = false; break; }
                        if lookahead_idx + l >= coords.len() { potential_x_line = false; break; }
                        let expected = Coordinate3D::new(coords[lookahead_idx].x.wrapping_add(l as u8), coords[lookahead_idx].y, coords[lookahead_idx].z);
                        if coords[lookahead_idx + l] != expected { potential_x_line = false; break; }
                    }
                    if potential_x_line { can_start_new_line_at_lookahead = true; }

                    // Check Y-line
                    if !can_start_new_line_at_lookahead {
                        let mut potential_y_line = true;
                        for l in 0..MIN_LINE_LEN {
                            if covered_indices.contains(&(lookahead_idx + l)) && l > 0 { potential_y_line = false; break; }
                            if lookahead_idx + l >= coords.len() { potential_y_line = false; break; }
                            let expected = Coordinate3D::new(coords[lookahead_idx].x, coords[lookahead_idx].y.wrapping_add(l as u8), coords[lookahead_idx].z);
                            if coords[lookahead_idx + l] != expected { potential_y_line = false; break; }
                        }
                        if potential_y_line { can_start_new_line_at_lookahead = true; }
                    }
                    // Check Z-line
                    if !can_start_new_line_at_lookahead {
                        let mut potential_z_line = true;
                        for l in 0..MIN_LINE_LEN {
                            if covered_indices.contains(&(lookahead_idx + l)) && l > 0 { potential_z_line = false; break; }
                            if lookahead_idx + l >= coords.len() { potential_z_line = false; break; }
                            let expected = Coordinate3D::new(coords[lookahead_idx].x, coords[lookahead_idx].y, coords[lookahead_idx].z.wrapping_add(l as u8));
                            if coords[lookahead_idx + l] != expected { potential_z_line = false; break; }
                        }
                        if potential_z_line { can_start_new_line_at_lookahead = true; }
                    }
                }

                if can_start_new_line_at_lookahead {
                    break; 
                }

                // NEW CHECK: Is coords[lookahead_idx] sequential to path_trace_waypoints.last()?
                let last_waypoint = path_trace_waypoints.last().unwrap();
                let next_coord = coords[lookahead_idx];
                
                let is_sequential_x = next_coord.y == last_waypoint.y && next_coord.z == last_waypoint.z && 
                                      (next_coord.x == last_waypoint.x.wrapping_add(1) || (last_waypoint.x > 0 && next_coord.x == last_waypoint.x.wrapping_sub(1)));
                let is_sequential_y = next_coord.x == last_waypoint.x && next_coord.z == last_waypoint.z &&
                                      (next_coord.y == last_waypoint.y.wrapping_add(1) || (last_waypoint.y > 0 && next_coord.y == last_waypoint.y.wrapping_sub(1)));
                let is_sequential_z = next_coord.x == last_waypoint.x && next_coord.y == last_waypoint.y &&
                                      (next_coord.z == last_waypoint.z.wrapping_add(1) || (last_waypoint.z > 0 && next_coord.z == last_waypoint.z.wrapping_sub(1)));

                if !(is_sequential_x || is_sequential_y || is_sequential_z) {
                    // If not immediately sequential in any primary direction (forward or backward by 1), end this PathTrace.
                    break; 
                }
                
                path_trace_waypoints.push(next_coord);
                current_op_indices.push(lookahead_idx);
            }

            operations.push(GeometricOperation::PathTrace {
                waypoints: path_trace_waypoints.clone(),
                interpolation: crate::compression::operations::InterpolationType::None,
                data_encoding: crate::compression::operations::EncodingScheme::Raw,
            });
            for idx_to_cover in &current_op_indices {
                covered_indices.insert(*idx_to_cover);
            }
            i += path_trace_waypoints.len();
        }
        Ok(operations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coordinates::Coordinate3D;
    use crate::compression::pattern_analyzer::PatternCandidate;
    use crate::compression::operations::{InterpolationType, EncodingScheme};


    fn get_test_coords() -> Vec<Coordinate3D> {
        vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), // Pattern P1
            Coordinate3D::new(10,0,0), // Separator
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), // P1 again
            Coordinate3D::new(5,5,5), Coordinate3D::new(5,6,5), Coordinate3D::new(5,7,5), // Y-Line L1
            Coordinate3D::new(10,10,10), // Single point S1
            Coordinate3D::new(20,0,0), Coordinate3D::new(21,0,0), // Short X-Line (len 2, below min for RegionFill)
        ]
    }

    #[test]
    fn test_optimize_operations_empty_input() {
        let optimizer = OperationOptimizer::new();
        let result = optimizer.optimize_operations(&[], &[], &[]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_optimize_prioritizes_patterns() {
        let optimizer = OperationOptimizer::new();
        let coords = get_test_coords(); // Uses indices 0,1,2 and 4,5,6 for P1

        let pattern1 = PatternCandidate {
            coordinates: vec![coords[0], coords[1], coords[2]],
            frequency: 2, spatial_density: 0.3, compression_potential: 20.0, first_occurrence_index: Some(0)
        };
        // A less optimal pattern that also matches at index 0
        let pattern2 = PatternCandidate {
            coordinates: vec![coords[0], coords[1]],
            frequency: 2, spatial_density: 0.5, compression_potential: 10.0, first_occurrence_index: Some(0)
        };
        let patterns = vec![pattern2, pattern1.clone()]; // P1 is better but listed second
        
        let result = optimizer.optimize_operations(&coords, &patterns, &[]);
        assert!(result.is_ok());
        let ops = result.unwrap();
        
        // Expect P1 (PatternReference for indices 0,1,2)
        // Then the rest processed by new logic:
        // coords[3] = (10,0,0) -> PathTrace (single point)
        // coords[4,5,6] (P1 again) are now covered by the first P1 due to greedy pattern application.
        // This needs refinement in how patterns are chosen if multiple non-overlapping can exist.
        // The current pattern logic in optimizer only applies one instance of a pattern.
        // Let's adjust test expectation: if P1 is applied, it covers 0,1,2.
        // The *current* pattern logic in the provided optimizer.rs stub is very basic:
        // it iterates sorted patterns and applies the *first available instance* of that pattern.
        // If pattern1 (P1) is chosen, it covers indices 0,1,2.
        // The second occurrence of P1 (indices 4,5,6) would be a *different* PatternCandidate if found by analyzer.
        // For this test, let's assume pattern analyzer gives only one candidate for P1 starting at 0.

        // Expected: P1 (0,1,2), Path(10,0,0), Path(1,0,0), Path(2,0,0), Path(3,0,0) ... this is not ideal.
        // The provided optimizer's pattern logic should be more robust in finding all occurrences.
        // For now, testing *my* new logic:
        // If P1 @ 0,1,2 is applied:
        // operations[0] = PatternReference for (1,0,0),(2,0,0),(3,0,0)
        // covered_indices = {0,1,2}
        // i = 3 (10,0,0) -> PathTrace for (10,0,0)
        // i = 4 (1,0,0) -> PathTrace for (1,0,0),(2,0,0),(3,0,0) (indices 4,5,6) - this should become a RegionFill or Path
        // i = 7 (5,5,5) -> RegionFill for (5,5,5),(5,6,5),(5,7,5) (indices 7,8,9)
        // i = 10 (10,10,10) -> PathTrace for (10,10,10) (index 10)
        // i = 11 (20,0,0) -> PathTrace for (20,0,0),(21,0,0) (indices 11,12)
        
        // Given the current simple pattern logic, it will apply pattern1 once.
        assert!(!ops.is_empty());
        if let GeometricOperation::PatternReference { base_coordinate, .. } = &ops[0] {
            assert_eq!(*base_coordinate, coords[0]);
        } else {
            panic!("Expected first op to be PatternReference for P1");
        }
        
        // Check that other operations are formed for the rest
        // Example: check for the Y-Line L1 -> (5,5,5),(5,6,5),(5,7,5)
        let y_line_op_exists = ops.iter().any(|op| matches!(op, GeometricOperation::RegionFill { start, end, .. } if *start == Coordinate3D::new(5,5,5) && *end == Coordinate3D::new(5,7,5) ));
        assert!(y_line_op_exists, "Y-Line L1 should form a RegionFill. Ops: {:?}", ops);

        // Check that the second P1 instance (indices 4,5,6) is now a PathTrace or RegionFill
        // (as it wasn't covered by a *separate* PatternCandidate for that specific instance)
        let second_p1_path_exists = ops.iter().any(|op| match op {
            GeometricOperation::PathTrace { waypoints, .. } => waypoints.len() == 3 && waypoints[0] == coords[4] && waypoints[1] == coords[5] && waypoints[2] == coords[6],
            GeometricOperation::RegionFill { start, end, .. } => *start == coords[4] && *end == coords[6], // if X-line logic catches it
            _ => false
        });
        assert!(second_p1_path_exists, "Second instance of P1 should be a PathTrace or RegionFill. Ops: {:?}", ops);
    }


    #[test]
    fn test_optimize_line_detection_x_axis() {
        let optimizer = OperationOptimizer::new();
        let coords = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), Coordinate3D::new(4,0,0), // X-Line
            Coordinate3D::new(10,0,0), // Separator
        ];
        let result = optimizer.optimize_operations(&coords, &[], &[]); // No patterns, no clusters
        assert!(result.is_ok());
        let ops = result.unwrap();
        assert_eq!(ops.len(), 2); // RegionFill for X-Line, PathTrace for separator
        match &ops[0] {
            GeometricOperation::RegionFill { start, end, fill_byte, .. } => {
                assert_eq!(*start, Coordinate3D::new(1,0,0));
                assert_eq!(*end, Coordinate3D::new(4,0,0));
                assert_eq!(*fill_byte, 0);
            }
            _ => panic!("Expected RegionFill for X-Line, got {:?}", ops[0]),
        }
        match &ops[1] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                assert_eq!(waypoints.len(), 1);
                assert_eq!(waypoints[0], Coordinate3D::new(10,0,0));
            }
            _ => panic!("Expected PathTrace for separator, got {:?}", ops[1]),
        }
    }

    #[test]
    fn test_optimize_line_detection_y_axis() {
        let optimizer = OperationOptimizer::new();
        let coords = vec![
            Coordinate3D::new(5,5,5), Coordinate3D::new(5,6,5), Coordinate3D::new(5,7,5), // Y-Line
        ];
        let result = optimizer.optimize_operations(&coords, &[], &[]);
        assert!(result.is_ok());
        let ops = result.unwrap();
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            GeometricOperation::RegionFill { start, end, .. } => {
                assert_eq!(*start, Coordinate3D::new(5,5,5));
                assert_eq!(*end, Coordinate3D::new(5,7,5));
            }
            _ => panic!("Expected RegionFill for Y-Line, got {:?}", ops[0]),
        }
    }
    
    #[test]
    fn test_optimize_short_sequence_becomes_pathtrace() {
        let optimizer = OperationOptimizer::new();
        let coords = vec![
            Coordinate3D::new(20,0,0), Coordinate3D::new(21,0,0), // Short X-Line (len 2)
        ];
        let result = optimizer.optimize_operations(&coords, &[], &[]);
        assert!(result.is_ok());
        let ops = result.unwrap();
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                assert_eq!(waypoints.len(), 2);
                assert_eq!(waypoints[0], Coordinate3D::new(20,0,0));
                assert_eq!(waypoints[1], Coordinate3D::new(21,0,0));
            }
            _ => panic!("Expected PathTrace for short sequence, got {:?}", ops[0]),
        }
    }

    #[test]
    fn test_optimize_individual_points_become_pathtraces() {
        let optimizer = OperationOptimizer::new();
        let coords = vec![
            Coordinate3D::new(1,1,1),
            Coordinate3D::new(3,3,3),
            Coordinate3D::new(5,5,5),
        ];
        let result = optimizer.optimize_operations(&coords, &[], &[]);
        assert!(result.is_ok());
        let ops = result.unwrap();
        // Each individual point, if not forming a region, will become its own PathTrace(vec![point])
        assert_eq!(ops.len(), 3); 
        for (idx, op) in ops.iter().enumerate() {
            match op {
                GeometricOperation::PathTrace { waypoints, .. } => {
                    assert_eq!(waypoints.len(), 1);
                    assert_eq!(waypoints[0], coords[idx]);
                }
                _ => panic!("Expected PathTrace for individual point, got {:?}", op),
            }
        }
    }
    
    #[test]
    fn test_mixed_operations_pattern_lines_points() {
        let optimizer = OperationOptimizer::new();
        let coords = vec![
            // Pattern P1
            Coordinate3D::new(0,0,0), Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), // Indices 0,1,2
            // Separator Point S1
            Coordinate3D::new(10,0,0),                                                    // Index 3
            // Line L1 (Y-axis)
            Coordinate3D::new(5,5,5), Coordinate3D::new(5,6,5), Coordinate3D::new(5,7,5), // Indices 4,5,6
            // Short sequence P2
            Coordinate3D::new(20,0,0), Coordinate3D::new(21,0,0),                         // Indices 7,8
        ];

        let pattern_p1 = PatternCandidate {
            coordinates: vec![coords[0], coords[1], coords[2]],
            frequency: 1, spatial_density: 0.8, compression_potential: 30.0, first_occurrence_index: Some(0)
        };
        let patterns = vec![pattern_p1];

        let result = optimizer.optimize_operations(&coords, &patterns, &[]);
        assert!(result.is_ok());
        let ops = result.unwrap();

        // Expected order:
        // 1. PatternReference for P1 (indices 0,1,2)
        // 2. PathTrace for S1 (index 3)
        // 3. RegionFill for L1 (indices 4,5,6)
        // 4. PathTrace for P2 (indices 7,8)
        assert_eq!(ops.len(), 4, "Expected 4 operations, got {:?}", ops);

        match &ops[0] {
            GeometricOperation::PatternReference { base_coordinate, .. } => assert_eq!(*base_coordinate, coords[0]),
            _ => panic!("Op 0: Expected PatternReference for P1, got {:?}", ops[0]),
        }
        match &ops[1] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                assert_eq!(waypoints.len(), 1);
                assert_eq!(waypoints[0], coords[3]);
            },
            _ => panic!("Op 1: Expected PathTrace for S1, got {:?}", ops[1]),
        }
        match &ops[2] {
            GeometricOperation::RegionFill { start, end, .. } => {
                assert_eq!(*start, coords[4]);
                assert_eq!(*end, coords[6]);
            },
            _ => panic!("Op 2: Expected RegionFill for L1, got {:?}", ops[2]),
        }
        match &ops[3] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                assert_eq!(waypoints.len(), 2);
                assert_eq!(waypoints[0], coords[7]);
                assert_eq!(waypoints[1], coords[8]);
            },
            _ => panic!("Op 3: Expected PathTrace for P2, got {:?}", ops[3]),
        }
    }

    // Original test from the prompt, slightly adapted
    #[test]
    fn test_optimize_operations_original_cases() {
        let optimizer = OperationOptimizer::new();
        let coords = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), // P1
            Coordinate3D::new(10,0,0), // Separator
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), // P1 again
        ];

        // Case 1: Prioritize best pattern (from original test)
        // This part of the test is more about how the optimizer handles pre-supplied PatternCandidates
        let pattern_cand_1 = PatternCandidate {
            coordinates: vec![coords[0], coords[1], coords[2]],
            frequency: 2, spatial_density: 0.3, compression_potential: 20.0, first_occurrence_index: Some(0)
        };
        let pattern_cand_2 = PatternCandidate { // Same pattern, different instance/potential, to test sorting/selection
            coordinates: vec![coords[4], coords[5], coords[6]],
            frequency: 2, spatial_density: 0.3, compression_potential: 19.0, first_occurrence_index: Some(4)
        };
        // The optimizer's pattern logic will apply pattern_cand_1.
        // Then, when my new logic runs, indices 0,1,2 are covered.
        // It will process coord[3] (PathTrace).
        // Then it will process coords[4,5,6] which will likely become a RegionFill (X-line) or PathTrace.
        // It will NOT use pattern_cand_2 because that candidate's occurrence at index 4 is processed
        // by the subsequent loop, not by the pattern selection part if pattern_cand_1 was already chosen and covered earlier indices.
        // The current pattern logic in optimizer.rs is:
        //   for pattern_candidate in sorted_patterns { ... if can_apply { operations.push(PatternRef); covered_indices.insert() }}
        // This means it will select pattern_cand_1. It will then iterate again for pattern_cand_2.
        // pattern_cand_2's occurrence is at index 4. If indices 0,1,2 are covered by pattern_cand_1,
        // then pattern_cand_2 (starting at index 4) *can* still be applied by the pattern logic.
        // This means the pattern logic itself can add multiple PatternReference ops if they don't overlap.

        let patterns_for_case1 = vec![pattern_cand_1.clone(), pattern_cand_2.clone()];
        let result_case1 = optimizer.optimize_operations(&coords, &patterns_for_case1, &[]);
        assert!(result_case1.is_ok());
        let ops_case1 = result_case1.unwrap();
        
        // Expected: Two PatternReference ops (one for each occurrence of P1), then one PathTrace for the separator.
        assert_eq!(ops_case1.len(), 3, "Expected two PatternRefs and one PathTrace for separator. Ops: {:?}", ops_case1);
        
        let mut pattern_ref_count = 0;
        let mut path_trace_count = 0;
        for op in &ops_case1 {
            match op {
                GeometricOperation::PatternReference { base_coordinate, .. } => {
                    pattern_ref_count += 1;
                    // Check if base_coordinate is one of the expected pattern starts
                    assert!(*base_coordinate == coords[0] || *base_coordinate == coords[4]);
                }
                GeometricOperation::PathTrace { waypoints, .. } => {
                    path_trace_count +=1;
                    assert_eq!(waypoints.len(), 1);
                    assert_eq!(waypoints[0], coords[3]);
                }
                _ => panic!("Unexpected operation type in Case 1: {:?}", op),
            }
        }
        assert_eq!(pattern_ref_count, 2, "Expected two PatternReference operations for Case 1");
        assert_eq!(path_trace_count, 1, "Expected one PathTrace operation for separator for Case 1");


        // Case 2: Cluster processing (original test used clusters input, my logic doesn't use it directly anymore)
        // My logic will find lines/paths from the main `coords` list.
        // If I provide cluster_coords as the main `coords`:
        let cluster_coords_as_main = vec![Coordinate3D::new(10,0,0), Coordinate3D::new(11,0,0), Coordinate3D::new(10,1,0)];
        let result_cluster_as_main = optimizer.optimize_operations(&cluster_coords_as_main, &[], &[]);
        assert!(result_cluster_as_main.is_ok());
        let ops_cluster_as_main = result_cluster_as_main.unwrap();
        // Current line logic only finds X or Y lines, not mixed like this to form a RegionFill for the bounding box.
        // (10,0,0) -> (11,0,0) is a short X-line (len 2), becomes PathTrace.
        // (10,1,0) is a single point, becomes PathTrace.
        // So, expected: PathTrace for (10,0,0),(11,0,0), then PathTrace for (10,1,0)
        assert_eq!(ops_cluster_as_main.len(), 2, "Expected 2 PathTraces for the cluster_coords. Ops: {:?}", ops_cluster_as_main);
        match &ops_cluster_as_main[0] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                 assert_eq!(waypoints.len(), 2);
                 assert_eq!(waypoints[0], cluster_coords_as_main[0]);
                 assert_eq!(waypoints[1], cluster_coords_as_main[1]);
            }
            _ => panic!("Expected PathTrace for first part of cluster_coords, got {:?}", ops_cluster_as_main[0]),
        }
         match &ops_cluster_as_main[1] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                 assert_eq!(waypoints.len(), 1);
                 assert_eq!(waypoints[0], cluster_coords_as_main[2]);
            }
            _ => panic!("Expected PathTrace for second part of cluster_coords, got {:?}", ops_cluster_as_main[1]),
        }
    }
}
