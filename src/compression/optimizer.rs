use crate::core::coordinates::Coordinate3D;
use crate::compression::operations::{GeometricOperation, InterpolationType, EncodingScheme, Matrix3x3};
use crate::compression::pattern_analyzer::PatternCandidate;
use crate::compression::engine::CompressionError;
use std::collections::HashSet;


#[cfg(feature = "parallel")]
use crate::core::parallel_processor::{ParallelProcessor, ParallelConfig};
use crate::compression::pattern_analyzer::PatternAnalyzer;


#[derive(Debug, Clone)] // Removed Default as new() is now custom
pub struct OperationOptimizer {
    pattern_analyzer: PatternAnalyzer,
    #[cfg(feature = "parallel")]
    parallel_processor: Option<ParallelProcessor>,
    // Add any optimizer specific config if needed, e.g. min_pattern_len for this stage
    min_pattern_window: usize, 
    max_pattern_window: usize,
}

impl OperationOptimizer {
    pub fn new(
        min_pattern_window: usize,
        max_pattern_window: usize,
        #[cfg(feature = "parallel")] parallel_enabled: bool,
        #[cfg(feature = "parallel")] parallel_config: Option<ParallelConfig>
    ) -> Self {
        let analyzer = PatternAnalyzer::new();
        
        #[cfg(feature = "parallel")]
        let pp = if parallel_enabled && parallel_config.is_some() {
            Some(ParallelProcessor::new(parallel_config.unwrap()))
        } else {
            None
        };

        Self {
            pattern_analyzer: analyzer,
            #[cfg(feature = "parallel")]
            parallel_processor: pp,
            min_pattern_window,
            max_pattern_window,
        }
    }
    
    // Helper method to find patterns (conditionally parallel)
    fn find_patterns_for_optimizer(
        &self,
        coords: &[Coordinate3D]
    ) -> Result<Vec<PatternCandidate>, CompressionError> {
        #[cfg(feature = "parallel")]
        if let Some(pp) = &self.parallel_processor {
            // parallel_pattern_analysis returns Vec, not Result.
            // For consistency, if find_patterns can fail, parallel should too, or handle error internally.
            // Assuming parallel_pattern_analysis is changed to return Result or this is adapted.
            // Current parallel_pattern_analysis takes analyzer as arg.
            return Ok(pp.parallel_pattern_analysis(
                coords, 
                &self.pattern_analyzer, 
                self.min_pattern_window, 
                self.max_pattern_window
            )); 
        }
        // Fallback to sequential if "parallel" feature is not enabled or pp is None
        self.pattern_analyzer.find_patterns(coords, self.min_pattern_window, self.max_pattern_window)
    }


