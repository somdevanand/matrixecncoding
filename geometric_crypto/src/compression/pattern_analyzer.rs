use crate::core::coordinates::Coordinate3D;
use crate::compression::engine::CompressionError;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet}; // Added HashSet for visited points in clustering

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

    pub first_occurrence_index: Option<usize>,
}


#[derive(Debug, Default, Clone)]
pub struct PatternAnalyzer;

// Helper function for Euclidean distance squared (to avoid sqrt) between Coordinate3D
// Or just use f32 for actual distance if precision is needed.
// For u8 coordinates, differences can be small.
fn distance_squared(c1: &Coordinate3D, c2: &Coordinate3D) -> u32 {
    let dx = (c1.x as i16 - c2.x as i16).abs() as u32;
    let dy = (c1.y as i16 - c2.y as i16).abs() as u32;
    let dz = (c1.z as i16 - c2.z as i16).abs() as u32;
    // Using squared L1 distance (Manhattan distance) for simplicity to avoid f32 and sqrt
    // dx + dy + dz for L1, or dx*dx + dy*dy + dz*dz for L2 squared
    dx * dx + dy * dy + dz * dz // L2 distance squared
}


impl PatternAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    // ... find_patterns method ... (no change from Turn 44)
    pub fn find_patterns(
        &self,
        coords: &[Coordinate3D],
        min_window: usize,
        max_window: usize,
    ) -> Result<Vec<PatternCandidate>, CompressionError> {
        if min_window == 0 || min_window > max_window || coords.is_empty() {
            return Ok(Vec::new());
        }
        let mut pattern_counts: HashMap<Vec<Coordinate3D>, (usize, Option<usize>)> = HashMap::new();
        for window_size in min_window..=max_window {
            if window_size > coords.len() {
                break;
            }
            for (idx, window_slice) in coords.windows(window_size).enumerate() {
                let entry = pattern_counts.entry(window_slice.to_vec()).or_insert((0, Some(idx)));
                entry.0 += 1;
            }
        }
        let candidates: Vec<PatternCandidate> = pattern_counts
            .into_iter()
            .filter(|(_sequence, (freq, _idx))| *freq > 1)
            .map(|(sequence, (freq, first_idx))| {
                let window_len = sequence.len();
                PatternCandidate {
                    coordinates: sequence,
                    frequency: freq,
                    spatial_density: 1.0 / window_len as f32,
                    compression_potential: ((freq - 1) * window_len) as f32,
                    first_occurrence_index: first_idx,
                }
            })
            .collect();
        Ok(candidates)
    }

    /// Basic spatial clustering: forms clusters from points within a given epsilon neighborhood.
    pub fn spatial_clustering(
        &self,
        coords: &[Coordinate3D],
        epsilon_sq: u32, // Use squared epsilon to avoid sqrt in distance calculation
        min_points: usize, // Min points to form a dense region (core point itself + neighbors)
    ) -> Result<Vec<Vec<Coordinate3D>>, CompressionError> {
        if coords.is_empty() || min_points == 0 {
            return Ok(Vec::new());
        }

        let mut clusters: Vec<Vec<Coordinate3D>> = Vec::new();
        let mut visited_indices: HashSet<usize> = HashSet::new();

        for i in 0..coords.len() {
            if visited_indices.contains(&i) {
                continue;
            }

            let mut current_cluster_neighbors_indices: Vec<usize> = Vec::new();
            // Find neighbors of coords[i]
            for j in 0..coords.len() {
                // if visited_indices.contains(&j) { continue; } // Optional: allow points to be boundary points of multiple clusters if not strictly partitioning
                if distance_squared(&coords[i], &coords[j]) <= epsilon_sq {
                    current_cluster_neighbors_indices.push(j);
                }
            }

            if current_cluster_neighbors_indices.len() >= min_points {
                // This point (coords[i]) and its neighbors form a potential cluster core.
                // For this basic version, just take these neighbors as the cluster.
                // A full DBSCAN would expand further from these neighbors if they are also core points.
                let mut new_cluster: Vec<Coordinate3D> = Vec::new();
                for neighbor_idx in current_cluster_neighbors_indices {
                    if !visited_indices.contains(&neighbor_idx) { // Add point to cluster only if not visited
                        visited_indices.insert(neighbor_idx);
                        new_cluster.push(coords[neighbor_idx]);
                    }
                }
                if !new_cluster.is_empty() { // Ensure cluster actually has points after visited check
                    clusters.push(new_cluster);
                }
            } else {
                // Mark as noise or handle later; for now, unclustered points are simply not added.
                // To ensure all points are processed or marked, one might add:
                // visited_indices.insert(i); // Mark as visited even if not part of a dense cluster
            }
        }
        Ok(clusters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ... existing tests for PatternCandidate and find_patterns ...
    #[test]
    fn test_pattern_candidate_creation() {
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

    #[test]
    fn test_find_patterns_simple_repetition() {
        let analyzer = PatternAnalyzer::new();
        let coords = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), 
            Coordinate3D::new(3,0,0),
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), 
            Coordinate3D::new(4,0,0),
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), 
        ];
        let result = analyzer.find_patterns(&coords, 2, 3);
        assert!(result.is_ok());
        let candidates = result.unwrap();
        let expected_pattern_coords = vec![Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0)];
        let found_candidate = candidates.iter().find(|c| c.coordinates == expected_pattern_coords);
        assert!(found_candidate.is_some(), "Expected pattern was not found");
        if let Some(p) = found_candidate {
            assert_eq!(p.frequency, 3);
            assert_eq!(p.first_occurrence_index, Some(0));
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
        let result = analyzer.find_patterns(&coords, 3, 3);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty(), "Should handle window larger than coords");
    }
    
    // Updated test for spatial_clustering
    #[test]
    fn test_spatial_clustering_basic() {
        let analyzer = PatternAnalyzer::new();
        let coords = vec![
            Coordinate3D::new(1,1,1), // C0
            Coordinate3D::new(2,2,2), // C1
            Coordinate3D::new(3,3,3), // C2
            Coordinate3D::new(10,10,10), // C3
            Coordinate3D::new(11,11,11), // C4
        ];
        // Epsilon_sq for L2 dist sq. (2-1)^2 + (2-1)^2 + (2-1)^2 = 1+1+1 = 3.
        // (3-1)^2*3 = 4*3 = 12.
        // (10-1)^2*3 = 9*9*3 = 81*3 = 243.
        let epsilon_sq = 3; // e.g., points within sqrt(3) distance (approx 1.73 in each dim off)
        let min_points = 2; // A point and at least one neighbor

        let result = analyzer.spatial_clustering(&coords, epsilon_sq, min_points);
        assert!(result.is_ok());
        let clusters = result.unwrap();
        
        // Expected: C0,C1,C2 might form a cluster or parts of one. C3,C4 another.
        // With epsilon_sq = 3:
        // C0 (1,1,1): dist_sq(C0,C1)=3. dist_sq(C0,C2)=12. Neighbors of C0: {C0, C1} (count 2) -> Forms cluster [C0, C1]
        // C1 (2,2,2): dist_sq(C1,C0)=3. dist_sq(C1,C2)=3. Neighbors of C1: {C0, C1, C2} (count 3) -> Forms cluster [C0,C1,C2] (if C0 not visited)
        // C2 (3,3,3): dist_sq(C2,C1)=3. dist_sq(C2,C0)=12. Neighbors of C2: {C1, C2} (count 2) -> Forms cluster [C1,C2] (if C1 not visited)
        // C3 (10,10,10): dist_sq(C3,C4)=3. Neighbors of C3: {C3,C4} (count 2) -> Forms cluster [C3,C4]
        // C4 (11,11,11): dist_sq(C4,C3)=3. Neighbors of C4: {C3,C4} (count 2) -> Forms cluster [C3,C4] (if C3 not visited)

        // The simple greedy algorithm forms clusters:
        // 1. i=0 (C0): neighbors {C0,C1}. min_points=2 met. Cluster [C0,C1]. visited={0,1}.
        // 2. i=1 (C1): visited. skip.
        // 3. i=2 (C2): not visited. neighbors {C1,C2}. C1 is visited. Only C2 is added. min_points not met if we only count unvisited for the density check, or cluster is just [C2].
        //    The current logic: current_cluster_neighbors_indices = {1,2}. This list has length 2. So, it forms a cluster.
        //    new_cluster adds unvisited from {C1,C2}. C1 is visited. So new_cluster = [C2]. This is pushed.
        // 4. i=3 (C3): not visited. neighbors {C3,C4}. Cluster [C3,C4]. visited={0,1,2,3,4}.
        // 5. i=4 (C4): visited. skip.
        // Expected: [[C0,C1], [C2], [C3,C4]] or similar depending on tie-breaking / exact visited logic.
        // Let's make the test more robust by checking properties.
        
        let mut total_clustered_points = 0;
        for cluster in &clusters {
            total_clustered_points += cluster.len();
            // Check if points in a cluster are close to at least one other point in the same cluster (not strictly DBSCAN but a property of this impl)
            if cluster.len() > 1 {
                let first_point = cluster[0];
                for i in 1..cluster.len() {
                    assert!(distance_squared(&first_point, &cluster[i]) <= epsilon_sq, "Point in cluster too far from first point");
                }
            }
        }
        // With current logic, C2 forms its own cluster because C1 is already visited when C2 is processed.
        // C0 forms [C0, C1]
        // C2 forms [C2] (neighbor C1 is visited)
        // C3 forms [C3, C4]
        assert_eq!(clusters.len(), 3, "Expected 3 clusters for the given setup.");
        // A more robust test would check set equality of clusters if order is not guaranteed.

        let empty_coords: Vec<Coordinate3D> = Vec::new();
        let result_empty = analyzer.spatial_clustering(&empty_coords, epsilon_sq, min_points);
        assert!(result_empty.is_ok());
        assert!(result_empty.unwrap().is_empty());
    }
}
