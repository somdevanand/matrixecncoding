use crate::core::coordinates::Coordinate3D;
use crate::compression::operations::{GeometricOperation, Matrix3x3}; // Assuming fill_byte change is in operations.rs
use crate::compression::pattern_analyzer::PatternCandidate;
use crate::compression::engine::CompressionError;
use std::collections::HashSet; 
use crate::compression::operations::{InterpolationType, EncodingScheme};

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
        coords: &[Coordinate3D], // Original coordinates
        // patterns: &[PatternCandidate], // Patterns will be found internally now
        clusters: &[Vec<Coordinate3D>], // Clusters are still passed in
    ) -> Result<Vec<GeometricOperation>, CompressionError> {
        let mut operations: Vec<GeometricOperation> = Vec::new();
        let mut covered_indices: HashSet<usize> = HashSet::new(); 

        // Find and prioritize patterns candidates once
        let mut sorted_patterns = self.find_patterns_for_optimizer(coords)?;
        sorted_patterns.sort_by(|a, b| b.compression_potential.partial_cmp(&a.compression_potential).unwrap_or(std::cmp::Ordering::Equal));

        // 1. Process all pattern occurrences
        for pattern_candidate in sorted_patterns.iter() {
            let pattern_len = pattern_candidate.coordinates.len();
            if pattern_len == 0 { continue; }

            // Find all occurrences of this pattern
            for i in 0..=coords.len().saturating_sub(pattern_len) {
                // Check if the segment from i matches the pattern and is not covered
                let mut is_match = true;
                let mut segment_covered = false;
                for k in 0..pattern_len {
                    if coords[i + k] != pattern_candidate.coordinates[k] {
                        is_match = false;
                        break;
                    }
                    if covered_indices.contains(&(i + k)) {
                        segment_covered = true;
                        break;
                    }
                }

                if is_match && !segment_covered {
                    // Apply PatternReference for this occurrence
                     operations.push(GeometricOperation::PatternReference {
                        base_coordinate: coords[i], 
                        pattern_id: 0, // Placeholder ID - ideally unique per pattern type
                        transformation_matrix: Matrix3x3::identity(), // Assuming no transformations yet
                    });
                    // Mark indices covered by this occurrence
                    for k in 0..pattern_len {
                        covered_indices.insert(i + k);
                    }
                }
            }
        }

        // 2. Process remaining uncovered coordinates for lines and paths
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
                // Check X-axis line
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
                         fill_byte: 0, // Placeholder
                         compression_ratio: line_len as f32, // Simple ratio
                     });
                      for k in 0..line_len { covered_indices.insert(i + k); }
                      i += line_len;
                      operation_applied = true;
                 }
            }
            
            if !operation_applied && coords.len() - i >= MIN_LINE_LEN {
                 let mut line_len = 1;
                 // Check Y-axis line
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
                          fill_byte: 0, // Placeholder
                          compression_ratio: line_len as f32, // Simple ratio
                      });
                       for k in 0..line_len { covered_indices.insert(i + k); }
                       i += line_len;
                       operation_applied = true;
                 }
            }

            if !operation_applied && coords.len() - i >= MIN_LINE_LEN {
                 let mut line_len = 1;
                 // Check Z-axis line
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
                          fill_byte: 0, // Placeholder
                          compression_ratio: line_len as f32, // Simple ratio
                      });
                       for k in 0..line_len { covered_indices.insert(i + k); }
                       i += line_len;
                       operation_applied = true;
                 }
            }

            // 3. If not covered by pattern or line, process remaining uncovered points
            if !operation_applied {
                 let mut path_trace_waypoints = vec![];
                 let mut current_idx = i;
                 // Collect consecutive uncovered points into a PathTrace
                 while current_idx < coords.len() && !covered_indices.contains(&current_idx) {
                     path_trace_waypoints.push(coords[current_idx]);
                     // Mark as covered as they are added to the potential PathTrace
                     covered_indices.insert(current_idx);
                     current_idx += 1;
                 }

                 // If any uncovered points were collected, create a PathTrace.
                 // This loop correctly handles both single points (waypoint list of size 1)
                 // and consecutive sequences.
                 if !path_trace_waypoints.is_empty() {
                     operations.push(GeometricOperation::PathTrace {
                         waypoints: path_trace_waypoints,
                         interpolation: InterpolationType::None,
                         data_encoding: EncodingScheme::Raw,
                     });
                 }
                 // The main while loop will continue from the next uncovered index, or the end if all were covered.
                 // current_idx is already at the start of the next potential segment (either covered or end).
                 i = current_idx;
            }
        }

        // Optional: Sort operations for determinism or other criteria if needed
        // operations.sort_by(...);

        Ok(operations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coordinates::Coordinate3D;
    use crate::compression::pattern_analyzer::PatternCandidate;
    use crate::compression::operations::{GeometricOperation, InterpolationType, EncodingScheme, Matrix3x3, CoordinateDelta, CompressedValues, MathFunction, CoordinateRegion, TrigFuncType};
    use crate::compression::engine::CompressionError;

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
        // 3. PathTrace for the separator at index 3
        // 4. PathTrace for the short Y-Line at indices 4, 5
        assert_eq!(ops.len(), 4, "Expected 4 operations, got {:?}", ops);

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

        // Op 2: PathTrace for Separator
        match &ops[2] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                assert_eq!(waypoints.len(), 1);
                assert_eq!(waypoints[0], Coordinate3D::new(10,0,0));
            },
            _ => panic!("Op 2: Expected PathTrace for Separator, got {:?}", ops[2]),
        }

        // Op 3: PathTrace for Short Y-Line
        match &ops[3] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                assert_eq!(waypoints.len(), 2);
                assert_eq!(waypoints[0], Coordinate3D::new(5,5,5));
                assert_eq!(waypoints[1], Coordinate3D::new(5,6,5));
            },
            _ => panic!("Op 3: Expected PathTrace for Short Y-Line, got {:?}", ops[3]),
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
        // Expected: Three PathTraces, each with one waypoint
        assert_eq!(ops.len(), 3, "Expected 3 operations, got {:?}", ops);
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
        // 3. PathTrace for S1 (index 3)
        // 4. RegionFill for L1 (indices 4,5,6)
        // 5. PathTrace for P2 (indices 7,8)
        assert_eq!(ops.len(), 5, "Expected 5 operations, got {:?}", ops);

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

        // Op 2: PathTrace for S1
        match &ops[2] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                assert_eq!(waypoints.len(), 1);
                assert_eq!(waypoints[0], Coordinate3D::new(10,0,0));
            },
            _ => panic!("Op 2: Expected PathTrace for S1, got {:?}", ops[2]),
        }

        // Op 3: RegionFill for L1
        match &ops[3] {
            GeometricOperation::RegionFill { start, end, .. } => {
                assert_eq!(*start, Coordinate3D::new(5,5,5));
                assert_eq!(*end, Coordinate3D::new(5,7,5));
            },
            _ => panic!("Op 3: Expected RegionFill for L1, got {:?}", ops[3]),
        }

        // Op 4: PathTrace for P2
        match &ops[4] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                assert_eq!(waypoints.len(), 2);
                assert_eq!(waypoints[0], Coordinate3D::new(20,0,0));
                assert_eq!(waypoints[1], Coordinate3D::new(21,0,0));
            },
            _ => panic!("Op 4: Expected PathTrace for P2, got {:?}", ops[4]),
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
        // 1. PathTrace for (10,0,0), (11,0,0)
        // 2. PathTrace for (10,1,0)

        let result_cluster_as_main = optimizer_case2.optimize_operations(&cluster_coords_as_main, &[]);
        assert!(result_cluster_as_main.is_ok());
        let ops_cluster_as_main = result_cluster_as_main.unwrap();
        
        assert_eq!(ops_cluster_as_main.len(), 2, "Case 2: Expected 2 PathTraces for the cluster_coords. Ops: {:?}", ops_cluster_as_main);
        
        match &ops_cluster_as_main[0] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                 assert_eq!(waypoints.len(), 2);
                 assert_eq!(waypoints[0], cluster_coords_as_main[0]);
                 assert_eq!(waypoints[1], cluster_coords_as_main[1]);
            }
            _ => panic!("Case 2 Op 0: Expected PathTrace, got {:?}", ops_cluster_as_main[0]),
        }

         match &ops_cluster_as_main[1] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                 assert_eq!(waypoints.len(), 1);
                 assert_eq!(waypoints[0], cluster_coords_as_main[2]);
            }
            _ => panic!("Case 2 Op 1: Expected PathTrace, got {:?}", ops_cluster_as_main[1]),
         }
    }
}
