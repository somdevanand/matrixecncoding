// geometric_crypto/src/core/parallel_processor.rs

#![cfg(feature = "parallel")] // Only compile this module if 'parallel' feature is enabled

use crate::core::coordinates::Coordinate3D;
use crate::core::matrix::GeometricMatrix;
use crate::compression::pattern_analyzer::{PatternAnalyzer, PatternCandidate};
use rayon::prelude::*; // For parallel iterators

#[derive(Debug, Clone)] // Added Clone
pub struct ParallelProcessor {
    chunk_size: usize,
    // No internal thread pool; Rayon's global pool is typically used by default.
    // If a custom pool is needed, it would be configured via rayon::ThreadPoolBuilder.
}

impl ParallelProcessor {
    pub fn new(chunk_size: usize) -> Self {
        if chunk_size == 0 {
            panic!("Chunk size for ParallelProcessor cannot be zero."); // Or return Result
        }
        Self { chunk_size }
    }

    /// Parallel coordinate generation.
    pub fn parallel_coordinate_generation(
        &self,
        data: &[u8],
        matrix: &GeometricMatrix, // Assumes GeometricMatrix is Sync
    ) -> Vec<Coordinate3D> {
        if data.is_empty() {
            return Vec::new();
        }
        
        // Ensure matrix has its mapping generated before parallel processing.
        // This should be handled by the caller or matrix.new().
        if matrix.byte_to_coord_map.is_empty() {
             // Or return Err, or handle as per specific project error strategy.
             // For now, panic, as this is a precondition.
             panic!("GeometricMatrix mapping not generated before parallel coordinate generation.");
        }


        data.par_chunks(self.chunk_size)
            .enumerate()
            .flat_map(|(chunk_idx, chunk)| {
                chunk.par_iter() // Parallel iteration within the chunk
                    .enumerate()
                    .map(|(byte_idx_in_chunk, &byte_val)| {
                        let global_position = chunk_idx * self.chunk_size + byte_idx_in_chunk;
                        matrix.coordinate_for_position(byte_val, global_position)
                    })
                    .collect::<Vec<Coordinate3D>>()
            })
            .collect()
    }

    /// Parallel pattern analysis (example: parallelizing window iteration).
    /// Note: PatternAnalyzer's find_patterns itself isn't inherently parallel from this call.
    /// This function demonstrates how one might parallelize part of the analysis if
    /// the underlying `analyzer.find_patterns` (or its logic) could be adapted.
    /// The current `analyzer.find_patterns` takes `&self`, so if `PatternAnalyzer` is `Sync`,
    /// it can be called from parallel threads.
    /// This stub will parallelize the iteration over different window sizes.
    pub fn parallel_pattern_analysis(
        &self,
        coords: &[Coordinate3D],
        analyzer: &PatternAnalyzer, // Assumes PatternAnalyzer is Sync
        min_w: usize,
        max_w: usize,
    ) -> Vec<PatternCandidate> {
         if coords.is_empty() || min_w == 0 || min_w > max_w {
             return Vec::new();
         }

        (min_w..=max_w)
            .into_par_iter() // Parallel iteration over window sizes
            .flat_map(|window_size| {
                // Call the existing synchronous find_patterns for each window size.
                // This specific call isn't making find_patterns internally parallel,
                // but rather, calls to find_patterns for different window_sizes run in parallel.
                // This is a simplification. A deeper parallelization would modify find_patterns itself.
                match analyzer.find_patterns(coords, window_size, window_size) {
                    Ok(candidates) => candidates,
                    Err(_) => Vec::new(), // Handle error from find_patterns, e.g. log or collect errors
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::matrix::GeometricMatrix;
    use crate::compression::pattern_analyzer::PatternAnalyzer;
    use crate::core::coordinates::Coordinate3D; // For direct use in tests

    #[test]
    #[should_panic]
    fn test_parallel_processor_new_zero_chunk_size() {
        ParallelProcessor::new(0);
    }

    #[test]
    fn test_parallel_coordinate_generation_empty_input() {
        let processor = ParallelProcessor::new(128);
        let data: Vec<u8> = Vec::new();
        let mut matrix = GeometricMatrix::new([0u8; 32]);
        matrix.generate_bijective_mapping().unwrap(); // Ensure mapping

        let result = processor.parallel_coordinate_generation(&data, &matrix);
        assert!(result.is_empty());
    }
    
    #[test]
    #[should_panic]
    fn test_parallel_coordinate_generation_mapping_not_generated() {
        let processor = ParallelProcessor::new(128);
        let data: Vec<u8> = vec![1,2,3];
        let matrix = GeometricMatrix::new([0u8; 32]); // Mapping not generated
        processor.parallel_coordinate_generation(&data, &matrix);
    }


    #[test]
    fn test_parallel_coordinate_generation_output_matches_sequential() {
        let processor = ParallelProcessor::new(2); // Small chunk size for testing par_chunks
        let data: Vec<u8> = (0..10u8).collect(); // Small dataset
        let mut matrix = GeometricMatrix::new([42u8; 32]);
        matrix.generate_bijective_mapping().unwrap();

        let parallel_result = processor.parallel_coordinate_generation(&data, &matrix);

        let sequential_result: Vec<Coordinate3D> = data.iter().enumerate().map(|(idx, &byte_val)| {
            matrix.coordinate_for_position(byte_val, idx)
        }).collect();
        
        assert_eq!(parallel_result.len(), sequential_result.len(), "Results length mismatch");
        // Order might differ if inter-chunk parallelism isn't order-preserving for flat_map's final collect,
        // but par_chunks -> enumerate -> flat_map -> collect should preserve order of chunks.
        // And par_iter within chunk + collect also preserves order relative to that chunk.
        assert_eq!(parallel_result, sequential_result, "Parallel and sequential results differ");
    }

    #[test]
    fn test_parallel_pattern_analysis_basic_check() {
        let processor = ParallelProcessor::new(128);
        let analyzer = PatternAnalyzer::new(); // Assuming this is Sync
        let coords = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0),
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0),
        ];
         // min_w=2, max_w=2 to focus on one window size for simplicity of test outcome
        let result = processor.parallel_pattern_analysis(&coords, &analyzer, 2, 2); 
        
        // Expect one pattern candidate: [ (1,0,0), (2,0,0) ] with freq 2
        assert_eq!(result.len(), 1);
        if !result.is_empty() {
            assert_eq!(result[0].frequency, 2);
            assert_eq!(result[0].coordinates, vec![Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0)]);
        }
    }
    
     #[test]
     fn test_parallel_pattern_analysis_empty_coords() {
         let processor = ParallelProcessor::new(128);
         let analyzer = PatternAnalyzer::new();
         let empty_coords: Vec<Coordinate3D> = vec![];
         let result = processor.parallel_pattern_analysis(&empty_coords, &analyzer, 2, 4);
         assert!(result.is_empty());
     }
}
