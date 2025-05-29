// geometric_crypto/src/core/parallel_processor.rs

#![cfg(feature = "parallel")] // Only compile this module if 'parallel' feature is enabled

use crate::core::coordinates::Coordinate3D;
use crate::core::matrix::GeometricMatrix;
use crate::compression::pattern_analyzer::{PatternAnalyzer, PatternCandidate};
use rayon::prelude::*; // For parallel iterators

#[derive(Debug, Clone, Copy)] // Added Copy as num_threads is Option<usize> and coord_gen_chunk_size is usize
pub struct ParallelConfig {
    pub num_threads: Option<usize>, // For Rayon thread pool, if explicit control is desired
    pub coord_gen_chunk_size: usize, // For parallel_coordinate_generation
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            num_threads: None, // Use Rayon's default
            coord_gen_chunk_size: 1024, // Default chunk size
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParallelProcessor {
    config: ParallelConfig,
    // No internal thread pool; Rayon's global pool is typically used by default.
}

impl ParallelProcessor {
    pub fn new(config: ParallelConfig) -> Self {
        if config.coord_gen_chunk_size == 0 {
            panic!("Coordinate generation chunk size for ParallelProcessor cannot be zero.");
        }
        // If config.num_threads is Some(n > 0), one could try to configure Rayon's global pool here,
        // but it's generally advised against library code configuring the global pool.
        // Applications should configure it if needed. Rayon typically does a good job by default.
        Self { config }
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


        data.par_chunks(self.config.coord_gen_chunk_size)
            .enumerate()
            .flat_map(|(chunk_idx, chunk)| {
                chunk.par_iter() // Parallel iteration within the chunk
                    .enumerate()
                    .map(|(byte_idx_in_chunk, &byte_val)| {
                        let global_position = chunk_idx * self.config.coord_gen_chunk_size + byte_idx_in_chunk;
                        matrix.coordinate_for_position(byte_val, global_position)
                    })
                    .collect::<Vec<Coordinate3D>>()
            })
            .collect()
    }

    /// Parallel pattern analysis (parallelizing window iteration).
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
    use crate::core::coordinates::Coordinate3D;
    use crate::compression::operations::GeometricOperation; // Not used directly, but good for context
    use std::collections::HashSet; // For comparing results as sets if order is not guaranteed


    // Helper function to create a default PatternAnalyzer for tests
    fn create_analyzer() -> PatternAnalyzer {
        PatternAnalyzer::new()
    }
    
    // Helper function for canonical sort of PatternCandidate Vec
    // Sorts candidates themselves, and also sorts coordinates within each candidate
    fn sort_pattern_candidates(candidates: &mut Vec<PatternCandidate>) {
        for candidate in candidates.iter_mut() {
            candidate.coordinates.sort(); // Sort inner coordinates
        }
        candidates.sort(); // Sort the outer Vec<PatternCandidate>
    }

    // Sequential calculation for reference
    fn sequential_pattern_analysis(
        coords: &[Coordinate3D],
        analyzer: &PatternAnalyzer,
        min_w: usize,
        max_w: usize,
    ) -> Vec<PatternCandidate> {
        if coords.is_empty() || min_w == 0 || min_w > max_w {
            return Vec::new();
        }
        let mut all_candidates = Vec::new();
        for window_size in min_w..=max_w {
            if let Ok(mut candidates) = analyzer.find_patterns(coords, window_size, window_size) {
                all_candidates.append(&mut candidates);
            }
        }
        // Remove duplicates that might arise if find_patterns for different window_size calls
        // happens to find the exact same PatternCandidate struct (e.g. if first_occurrence_index is the same)
        // This is unlikely if find_patterns is deterministic for a given (coords, ws, ws) call.
        // However, to be robust for comparison, ensure a unique set, then sort.
        // The `parallel_pattern_analysis` also uses flat_map and collect, which might produce duplicates if
        // `find_patterns` itself isn't perfectly distinct per window_size search.
        // Let's assume `find_patterns(coords, ws, ws)` generates candidates unique to that `ws` context.
        // The main concern for comparison is order.
        sort_pattern_candidates(&mut all_candidates);
        all_candidates
    }


    #[test]
    #[should_panic]
    fn test_parallel_processor_new_zero_chunk_size() {
        ParallelProcessor::new(ParallelConfig { num_threads: None, coord_gen_chunk_size: 0 });
    }

    #[test]
    fn test_parallel_coordinate_generation_empty_input() {
        let processor = ParallelProcessor::new(ParallelConfig::default());
        let data: Vec<u8> = Vec::new();
        let mut matrix = GeometricMatrix::new([0u8; 32]);
        matrix.generate_bijective_mapping().unwrap(); // Ensure mapping

        let result = processor.parallel_coordinate_generation(&data, &matrix);
        assert!(result.is_empty());
    }
    
    #[test]
    #[should_panic]
    fn test_parallel_coordinate_generation_mapping_not_generated() {
        let processor = ParallelProcessor::new(ParallelConfig::default());
        let data: Vec<u8> = vec![1,2,3];
        let matrix = GeometricMatrix::new([0u8; 32]); // Mapping not generated
        processor.parallel_coordinate_generation(&data, &matrix);
    }


