use crate::core::coordinates::Coordinate3D;
use super::operations::GeometricOperation;
use super::pattern_analyzer::PatternAnalyzer;
use super::optimizer::OperationOptimizer;
use super::encoding::GeometricEncoder;
use crate::core::memory_optimizer::MemoryOptimizer;
#[cfg(feature = "parallel")]
use crate::core::parallel_processor::ParallelConfig;

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

#[derive(Debug, thiserror::Error)]
pub enum DecompressionError {
    #[error("Invalid operation data: {0}")]
    InvalidOperationData(String),
    #[error("Unsupported operation for decompression: {0}")]
    UnsupportedOperation(String),
    #[error("Coordinate overflow/underflow during reconstruction")]
    CoordinateOverflow,
    #[error("Pattern ID not found: {0}")]
    PatternNotFound(u32),
    #[error("Failed to apply transformation: {0}")]
    TransformationError(String),
    #[error("Error during function evaluation: {0}")]
    FunctionEvaluationError(String),
}

#[derive(Debug, Clone)]
pub struct CompressionEngine {
    pattern_analyzer: PatternAnalyzer,
    operation_optimizer: OperationOptimizer,
    geometric_encoder: GeometricEncoder,
}

impl CompressionEngine {
    pub fn new(
        #[cfg(feature = "parallel")] parallel_enabled: bool,
        #[cfg(feature = "parallel")] parallel_config: Option<ParallelConfig>
    ) -> Self {
        let min_pattern_window = 4; 
        let max_pattern_window = 32;

        Self {
            pattern_analyzer: PatternAnalyzer::new(),
            operation_optimizer: OperationOptimizer::new(
                min_pattern_window, 
                max_pattern_window,
                #[cfg(feature = "parallel")] parallel_enabled,
                #[cfg(feature = "parallel")] parallel_config
            ),
            geometric_encoder: GeometricEncoder::new(),
        }
    }

    pub fn compress_coordinate_sequence(
        &self,
        coords: &[Coordinate3D]
    ) -> Result<Vec<GeometricOperation>, CompressionError> { 
        let clusters = self.pattern_analyzer.spatial_clustering(coords, 3, 2) 
            .map_err(|e| CompressionError::ComponentError(format!("Spatial clustering failed: {}", e)))?;
        
        let optimized_ops = self.operation_optimizer.optimize_operations(coords, &clusters)
            .map_err(|e| CompressionError::ComponentError(format!("Operation optimization failed: {}", e)))?;
        
        let final_ops = self.geometric_encoder.encode_operations(&optimized_ops) 
            .map_err(|e| CompressionError::ComponentError(format!("Geometric encoding (validation) failed: {}", e)))?;

        Ok(final_ops)
    }

