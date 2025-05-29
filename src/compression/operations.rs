use serde::{Serialize, Deserialize};
use crate::core::coordinates::Coordinate3D;

// ... (PatternId, InterpolationType, EncodingScheme, Matrix3x3, CoordinateDelta, CompressedValues, TrigFuncType, MathFunction, CoordinateRegion structs/enums remain the same) ...
pub type PatternId = u32;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InterpolationType {
    None,
    Linear,
    Bezier,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EncodingScheme {
    Raw,
    DeltaRLE,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Matrix3x3([[f32; 3]; 3]);

impl Matrix3x3 {
    pub fn identity() -> Self {
        Self([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoordinateDelta {
    pub dx: i8,
    pub dy: i8,
    pub dz: i8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompressedValues {
    Bytes(Vec<u8>),
    Rle(Vec<(u8, usize)>),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrigFuncType {
    Sin,
    Cos,
    Tan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MathFunction {
    Polynomial(Vec<f64>),
    Trigonometric {
        func_type: TrigFuncType,
        amplitude: f64,
        frequency: f64,
        phase: f64,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoordinateRegion {
    pub min_coord: Coordinate3D,
    pub max_coord: Coordinate3D,
}


// Modify GeometricOperation enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GeometricOperation {
    RegionFill {
        start: Coordinate3D,
        end: Coordinate3D,
        fill_byte: u8, // Changed from 'pattern: PatternId'
        compression_ratio: f32,
    },
    PathTrace {
        waypoints: Vec<Coordinate3D>,
        interpolation: InterpolationType,
        // For data_encoding: EncodingScheme::Raw implies the data is implicitly
        // represented by the waypoints themselves or a sequence of bytes that
        // these waypoints help to decode (e.g. each waypoint marks a change in byte value).
        // If data were explicitly stored, a field like `data: Vec<u8>` or `data_ref: DataId`
        // might be needed. For now, no change to fields, just noting the interpretation.
        data_encoding: EncodingScheme,
    },
    PatternReference {
        base_coordinate: Coordinate3D,
        pattern_id: PatternId,
        transformation_matrix: Matrix3x3,
    },
    SparseMapping {
        coordinate_deltas: Vec<CoordinateDelta>,
        value_encoding: CompressedValues,
    },
    FunctionGeneration {
        mathematical_function: MathFunction,
        domain: CoordinateRegion,
        parameters: Vec<f64>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use crate::core::coordinates::Coordinate3D; // Ensure Coordinate3D is in scope for tests

    // ... (test_interpolation_type_serde, test_matrix3x3_serde remain the same) ...
    #[test]
    fn test_interpolation_type_serde() {
        let it = InterpolationType::Bezier;
        let json = serde_json::to_string(&it).unwrap();
        let it_deser: InterpolationType = serde_json::from_str(&json).unwrap();
        assert_eq!(it, it_deser);
    }

    #[test]
    fn test_matrix3x3_serde() {
        let mat = Matrix3x3::identity();
        let json = serde_json::to_string(&mat).unwrap();
        let mat_deser: Matrix3x3 = serde_json::from_str(&json).unwrap();
         assert_eq!(mat.0, mat_deser.0, "Matrix deserialization mismatch");
    }

    // Updated test for RegionFill
    #[test]
    fn test_geometric_operation_regionfill_serde_updated() { // Renamed test
        let op = GeometricOperation::RegionFill {
            start: Coordinate3D::new(0,0,0),
            end: Coordinate3D::new(10,10,10),
            fill_byte: 42u8, // Changed to fill_byte
            compression_ratio: 2.5,
        };
        let json = serde_json::to_string(&op).unwrap();
        let op_deser: GeometricOperation = serde_json::from_str(&json).unwrap();
        assert_eq!(op, op_deser); // Relies on PartialEq for GeometricOperation
        
        // Verify the specific field change
        if let GeometricOperation::RegionFill { fill_byte, .. } = op_deser {
            assert_eq!(fill_byte, 42u8);
        } else {
            panic!("Deserialized into wrong variant or unable to extract fill_byte");
        }
    }

    // Add a basic test for PathTrace serde as well
    #[test]
    fn test_geometric_operation_pathtrace_serde() {
        let op = GeometricOperation::PathTrace {
            waypoints: vec![Coordinate3D::new(0,0,0), Coordinate3D::new(1,1,1)],
            interpolation: InterpolationType::Linear,
            data_encoding: EncodingScheme::Raw,
        };
        let json = serde_json::to_string(&op).unwrap();
        let op_deser: GeometricOperation = serde_json::from_str(&json).unwrap();
        assert_eq!(op, op_deser);
    }
}
