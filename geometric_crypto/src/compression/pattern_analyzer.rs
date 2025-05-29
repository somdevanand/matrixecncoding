use crate::core::coordinates::Coordinate3D;
use crate::compression::engine::CompressionError;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::cmp::Ordering; // For Ord implementation

// PatternCandidate struct and distance_squared helper (no change from Turn 55)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)] 
pub struct PatternCandidate {
    pub coordinates: Vec<Coordinate3D>, 
    pub frequency: usize,
    pub spatial_density: f32,
    pub compression_potential: f32, 
    pub first_occurrence_index: Option<usize>,
}

impl Eq for PatternCandidate {} // Manual implementation of Eq, relies on derived PartialEq

impl PartialOrd for PatternCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PatternCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.coordinates.cmp(&other.coordinates)
            .then_with(|| self.frequency.cmp(&other.frequency))
            .then_with(|| self.first_occurrence_index.cmp(&other.first_occurrence_index))
            // Ignoring f32 fields for stable sorting in tests
    }
}

fn distance_squared(c1: &Coordinate3D, c2: &Coordinate3D) -> u32 {
    let dx = (c1.x as i16 - c2.x as i16).abs() as u32;
    let dy = (c1.y as i16 - c2.y as i16).abs() as u32;
    let dz = (c1.z as i16 - c2.z as i16).abs() as u32;
    dx * dx + dy * dy + dz * dz
}

#[derive(Debug, Default, Clone)]
pub struct PatternAnalyzer;

const NOISE: usize = 0; // Cluster ID for noise points
const UNDEFINED: usize = usize::MAX; // Point not yet visited