    pub fn decompress_operations(
        &self,
        operations: &[GeometricOperation],
        memory_optimizer: &mut MemoryOptimizer 
    ) -> Result<Vec<Coordinate3D>, DecompressionError> {
        let mut current_coordinates = Vec::new();
        let mut last_coord_pooled = memory_optimizer.coordinate_pool.get();
        last_coord_pooled.x = 0; last_coord_pooled.y = 0; last_coord_pooled.z = 0;
        let mut last_coord = last_coord_pooled; 

        for operation in operations {
            match operation {
                GeometricOperation::RegionFill { start, end, .. } => {
                    for x_val in start.x..=end.x {
                        for y_val in start.y..=end.y {
                            for z_val in start.z..=end.z {
                                let mut pooled_coord = memory_optimizer.coordinate_pool.get();
                                pooled_coord.x = x_val;
                                pooled_coord.y = y_val;
                                pooled_coord.z = z_val;
                                current_coordinates.push(pooled_coord);
                            }
                        }
                    }
                    if let Some(lc) = current_coordinates.last() {
                        last_coord = *lc; 
                    }
                }
                GeometricOperation::PathTrace { waypoints, .. } => {
                    current_coordinates.extend(waypoints.iter().cloned());
                    if let Some(lc) = waypoints.last() {
                        last_coord = *lc;
                    }
                }
                GeometricOperation::PatternReference { pattern_id, .. } => {
                     return Err(DecompressionError::UnsupportedOperation(format!("PatternReference with ID {} not supported - pattern storage not implemented", pattern_id)));
                }
                GeometricOperation::SparseMapping { coordinate_deltas, .. } => {
                    let mut current_point_val = last_coord; 
                    for delta in coordinate_deltas {
                        let new_x_i32 = current_point_val.x as i32 + delta.dx as i32;
                        let new_y_i32 = current_point_val.y as i32 + delta.dy as i32;
                        let new_z_i32 = current_point_val.z as i32 + delta.dz as i32;

                        if new_x_i32 < 0 || new_x_i32 > 255 || new_y_i32 < 0 || new_y_i32 > 255 || new_z_i32 < 0 || new_z_i32 > 255 {
                            return Err(DecompressionError::CoordinateOverflow);
                        }
                        
                        let mut pooled_coord = memory_optimizer.coordinate_pool.get();
                        pooled_coord.x = new_x_i32 as u8;
                        pooled_coord.y = new_y_i32 as u8;
                        pooled_coord.z = new_z_i32 as u8;
                        
                        current_coordinates.push(pooled_coord);
                        current_point_val = pooled_coord; 
                    }
                    if let Some(lc) = current_coordinates.last() {
                        last_coord = *lc;
                    }
                }
                GeometricOperation::FunctionGeneration { .. } => {
                    return Err(DecompressionError::UnsupportedOperation("FunctionGeneration not fully implemented".to_string()));
                }
            }
        }
        Ok(current_coordinates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coordinates::Coordinate3D;
    use crate::core::memory_optimizer::MemoryOptimizer; 
    use std::num::NonZeroUsize; 
    #[cfg(feature = "parallel")]
    use crate::core::parallel_processor::ParallelConfig;

    fn new_test_engine() -> CompressionEngine {
        #[cfg(feature = "parallel")]
        return CompressionEngine::new(false, None);
        #[cfg(not(feature = "parallel"))]
        return CompressionEngine::new();
    }

    #[test]
    fn test_compression_engine_new() {
        let _engine = new_test_engine();
    }

    #[test]
    fn test_compress_coordinate_sequence_sync_flow() {
        let engine = new_test_engine();
        
        let empty_coords: Vec<Coordinate3D> = Vec::new();
        let result_empty = engine.compress_coordinate_sequence(&empty_coords);
        assert!(result_empty.is_ok());
        assert!(result_empty.unwrap().is_empty());
        
        let patterned_coords = vec![
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), Coordinate3D::new(4,0,0),
            Coordinate3D::new(1,0,0), Coordinate3D::new(2,0,0), Coordinate3D::new(3,0,0), Coordinate3D::new(4,0,0),
        ];
        let result_patterned = engine.compress_coordinate_sequence(&patterned_coords);
        assert!(result_patterned.is_ok(), "Patterned input: {:?}", result_patterned.err());
        let ops = result_patterned.unwrap();
        println!("Generated operations: {:?}", ops);
        assert_eq!(ops.len(), 2); 
        match &ops[0] {
            GeometricOperation::PatternReference { base_coordinate, pattern_id, .. } => {
                assert_eq!(*base_coordinate, Coordinate3D::new(1,0,0));
                assert_eq!(*pattern_id, 0);
            }
            _ => panic!("Expected PatternReference for patterned input"),
        }
    }

    #[test]
    fn test_decompress_operations_stub() { 
        let engine = new_test_engine();
        let mut mem_opt = MemoryOptimizer::new(NonZeroUsize::new(1).unwrap(),10,10);
        let ops: Vec<GeometricOperation> = vec![];
        let decompressed = engine.decompress_operations(&ops, &mut mem_opt).unwrap();
        assert!(decompressed.is_empty());
    }
}

#[cfg(test)]
mod tests_decompression {
    use super::*; 
    use crate::core::coordinates::Coordinate3D;
    use crate::compression::operations::{GeometricOperation, InterpolationType, EncodingScheme, Matrix3x3, CoordinateDelta, CompressedValues, MathFunction, TrigFuncType, CoordinateRegion};
    use crate::core::memory_optimizer::MemoryOptimizer;
    use std::num::NonZeroUsize;
    #[cfg(feature = "parallel")]
    use crate::core::parallel_processor::ParallelConfig;

    fn new_test_engine_for_decompression() -> CompressionEngine {
        #[cfg(feature = "parallel")]
        return CompressionEngine::new(false, None);
        #[cfg(not(feature = "parallel"))]
        return CompressionEngine::new();
    }

