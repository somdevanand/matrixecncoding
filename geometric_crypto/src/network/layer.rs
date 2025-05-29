use super::NetworkError;
use crate::security::IntegrityProof;
use crate::compression::operations::GeometricOperation; // Added
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Read}; // Added std::io::Read
use bincode; // Added for internal serialization

#[derive(Debug, Default, Clone)]
pub struct NetworkLayer;

impl NetworkLayer {
    pub fn new() -> Self { Default::default() }

    pub fn package_data(
        &self,
        version: u16,
        operations: &[GeometricOperation], // Changed from operations_bytes: &[u8]
        proof: Option<&IntegrityProof>,
    ) -> Result<Vec<u8>, NetworkError> {
        let mut packaged_data = Vec::new();
        packaged_data.write_u16::<BigEndian>(version).map_err(|e| NetworkError::SerializationError(format!("Ver: {}", e)))?;
        
        let serialized_ops = bincode::serialize(operations).map_err(|e| NetworkError::SerializationError(format!("Ops: {}", e)))?;

        if let Some(p) = proof {
            if p.0.len() > u16::MAX as usize { return Err(NetworkError::SerializationError("Proof too large".to_string())); }
            packaged_data.write_u16::<BigEndian>(p.0.len() as u16).map_err(|e| NetworkError::SerializationError(format!("ProofLen: {}", e)))?;
            packaged_data.extend_from_slice(&p.0);
        } else {
            packaged_data.write_u16::<BigEndian>(0).map_err(|e| NetworkError::SerializationError(format!("ZeroProofLen: {}", e)))?;
        }
        packaged_data.extend_from_slice(&serialized_ops); // Append serialized operations
        Ok(packaged_data)
    }

    pub fn unpackage_data(
        &self,
        payload: &[u8],
    ) -> Result<(u16, Vec<GeometricOperation>, Option<IntegrityProof>), NetworkError> { // Return Vec<GeometricOperation>
        if payload.len() < 4 { return Err(NetworkError::InvalidFormat("Payload too short for header".to_string())); }
        let mut cursor = Cursor::new(payload);
        let version = cursor.read_u16::<BigEndian>().map_err(|e| NetworkError::DeserializationError(format!("Ver: {}", e)))?;
        let proof_len = cursor.read_u16::<BigEndian>().map_err(|e| NetworkError::DeserializationError(format!("ProofLen: {}", e)))? as usize;

        let proof = if proof_len > 0 {
            if cursor.position() as usize + proof_len > payload.len() { return Err(NetworkError::InvalidFormat("Payload too short for proof".to_string())); }
            let mut proof_bytes = vec![0u8; proof_len];
            cursor.read_exact(&mut proof_bytes).map_err(|e| NetworkError::DeserializationError(format!("ProofData: {}", e)))?;
            Some(IntegrityProof(proof_bytes))
        } else { None };

        let ops_bytes_start = cursor.position() as usize;
        let operations: Vec<GeometricOperation> = bincode::deserialize(&payload[ops_bytes_start..]).map_err(|e| NetworkError::DeserializationError(format!("Ops: {}", e)))?;
        
        Ok((version, operations, proof))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::IntegrityProof; 
    use crate::core::coordinates::Coordinate3D; // For sample ops
    use crate::compression::operations::{GeometricOperation, InterpolationType, EncodingScheme}; // For sample ops

    #[test]
    fn test_package_unpackage_data_no_proof_ops() { // Renamed
        let layer = NetworkLayer::new();
        let version = 1u16;
        let ops = vec![GeometricOperation::RegionFill{start:Coordinate3D::default(),end:Coordinate3D::default(),fill_byte:0,compression_ratio:0.0}]; // Sample ops
        let serialized_ops_len = bincode::serialize(&ops).unwrap().len();

        let packaged = layer.package_data(version, &ops, None).unwrap();
        assert_eq!(packaged.len(), 2 + 2 + serialized_ops_len);

        let (p_ver, p_ops, p_proof) = layer.unpackage_data(&packaged).unwrap();
        assert_eq!(p_ver, version);
        assert_eq!(p_ops, ops); // Compare Vec<GeometricOperation>
        assert!(p_proof.is_none());
    }
    #[test]
    fn test_package_unpackage_data_with_proof_ops() { // Renamed
        let layer = NetworkLayer::new();
        let version = 2u16;
        let ops = vec![GeometricOperation::PathTrace{waypoints:vec![Coordinate3D::new(1,1,1)], interpolation:InterpolationType::Linear, data_encoding:EncodingScheme::Raw}];
        let proof_data = IntegrityProof(vec![1,2,3,4]);
        let serialized_ops_len = bincode::serialize(&ops).unwrap().len();
        
        let packaged = layer.package_data(version, &ops, Some(&proof_data)).unwrap();
        assert_eq!(packaged.len(), 2 + 2 + proof_data.0.len() + serialized_ops_len);

        let (p_ver, p_ops, p_proof_opt) = layer.unpackage_data(&packaged).unwrap();
        assert_eq!(p_ver, version);
        assert_eq!(p_ops, ops);
        assert_eq!(p_proof_opt.unwrap(), proof_data);
    }
    
    #[test]
    fn test_unpackage_data_too_short() {
        let layer = NetworkLayer::new();
        let short_payload = vec![0,1]; 
        let result = layer.unpackage_data(&short_payload);
        assert!(result.is_err());
        match result.err().unwrap() {
            NetworkError::InvalidFormat(msg) => assert!(msg.contains("Payload too short for header")),
            e => panic!("Unexpected error type: {:?}", e),
        }

        let short_payload_for_proof = vec![0,1,0,5, 1,2]; 
        let result2 = layer.unpackage_data(&short_payload_for_proof);
        assert!(result2.is_err());
        match result2.err().unwrap() {
            NetworkError::InvalidFormat(msg) => assert!(msg.contains("Payload too short for proof")),
            e => panic!("Unexpected error type: {:?}", e),
        }
    }
    #[test]
    fn test_package_data_proof_too_large() {
        let layer = NetworkLayer::new();
        let version = 1u16;
        let ops = vec![GeometricOperation::RegionFill{start:Coordinate3D::default(),end:Coordinate3D::default(),fill_byte:0,compression_ratio:0.0}];
        let large_proof_vec = vec![0u8; (u16::MAX as usize) + 1];
        let large_proof = IntegrityProof(large_proof_vec);

        let result = layer.package_data(version, &ops, Some(&large_proof));
        assert!(result.is_err());
        match result.err().unwrap() {
            NetworkError::SerializationError(msg) => assert!(msg.contains("Proof too large")),
            e => panic!("Unexpected error type: {:?}", e),
        }
    }
}
