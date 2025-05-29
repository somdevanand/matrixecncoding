use crate::core::coordinates::Coordinate3D;
use crate::compression::operations::{GeometricOperation, InterpolationType, EncodingScheme, Matrix3x3}; // Ensure all used types are imported
use super::pattern_analyzer::PatternAnalyzer;
use super::optimizer::OperationOptimizer;
use super::encoding::GeometricEncoder;


#[derive(Debug, thiserror::Error)]
pub enum CompressionError {
    #[error("Operation not implemented yet: {0}")]
    NotImplementedDetails(String), 
    #[error("An underlying component failed: {0}")]
    ComponentError(String), 
    #[error("No solution found by optimizer")]
    OptimizationFailed,
    #[error("NotImplemented")] 
    NotImplemented,
}

#[derive(Debug, Clone)] 
pub struct CompressionEngine {
    pattern_analyzer: PatternAnalyzer,
    operation_optimizer: OperationOptimizer,
    geometric_encoder: GeometricEncoder,
}

impl CompressionEngine {
    pub fn new() -> Self {
        Self {
            pattern_analyzer: PatternAnalyzer::new(),
            operation_optimizer: OperationOptimizer::new(),
            geometric_encoder: GeometricEncoder::new(),
        }
    }

    pub fn compress_coordinate_sequence(
        &self,
        coords: &[Coordinate3D]
    ) -> Result<Vec<GeometricOperation>, CompressionError> { 
        let min_window_size = 4;
        let max_window_size = 32;
        
        let patterns = self.pattern_analyzer.find_patterns(coords, min_window_size, max_window_size)
            .map_err(|e| CompressionError::ComponentError(format!("Pattern analysis (find_patterns) failed: {}", e)))?;
        
        let clusters = self.pattern_analyzer.spatial_clustering(coords, 3, 2) 
            .map_err(|e| CompressionError::ComponentError(format!("Spatial clustering failed: {}", e)))?;
        
        let optimized_ops = self.operation_optimizer.optimize_operations(coords, &patterns, &clusters)
            .map_err(|e| CompressionError::ComponentError(format!("Operation optimization failed: {}", e)))?;
        
        let final_ops = self.geometric_encoder.encode_operations(optimized_ops)
            .map_err(|e| CompressionError::ComponentError(format!("Geometric encoding failed: {}", e)))?;

        Ok(final_ops) 
    }

