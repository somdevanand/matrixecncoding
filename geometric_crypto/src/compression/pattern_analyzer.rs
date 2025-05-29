use crate::core::coordinates::Coordinate3D;
use crate::compression::engine::CompressionError;
use serde::{Serialize, Deserialize};
use std::collections::HashMap; // Ensure HashMap is imported

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatternCandidate {
    pub coordinates: Vec<Coordinate3D>,
    pub frequency: usize,
    pub spatial_density: f32,       // Placeholder, will be 1.0 / window_size
    pub compression_potential: f32, // Placeholder, (frequency - 1) * window_size
    pub first_occurrence_index: Option<usize>, // Added for potential future use
}

#[derive(Debug, Default, Clone)]
pub struct PatternAnalyzer;

impl PatternAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Identifies repeating patterns in a sequence of coordinates using a sliding window.
    pub fn find_patterns(
        &self,
        coords: &[Coordinate3D],
        min_window: usize,
        max_window: usize,
    ) -> Result<Vec<PatternCandidate>, CompressionError> {
        if min_window == 0 || min_window > max_window || coords.is_empty() {
            return Ok(Vec::new()); // Or return an InvalidInput error
        }

        // HashMap to store sequence -> (frequency, first_occurrence_index)
        let mut pattern_counts: HashMap<Vec<Coordinate3D>, (usize, Option<usize>)> = HashMap::new();

        for window_size in min_window..=max_window {
            if window_size > coords.len() {
                break; // Window size cannot be larger than the coordinate sequence itself
            }
            for (idx, window_slice) in coords.windows(window_size).enumerate() {
                let entry = pattern_counts.entry(window_slice.to_vec()).or_insert((0, Some(idx)));
                entry.0 += 1; // Increment frequency
                // Keep the first_occurrence_index from the first time it was inserted.
            }
        }

        let candidates: Vec<PatternCandidate> = pattern_counts
            .into_iter()
            .filter(|(_sequence, (freq, _idx))| *freq > 1) // Only consider patterns that repeat
            .map(|(sequence, (freq, first_idx))| {
                let window_len = sequence.len();
                PatternCandidate {
                    coordinates: sequence,
                    frequency: freq,
                    // Placeholder for spatial_density: lower value means more spread out.
                    // For a linear sequence, density might relate to its bounding box volume vs length.
                    // For now, simple placeholder.
                    spatial_density: 1.0 / window_len as f32, 
                    // Basic compression_potential: (references * (len - overhead_per_ref)) - definition_cost
                    // Simplified: (freq - 1) * len_of_sequence_in_bytes - len_of_sequence_in_bytes (to store it once)
                    // Or, more simply, how many coordinates are "saved": (freq - 1) * window_len
                    compression_potential: ((freq - 1) * window_len) as f32,
                    first_occurrence_index: first_idx,
                }
            })
            .collect();

        Ok(candidates)
    }

    /// Performs spatial clustering analysis on coordinates. (Synchronous Stub)
    /// Returns each coordinate as its own cluster.
    pub fn spatial_clustering(
        &self,
        coords: &[Coordinate3D],
    ) -> Result<Vec<Vec<Coordinate3D>>, CompressionError> {
        if coords.is_empty() {
            return Ok(Vec::new());
        }
        // Basic stub: each coordinate is its own cluster
        let clusters = coords.iter().map(|c| vec![*c]).collect();
        Ok(clusters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coordinates::Coordinate3D;

    #[test]
    fn test_pattern_candidate_creation() { // This test can remain as is
        let candidate = PatternCandidate {
            coordinates: vec![Coordinate3D::new(1,2,3), Coordinate3D::new(4,5,6)],
            frequency: 10,
            spatial_density: 0.75,
            compression_potential: 5.0,
            first_occurrence_index: Some(0),
        };
        assert_eq!(candidate.frequency, 10);
        assert!((candidate.spatial_density - 0.75).abs() < f32::EPSILON);
    }

    // Updated test for find_patterns (non-async)
    #[test]
    fn test_find_patterns_simple_repetition() {
        let analyzer = PatternAnalyzer::new();
        let coords = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), // Pattern P1
            Coordinate3D::new(3,0,0),
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), // Pattern P1
            Coordinate3D::new(4,0,0),
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), // Pattern P1
        ];

        let result = analyzer.find_patterns(&coords, 2, 3); // Window sizes 2 and 3
        assert!(result.is_ok());
        let candidates = result.unwrap();

        // Expected pattern: [ (1,0,0), (2,0,0) ] should appear 3 times.
        let expected_pattern_coords = vec![Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0)];
        let found_candidate = candidates.iter().find(|c| c.coordinates == expected_pattern_coords);
        
        assert!(found_candidate.is_some(), "Expected pattern was not found");
        if let Some(p) = found_candidate {
            assert_eq!(p.frequency, 3);
            assert_eq!(p.first_occurrence_index, Some(0));
            // Potential: ((3-1) * 2) = 4
            assert!((p.compression_potential - 4.0).abs() < f32::EPSILON);
        }
    }
    
    #[test]
    fn test_find_patterns_no_repetition() {
        let analyzer = PatternAnalyzer::new();
        let coords = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0)
        ];
        let result = analyzer.find_patterns(&coords, 2, 2);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty(), "Should find no repeating patterns");
    }

    #[test]
    fn test_find_patterns_empty_input() {
        let analyzer = PatternAnalyzer::new();
        let coords: Vec<Coordinate3D> = vec![];
        let result = analyzer.find_patterns(&coords, 2, 3);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty(), "Should handle empty input gracefully");
    }

    #[test]
    fn test_find_patterns_window_too_large() {
        let analyzer = PatternAnalyzer::new();
        let coords = vec![Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0)];
        let result = analyzer.find_patterns(&coords, 3, 3); // Window size 3, coords len 2
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty(), "Should handle window larger than coords");
    }
    
    // Updated test for spatial_clustering (non-async)
    #[test]
    fn test_spatial_clustering_stub_sync() {
        let analyzer = PatternAnalyzer::new();
        let coords = vec![
            Coordinate3D::new(1,1,1),
            Coordinate3D::new(2,2,2),
        ];
        let result = analyzer.spatial_clustering(&coords);
        assert!(result.is_ok());
        let clusters = result.unwrap();
        assert_eq!(clusters.len(), 2); // Each coord is its own cluster
        assert_eq!(clusters[0], vec![Coordinate3D::new(1,1,1)]);
        assert_eq!(clusters[1], vec![Coordinate3D::new(2,2,2)]);

        let empty_coords: Vec<Coordinate3D> = Vec::new();
        let result_empty = analyzer.spatial_clustering(&empty_coords);
        assert!(result_empty.is_ok());
        assert!(result_empty.unwrap().is_empty());
    }
}
