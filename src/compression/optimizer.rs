use crate::core::coordinates::Coordinate3D;
use crate::compression::operations::{GeometricOperation, Matrix3x3}; // Assuming fill_byte change is in operations.rs
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
        coords: &[Coordinate3D], // Original coordinates
        // patterns: &[PatternCandidate], // Patterns will be found internally now
        clusters: &[Vec<Coordinate3D>], // Clusters are still passed in
    ) -> Result<Vec<GeometricOperation>, CompressionError> {
        let mut operations: Vec<GeometricOperation> = Vec::new();
        let mut covered_indices: HashSet<usize> = HashSet::new(); 

        // Find and prioritize patterns candidates once
        let mut sorted_patterns = self.find_patterns_for_optimizer(coords)?;
        sorted_patterns.sort_by(|a, b| b.compression_potential.partial_cmp(&a.compression_potential).unwrap_or(std::cmp::Ordering::Equal));

        // Iterate through coordinates, applying the first matching and most beneficial operation (Pattern, then Line, then Path)
        let mut i = 0;
        while i < coords.len() {
            if covered_indices.contains(&i) {
                i += 1;
                continue;
            }

            let coord_start = coords[i];

            let mut operation_applied = false;

            // 1. Check for Pattern occurrences starting at the current index 'i'
            for pattern_candidate in sorted_patterns.iter() {
                let pattern_len = pattern_candidate.coordinates.len();
                 if pattern_len == 0 || i + pattern_len > coords.len() { continue; }

                 // Check if the segment from i matches the pattern and is not covered
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
                     // Apply PatternReference
                      operations.push(GeometricOperation::PatternReference {
                         base_coordinate: coords[i], 
                         pattern_id: 0, // Placeholder ID
                         transformation_matrix: Matrix3x3::identity(),
                     });
                     // Mark indices covered by this occurrence
                     for k in 0..pattern_len {
                         covered_indices.insert(i + k);
                     }
                     i += pattern_len; // Move index past the applied pattern
                     operation_applied = true; // A pattern was applied
                     break; // Prioritize the first found pattern at this index
                 }
            }

            if operation_applied { continue; } // If a pattern was applied, move to the next iteration

            // 2. If no pattern applied, check if the current segment is a line (RegionFill) from the current index 'i'
            const MIN_LINE_LEN: usize = 3;

            // Check for X-axis line starting at i
            if coords.len() - i >= MIN_LINE_LEN {
                let mut is_x_line = true;
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
                if is_x_line && line_len >= MIN_LINE_LEN {
                    let line_end_idx = i + line_len - 1;
                     operations.push(GeometricOperation::RegionFill {
                         start: coords[i],
                         end: coords[line_end_idx],
                         fill_byte: 0, // Placeholder
                         compression_ratio: line_len as f32,
                     });
                      for k in 0..line_len { covered_indices.insert(i + k); }
                      i += line_len; // Move index past the applied line
                      operation_applied = true; // A region fill was applied
                 }
             }

             if !operation_applied && coords.len() - i >= MIN_LINE_LEN {
                  // Check for Y-axis line starting at i
                 let mut is_y_line = true;
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
                  if is_y_line && line_len >= MIN_LINE_LEN {
                       let line_end_idx = i + line_len - 1;
                      operations.push(GeometricOperation::RegionFill {
                          start: coords[i],
                          end: coords[line_end_idx],
                          fill_byte: 0, // Placeholder
                          compression_ratio: line_len as f32,
                      });
                       for k in 0..line_len { covered_indices.insert(i + k); }
                       i += line_len;
                       operation_applied = true;
                  }
             }

            if !operation_applied && coords.len() - i >= MIN_LINE_LEN {
                // Check for Z-axis line starting at i
                 let mut is_z_line = true;
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
                  if is_z_line && line_len >= MIN_LINE_LEN {
                       let line_end_idx = i + line_len - 1;
                      operations.push(GeometricOperation::RegionFill {
                          start: coords[i],
                          end: coords[line_end_idx],
                          fill_byte: 0, // Placeholder
                          compression_ratio: line_len as f32,
                      });
                       for k in 0..line_len { covered_indices.insert(i + k); }
                       i += line_len;
                       operation_applied = true;
                  }
            }

            // 3. If no pattern or line applied, create a PathTrace for the current coordinate and subsequent uncovered points
            if !operation_applied {
                 let mut path_trace_waypoints = vec![coord_start];
                 covered_indices.insert(i);
                 let mut current_idx = i + 1;
                 while current_idx < coords.len() && !covered_indices.contains(&current_idx) {
                     path_trace_waypoints.push(coords[current_idx]);
                     covered_indices.insert(current_idx);
                     current_idx += 1;
                 }
                 operations.push(GeometricOperation::PathTrace {
                     waypoints: path_trace_waypoints,
                     interpolation: crate::compression::operations::InterpolationType::None, // Assuming no interpolation for raw points
                     data_encoding: crate::compression::operations::EncodingScheme::Raw, // Assuming raw data
                 });
                 i = current_idx; // Move index to the start of the next uncovered segment
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

    #[cfg(feature = "parallel")]
    use crate::core::parallel_processor::ParallelConfig;

    // Helper to create optimizer for tests, assuming non-parallel for most optimizer logic tests
    fn create_test_optimizer() -> OperationOptimizer {
        #[cfg(feature = "parallel")]
        return OperationOptimizer::new(4, 32, false, None);
        #[cfg(not(feature = "parallel"))]
        return OperationOptimizer::new(4, 32);
    }

    #[test]
    fn test_optimize_operations_empty_input() {
        let optimizer = create_test_optimizer();
        let result = optimizer.optimize_operations(&[], &[]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_optimize_prioritizes_patterns() {
        let optimizer = create_test_optimizer();
        let coords = get_test_coords();

        // Patterns would be found by find_patterns_for_optimizer. 
        // To test prioritization, we'd mock or ensure specific patterns are found.
        // The current test structure for OperationOptimizer assumes patterns are passed in.
        // Since find_patterns_for_optimizer is now internal, this test needs to be re-thought
        // or we test find_patterns_for_optimizer separately if we want to assert on PatternCandidate creation.
        // For now, let's test with coords that *should* produce known patterns via internal call.
        
        // Coords for this test:
        // P1: (1,0,0),(2,0,0),(3,0,0) - occurs twice
        // Separator: (10,0,0)
        // Y-Line L1: (5,5,5),(5,6,5),(5,7,5)
        // Single S1: (10,10,10)
        // Short X-Line: (20,0,0),(21,0,0)
        
        // Let's use a simpler coord set for direct pattern testing if find_patterns_for_optimizer is called.
        let test_coords_for_patterns = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), // P1
            Coordinate3D::new(10,0,0), // Separator
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), // P1 again
        ];

        // This test will rely on the internal find_patterns_for_optimizer finding these.
        // The optimizer itself sorts by compression_potential.
        // find_patterns calculates potential as ((freq-1)*len).
        // For P1 (len 3, freq 2): (2-1)*3 = 3.
        // If there was another pattern P_short (len 2, freq 3): (3-1)*2 = 4. P_short would be prioritized.
        // In test_coords_for_patterns, only P1 (len 3, freq 2) should be found by default window sizes (e.g. 2-4).
        
        let result = optimizer.optimize_operations(&test_coords_for_patterns, &[]);
        assert!(result.is_ok());
        let ops = result.unwrap();
        
        // Expect P1 (0,1,2), Path(10,0,0), Path(1,0,0), Path(2,0,0), Path(3,0,0) ... this is not ideal.
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
        let optimizer = create_test_optimizer();
        let coords = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), Coordinate3D::new(4,0,0), // X-Line
            Coordinate3D::new(10,0,0), // Separator
        ];
        let result = optimizer.optimize_operations(&coords, &[]);
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
        let optimizer = create_test_optimizer();
        let coords = vec![
            Coordinate3D::new(5,5,5), Coordinate3D::new(5,6,5), Coordinate3D::new(5,7,5), // Y-Line
        ];
        let result = optimizer.optimize_operations(&coords, &[]);
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
        let optimizer = create_test_optimizer();
        let coords = vec![
            Coordinate3D::new(20,0,0), Coordinate3D::new(21,0,0), // Short X-Line (len 2)
        ];
        let result = optimizer.optimize_operations(&coords, &[]);
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
        let optimizer = create_test_optimizer();
        let coords = vec![
            Coordinate3D::new(1,1,1),
            Coordinate3D::new(3,3,3),
            Coordinate3D::new(5,5,5),
        ];
        let result = optimizer.optimize_operations(&coords, &[]);
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
        // This test will use an optimizer configured to find the P1 pattern.
        // Other patterns/lines will be found by the subsequent logic.
        #[cfg(feature = "parallel")]
        let optimizer = OperationOptimizer::new(3,3,false, None); // min/max window = 3 to find P1
        #[cfg(not(feature = "parallel"))]
        let optimizer = OperationOptimizer::new(3,3);


        let coords = vec![
            Coordinate3D::new(0,0,0), Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), // P1
            Coordinate3D::new(10,0,0),                                                    // S1
            Coordinate3D::new(5,5,5), Coordinate3D::new(5,6,5), Coordinate3D::new(5,7,5), // L1 (Y-axis)
            Coordinate3D::new(20,0,0), Coordinate3D::new(21,0,0),                         // P2 (short path)
            Coordinate3D::new(0,0,0), Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), // P1 again
        ];
        // Pattern finding for P1 (len 3) should be done internally.
        let result = optimizer.optimize_operations(&coords, &[]);
        assert!(result.is_ok());
        let ops = result.unwrap();

        // Expected order:
        // 1. PatternReference for P1 (indices 0,1,2)
        // 2. PatternReference for P1 (indices 9,10,11)
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
            _ => panic!("Op 1: Expected PatternReference for P1, got {:?}", ops[1]),
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
        #[cfg(feature = "parallel")]
        let optimizer_case1 = OperationOptimizer::new(3, 3, false, None); // min/max window = 3 to find P1
        #[cfg(not(feature = "parallel"))]
        let optimizer_case1 = OperationOptimizer::new(3, 3);
        
        let coords_case1 = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), // P1
            Coordinate3D::new(10,0,0), // Separator
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), // P1 again
        ];
        // Patterns are now found internally. The internal find_patterns_for_optimizer
        // should find two instances of P1 if it correctly identifies non-overlapping occurrences,
        // or if the current PatternCandidate logic (which only has first_occurrence_index) is adapted.
        // The current pattern logic in optimize_operations iterates sorted_patterns and applies one instance.
        // If find_patterns_for_optimizer returns multiple PatternCandidate structs for the same pattern
        // at different locations, they'd be sorted by potential.
        // Let's assume find_patterns_for_optimizer (as it calls analyzer.find_patterns)
        // will return one PatternCandidate for "[(1,0,0),(2,0,0),(3,0,0)]" with freq=2, first_idx=0.
        // The optimizer will apply this once. The second occurrence will be handled by subsequent logic.

        let result_case1 = optimizer_case1.optimize_operations(&coords_case1, &[]);
        assert!(result_case1.is_ok());
        let ops_case1 = result_case1.unwrap();
        
        // Updated assertion: Expect 6 operations based on new logic
        assert_eq!(ops_case1.len(), 6, "Case 1: Expected 6 operations. Ops: {:?}", ops_case1);

        // Assertions for Case 1 operations:
        // Op 0: First PatternReference
        match &ops_case1[0] {
            GeometricOperation::PatternReference { base_coordinate, .. } => assert_eq!(*base_coordinate, Coordinate3D::new(1,0,0)), // Base coord of the pattern
            _ => panic!("Case 1 Op 0: Expected PatternReference, got {:?}", ops_case1[0]),
        }
         // Op 1: Second PatternReference
         match &ops_case1[1] {
            GeometricOperation::PatternReference { base_coordinate, .. } => assert_eq!(*base_coordinate, Coordinate3D::new(1,0,0)), // Base coord of the pattern
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

        // Op 3: RegionFill for the Y-axis line (5,5,5) to (5,7,5)
        match &ops_case1[3] {
            GeometricOperation::RegionFill { start, end, .. } => {
                assert_eq!(*start, Coordinate3D::new(5,5,5));
                assert_eq!(*end, Coordinate3D::new(5,7,5));
            }
             _ => panic!("Case 1 Op 3: Expected RegionFill, got {:?}", ops_case1[3]),
        }

        // Op 4: PathTrace for the single point (10,10,10)
        match &ops_case1[4] {
            GeometricOperation::PathTrace { waypoints, .. } => {
                assert_eq!(waypoints.len(), 1);
                assert_eq!(waypoints[0], Coordinate3D::new(10,10,10));
            }
             _ => panic!("Case 1 Op 4: Expected PathTrace, got {:?}", ops_case1[4]),
        }

         // Op 5: PathTrace for the short X-line (20,0,0) to (21,0,0)
         match &ops_case1[5] {
             GeometricOperation::PathTrace { waypoints, .. } => {
                 assert_eq!(waypoints.len(), 2);
                 assert_eq!(waypoints[0], Coordinate3D::new(20,0,0));
                 assert_eq!(waypoints[1], Coordinate3D::new(21,0,0));
             }
             _ => panic!("Case 1 Op 5: Expected PathTrace, got {:?}", ops_case1[5]),
         }


        // Case 2: Cluster-like data (no actual clusters passed, just coords)
        #[cfg(feature = "parallel")]
        let optimizer_case2 = OperationOptimizer::new(2, 2, false, None);
        #[cfg(not(feature = "parallel"))]
        let optimizer_case2 = OperationOptimizer::new(2, 2);

        let cluster_coords_as_main = vec![Coordinate3D::new(10,0,0), Coordinate3D::new(11,0,0), Coordinate3D::new(10,1,0)];
        let result_cluster_as_main = optimizer_case2.optimize_operations(&cluster_coords_as_main, &[]);
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
