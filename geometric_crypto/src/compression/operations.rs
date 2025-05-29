use serde::{Serialize, Deserialize};
use crate::core::coordinates::Coordinate3D; // Make sure Coordinate3D is accessible

// Type Aliases and Simple Enums/Structs
pub type PatternId = u32;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)] // Added PartialEq, Eq for enums
pub enum InterpolationType {
    None,
    Linear,
    Bezier,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)] // Added PartialEq, Eq for enums
pub enum EncodingScheme {
    Raw,
    DeltaRLE, // Delta Run-Length Encoding
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)] // PartialEq for f32 comparison needs care
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)] // PartialEq for f32/Vec comparison
pub enum CompressedValues {
    Bytes(Vec<u8>),
    Rle(Vec<(u8, usize)>), // (value, count)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)] // Added PartialEq, Eq for enums
pub enum TrigFuncType {
    Sin,
    Cos,
    Tan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)] // PartialEq for f32/Vec comparison
pub enum MathFunction {
    Polynomial(Vec<f64>), // Coefficients, e.g., p[0] + p[1]*x + p[2]*x^2 ...
    Trigonometric {
        func_type: TrigFuncType,
        amplitude: f64,
        frequency: f64, // angular frequency or spatial frequency
        phase: f64,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoordinateRegion {
    pub min_coord: Coordinate3D,
    pub max_coord: Coordinate3D,
}

// The main GeometricOperation enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)] // PartialEq for f32/Vec comparison
pub enum GeometricOperation {
    RegionFill {
        start: Coordinate3D,
        end: Coordinate3D,
        pattern: PatternId, // Could be a specific byte, or an ID to a more complex pattern
        compression_ratio: f32, // Estimated compression for this op
    },
    PathTrace {
        waypoints: Vec<Coordinate3D>,
        interpolation: InterpolationType,
        data_encoding: EncodingScheme, // How data along the path is represented
    },
    PatternReference {
        base_coordinate: Coordinate3D,
        pattern_id: PatternId, // ID of a pre-defined or discovered pattern
        transformation_matrix: Matrix3x3, // To orient/scale the referenced pattern
    },
    SparseMapping {
        // Represents a set of coordinates with associated values, efficiently.
        // Deltas could be from a common reference point or chained.
        coordinate_deltas: Vec<CoordinateDelta>, // Differences from a base or previous coord
        value_encoding: CompressedValues, // How the values at these sparse points are stored
    },
    FunctionGeneration {
        mathematical_function: MathFunction, // Defines the function (e.g., polynomial, trigonometric)
        domain: CoordinateRegion,           // The region in space where this function applies
        parameters: Vec<f64>,               // Additional parameters if not in MathFunction itself
    },
}

// Basic tests to ensure definitions compile and serde works (optional for this step, but good practice)
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

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
        // Note: direct string comparison for f32 can be tricky. Deserialize and check values.
        let mat_deser: Matrix3x3 = serde_json::from_str(&json).unwrap();
         assert_eq!(mat.0, mat_deser.0, "Matrix deserialization mismatch");
    }
    
    #[test]
    fn test_geometric_operation_regionfill_serde() {
        let op = GeometricOperation::RegionFill {
            start: Coordinate3D::new(0,0,0),
            end: Coordinate3D::new(10,10,10),
            pattern: 1,
            compression_ratio: 2.5,
        };
        let json = serde_json::to_string(&op).unwrap();
        let op_deser: GeometricOperation = serde_json::from_str(&json).unwrap();
        assert_eq!(op, op_deser);
    }
}