impl PatternAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    // find_patterns method (no change from Turn 55)
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

    /// Performs DBSCAN-like spatial clustering.
    pub fn spatial_clustering(
        &self,
        coords: &[Coordinate3D],
        epsilon_sq: u32, // Squared Euclidean distance
        min_points: usize, // Minimum number of points to form a dense region
    ) -> Result<Vec<Vec<Coordinate3D>>, CompressionError> {
        if coords.is_empty() {
            return Ok(Vec::new());
        }
        if min_points == 0 {
            return Err(CompressionError::NotImplementedDetails("min_points must be > 0".to_string()));
        }

        let n = coords.len();
        let mut cluster_ids = vec![UNDEFINED; n]; // Stores cluster_id for each point, initialized to UNDEFINED
        let mut current_cluster_id = NOISE + 1; // Cluster IDs start from 1 (0 is for NOISE)

        // Main loop: Iterate through each point
        for i in 0..n {
            if cluster_ids[i] != UNDEFINED { // Point already visited and classified (part of a cluster or noise)
                continue;
            }

            // Find all points within epsilon_sq distance (neighbors)
            let neighbors_indices = self.region_query(coords, i, epsilon_sq);

            if neighbors_indices.len() < min_points {
                // Not enough neighbors to form a core point's environment
                cluster_ids[i] = NOISE; // Mark as noise
                continue;
            }

            // Point 'i' is a core point, start a new cluster
            cluster_ids[i] = current_cluster_id;
            
            // Use a queue to expand the cluster from this core point
            // Initialize queue with the direct neighbors of the core point 'i'
            // (excluding 'i' itself as it's already processed)
            let mut queue: Vec<usize> = neighbors_indices.into_iter().filter(|&idx| idx != i).collect();
            
            // Process points in the queue to expand the cluster
            let mut head = 0; // Use head index for queue to avoid inefficient pop(0)
            while head < queue.len() {
                let q_idx = queue[head]; // Current point from queue to process
                head += 1;

                if cluster_ids[q_idx] == NOISE {
                    // Noise point is density-reachable, so it becomes a border point of the current cluster
                    cluster_ids[q_idx] = current_cluster_id;
                }
                
                // If already processed and part of another cluster, or already part of this one, skip.
                // Note: A point initially marked NOISE can be added to a cluster as a border point.
                // If it's UNDEFINED, it's a new point for the cluster.
                if cluster_ids[q_idx] != UNDEFINED && cluster_ids[q_idx] != NOISE { 
                    continue;
                }
                
                cluster_ids[q_idx] = current_cluster_id; // Add to current cluster

                // Check if this newly added point q_idx is also a core point
                let q_neighbors_indices = self.region_query(coords, q_idx, epsilon_sq);
                if q_neighbors_indices.len() >= min_points {
                    // q_idx is a core point, add its unvisited/noise neighbors to the queue
                    for neighbor_idx in q_neighbors_indices {
                        if cluster_ids[neighbor_idx] == UNDEFINED || cluster_ids[neighbor_idx] == NOISE {
                            // Avoid re-adding points already in the queue or processed.
                            // If it was noise, it now becomes part of this cluster (potentially border).
                            // If it's UNDEFINED, it's a candidate for expansion.
                            if !queue[head..].contains(&neighbor_idx) && cluster_ids[neighbor_idx] == UNDEFINED { // Check only unprocessed part of queue
                                queue.push(neighbor_idx);
                            } else if cluster_ids[neighbor_idx] == NOISE { // If it was noise, make it border
                                cluster_ids[neighbor_idx] = current_cluster_id;
                            }
                        }
                    }
                }
            }
            current_cluster_id += 1; // Finished with the current cluster, move to the next ID
        }

        // Group coordinates by their assigned cluster_ids
        let mut result_clusters_map: HashMap<usize, Vec<Coordinate3D>> = HashMap::new();
        for i in 0..n {
            if cluster_ids[i] != NOISE && cluster_ids[i] != UNDEFINED { // Ensure point was assigned to a valid cluster
                result_clusters_map.entry(cluster_ids[i])
                    .or_default()
                    .push(coords[i]);
            }
        }
        Ok(result_clusters_map.into_values().collect())
    }

    // Helper function to find indices of points within epsilon_sq of coords[point_idx]
    fn region_query(&self, coords: &[Coordinate3D], point_idx: usize, epsilon_sq: u32) -> Vec<usize> {
        let mut neighbors = Vec::new();
        for i in 0..coords.len() {
            if distance_squared(&coords[point_idx], &coords[i]) <= epsilon_sq {
                neighbors.push(i);
            }
        }
        neighbors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // PatternCandidate and find_patterns tests (no change from Turn 55)
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
    fn test_spatial_clustering_dbscan_like() { 
        let analyzer = PatternAnalyzer::new();
        let coords = vec![
            Coordinate3D::new(1,1,1),   // P0 - C1
            Coordinate3D::new(2,1,1),   // P1 - C1
            Coordinate3D::new(1,2,1),   // P2 - C1
            Coordinate3D::new(10,10,10), // P3 (Noise)
            Coordinate3D::new(20,20,20), // P4 - C2 Core
            Coordinate3D::new(21,20,20), // P5 - C2 Neighbor
            Coordinate3D::new(20,21,20), // P6 - C2 Neighbor
            Coordinate3D::new(21,21,20), // P7 - C2 Density-reachable from P6
        ];
        // Epsilon_sq = 2 means points are neighbors if dist <= sqrt(2) (~1.414)
        // P0-P1: dist_sq=1, P0-P2: dist_sq=1, P1-P2: dist_sq=2. All <= 2.
        // P4-P5: dist_sq=1, P4-P6: dist_sq=1. P5-P7: dist_sq=1. P6-P7: dist_sq=1.
        let epsilon_sq = 2; 
        let min_points = 3; // A point needs at least 2 other points (total 3) in its neighborhood to be a core point.

        let result = analyzer.spatial_clustering(&coords, epsilon_sq, min_points);
        assert!(result.is_ok());
        let mut clusters = result.unwrap();
        
        // Sort clusters and their contents for consistent comparison
        for cluster in &mut clusters { cluster.sort_by_key(|c| (c.x, c.y, c.z)); }
        clusters.sort_by_key(|c| if c.is_empty() { (0,0,0) } else { (c[0].x, c[0].y, c[0].z) });

        assert_eq!(clusters.len(), 2, "Expected 2 clusters. Found: {:?}", clusters);

        let mut expected_c1_sorted = vec![coords[0], coords[1], coords[2]]; // P0,P1,P2
        expected_c1_sorted.sort_by_key(|c| (c.x, c.y, c.z));
        // Ensure clusters[0] is also sorted for comparison (already done by the loop)
        assert_eq!(clusters[0], expected_c1_sorted, "Cluster 1 mismatch");

        let mut expected_c2_sorted = vec![coords[4], coords[5], coords[6], coords[7]]; // P4,P5,P6,P7
        expected_c2_sorted.sort_by_key(|c| (c.x, c.y, c.z));
        // Ensure clusters[1] is also sorted for comparison (already done by the loop)
        assert_eq!(clusters[1], expected_c2_sorted, "Cluster 2 mismatch");
        
        // Test empty input
        let empty_coords: Vec<Coordinate3D> = Vec::new();
        let result_empty = analyzer.spatial_clustering(&empty_coords, epsilon_sq, min_points);
        assert!(result_empty.is_ok());
        assert!(result_empty.unwrap().is_empty());

        // Test min_points = 0 (error case)
        let result_min_points_zero = analyzer.spatial_clustering(&coords, epsilon_sq, 0);
        assert!(result_min_points_zero.is_err());
        match result_min_points_zero.err().unwrap() {
            CompressionError::NotImplementedDetails(msg) => assert_eq!(msg, "min_points must be > 0"),
            _ => panic!("Expected NotImplementedDetails error for min_points = 0"),
        }
    }

    #[test]
    fn test_spatial_clustering_all_noise() {
        let analyzer = PatternAnalyzer::new();
        let coords = vec![
            Coordinate3D::new(1,1,1),
            Coordinate3D::new(10,10,10),
            Coordinate3D::new(20,20,20),
        ];
        // Epsilon such that no two points are neighbors
        let result = analyzer.spatial_clustering(&coords, 1, 2);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty(), "Expected no clusters, all points should be noise");
    }

    #[test]
    fn test_spatial_clustering_border_point() {
        let analyzer = PatternAnalyzer::new();
        // P0, P1, P2 form a core. P3 is reachable by P2 but P3 itself is not core.
        let coords = vec![
            Coordinate3D::new(1,1,1), // P0 - Core
            Coordinate3D::new(2,1,1), // P1 - Core
            Coordinate3D::new(1,2,1), // P2 - Core
            Coordinate3D::new(1,3,1), // P3 - Border, reachable from P2
        ];
        let epsilon_sq = 1; // Max dist 1 for neighborhood
        let min_points = 3; // P0,P1,P2 are core. P3 has only P2 as neighbor.
        
        // P0 neighbors: P1, P2 (dist_sq=1 for both). Count=3. Core.
        // P1 neighbors: P0 (dist_sq=1). Count=2. Not core by itself initially, but becomes part of P0's cluster.
        // P2 neighbors: P0, P3 (dist_sq=1 for both). Count=3. Core.
        // P3 neighbors: P2 (dist_sq=1). Count=2. Not core.
        
        let result = analyzer.spatial_clustering(&coords, epsilon_sq, min_points);
        assert!(result.is_ok());
        let mut clusters = result.unwrap();
        assert_eq!(clusters.len(), 1, "Expected 1 cluster");
        
        let mut actual_cluster0_sorted = clusters[0].clone(); // Clone to sort if not already
        actual_cluster0_sorted.sort_by_key(|c| (c.x,c.y,c.z)); // Ensure this specific cluster is sorted

        let mut expected_c1_border_sorted = vec![coords[0], coords[1], coords[2], coords[3]];
        expected_c1_border_sorted.sort_by_key(|c| (c.x,c.y,c.z));
        assert_eq!(actual_cluster0_sorted, expected_c1_border_sorted, "Cluster should include the border point P3");
    }

    #[test]
    fn test_spatial_clustering_distinct_clusters_close_proximity() {
        let analyzer = PatternAnalyzer::new();
        let coords = vec![
            Coordinate3D::new(1,1,1), Coordinate3D::new(2,1,1), // Cluster 1
            Coordinate3D::new(4,1,1), Coordinate3D::new(5,1,1), // Cluster 2
        ];
        // Epsilon_sq=1 makes (1,1,1)-(2,1,1) neighbors, and (4,1,1)-(5,1,1) neighbors.
        // But (2,1,1) and (4,1,1) have dist_sq = (4-2)^2 = 4.
        let result = analyzer.spatial_clustering(&coords, 1, 2); // min_points = 2
        assert!(result.is_ok());
        let mut clusters = result.unwrap();
        for cluster in &mut clusters { cluster.sort_by_key(|c| (c.x,c.y,c.z));}
        clusters.sort_by_key(|c| if c.is_empty() { (0,0,0) } else { (c[0].x,c[0].y,c[0].z) });
        
        assert_eq!(clusters.len(), 2, "Expected 2 distinct clusters");
        assert_eq!(clusters[0], vec![coords[0], coords[1]]);
        assert_eq!(clusters[1], vec![coords[2], coords[3]]);
    }

    #[test]
    fn test_spatial_clustering_merged_by_bridge_point() {
        let analyzer = PatternAnalyzer::new();
        let coords = vec![
            Coordinate3D::new(1,1,1), Coordinate3D::new(2,1,1), // Group A
            Coordinate3D::new(3,1,1),                         // Bridge Point P_bridge
            Coordinate3D::new(4,1,1), Coordinate3D::new(5,1,1), // Group B
        ];
        // P_bridge is neighbor to (2,1,1) and (4,1,1) with epsilon_sq=1.
        // If min_points=2, (2,1,1) can be core, P_bridge becomes its neighbor.
        // If P_bridge is also core (needs one other neighbor, e.g. (2,1,1) or (4,1,1)), it can merge.
        // Let min_points = 2.
        // (1,1,1) is core with (2,1,1).
        // (2,1,1) is core with (1,1,1) and (3,1,1).
        // (3,1,1) is core with (2,1,1) and (4,1,1). This merges.
        // (4,1,1) is core with (3,1,1) and (5,1,1).
        // (5,1,1) is core with (4,1,1).
        let result = analyzer.spatial_clustering(&coords, 1, 2); // min_points = 2
        assert!(result.is_ok());
        let mut clusters = result.unwrap();
        assert_eq!(clusters.len(), 1, "Expected 1 merged cluster");
        assert_eq!(clusters[0].len(), 5, "Merged cluster should have 5 points");
    }
}
