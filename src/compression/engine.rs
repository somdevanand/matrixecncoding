use crate::core::coordinates::Coordinate3D;
use super::operations::GeometricOperation; // Use super:: to access sibling module
use super::pattern_analyzer::PatternAnalyzer;
use super::optimizer::OperationOptimizer;
use super::encoding::GeometricEncoder;
// Remove placeholder struct definitions for PatternAnalyzer, etc. if they were here.

#[derive(Debug, thiserror::Error)]
pub enum CompressionError {
    #[error("Operation not implemented yet: {0}")]
    NotImplementedDetails(String), // Added a variant to provide more context

    #[error("An underlying component failed: {0}")]
    ComponentError(String), // For errors from sub-components

    #[error("No solution found by optimizer")]
    OptimizationFailed,
    
    #[error("NotImplemented")] // Keep the generic one if preferred for stubs
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

#[derive(Debug, Clone)] // Added Clone
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

    /// Multi-pass compression analysis (now fully synchronous)
    pub fn compress_coordinate_sequence(
        &self,
        coords: &[Coordinate3D]
    ) -> Result<Vec<GeometricOperation>, CompressionError> { // Return type Vec<GeometricOperation>
        let min_window_size = 4;
        let max_window_size = 32;
        
        let patterns = self.pattern_analyzer.find_patterns(coords, min_window_size, max_window_size)
            .map_err(|e| CompressionError::ComponentError(format!("Pattern analysis (find_patterns) failed: {}", e)))?;
        
        // Pass example values for epsilon_sq and min_points
        let clusters = self.pattern_analyzer.spatial_clustering(coords, 3, 2) 
            .map_err(|e| CompressionError::ComponentError(format!("Spatial clustering failed: {}", e)))?;
        
        let optimized_ops = self.operation_optimizer.optimize_operations(coords, &patterns, &clusters)
            .map_err(|e| CompressionError::ComponentError(format!("Operation optimization failed: {}", e)))?;
        
        // Call to geometric_encoder.encode_operations now returns Vec<GeometricOperation>
        // Pass optimized_ops as a slice by borrowing
        let final_ops = self.geometric_encoder.encode_operations(&optimized_ops) 
            .map_err(|e| CompressionError::ComponentError(format!("Geometric encoding (validation) failed: {}", e)))?;

        Ok(final_ops) // Return the Vec<GeometricOperation>
    }