    #[test]
    fn decompress_empty_operations() {
        let engine = new_test_engine_for_decompression();
        let mut mem_opt = MemoryOptimizer::new(NonZeroUsize::new(1).unwrap(), 10, 10);
        let ops = Vec::new();
        let result = engine.decompress_operations(&ops, &mut mem_opt);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn decompress_region_fill() {
        let engine = new_test_engine_for_decompression();
        let mut mem_opt = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 256, 10);
        let start_coord = Coordinate3D::new(1, 1, 1);
        let end_coord = Coordinate3D::new(1, 1, 2);
        let ops = vec![GeometricOperation::RegionFill {
            start: start_coord, end: end_coord, fill_byte: 0, compression_ratio: 0.0,
        }];
        let result = engine.decompress_operations(&ops, &mut mem_opt);
        assert!(result.is_ok());
        let coords = result.unwrap();
        let expected_coords = vec![Coordinate3D::new(1,1,1), Coordinate3D::new(1,1,2)];
        assert_eq!(coords.len(), 2);
        assert_eq!(coords, expected_coords);
    }
    
    #[test]
    fn decompress_region_fill_larger() {
        let engine = new_test_engine_for_decompression();
        let mut mem_opt = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 256, 10);
        let start_coord = Coordinate3D::new(10, 20, 30);
        let end_coord = Coordinate3D::new(10, 20, 30);
        let ops = vec![GeometricOperation::RegionFill {
            start: start_coord, end: end_coord, fill_byte: 0, compression_ratio: 0.0,
        }];
        let result = engine.decompress_operations(&ops, &mut mem_opt);
        assert!(result.is_ok());
        let coords = result.unwrap();
        assert_eq!(coords.len(), 1);
        assert_eq!(coords[0], start_coord);
    }

