use crate::compression::operations::GeometricOperation;
use crate::compression::engine::CompressionError;
// Add bincode for serialization
use bincode; 

#[derive(Debug, Default, Clone)]
pub struct GeometricEncoder;

impl GeometricEncoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Encodes a list of optimized GeometricOperations into a final byte vector.
    pub fn encode_operations(
        &self,
        operations: Vec<GeometricOperation>
    ) -> Result<Vec<u8>, CompressionError> { // Return type changed to Vec<u8>
        if operations.is_empty() {
            return Ok(Vec::new());
        }
        
        bincode::serialize(&operations).map_err(|e| {
            CompressionError::NotImplementedDetails(format!("Bincode serialization failed: {}", e))
            // Or a more specific error variant like SerializationError(String) if added to CompressionError
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compression::operations::{GeometricOperation, InterpolationType, EncodingScheme, Matrix3x3}; // Added Matrix3x3
    use crate::core::coordinates::Coordinate3D;

    #[test]
    fn test_encode_operations_bincode_serialization() { // Renamed test
        let encoder = GeometricEncoder::new();
        
        // Test with empty operations vector
        let empty_ops: Vec<GeometricOperation> = Vec::new();
        let result_empty = encoder.encode_operations(empty_ops.clone());
        assert!(result_empty.is_ok());
        assert!(result_empty.unwrap().is_empty(), "Encoding empty ops should result in empty Vec<u8>");

        // Test with some operations
        let ops: Vec<GeometricOperation> = vec![
            GeometricOperation::PathTrace {
                waypoints: vec![Coordinate3D::new(0,0,0), Coordinate3D::new(1,1,1)],
                interpolation: InterpolationType::Linear,
                data_encoding: EncodingScheme::Raw,
            },
            GeometricOperation::RegionFill {
                start: Coordinate3D::new(10,10,10),
                end: Coordinate3D::new(20,20,20),
                fill_byte: 128, // fill_byte was updated
                compression_ratio: 5.0,
            }
        ];
        let result_with_ops = encoder.encode_operations(ops.clone());
        assert!(result_with_ops.is_ok(), "Encoding operations failed: {:?}", result_with_ops.err());
        let encoded_bytes = result_with_ops.unwrap();
        assert!(!encoded_bytes.is_empty(), "Encoded bytes should not be empty for non-empty operations");

        // Try to deserialize back to confirm format (optional, but good for sanity)
        let deserialized_ops: Vec<GeometricOperation> = bincode::deserialize(&encoded_bytes).expect("Bincode deserialization failed for verification");
        assert_eq!(deserialized_ops, ops, "Deserialized operations do not match original");
    }
}
