use crate::compression::operations::GeometricOperation;
use crate::core::coordinates::Coordinate3D; // For EncodingError
use thiserror::Error; // For deriving Error trait

#[derive(Debug, Error, PartialEq)]
pub enum EncodingError {
    #[error("Invalid RegionFill: start ({start:?}) must be less than or equal to end ({end:?}) on all axes.")]
    InvalidRegionFill { start: Coordinate3D, end: Coordinate3D },
    #[error("PathTrace must contain at least one waypoint.")]
    EmptyPathTrace,
    #[error("SparseMapping must contain at least one delta.")]
    EmptySparseMappingDeltas,
    // #[error("Operation type not supported for validation: {0:?}")]
    // UnsupportedOperationForValidation(GeometricOperation), // Optional
}

#[derive(Debug, Default, Clone)]
pub struct GeometricEncoder;

impl GeometricEncoder {
    pub fn new() -> Self { Default::default() }

    pub fn encode_operations(
        &self,
        operations: &[GeometricOperation] // Changed to slice
    ) -> Result<Vec<GeometricOperation>, EncodingError> { // Return EncodingError
        if operations.is_empty() {
            return Ok(Vec::new());
        }

        for op in operations {
            match op {
                GeometricOperation::RegionFill { start, end, .. } => {
                    if !(start.x <= end.x && start.y <= end.y && start.z <= end.z) {
                        return Err(EncodingError::InvalidRegionFill { start: *start, end: *end });
                    }
                }
                GeometricOperation::PathTrace { waypoints, .. } => {
                    if waypoints.is_empty() {
                        return Err(EncodingError::EmptyPathTrace);
                    }
                }
                GeometricOperation::SparseMapping { coordinate_deltas, .. } => { // Changed from deltas to coordinate_deltas
                    if coordinate_deltas.is_empty() {
                        return Err(EncodingError::EmptySparseMappingDeltas);
                    }
                }
                GeometricOperation::PatternReference { .. } | GeometricOperation::FunctionGeneration { .. } => {
                    // Pass-through for now
                }
            }
        }
        Ok(operations.to_vec()) // Return a copy if valid
    }
}

#[cfg(test)]
mod tests_original { // Renamed original tests module
    use super::*;
    use crate::compression::operations::{GeometricOperation, InterpolationType, EncodingScheme};
    use crate::core::coordinates::Coordinate3D;

    #[test]
    fn test_original_encode_operations_passthrough_valid() { 
        let encoder = GeometricEncoder::new();
        
        let empty_ops: Vec<GeometricOperation> = Vec::new();
        let result_empty = encoder.encode_operations(&empty_ops); // Pass slice
        assert!(result_empty.is_ok());
        assert!(result_empty.unwrap().is_empty());

        let ops_vec: Vec<GeometricOperation> = vec![
            GeometricOperation::PathTrace {
                waypoints: vec![Coordinate3D::new(0,0,0)],
                interpolation: InterpolationType::Linear,
                data_encoding: EncodingScheme::Raw,
            }
        ];
        let result_with_ops = encoder.encode_operations(&ops_vec); // Pass slice
        assert!(result_with_ops.is_ok());
        assert_eq!(result_with_ops.unwrap(), ops_vec); 
    }
}

#[cfg(test)]
mod tests_encoder_validation {
    use super::*;
    use crate::core::coordinates::Coordinate3D;
    use crate::compression::operations::{
        GeometricOperation, InterpolationType, EncodingScheme, Matrix3x3, 
        CoordinateDelta, CompressedValues, MathFunction, CoordinateRegion, TrigFuncType
    };

    #[test]
    fn valid_region_fill() {
        let encoder = GeometricEncoder::new();
        let ops = vec![GeometricOperation::RegionFill {
            start: Coordinate3D::new(1,1,1),
            end: Coordinate3D::new(5,5,5),
            fill_byte: 0,
            compression_ratio: 1.0,
        }];
        assert!(encoder.encode_operations(&ops).is_ok());
    }