    #[test]
    fn decompress_path_trace_no_interpolation() {
        let engine = new_test_engine_for_decompression();
        let mut mem_opt = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 256, 10);
        let waypoints = vec![
            Coordinate3D::new(1, 2, 3), Coordinate3D::new(4, 5, 6), Coordinate3D::new(7, 8, 9),
        ];
        let ops = vec![GeometricOperation::PathTrace {
            waypoints: waypoints.clone(), interpolation: InterpolationType::None, data_encoding: EncodingScheme::Raw,
        }];
        let result = engine.decompress_operations(&ops, &mut mem_opt);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), waypoints);
    }

    #[test]
    fn decompress_pattern_reference_unsupported() {
        let engine = new_test_engine_for_decompression();
        let mut mem_opt = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 256, 10);
        let ops = vec![GeometricOperation::PatternReference {
            base_coordinate: Coordinate3D::new(1, 1, 1), pattern_id: 101, transformation_matrix: Matrix3x3::identity(),
        }];
        let result = engine.decompress_operations(&ops, &mut mem_opt);
        assert!(result.is_err());
        match result.err().unwrap() {
            DecompressionError::UnsupportedOperation(msg) => assert!(msg.contains("PatternReference with ID 101 not supported")),
            _ => panic!("Expected UnsupportedOperation error"),
        }
    }

    #[test]
    fn decompress_sparse_mapping() {
        let engine = new_test_engine_for_decompression();
        let mut mem_opt = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 256, 10);
        let initial_point = Coordinate3D::new(5,5,5);
        let ops = vec![
            GeometricOperation::PathTrace { waypoints: vec![initial_point], interpolation: InterpolationType::None, data_encoding: EncodingScheme::Raw, },
            GeometricOperation::SparseMapping {
                coordinate_deltas: vec![
                    CoordinateDelta { dx: 1, dy: 1, dz: 1 }, 
                    CoordinateDelta { dx: -1, dy: 2, dz: -3 }, 
                ],
                value_encoding: CompressedValues::Bytes(Vec::new()), 
            }
        ];
        let result = engine.decompress_operations(&ops, &mut mem_opt);
        assert!(result.is_ok(), "SparseMapping failed: {:?}", result.err());
        let coords = result.unwrap();
        let expected_coords = vec![
            initial_point, 
            Coordinate3D::new(6, 6, 6),
            Coordinate3D::new(5, 8, 3),
        ];
        assert_eq!(coords, expected_coords);
    }
    
    #[test]
    fn decompress_sparse_mapping_from_origin() {
        let engine = new_test_engine_for_decompression();
        let mut mem_opt = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 256, 10);
        let ops = vec![
            GeometricOperation::SparseMapping {
                coordinate_deltas: vec![
                    CoordinateDelta { dx: 1, dy: 2, dz: 3 }, 
                    CoordinateDelta { dx: 10, dy: 20, dz: 30 }, 
                ],
                value_encoding: CompressedValues::Bytes(Vec::new()),
            }
        ];
        let result = engine.decompress_operations(&ops, &mut mem_opt);
        assert!(result.is_ok(), "SparseMapping from origin: {:?}", result.err());
        let coords = result.unwrap();
        let expected_coords = vec![Coordinate3D::new(1,2,3), Coordinate3D::new(11,22,33)];
        assert_eq!(coords, expected_coords);
    }

    #[test]
    fn decompress_sparse_mapping_overflow() {
        let engine = new_test_engine_for_decompression();
        let mut mem_opt = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 256, 10);
        let initial_ops = vec![GeometricOperation::PathTrace { 
            waypoints: vec![Coordinate3D::new(0, 0, 200)], interpolation: InterpolationType::None, data_encoding: EncodingScheme::Raw,
        }];
        let mut ops = initial_ops;
        ops.push(GeometricOperation::SparseMapping {
            coordinate_deltas: vec![CoordinateDelta { dx: 0, dy: 0, dz: 100 }], 
            value_encoding: CompressedValues::Bytes(Vec::new()),
        });
        let result = engine.decompress_operations(&ops, &mut mem_opt);
        assert!(result.is_err());
        match result.err().unwrap() {
            DecompressionError::CoordinateOverflow => {}
            _ => panic!("Expected CoordinateOverflow error"),
        }
    }
    
    #[test]
    fn decompress_sparse_mapping_underflow() {
        let engine = new_test_engine_for_decompression();
        let mut mem_opt = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 256, 10);
        let initial_ops = vec![GeometricOperation::PathTrace { 
            waypoints: vec![Coordinate3D::new(0, 0, 20)], interpolation: InterpolationType::None, data_encoding: EncodingScheme::Raw,
        }];
        let mut ops = initial_ops;
        ops.push(GeometricOperation::SparseMapping {
            coordinate_deltas: vec![CoordinateDelta { dx: 0, dy: 0, dz: -50 }], 
            value_encoding: CompressedValues::Bytes(Vec::new()),
        });
        let result = engine.decompress_operations(&ops, &mut mem_opt);
        assert!(result.is_err());
        match result.err().unwrap() {
            DecompressionError::CoordinateOverflow => {}
            _ => panic!("Expected CoordinateOverflow error for underflow"),
        }
    }

    #[test]
    fn decompress_function_generation_unsupported() {
        let engine = new_test_engine_for_decompression();
        let mut mem_opt = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 256, 10);
        let ops = vec![GeometricOperation::FunctionGeneration {
            mathematical_function: MathFunction::Polynomial(vec![1.0, 2.0]),
            domain: CoordinateRegion { min_coord: Coordinate3D::new(0,0,0), max_coord: Coordinate3D::new(10,10,10)},
            parameters: Vec::new(),
        }];
        let result = engine.decompress_operations(&ops, &mut mem_opt);
        assert!(result.is_err());
        match result.err().unwrap() {
            DecompressionError::UnsupportedOperation(msg) => assert!(msg.contains("FunctionGeneration not fully implemented")),
            _ => panic!("Expected UnsupportedOperation error"),
        }
    }

    #[test]
    fn decompress_multiple_operations() {
        let engine = new_test_engine_for_decompression();
        let mut mem_opt = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 256, 10);
        let ops = vec![
            GeometricOperation::PathTrace {
                waypoints: vec![Coordinate3D::new(1,1,1), Coordinate3D::new(2,2,2)], 
                interpolation: InterpolationType::None, data_encoding: EncodingScheme::Raw,
            },
            GeometricOperation::RegionFill {
                start: Coordinate3D::new(3,3,3), end: Coordinate3D::new(3,3,4),
                fill_byte: 0, compression_ratio: 0.0,
            },
            GeometricOperation::SparseMapping {
                coordinate_deltas: vec![CoordinateDelta{dx:1,dy:1,dz:1}], 
                value_encoding: CompressedValues::Bytes(Vec::new()),
            }
        ];
        let result = engine.decompress_operations(&ops, &mut mem_opt);
        assert!(result.is_ok());
        let coords = result.unwrap();
        let expected_coords = vec![
            Coordinate3D::new(1,1,1), Coordinate3D::new(2,2,2),
            Coordinate3D::new(3,3,3), Coordinate3D::new(3,3,4),
            Coordinate3D::new(4,4,5),
        ];
        assert_eq!(coords, expected_coords);
    }
}
