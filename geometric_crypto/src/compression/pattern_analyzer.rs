use crate::core::coordinates::Coordinate3D;
use crate::compression::engine::CompressionError;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet}; // Added HashSet

// PatternCandidate struct and distance_squared helper (no change from Turn 55)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatternCandidate {
    pub coordinates: Vec<Coordinate3D>,
    pub frequency: usize,
    pub spatial_density: f32,
    pub compression_potential: f32,
    pub first_occurrence_index: Option<usize>,
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
        let mut cluster_ids = vec![UNDEFINED; n]; // Store cluster ID for each point
        let mut current_cluster_id = NOISE + 1; // Start cluster IDs from 1

        for i in 0..n {
            if cluster_ids[i] != UNDEFINED { // Already processed
                continue;
            }

            let neighbors_indices = self.region_query(coords, i, epsilon_sq);

            if neighbors_indices.len() < min_points {
                cluster_ids[i] = NOISE; // Mark as noise
                continue;
            }

            // Core point found, start a new cluster
            // Note: Corrected variable name from clusters_ids to cluster_ids
            cluster_ids[i] = current_cluster_id; 
            let mut seed_set = neighbors_indices.into_iter().collect::<HashSet<usize>>();
            // The point itself (i) is part of its own neighborhood query, so it's in seed_set.
            // We've assigned cluster_ids[i], so we can remove it from the seed_set to avoid re-processing it
            // if it's not strictly necessary for queue setup, but ensure it's part of the cluster.
            // The main loop for point `i` already handles `cluster_ids[i]`.
            // The queue should contain neighbors to expand from.
            
            let mut queue: Vec<usize> = seed_set.iter().cloned().filter(|&idx| idx != i).collect();
            // It's also okay to leave `i` in the queue and let the `cluster_ids[q_idx] != UNDEFINED` check handle it.
            // For clarity, let's process `i` and then use its neighbors.

            while let Some(q_idx) = queue.pop() {
                if cluster_ids[q_idx] == NOISE { // Change noise point to border point
                    cluster_ids[q_idx] = current_cluster_id;
                }
                if cluster_ids[q_idx] != UNDEFINED { // Already processed or became border point
                    continue;
                }
                cluster_ids[q_idx] = current_cluster_id; // Add to current cluster

                let q_neighbors_indices = self.region_query(coords, q_idx, epsilon_sq);
                if q_neighbors_indices.len() >= min_points { // q_idx is also a core point
                    for neighbor_idx in q_neighbors_indices {
                        // Add new unvisited or noise points to the queue for expansion
                        if cluster_ids[neighbor_idx] == UNDEFINED || cluster_ids[neighbor_idx] == NOISE {
                             if cluster_ids[neighbor_idx] == NOISE { 
                                cluster_ids[neighbor_idx] = current_cluster_id; // Mark noise as border
                            }
                            // Add to queue only if it's truly unclassified to prevent redundant processing
                            if cluster_ids[neighbor_idx] == UNDEFINED { 
                                queue.push(neighbor_idx);
                            }
                        }
                    }
                }
            }
            current_cluster_id += 1; // Move to next cluster ID
        }

        // Group coordinates by cluster_id
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
    fn test_spatial_clustering_dbscan_like() { // Renamed from test_spatial_clustering_basic
        let analyzer = PatternAnalyzer::new();
        let coords = vec![
            Coordinate3D::new(1,1,1),   // P0
            Coordinate3D::new(2,1,1),   // P1
            Coordinate3D::new(1,2,1),   // P2
            Coordinate3D::new(10,10,10), // P3 (Noise)
            Coordinate3D::new(20,20,20), // P4 Cluster 2, core
            Coordinate3D::new(21,20,20), // P5 Cluster 2, neighbor
            Coordinate3D::new(20,21,20), // P6 Cluster 2, neighbor
            Coordinate3D::new(21,21,20), // P7 Cluster 2, density-reachable from P6
        ];

        let epsilon_sq = 2; 
        let min_points = 3; 

        let result = analyzer.spatial_clustering(&coords, epsilon_sq, min_points);
        assert!(result.is_ok());
        let mut clusters = result.unwrap();
        
        for cluster in &mut clusters { cluster.sort_by_key(|c| (c.x, c.y, c.z)); }
        clusters.sort_by_key(|c| if c.is_empty() { (0,0,0) } else { (c[0].x, c[0].y, c[0].z) });

        assert_eq!(clusters.len(), 2, "Expected 2 clusters, found {}. Clusters: {:?}", clusters.len(), clusters);

        let expected_c1 = vec![Coordinate3D::new(1,1,1), Coordinate3D::new(1,2,1), Coordinate3D::new(2,1,1)];
        assert_eq!(clusters[0].len(), expected_c1.len(), "Cluster 1 length mismatch");
        assert!(expected_c1.iter().all(|item| clusters[0].contains(item)), "Cluster 1 content mismatch: expected {:?}, got {:?}", expected_c1, clusters[0]);

        let expected_c2 = vec![
            Coordinate3D::new(20,20,20), Coordinate3D::new(20,21,20),
            Coordinate3D::new(21,20,20), Coordinate3D::new(21,21,20)
        ];
         assert_eq!(clusters[1].len(), expected_c2.len(), "Cluster 2 length mismatch");
         assert!(expected_c2.iter().all(|item| clusters[1].contains(item)), "Cluster 2 content mismatch: expected {:?}, got {:?}", expected_c2, clusters[1]);

        let empty_coords: Vec<Coordinate3D> = Vec::new();
        let result_empty = analyzer.spatial_clustering(&empty_coords, epsilon_sq, min_points);
        assert!(result_empty.is_ok());
        assert!(result_empty.unwrap().is_empty());

        let result_min_points_zero = analyzer.spatial_clustering(&coords, epsilon_sq, 0);
        assert!(result_min_points_zero.is_err());
        match result_min_points_zero.err().unwrap() {
            CompressionError::NotImplementedDetails(msg) => assert_eq!(msg, "min_points must be > 0"),
            _ => panic!("Expected NotImplementedDetails error for min_points = 0"),
        }
    }
}