    #[test]
    fn invalid_region_fill_x_axis() {
        let encoder = GeometricEncoder::new();
        let start_coord = Coordinate3D::new(6,1,1);
        let end_coord = Coordinate3D::new(5,5,5);
        let ops = vec![GeometricOperation::RegionFill {
            start: start_coord,
            end: end_coord,
            fill_byte: 0,
            compression_ratio: 1.0,
        }];
        let result = encoder.encode_operations(&ops);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), EncodingError::InvalidRegionFill{ start: start_coord, end: end_coord });
    }

    #[test]
    fn valid_path_trace() {
        let encoder = GeometricEncoder::new();
        let ops = vec![GeometricOperation::PathTrace {
            waypoints: vec![Coordinate3D::new(1,1,1), Coordinate3D::new(2,2,2)],
            interpolation: InterpolationType::None,
            data_encoding: EncodingScheme::Raw,
        }];
        assert!(encoder.encode_operations(&ops).is_ok());
    }

    #[test]
    fn invalid_empty_path_trace() {
        let encoder = GeometricEncoder::new();
        let ops = vec![GeometricOperation::PathTrace {
            waypoints: vec![],
            interpolation: InterpolationType::None,
            data_encoding: EncodingScheme::Raw,
        }];
        let result = encoder.encode_operations(&ops);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), EncodingError::EmptyPathTrace);
    }

    #[test]
    fn valid_sparse_mapping() {
        let encoder = GeometricEncoder::new();
        let ops = vec![GeometricOperation::SparseMapping {
            coordinate_deltas: vec![CoordinateDelta { dx:1, dy:1, dz:1 }],
            value_encoding: CompressedValues::Bytes(vec![0]),
        }];
        assert!(encoder.encode_operations(&ops).is_ok());
    }

    #[test]
    fn invalid_empty_sparse_mapping_deltas() {
        let encoder = GeometricEncoder::new();
        let ops = vec![GeometricOperation::SparseMapping {
            coordinate_deltas: vec![],
            value_encoding: CompressedValues::Bytes(vec![0]),
        }];
        let result = encoder.encode_operations(&ops);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), EncodingError::EmptySparseMappingDeltas);
    }

    #[test]
    fn valid_pattern_reference_passes() {
        let encoder = GeometricEncoder::new();
        let ops = vec![GeometricOperation::PatternReference {
            base_coordinate: Coordinate3D::new(0,0,0),
            pattern_id: 1,
            transformation_matrix: Matrix3x3::identity(),
        }];
        assert!(encoder.encode_operations(&ops).is_ok());
    }

    #[test]
    fn valid_function_generation_passes() {
        let encoder = GeometricEncoder::new();
        let ops = vec![GeometricOperation::FunctionGeneration {
            mathematical_function: MathFunction::Polynomial(vec![1.0, 0.0]),
            domain: CoordinateRegion { min_coord: Coordinate3D::new(0,0,0), max_coord: Coordinate3D::new(10,10,10)},
            parameters: vec![],
        }];
        assert!(encoder.encode_operations(&ops).is_ok());
    }

    #[test]
    fn mixed_operations_first_invalid() {
        let encoder = GeometricEncoder::new();
        let start_coord_invalid = Coordinate3D::new(6,1,1);
        let end_coord_invalid = Coordinate3D::new(5,5,5);
        let ops = vec![
            GeometricOperation::RegionFill { // Invalid
                start: start_coord_invalid,
                end: end_coord_invalid,
                fill_byte: 0,
                compression_ratio: 1.0,
            },
            GeometricOperation::PathTrace { // Valid
                waypoints: vec![Coordinate3D::new(1,1,1)],
                interpolation: InterpolationType::None,
                data_encoding: EncodingScheme::Raw,
            }
        ];
        let result = encoder.encode_operations(&ops);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), EncodingError::InvalidRegionFill{ start: start_coord_invalid, end: end_coord_invalid });
    }

     #[test]
    fn mixed_operations_second_invalid() {
        let encoder = GeometricEncoder::new();
        let ops = vec![
            GeometricOperation::PathTrace { // Valid
                waypoints: vec![Coordinate3D::new(1,1,1)],
                interpolation: InterpolationType::None,
                data_encoding: EncodingScheme::Raw,
            },
            GeometricOperation::PathTrace { // Invalid
                waypoints: vec![],
                interpolation: InterpolationType::None,
                data_encoding: EncodingScheme::Raw,
            }
        ];
        let result = encoder.encode_operations(&ops);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), EncodingError::EmptyPathTrace);
    }

    #[test]
    fn all_valid_mixed_operations() {
         let encoder = GeometricEncoder::new();
        let ops = vec![
            GeometricOperation::RegionFill { 
                start: Coordinate3D::new(0,0,0),
                end: Coordinate3D::new(1,1,1),
                fill_byte: 0,
                compression_ratio: 1.0,
            },
            GeometricOperation::PathTrace {
                waypoints: vec![Coordinate3D::new(1,1,1)],
                interpolation: InterpolationType::None,
                data_encoding: EncodingScheme::Raw,
            },
            GeometricOperation::SparseMapping {
                coordinate_deltas: vec![CoordinateDelta{dx:1, dy:0, dz:-1}],
                value_encoding: CompressedValues::Rle(vec![(1,1)])
            }
        ];
        let result = encoder.encode_operations(&ops);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ops);
    }
}