    /// Optimizes operations using an enhanced greedy approach.
    pub fn optimize_operations(
        &self,
        coords: &[Coordinate3D],
        clusters: &[Vec<Coordinate3D>],
    ) -> Result<Vec<GeometricOperation>, CompressionError> {
        let mut operations: Vec<GeometricOperation> = Vec::new();
        let mut covered_indices: HashSet<usize> = HashSet::new();

        // 1. Find and prioritize patterns FIRST
        let mut sorted_patterns = self.find_patterns_for_optimizer(coords)?;
        sorted_patterns.sort_by(|a, b| 
            b.compression_potential.partial_cmp(&a.compression_potential)
             .unwrap_or(std::cmp::Ordering::Equal)
        );

        // 2. Process all pattern occurrences
        for pattern_candidate in sorted_patterns.iter() {
            let pattern_len = pattern_candidate.coordinates.len();
            if pattern_len == 0 { continue; }

            let mut i = 0;
            while i <= coords.len().saturating_sub(pattern_len) {
                let mut is_match = true;
                let mut segment_covered = false;
                for k in 0..pattern_len {
                    if i + k >= coords.len() || coords[i + k] != pattern_candidate.coordinates[k] {
                        is_match = false;
                        break;
                    }
                    if covered_indices.contains(&(i + k)) {
                        segment_covered = true;
                        break;
                    }
                }
                if is_match && !segment_covered {
                    operations.push(GeometricOperation::PatternReference {
                        base_coordinate: coords[i],
                        pattern_id: 0, // In a real implementation, use a unique pattern ID
                        transformation_matrix: Matrix3x3::identity(),
                    });
                    for k in 0..pattern_len {
                        covered_indices.insert(i + k);
                    }
                    i += pattern_len;
                } else {
                    i += 1;
                }
            }
        }

        // 3. Process clusters if provided (for uncovered points)
        for cluster in clusters {
            if cluster.is_empty() {
                continue;
            }
            let all_uncovered = cluster.iter().all(|c| {
                coords.iter().position(|&x| x == *c)
                    .map(|idx| !covered_indices.contains(&idx))
                    .unwrap_or(false)
            });
            if all_uncovered {
                let mut uncovered_points = Vec::new();
                for coord in cluster {
                    if let Some(idx) = coords.iter().position(|&x| x == *coord) {
                        if !covered_indices.contains(&idx) {
                            uncovered_points.push(*coord);
                        }
                    }
                }
                if !uncovered_points.is_empty() {
                    operations.push(GeometricOperation::PathTrace {
                        waypoints: uncovered_points,
                        interpolation: InterpolationType::None,
                        data_encoding: EncodingScheme::Raw,
                    });
                }
                for coord in cluster {
                    if let Some(idx) = coords.iter().position(|&x| x == *coord) {
                        covered_indices.insert(idx);
                    }
                }
            }
        }

        // 4. Process remaining uncovered coordinates for lines and paths
        let mut i = 0;
        const MIN_LINE_LEN: usize = 3;
        while i < coords.len() {
            if covered_indices.contains(&i) {
                i += 1;
                continue;
            }
            let coord_start = coords[i];
            let mut operation_applied = false;
            // Check for Lines (RegionFill)
            if coords.len() - i >= MIN_LINE_LEN {
                let mut line_len = 1;
                for k in 1..(coords.len() - i) {
                    if covered_indices.contains(&(i + k)) { break; }
                    let expected = Coordinate3D::new(coords[i].x.wrapping_add(k as u8), coords[i].y, coords[i].z);
                    if coords[i + k] == expected {
                        line_len += 1;
                    } else {
                        break;
                    }
                }
                if line_len >= MIN_LINE_LEN {
                    let line_end_idx = i + line_len - 1;
                    operations.push(GeometricOperation::RegionFill {
                        start: coords[i],
                        end: coords[line_end_idx],
                        fill_byte: 0,
                        compression_ratio: line_len as f32,
                    });
                    for k in 0..line_len { covered_indices.insert(i + k); }
                    i += line_len;
                    operation_applied = true;
                }
            }
            if !operation_applied && coords.len() - i >= MIN_LINE_LEN {
                let mut line_len = 1;
                for k in 1..(coords.len() - i) {
                    if covered_indices.contains(&(i + k)) { break; }
                    let expected = Coordinate3D::new(coords[i].x, coords[i].y.wrapping_add(k as u8), coords[i].z);
                    if coords[i + k] == expected {
                        line_len += 1;
                    } else {
                        break;
                    }
                }
                if line_len >= MIN_LINE_LEN {
                    let line_end_idx = i + line_len - 1;
                    operations.push(GeometricOperation::RegionFill {
                        start: coords[i],
                        end: coords[line_end_idx],
                        fill_byte: 0,
                        compression_ratio: line_len as f32,
                    });
                    for k in 0..line_len { covered_indices.insert(i + k); }
                    i += line_len;
                    operation_applied = true;
                }
            }
            if !operation_applied && coords.len() - i >= MIN_LINE_LEN {
                let mut line_len = 1;
                for k in 1..(coords.len() - i) {
                    if covered_indices.contains(&(i + k)) { break; }
                    let expected = Coordinate3D::new(coords[i].x, coords[i].y, coords[i].z.wrapping_add(k as u8));
                    if coords[i + k] == expected {
                        line_len += 1;
                    } else {
                        break;
                    }
                }
                if line_len >= MIN_LINE_LEN {
                    let line_end_idx = i + line_len - 1;
                    operations.push(GeometricOperation::RegionFill {
                        start: coords[i],
                        end: coords[line_end_idx],
                        fill_byte: 0,
                        compression_ratio: line_len as f32,
                    });
                    for k in 0..line_len { covered_indices.insert(i + k); }
                    i += line_len;
                    operation_applied = true;
                }
            }
            if !operation_applied {
                let mut path_trace_waypoints = vec![];
                let mut current_idx = i;
                while current_idx < coords.len() && !covered_indices.contains(&current_idx) {
                    path_trace_waypoints.push(coords[current_idx]);
                    covered_indices.insert(current_idx);
                    current_idx += 1;
                }
                if !path_trace_waypoints.is_empty() {
                    operations.push(GeometricOperation::PathTrace {
                        waypoints: path_trace_waypoints,
                        interpolation: InterpolationType::None,
                        data_encoding: EncodingScheme::Raw,
                    });
                }
                i = current_idx;
            }
        }
        Ok(operations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coordinates::Coordinate3D;
    use crate::compression::operations::{GeometricOperation, InterpolationType, EncodingScheme};

    #[cfg(feature = "parallel")]
    use crate::core::parallel_processor::ParallelConfig;

    // Helper to create optimizer for tests, assuming non-parallel for most optimizer logic tests
    fn create_test_optimizer() -> OperationOptimizer {
        #[cfg(feature = "parallel")]
        return OperationOptimizer::new(4, 32, false, None);
        #[cfg(not(feature = "parallel"))]
        return OperationOptimizer::new(4, 32);
    }

    fn get_test_coords_for_patterns() -> Vec<Coordinate3D> {
        vec![
            Coordinate3D::new(0,0,0), Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), // Pattern P1
            Coordinate3D::new(10,0,0), // Separator S1
            Coordinate3D::new(5,5,5), Coordinate3D::new(5,6,5), Coordinate3D::new(5,7,5), // Y-Line L1
            Coordinate3D::new(20,0,0), Coordinate3D::new(21,0,0), // Short X-Line P2
            Coordinate3D::new(0,0,0), Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), // P1 again
            Coordinate3D::new(10,10,10), // Single point S2
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), // Another pattern P3 (different from P1)
        ]
    }

    #[test]
    fn test_optimize_operations_empty_input() {
        let optimizer = create_test_optimizer();
        let coords: Vec<Coordinate3D> = vec![];
        let result = optimizer.optimize_operations(&coords, &[]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_optimize_prioritizes_patterns() {
        let optimizer = OperationOptimizer::new(3, 3, #[cfg(feature = "parallel")] false, #[cfg(feature = "parallel")] None); // Configure to find P1
        let coords = vec![
            Coordinate3D::new(0,0,0), Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), // P1 (len 3, freq 2)
            Coordinate3D::new(10,0,0), // Separator
            Coordinate3D::new(5,5,5), Coordinate3D::new(5,6,5), // Short Y-Line (len 2)
            Coordinate3D::new(0,0,0), Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), // P1 again
        ];
        // In get_test_coords_for_patterns logic:
        // P1: [(0,0,0),(1,0,0),(2,0,0)], len 3, freq 2. Potential (2-1)*3 = 3

        // This test will rely on the internal find_patterns_for_optimizer finding these.
        // The optimizer itself sorts by compression_potential.
        // find_patterns calculates potential as ((freq-1)*len).
        // For P1 (len 3, freq 2): (2-1)*3 = 3.
        // If there was another pattern P_short (len 2, freq 3): (3-1)*2 = 4. P_short would be prioritized.
        // In this test, only P1 should be found by default window sizes (e.g. 2-4).

        let result = optimizer.optimize_operations(&coords, &[]);
        assert!(result.is_ok());
        let ops = result.unwrap();

        // Expected operations after optimization:
        // 1. PatternReference for the first P1 instance at index 0
        // 2. PatternReference for the second P1 instance at index 6
        // 3. PathTrace for the separator and short Y-Line at indices 3, 4, 5 (consecutive uncovered points)
        assert_eq!(ops.len(), 3, "Expected 3 operations, got {:?}", ops);

        // Assertions for the operations based on expected order:
        // Op 0: First PatternReference for P1
        match &ops[0] {
            GeometricOperation::PatternReference { base_coordinate, .. } => assert_eq!(*base_coordinate, Coordinate3D::new(0,0,0)),
            _ => panic!("Op 0: Expected PatternReference for P1, got {:?}", ops[0]),
        }

        // Op 1: Second PatternReference
        match &ops[1] {
            GeometricOperation::PatternReference { base_coordinate, .. } => assert_eq!(*base_coordinate, Coordinate3D::new(0,0,0)),
            _ => panic!("Op 1: Expected second PatternReference for P1, got {:?}", ops[1]),
        }

        // Op 2: PathTrace for Separator and Short Y-Line (consecutive uncovered points)
        match &ops[2] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                assert_eq!(waypoints.len(), 3);
                assert_eq!(waypoints[0], Coordinate3D::new(10,0,0)); // Separator
                assert_eq!(waypoints[1], Coordinate3D::new(5,5,5));  // First Y-line point
                assert_eq!(waypoints[2], Coordinate3D::new(5,6,5));  // Second Y-line point
            },
            _ => panic!("Op 2: Expected PathTrace for consecutive uncovered points, got {:?}", ops[2]),
        }
    }


    #[test]
    fn test_optimize_line_detection_x_axis() {
        let optimizer = create_test_optimizer();
        let coords = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), Coordinate3D::new(4,0,0), // X-Line
            Coordinate3D::new(10,0,0), // Separator
        ];
        let result = optimizer.optimize_operations(&coords, &[]);
        assert!(result.is_ok());
        let ops = result.unwrap();
        // Expected: RegionFill for X-Line, PathTrace for separator
        assert_eq!(ops.len(), 2, "Expected 2 operations, got {:?}", ops);
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
        let optimizer = create_test_optimizer();
        let coords = vec![
            Coordinate3D::new(5,5,5), Coordinate3D::new(5,6,5), Coordinate3D::new(5,7,5), // Y-Line
        ];
        let result = optimizer.optimize_operations(&coords, &[]);
        assert!(result.is_ok());
        let ops = result.unwrap();
        // Expected: RegionFill for Y-Line
        assert_eq!(ops.len(), 1, "Expected 1 operation, got {:?}", ops);
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
        let optimizer = create_test_optimizer();
        let coords = vec![
            Coordinate3D::new(20,0,0), Coordinate3D::new(21,0,0), // Short X-Line (len 2)
        ];
        let result = optimizer.optimize_operations(&coords, &[]);
        assert!(result.is_ok());
        let ops = result.unwrap();
        // Expected: PathTrace for the two points
        assert_eq!(ops.len(), 1, "Expected 1 operation, got {:?}", ops);
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
        let optimizer = create_test_optimizer();
        let coords = vec![
            Coordinate3D::new(1,1,1),
            Coordinate3D::new(3,3,3),
            Coordinate3D::new(5,5,5),
        ];
        let result = optimizer.optimize_operations(&coords, &[]);
        assert!(result.is_ok());
        let ops = result.unwrap();
        // Expected: A single PathTrace with all waypoints
        assert_eq!(ops.len(), 1, "Expected 1 PathTrace operation, got {:?}", ops);
        match &ops[0] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                assert_eq!(waypoints.len(), 3, "Expected 3 waypoints in the PathTrace");
                assert_eq!(waypoints, &coords, "Waypoints don't match input coordinates");
            }
            _ => panic!("Expected PathTrace operation, got {:?}", ops[0]),
        }
    }

    #[test]
    fn test_mixed_operations_pattern_lines_points() {
        // This test will use an optimizer configured to find the P1 pattern.
        // Other patterns/lines will be found by the subsequent logic.
        let optimizer = OperationOptimizer::new(3, 3, #[cfg(feature = "parallel")] false, #[cfg(feature = "parallel")] None); // Configure to find P1

        let coords = vec![
            Coordinate3D::new(0,0,0), Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), // P1
            Coordinate3D::new(10,0,0),                                                    // S1
            Coordinate3D::new(5,5,5), Coordinate3D::new(5,6,5), Coordinate3D::new(5,7,5), // L1 (Y-axis)
            Coordinate3D::new(20,0,0), Coordinate3D::new(21,0,0),                         // P2 (short path)
            Coordinate3D::new(0,0,0), Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), // P1 again
        ];

        let result = optimizer.optimize_operations(&coords, &[]);
        assert!(result.is_ok());
        let ops = result.unwrap();

        // Expected order based on the revised logic:
        // Patterns are found first, in order of appearance in the original list.
        // 1. PatternReference for P1 (indices 0,1,2)
        // 2. PatternReference for P1 (indices 9,10,11)
        // Then the remaining uncovered points are processed sequentially:
        // 3. PathTrace for all remaining uncovered points (indices 3,4,5,6,7,8) - consecutive uncovered points
        assert_eq!(ops.len(), 3, "Expected 3 operations, got {:?}", ops);

        // Assertions for the operations based on expected order:
        // Op 0: First PatternReference
        match &ops[0] {
            GeometricOperation::PatternReference { base_coordinate, .. } => assert_eq!(*base_coordinate, Coordinate3D::new(0,0,0)),
            _ => panic!("Op 0: Expected PatternReference for P1, got {:?}", ops[0]),
        }

        // Op 1: Second PatternReference
        match &ops[1] {
            GeometricOperation::PatternReference { base_coordinate, .. } => assert_eq!(*base_coordinate, Coordinate3D::new(0,0,0)),
            _ => panic!("Op 1: Expected second PatternReference for P1, got {:?}", ops[1]),
        }

        // Op 2: PathTrace for all remaining uncovered points
        match &ops[2] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                assert_eq!(waypoints.len(), 6);
                assert_eq!(waypoints[0], Coordinate3D::new(10,0,0)); // S1
                assert_eq!(waypoints[1], Coordinate3D::new(5,5,5));  // L1 start
                assert_eq!(waypoints[2], Coordinate3D::new(5,6,5));  // L1 middle
                assert_eq!(waypoints[3], Coordinate3D::new(5,7,5));  // L1 end
                assert_eq!(waypoints[4], Coordinate3D::new(20,0,0)); // P2 start
                assert_eq!(waypoints[5], Coordinate3D::new(21,0,0)); // P2 end
            },
            _ => panic!("Op 2: Expected PathTrace for all remaining uncovered points, got {:?}", ops[2]),
        }
    }

    // Original test from the prompt, slightly adapted
    #[test]
    fn test_optimize_operations_original_cases() {
        // Case 1: Two occurrences of a pattern
        let optimizer_case1 = OperationOptimizer::new(3, 3, #[cfg(feature = "parallel")] false, #[cfg(feature = "parallel")] None); // Configure to find P1
        
        let coords_case1 = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), // P1
            Coordinate3D::new(10,0,0), // Separator
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), // P1 again
        ];

        let result_case1 = optimizer_case1.optimize_operations(&coords_case1, &[]);
        assert!(result_case1.is_ok());
        let ops_case1 = result_case1.unwrap();
        
        // Expected operations based on the revised logic:
        // 1. PatternReference for the first P1 instance at index 0
        // 2. PatternReference for the second P1 instance at index 4
        // 3. PathTrace for the separator at index 3
        assert_eq!(ops_case1.len(), 3, "Case 1: Expected 3 operations. Ops: {:?}", ops_case1);

        // Assertions for Case 1 operations based on expected order:
        // Op 0: First PatternReference
        match &ops_case1[0] {
            GeometricOperation::PatternReference { base_coordinate, .. } => assert_eq!(*base_coordinate, Coordinate3D::new(1,0,0)),
            _ => panic!("Case 1 Op 0: Expected PatternReference, got {:?}", ops_case1[0]),
        }

        // Op 1: Second PatternReference
         match &ops_case1[1] {
            GeometricOperation::PatternReference { base_coordinate, .. } => assert_eq!(*base_coordinate, Coordinate3D::new(1,0,0)),
            _ => panic!("Case 1 Op 1: Expected second PatternReference, got {:?}", ops_case1[1]),
        }

         // Op 2: PathTrace for the separator (10,0,0)
         match &ops_case1[2] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                 assert_eq!(waypoints.len(), 1);
                 assert_eq!(waypoints[0], Coordinate3D::new(10,0,0));
            }
            _ => panic!("Case 1 Op 2: Expected PathTrace, got {:?}", ops_case1[2]),
        }

        // Case 2: Cluster-like data (no actual clusters passed, just coords)
        let optimizer_case2 = OperationOptimizer::new(2, 2, #[cfg(feature = "parallel")] false, #[cfg(feature = "parallel")] None); // Configure to find patterns of len 2

        let cluster_coords_as_main = vec![Coordinate3D::new(10,0,0), Coordinate3D::new(11,0,0), Coordinate3D::new(10,1,0)];
        // No patterns of length 2 are present in cluster_coords_as_main.
        // Expected operations:
        // 1. PathTrace for all three points (consecutive uncovered points)

        let result_cluster_as_main = optimizer_case2.optimize_operations(&cluster_coords_as_main, &[]);
        assert!(result_cluster_as_main.is_ok());
        let ops_cluster_as_main = result_cluster_as_main.unwrap();
        
        assert_eq!(ops_cluster_as_main.len(), 1, "Case 2: Expected 1 PathTrace for the cluster_coords. Ops: {:?}", ops_cluster_as_main);
        
        match &ops_cluster_as_main[0] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                 assert_eq!(waypoints.len(), 3);
                 assert_eq!(waypoints[0], cluster_coords_as_main[0]);
                 assert_eq!(waypoints[1], cluster_coords_as_main[1]);
                 assert_eq!(waypoints[2], cluster_coords_as_main[2]);
            }
            _ => panic!("Case 2 Op 0: Expected PathTrace, got {:?}", ops_cluster_as_main[0]),
        }
    }
}
