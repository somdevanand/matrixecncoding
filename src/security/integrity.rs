use crate::compression::operations::GeometricOperation;
use super::{SecurityError, IntegrityProof}; // Use types from security/mod.rs
use bincode; // For serializing operations to hash them
use blake3; // For hashing

#[derive(Debug, Default, Clone)]
pub struct IntegrityVerifier;

impl IntegrityVerifier {
    pub fn new() -> Self {
        Default::default()
    }

    /// Generates an integrity proof (hash) for a sequence of geometric operations.
    pub fn generate_integrity_proof(
        &self,
        operations: &[GeometricOperation]
    ) -> Result<IntegrityProof, SecurityError> {
        if operations.is_empty() {
            // Decide behavior for empty operations: empty proof or error?
            // For now, an empty proof (hash of empty bytes).
            let hash = blake3::hash(&[]);
            return Ok(IntegrityProof(hash.as_bytes().to_vec()));
        }

        // Serialize the operations to bytes for hashing
        // Note: Serialization must be canonical/deterministic for hashes to match.
        // bincode is generally deterministic if field order in structs is consistent
        // and map iteration order (if any) doesn't affect serialization output for hashing.
        // For Vec<GeometricOperation>, bincode should be fine.
        let serialized_ops = bincode::serialize(operations).map_err(|e| {
            SecurityError::NotImplemented(format!("Failed to serialize operations for integrity proof: {}", e))
            // Consider a more specific error variant like SerializationForProofFailed
        })?;
        
        let hash = blake3::hash(&serialized_ops);
        Ok(IntegrityProof(hash.as_bytes().to_vec()))
    }

    /// Verifies an integrity proof against a sequence of geometric operations.
    pub fn verify_integrity_proof(
        &self,
        operations: &[GeometricOperation],
        proof: &IntegrityProof
    ) -> Result<bool, SecurityError> {
        let calculated_proof = self.generate_integrity_proof(operations)?;
        Ok(calculated_proof == *proof) // Relies on PartialEq for IntegrityProof
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coordinates::Coordinate3D;
    use crate::compression::operations::{InterpolationType, EncodingScheme}; // For test data

    #[test]
    fn test_generate_and_verify_integrity_proof() {
        let verifier = IntegrityVerifier::new();
        let ops: Vec<GeometricOperation> = vec![
            GeometricOperation::PathTrace {
                waypoints: vec![Coordinate3D::new(0,0,0), Coordinate3D::new(1,1,1)],
                interpolation: InterpolationType::Linear,
                data_encoding: EncodingScheme::Raw,
            },
            GeometricOperation::RegionFill {
                start: Coordinate3D::new(10,10,10),
                end: Coordinate3D::new(20,20,20),
                fill_byte: 128,
                compression_ratio: 5.0,
            }
        ];

        let proof_result = verifier.generate_integrity_proof(&ops);
        assert!(proof_result.is_ok());
        let proof = proof_result.unwrap();
        assert!(!proof.0.is_empty(), "Proof bytes should not be empty for non-empty ops");

        // Verify success
        let verification_result = verifier.verify_integrity_proof(&ops, &proof);
        assert!(verification_result.is_ok());
        assert!(verification_result.unwrap(), "Integrity proof verification should succeed for original ops");

        // Verify failure with modified ops
        let mut modified_ops = ops.clone();
        if let Some(GeometricOperation::RegionFill { ref mut fill_byte, .. }) = modified_ops.get_mut(1) {
            *fill_byte = 100; // Change a byte
        } else {
            panic!("Could not get mutable access to modify operation for test");
        }
        
        let verification_fail_result = verifier.verify_integrity_proof(&modified_ops, &proof);
        assert!(verification_fail_result.is_ok()); // The verification itself runs ok
        assert!(!verification_fail_result.unwrap(), "Integrity proof verification should fail for modified ops");
    }

    #[test]
    fn test_integrity_proof_empty_ops() {
        let verifier = IntegrityVerifier::new();
        let empty_ops: Vec<GeometricOperation> = Vec::new();
        
        let proof_result = verifier.generate_integrity_proof(&empty_ops);
        assert!(proof_result.is_ok());
        let proof1 = proof_result.unwrap();

        let proof2_result = verifier.generate_integrity_proof(&empty_ops);
        assert!(proof2_result.is_ok());
        let proof2 = proof2_result.unwrap();

        assert_eq!(proof1, proof2, "Proof for empty ops should be deterministic");
        
        let verification_result = verifier.verify_integrity_proof(&empty_ops, &proof1);
        assert!(verification_result.is_ok());
        assert!(verification_result.unwrap(), "Verification of empty ops proof should succeed");
    }
}