    pub fn decompress_operations(
        &self,
        operations: &[GeometricOperation],
    ) -> Result<Vec<Coordinate3D>, CompressionError> {
        let mut resulting_coords: Vec<Coordinate3D> = Vec::new();

        if operations.is_empty() {
            return Ok(resulting_coords);
        }

        for op in operations {
            match op {
                GeometricOperation::RegionFill { start, end, fill_byte: _, compression_ratio: _ } => {
                    for x in start.x..=end.x {
                        for y in start.y..=end.y {
                            for z in start.z..=end.z {
                                resulting_coords.push(Coordinate3D::new(x, y, z));
                            }
                        }
                    }
                }
                GeometricOperation::PathTrace { waypoints, interpolation, data_encoding: _ } => {
                    if waypoints.is_empty() {
                        continue;
                    }
                    match interpolation {
                        InterpolationType::None => {
                            resulting_coords.extend(waypoints.iter().copied());
                        }
                        InterpolationType::Linear => {
                            if waypoints.len() < 2 {
                                resulting_coords.extend(waypoints.iter().copied());
                            } else {
                                resulting_coords.push(waypoints[0]); 
                                for i in 0..(waypoints.len() - 1) {
                                    let p1 = waypoints[i];
                                    let p2 = waypoints[i+1];
                                    
                                    let mid_x_f32 = (p1.x as f32 + p2.x as f32) / 2.0;
                                    let mid_y_f32 = (p1.y as f32 + p2.y as f32) / 2.0;
                                    let mid_z_f32 = (p1.z as f32 + p2.z as f32) / 2.0;

                                    let mid_coord = Coordinate3D::new(
                                        mid_x_f32.round() as u8, 
                                        mid_y_f32.round() as u8,
                                        mid_z_f32.round() as u8
                                    );
                                    
                                    if mid_coord != p1 && mid_coord != p2 {
                                        resulting_coords.push(mid_coord);
                                    }
                                    resulting_coords.push(p2); 
                                }
                                resulting_coords.dedup();
                            }
                        }
                        InterpolationType::Bezier => {
                            return Err(CompressionError::NotImplementedDetails("Bezier interpolation for PathTrace decompression not implemented".to_string()));
                        }
                    }
                }
                GeometricOperation::PatternReference { .. } => {
                    return Err(CompressionError::NotImplementedDetails("PatternReference decompression not implemented".to_string()));
                }
                GeometricOperation::SparseMapping { .. } => {
                    return Err(CompressionError::NotImplementedDetails("SparseMapping decompression not implemented".to_string()));
                }
                GeometricOperation::FunctionGeneration { .. } => {
                    return Err(CompressionError::NotImplementedDetails("FunctionGeneration decompression not implemented".to_string()));
                }
            }
        }
        Ok(resulting_coords)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Coordinate3D and GeometricOperation are already imported by super::*

    #[test]
    fn test_compression_engine_new() {
        let _engine = CompressionEngine::new();
        // Basic check
    }

    #[test]
    fn test_compress_coordinate_sequence_sync_flow() {
        let engine = CompressionEngine::new();
        
        let empty_coords: Vec<Coordinate3D> = Vec::new();
        let result_empty = engine.compress_coordinate_sequence(&empty_coords);
        assert!(result_empty.is_ok(), "Empty input should result in Ok");
        assert!(result_empty.unwrap().is_empty(), "Expected empty operations for empty input");
        
        let patterned_coords = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), Coordinate3D::new(4,0,0),
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), Coordinate3D::new(4,0,0),
        ];
        let result_patterned = engine.compress_coordinate_sequence(&patterned_coords);
        assert!(result_patterned.is_ok(), "Patterned input should result in Ok: {:?}", result_patterned.err());
        let ops = result_patterned.unwrap();
        assert_eq!(ops.len(), 1, "Expected one operation for the patterned input"); 
        match &ops[0] {
            GeometricOperation::PatternReference { base_coordinate, pattern_id, .. } => {
                assert_eq!(*base_coordinate, Coordinate3D::new(1,0,0));
                assert_eq!(*pattern_id, 0);
            }
            _ => panic!("Expected PatternReference operation for patterned input"),
        }
    }

    #[test]
    fn test_decompress_operations_partial() { 
        let engine = CompressionEngine::new();
        
        let empty_ops: Vec<GeometricOperation> = Vec::new();
        assert!(engine.decompress_operations(&empty_ops).unwrap().is_empty());

        let rf_op = GeometricOperation::RegionFill {
            start: Coordinate3D::new(0,0,0),
            end: Coordinate3D::new(1,1,0), 
            fill_byte: 0, compression_ratio: 0.0
        };
        let rf_coords = engine.decompress_operations(&[rf_op.clone()]).unwrap();
        assert_eq!(rf_coords.len(), 2 * 2 * 1);
        assert!(rf_coords.contains(&Coordinate3D::new(0,0,0)));
        assert!(rf_coords.contains(&Coordinate3D::new(1,1,0)));
        assert!(!rf_coords.contains(&Coordinate3D::new(2,0,0)));

        let pt_none_op = GeometricOperation::PathTrace {
            waypoints: vec![Coordinate3D::new(1,1,1), Coordinate3D::new(2,2,2)],
            interpolation: InterpolationType::None,
            data_encoding: EncodingScheme::Raw, 
        };
        let pt_none_coords = engine.decompress_operations(&[pt_none_op.clone()]).unwrap();
        assert_eq!(pt_none_coords, vec![Coordinate3D::new(1,1,1), Coordinate3D::new(2,2,2)]);

        let pt_linear_op = GeometricOperation::PathTrace {
            waypoints: vec![Coordinate3D::new(0,0,0), Coordinate3D::new(4,4,4)], 
            interpolation: InterpolationType::Linear,
            data_encoding: EncodingScheme::Raw,
        };
        let pt_linear_coords = engine.decompress_operations(&[pt_linear_op.clone()]).unwrap();
        assert_eq!(pt_linear_coords.len(), 3); 
        assert_eq!(pt_linear_coords[0], Coordinate3D::new(0,0,0));
        assert_eq!(pt_linear_coords[1], Coordinate3D::new(2,2,2));
        assert_eq!(pt_linear_coords[2], Coordinate3D::new(4,4,4));
        
        let pt_linear_short_op = GeometricOperation::PathTrace {
            waypoints: vec![Coordinate3D::new(0,0,0), Coordinate3D::new(1,1,1)], 
            interpolation: InterpolationType::Linear,
            data_encoding: EncodingScheme::Raw,
        };
        let pt_linear_short_coords = engine.decompress_operations(&[pt_linear_short_op.clone()]).unwrap();
        assert_eq!(pt_linear_short_coords.len(), 2); 
        assert_eq!(pt_linear_short_coords, vec![Coordinate3D::new(0,0,0), Coordinate3D::new(1,1,1)]);

        let pr_op = GeometricOperation::PatternReference {
            base_coordinate: Default::default(), pattern_id: 0, transformation_matrix: Matrix3x3::identity()
        };
        assert!(engine.decompress_operations(&[pr_op]).is_err());
    }
}