    #[test]
    fn test_parallel_coordinate_generation_output_matches_sequential() {
        let processor = ParallelProcessor::new(ParallelConfig { num_threads: None, coord_gen_chunk_size: 2 }); // Small chunk size
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
        let processor = ParallelProcessor::new(ParallelConfig::default());
        let analyzer = create_analyzer();
        let coords = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), // P1
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), // P1
        ];
        let min_w = 2;
        let max_w = 2;

        let mut expected = sequential_pattern_analysis(&coords, &analyzer, min_w, max_w);
        let mut actual = processor.parallel_pattern_analysis(&coords, &analyzer, min_w, max_w);
        
        sort_pattern_candidates(&mut actual);
        // Expected is already sorted by sequential_pattern_analysis helper

        assert_eq!(actual.len(), 1, "Expected one pattern candidate");
        if !actual.is_empty() {
             assert_eq!(actual[0].frequency, 2);
             let mut expected_coords_in_pattern = vec![Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0)];
             expected_coords_in_pattern.sort(); // ensure this is sorted if coordinates in PatternCandidate aren't always
             assert_eq!(actual[0].coordinates, expected_coords_in_pattern);
        }
        assert_eq!(actual, expected);
    }
    
    #[test]
    fn test_parallel_empty_coordinates() {
        let processor = ParallelProcessor::new(ParallelConfig::default());
        let analyzer = create_analyzer();
        let coords: Vec<Coordinate3D> = Vec::new();
        let min_w = 2;
        let max_w = 4;

        let expected = sequential_pattern_analysis(&coords, &analyzer, min_w, max_w);
        let mut actual = processor.parallel_pattern_analysis(&coords, &analyzer, min_w, max_w);
        sort_pattern_candidates(&mut actual);
        
        assert!(actual.is_empty());
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parallel_no_patterns_found() {
        let processor = ParallelProcessor::new(ParallelConfig::default());
        let analyzer = create_analyzer();
        let coords = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0),
        ];
        let min_w = 2;
        let max_w = 3;

        let expected = sequential_pattern_analysis(&coords, &analyzer, min_w, max_w);
        let mut actual = processor.parallel_pattern_analysis(&coords, &analyzer, min_w, max_w);
        sort_pattern_candidates(&mut actual);

        assert!(actual.is_empty());
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parallel_single_pattern_found() {
        let processor = ParallelProcessor::new(ParallelConfig::default());
        let analyzer = create_analyzer();
        let coords = vec![
            Coordinate3D::new(1,1,1), Coordinate3D::new(2,2,2),
            Coordinate3D::new(1,1,1), Coordinate3D::new(2,2,2),
            Coordinate3D::new(3,3,3),
        ];
        let min_w = 2;
        let max_w = 2;

        let mut expected = sequential_pattern_analysis(&coords, &analyzer, min_w, max_w);
        let mut actual = processor.parallel_pattern_analysis(&coords, &analyzer, min_w, max_w);
        sort_pattern_candidates(&mut actual);
        
        assert_eq!(actual.len(), 1);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parallel_multiple_patterns_various_windows() {
        let processor = ParallelProcessor::new(ParallelConfig::default());
        let analyzer = create_analyzer();
        let coords = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), // P1 (len 2)
            Coordinate3D::new(3,0,0), Coordinate3D::new(4,0,0), Coordinate3D::new(5,0,0), // P2 (len 3)
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), // P1 again
            Coordinate3D::new(3,0,0), Coordinate3D::new(4,0,0), Coordinate3D::new(5,0,0), // P2 again
        ];
        let min_w = 2;
        let max_w = 3;

        let mut expected = sequential_pattern_analysis(&coords, &analyzer, min_w, max_w);
        let mut actual = processor.parallel_pattern_analysis(&coords, &analyzer, min_w, max_w);
        sort_pattern_candidates(&mut actual);

        // Expected: P1, and other patterns of length 2, then P2 and other patterns of length 3.
        // The exact number depends on all sub-sequences that repeat.
        // `expected` is already calculated by the sequential helper which mirrors this logic.
        assert_eq!(actual.len(), expected.len(), "Mismatch in number of patterns found");
        assert_eq!(actual, expected);
    }
    
    #[test]
    fn test_parallel_window_range_min_greater_than_max() {
        let processor = ParallelProcessor::new(ParallelConfig::default());
        let analyzer = create_analyzer();
        let coords = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0),
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0),
        ];
        let min_w = 3;
        let max_w = 2;

        let expected = sequential_pattern_analysis(&coords, &analyzer, min_w, max_w);
        let mut actual = processor.parallel_pattern_analysis(&coords, &analyzer, min_w, max_w);
        sort_pattern_candidates(&mut actual);

        assert!(actual.is_empty());
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parallel_window_range_min_equals_max() {
        let processor = ParallelProcessor::new(ParallelConfig::default());
        let analyzer = create_analyzer();
        let coords = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0),
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0),
        ];
        let min_w = 3;
        let max_w = 3;

        let mut expected = sequential_pattern_analysis(&coords, &analyzer, min_w, max_w);
        let mut actual = processor.parallel_pattern_analysis(&coords, &analyzer, min_w, max_w);
        sort_pattern_candidates(&mut actual);
        
        assert_eq!(actual.len(), 1);
        assert_eq!(actual, expected);
    }
}