    pub fn decompress_operations(
        &self,
        operations: &[GeometricOperation]
    ) -> Result<Vec<Coordinate3D>, DecompressionError> {
        let mut current_coordinates = Vec::new();
        let mut last_coord = Coordinate3D::new(0,0,0); // For delta-based operations

        for operation in operations {
            match operation {
                GeometricOperation::RegionFill { start, end, .. } => {
                    // Assuming RegionFill means all integer coordinates in the bounding box.
                    // This could generate a lot of coordinates.
                    // For simplicity, just adding start and end. A full fill is more complex.
                    // A true region fill would iterate x from start.x to end.x, etc.
                    // and create all Coordinate3D points in between.
                    // current_coordinates.push(*start);
                    // current_coordinates.push(*end);
                    for x in start.x..=end.x {
                        for y in start.y..=end.y {
                            for z in start.z..=end.z {
                                current_coordinates.push(Coordinate3D::new(x,y,z));
                            }
                        }
                    }
                    if !current_coordinates.is_empty() {
                        last_coord = *current_coordinates.last().unwrap();
                    }
                }
                GeometricOperation::PathTrace { waypoints, .. } => {
                    // Decompression currently just uses the waypoints directly.
                    // Interpolation would require more logic and parameters (e.g., number of points).
                    current_coordinates.extend(waypoints.iter().cloned());
                    if !waypoints.is_empty() {
                        last_coord = *waypoints.last().unwrap();
                    }
                }
                GeometricOperation::PatternReference { base_coordinate, pattern_id, transformation_matrix } => {
                    // This is complex. The engine doesn't store patterns.
                    // For now, we can return the base_coordinate or an error.
                    // Returning an error indicating that pattern lookup is not implemented.
                    // In a real scenario, one would look up pattern_id, get its coordinates,
                    // apply transformation_matrix, then translate by base_coordinate.
                     return Err(DecompressionError::UnsupportedOperation(format!("PatternReference with ID {} not supported - pattern storage not implemented", pattern_id)));
                    // As a placeholder, let's just add the base coordinate if we weren't erroring:
                    // current_coordinates.push(*base_coordinate);
                    // last_coord = *base_coordinate;
                }
                GeometricOperation::SparseMapping { coordinate_deltas, .. } => {
                    // Deltas are applied sequentially, starting from the last known coordinate.
                    let mut current_point = last_coord;
                    for delta in coordinate_deltas {
                        // Need to handle potential overflow/underflow if Coordinate3D had different types
                        // For u8, wrapping_add and wrapping_sub are used in Coordinate3D methods.
                        // Here, we construct a new Coordinate3D from i8 deltas.
                        let new_x = current_point.x.wrapping_add(delta.dx as u8); // Potential issue if dx is negative
                        let new_y = current_point.y.wrapping_add(delta.dy as u8); // Potential issue if dy is negative
                        let new_z = current_point.z.wrapping_add(delta.dz as u8);
                        // A more robust way for i8 deltas:
                        let new_x_i32 = current_point.x as i32 + delta.dx as i32;
                        let new_y_i32 = current_point.y as i32 + delta.dy as i32;
                        let new_z_i32 = current_point.z as i32 + delta.dz as i32;

                        if new_x_i32 < 0 || new_x_i32 > 255 || new_y_i32 < 0 || new_y_i32 > 255 || new_z_i32 < 0 || new_z_i32 > 255 {
                            return Err(DecompressionError::CoordinateOverflow);
                        }
                        current_point = Coordinate3D::new(new_x_i32 as u8, new_y_i32 as u8, new_z_i32 as u8);
                        current_coordinates.push(current_point);
                    }
                    if !current_coordinates.is_empty() {
                        last_coord = *current_coordinates.last().unwrap();
                    }
                }
                GeometricOperation::FunctionGeneration { mathematical_function, domain, parameters } => {
                    // This is also complex. Requires evaluating the function.
                    // For now, returning an error or a few sample points.
                    // Example: just return domain min and max
                    // current_coordinates.push(domain.min_coord);
                    // current_coordinates.push(domain.max_coord);
                    // last_coord = domain.max_coord;
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
    use crate::compression::operations::{GeometricOperation, InterpolationType, EncodingScheme, Matrix3x3, CoordinateDelta, CompressedValues, MathFunction, TrigFuncType, CoordinateRegion};

    #[test]
    fn test_compression_engine_new() {
        let _engine = CompressionEngine::new();
        // Basic check
    }

    #[test]
    fn test_compress_coordinate_sequence_sync_flow() {
        let engine = CompressionEngine::new();
        
        // Scenario 1: Empty input coordinates
        let empty_coords: Vec<Coordinate3D> = Vec::new();
        let result_empty = engine.compress_coordinate_sequence(&empty_coords);
        assert!(result_empty.is_ok(), "Empty input should result in Ok");
        assert!(result_empty.unwrap().is_empty(), "Expected empty operations for empty input");
        
        // Scenario 3: Coords that form a pattern
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
    fn test_decompress_operations_stub() {
        let engine = CompressionEngine::new();
        let ops: Vec<GeometricOperation> = vec![];
        let decompressed = engine.decompress_operations(&ops).unwrap();
        assert!(decompressed.is_empty());
        
        // Previous stub test for non-empty (now more specific tests will be added)
        // let non_empty_ops = vec![GeometricOperation::RegionFill{start:Default::default(), end:Default::default(), fill_byte:0, compression_ratio:0.0}];
        // assert!(!engine.decompress_operations(&non_empty_ops).unwrap().is_empty());
    }
}

// New test module for decompression
#[cfg(test)]
mod tests_decompression {
    use super::*; // Brings in CompressionEngine, DecompressionError
    use crate::core::coordinates::Coordinate3D;
    use crate::compression::operations::{GeometricOperation, InterpolationType, EncodingScheme, Matrix3x3, CoordinateDelta, CompressedValues, MathFunction, TrigFuncType, CoordinateRegion};

    #[test]
    fn decompress_empty_operations() {
        let engine = CompressionEngine::new();
        let ops = Vec::new();
        let result = engine.decompress_operations(&ops);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn decompress_region_fill() {
        let engine = CompressionEngine::new();
        let start_coord = Coordinate3D::new(1, 1, 1);
        let end_coord = Coordinate3D::new(1, 1, 2); // Small region for testing
        let ops = vec![GeometricOperation::RegionFill {
            start: start_coord,
            end: end_coord,
            fill_byte: 0,
            compression_ratio: 0.0,
        }];
        let result = engine.decompress_operations(&ops);
        assert!(result.is_ok());
        let coords = result.unwrap();
        // Expected: (1,1,1), (1,1,2)
        let expected_coords = vec![
            Coordinate3D::new(1,1,1),
            Coordinate3D::new(1,1,2),
        ];
        assert_eq!(coords.len(), 2);
        assert_eq!(coords, expected_coords);
    }
    
    #[test]
    fn decompress_region_fill_larger() {
        let engine = CompressionEngine::new();
        let start_coord = Coordinate3D::new(10, 20, 30);
        let end_coord = Coordinate3D::new(10, 20, 30); // Single point region
        let ops = vec![GeometricOperation::RegionFill {
            start: start_coord,
            end: end_coord,
            fill_byte: 0,
            compression_ratio: 0.0,
        }];
        let result = engine.decompress_operations(&ops);
        assert!(result.is_ok());
        let coords = result.unwrap();
        assert_eq!(coords.len(), 1);
        assert_eq!(coords[0], start_coord);
    }


    #[test]
    fn decompress_path_trace_no_interpolation() {
        let engine = CompressionEngine::new();
        let waypoints = vec![
            Coordinate3D::new(1, 2, 3),
            Coordinate3D::new(4, 5, 6),
            Coordinate3D::new(7, 8, 9),
        ];
        let ops = vec![GeometricOperation::PathTrace {
            waypoints: waypoints.clone(),
            interpolation: InterpolationType::None,
            data_encoding: EncodingScheme::Raw,
        }];
        let result = engine.decompress_operations(&ops);
        assert!(result.is_ok());
        let coords = result.unwrap();
        assert_eq!(coords, waypoints);
    }

    #[test]
    fn decompress_pattern_reference_unsupported() {
        let engine = CompressionEngine::new();
        let ops = vec![GeometricOperation::PatternReference {
            base_coordinate: Coordinate3D::new(1, 1, 1),
            pattern_id: 101,
            transformation_matrix: Matrix3x3::identity(),
        }];
        let result = engine.decompress_operations(&ops);
        assert!(result.is_err());
        match result.err().unwrap() {
            DecompressionError::UnsupportedOperation(msg) => {
                assert!(msg.contains("PatternReference with ID 101 not supported"));
            }
            _ => panic!("Expected UnsupportedOperation error"),
        }
    }

    #[test]
    fn decompress_sparse_mapping() {
        let engine = CompressionEngine::new();
        // Assumes starting point of (0,0,0) if it's the first op, or last_coord from previous.
        // Let's test with a preceding operation to set last_coord.
        let initial_point = Coordinate3D::new(5,5,5);
        let ops = vec![
            GeometricOperation::PathTrace { // To set last_coord
                waypoints: vec![initial_point],
                interpolation: InterpolationType::None,
                data_encoding: EncodingScheme::Raw,
            },
            GeometricOperation::SparseMapping {
                coordinate_deltas: vec![
                    CoordinateDelta { dx: 1, dy: 1, dz: 1 }, // (6,6,6)
                    CoordinateDelta { dx: -1, dy: 2, dz: -3 }, // (5,8,3)
                ],
                value_encoding: CompressedValues::Bytes(Vec::new()), // Not used in coord reconstruction
            }
        ];
        let result = engine.decompress_operations(&ops);
        assert!(result.is_ok(), "SparseMapping decompression failed: {:?}", result.err());
        let coords = result.unwrap();
        let expected_coords = vec![
            initial_point, // From PathTrace
            Coordinate3D::new(6, 6, 6),
            Coordinate3D::new(5, 8, 3),
        ];
        assert_eq!(coords, expected_coords);
    }
    
    #[test]
    fn decompress_sparse_mapping_from_origin() {
        let engine = CompressionEngine::new();
        let ops = vec![
            GeometricOperation::SparseMapping {
                coordinate_deltas: vec![
                    CoordinateDelta { dx: 1, dy: 2, dz: 3 }, // (1,2,3) from (0,0,0)
                    CoordinateDelta { dx: 10, dy: 20, dz: 30 }, // (11,22,33)
                ],
                value_encoding: CompressedValues::Bytes(Vec::new()),
            }
        ];
        let result = engine.decompress_operations(&ops);
        assert!(result.is_ok(), "SparseMapping from origin failed: {:?}", result.err());
        let coords = result.unwrap();
        let expected_coords = vec![
            Coordinate3D::new(1,2,3),
            Coordinate3D::new(11,22,33),
        ];
        assert_eq!(coords, expected_coords);
    }


    #[test]
    fn decompress_sparse_mapping_overflow() {
        let engine = CompressionEngine::new();
        let initial_ops = vec![GeometricOperation::PathTrace { // Set last_coord
            waypoints: vec![Coordinate3D::new(0, 0, 200)],
            interpolation: InterpolationType::None,
            data_encoding: EncodingScheme::Raw,
        }];
        let mut ops = initial_ops;
        ops.push(GeometricOperation::SparseMapping {
            coordinate_deltas: vec![CoordinateDelta { dx: 0, dy: 0, dz: 100 }], // 200 + 100 = 300, overflows u8
            value_encoding: CompressedValues::Bytes(Vec::new()),
        });
        let result = engine.decompress_operations(&ops);
        assert!(result.is_err());
        match result.err().unwrap() {
            DecompressionError::CoordinateOverflow => {} // Expected
            _ => panic!("Expected CoordinateOverflow error"),
        }
    }
    
    #[test]
    fn decompress_sparse_mapping_underflow() {
        let engine = CompressionEngine::new();
        let initial_ops = vec![GeometricOperation::PathTrace { // Set last_coord
            waypoints: vec![Coordinate3D::new(0, 0, 20)],
            interpolation: InterpolationType::None,
            data_encoding: EncodingScheme::Raw,
        }];
        let mut ops = initial_ops;
        ops.push(GeometricOperation::SparseMapping {
            coordinate_deltas: vec![CoordinateDelta { dx: 0, dy: 0, dz: -50 }], // 20 - 50 = -30, underflows u8
            value_encoding: CompressedValues::Bytes(Vec::new()),
        });
        let result = engine.decompress_operations(&ops);
        assert!(result.is_err());
        match result.err().unwrap() {
            DecompressionError::CoordinateOverflow => {} // Expected (CoordinateOverflow is used for both under/over)
            _ => panic!("Expected CoordinateOverflow error due to underflow"),
        }
    }

    #[test]
    fn decompress_function_generation_unsupported() {
        let engine = CompressionEngine::new();
        let ops = vec![GeometricOperation::FunctionGeneration {
            mathematical_function: MathFunction::Polynomial(vec![1.0, 2.0]),
            domain: CoordinateRegion {
                min_coord: Coordinate3D::new(0, 0, 0),
                max_coord: Coordinate3D::new(10, 10, 10),
            },
            parameters: Vec::new(),
        }];
        let result = engine.decompress_operations(&ops);
        assert!(result.is_err());
        match result.err().unwrap() {
            DecompressionError::UnsupportedOperation(msg) => {
                assert!(msg.contains("FunctionGeneration not fully implemented"));
            }
            _ => panic!("Expected UnsupportedOperation error for FunctionGeneration"),
        }
    }

    #[test]
    fn decompress_multiple_operations() {
        let engine = CompressionEngine::new();
        let ops = vec![
            GeometricOperation::PathTrace {
                waypoints: vec![Coordinate3D::new(1, 1, 1), Coordinate3D::new(2, 2, 2)],
                interpolation: InterpolationType::None,
                data_encoding: EncodingScheme::Raw,
            }, // last_coord = (2,2,2)
            GeometricOperation::RegionFill {
                start: Coordinate3D::new(3, 3, 3),
                end: Coordinate3D::new(3, 3, 4),
                fill_byte: 0,
                compression_ratio: 0.0,
            }, // last_coord = (3,3,4)
            GeometricOperation::SparseMapping {
                coordinate_deltas: vec![CoordinateDelta { dx: 1, dy: 1, dz: 1 }], // (3,3,4) + (1,1,1) = (4,4,5)
                value_encoding: CompressedValues::Bytes(Vec::new()),
            }
        ];
        let result = engine.decompress_operations(&ops);
        assert!(result.is_ok());
        let coords = result.unwrap();
        let expected_coords = vec![
            Coordinate3D::new(1, 1, 1),
            Coordinate3D::new(2, 2, 2),
            Coordinate3D::new(3, 3, 3),
            Coordinate3D::new(3, 3, 4),
            Coordinate3D::new(4, 4, 5),
        ];
        assert_eq!(coords, expected_coords);
    }
}
