use super::NetworkError; // From network/mod.rs
use crate::security::IntegrityProof; // Assuming IntegrityProof is pub from security module
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt}; // For writing u16 version/lengths
use std::io::Cursor;

#[derive(Debug, Default, Clone)]
pub struct NetworkLayer;

impl NetworkLayer {
    pub fn new() -> Self {
        Default::default()
    }

    /// Packages operations bytes with version and an optional integrity proof.
    /// Format: [version_u16_BE][proof_len_u16_BE][optional_proof_bytes][operations_bytes]
    pub fn package_data(
        &self,
        version: u16,
        operations_bytes: &[u8],
        proof: Option<&IntegrityProof>,
    ) -> Result<Vec<u8>, NetworkError> {
        let mut packaged_data = Vec::new();

        // Write version (u16 Big Endian)
        packaged_data.write_u16::<BigEndian>(version).map_err(|e| NetworkError::SerializationError(format!("Failed to write version: {}", e)))?;

        // Write proof length (u16 Big Endian) and proof itself
        if let Some(p) = proof {
            if p.0.len() > u16::MAX as usize {
                return Err(NetworkError::SerializationError("Proof too large for u16 length".to_string()));
            }
            packaged_data.write_u16::<BigEndian>(p.0.len() as u16).map_err(|e| NetworkError::SerializationError(format!("Failed to write proof length: {}", e)))?;
            packaged_data.extend_from_slice(&p.0);
        } else {
            packaged_data.write_u16::<BigEndian>(0).map_err(|e| NetworkError::SerializationError(format!("Failed to write zero proof length: {}", e)))?; // Zero length for no proof
        }

        // Append operations bytes
        packaged_data.extend_from_slice(operations_bytes);

        Ok(packaged_data)
    }

    /// Unpackages data into version, operations bytes, and an optional integrity proof.
    /// Expects format: [version_u16_BE][proof_len_u16_BE][optional_proof_bytes][operations_bytes]
    pub fn unpackage_data(
        &self,
        payload: &[u8],
    ) -> Result<(u16, Vec<u8>, Option<IntegrityProof>), NetworkError> {
        if payload.len() < 4 { // Minimum size for version (2 bytes) + proof_len (2 bytes)
            return Err(NetworkError::InvalidFormat("Payload too short for header".to_string()));
        }

        let mut cursor = Cursor::new(payload);

        // Read version
        let version = cursor.read_u16::<BigEndian>().map_err(|e| NetworkError::DeserializationError(format!("Failed to read version: {}", e)))?;

        // Read proof length
        let proof_len = cursor.read_u16::<BigEndian>().map_err(|e| NetworkError::DeserializationError(format!("Failed to read proof length: {}", e)))? as usize;

        // Read proof if length > 0
        let proof = if proof_len > 0 {
            if cursor.position() as usize + proof_len > payload.len() {
                return Err(NetworkError::InvalidFormat("Payload too short for proof data".to_string()));
            }
            let mut proof_bytes = vec![0u8; proof_len];
            cursor.read_exact(&mut proof_bytes).map_err(|e| NetworkError::DeserializationError(format!("Failed to read proof data: {}", e)))?;
            Some(IntegrityProof(proof_bytes))
        } else {
            None
        };

        // The rest is operations_bytes
        let operations_bytes_start = cursor.position() as usize;
        let operations_bytes = payload[operations_bytes_start..].to_vec();

        Ok((version, operations_bytes, proof))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::IntegrityProof; // Ensure this is accessible

    #[test]
    fn test_package_unpackage_data_no_proof() {
        let layer = NetworkLayer::new();
        let version = 1u16;
        let ops_bytes = vec![1,2,3,4,5];
        
        let packaged_result = layer.package_data(version, &ops_bytes, None);
        assert!(packaged_result.is_ok());
        let packaged = packaged_result.unwrap();

        // Expected: [0,1 (ver)][0,0 (p_len)][1,2,3,4,5 (ops)]
        assert_eq!(packaged.len(), 2 + 2 + ops_bytes.len());

        let unpackaged_result = layer.unpackage_data(&packaged);
        assert!(unpackaged_result.is_ok());
        let (p_ver, p_ops, p_proof) = unpackaged_result.unwrap();

        assert_eq!(p_ver, version);
        assert_eq!(p_ops, ops_bytes);
        assert!(p_proof.is_none());
    }

    #[test]
    fn test_package_unpackage_data_with_proof() {
        let layer = NetworkLayer::new();
        let version = 2u16;
        let ops_bytes = vec![10,20,30];
        let proof_data = IntegrityProof(vec![1,2,3,4,5,6,7,8]);
        
        let packaged = layer.package_data(version, &ops_bytes, Some(&proof_data)).unwrap();
        
        // Expected: [0,2 (ver)][0,8 (p_len)][1..8 (proof)][10,20,30 (ops)]
        assert_eq!(packaged.len(), 2 + 2 + proof_data.0.len() + ops_bytes.len());

        let (p_ver, p_ops, p_proof_opt) = layer.unpackage_data(&packaged).unwrap();
        assert_eq!(p_ver, version);
        assert_eq!(p_ops, ops_bytes);
        assert!(p_proof_opt.is_some());
        assert_eq!(p_proof_opt.unwrap(), proof_data);
    }

    #[test]
    fn test_unpackage_data_too_short() {
        let layer = NetworkLayer::new();
        let short_payload = vec![0,1]; // Only 2 bytes, less than min header
        let result = layer.unpackage_data(&short_payload);
        assert!(result.is_err());
        match result.err().unwrap() {
            NetworkError::InvalidFormat(msg) => assert!(msg.contains("Payload too short for header")),
            e => panic!("Unexpected error type: {:?}", e),
        }

        let short_payload_for_proof = vec![0,1,0,5, 1,2]; // Version, proof_len=5, but only 2 bytes of proof
        let result2 = layer.unpackage_data(&short_payload_for_proof);
        assert!(result2.is_err());
        match result2.err().unwrap() {
            NetworkError::InvalidFormat(msg) => assert!(msg.contains("Payload too short for proof data")),
            e => panic!("Unexpected error type: {:?}", e),
        }
    }
    #[test]
    fn test_package_data_proof_too_large() {
        let layer = NetworkLayer::new();
        let version = 1u16;
        let ops_bytes = vec![1, 2, 3];
        let large_proof_vec = vec![0u8; (u16::MAX as usize) + 1];
        let large_proof = IntegrityProof(large_proof_vec);

        let result = layer.package_data(version, &ops_bytes, Some(&large_proof));
        assert!(result.is_err());
        match result.err().unwrap() {
            NetworkError::SerializationError(msg) => assert!(msg.contains("Proof too large")),
            e => panic!("Unexpected error type: {:?}", e),
        }
    }
}
