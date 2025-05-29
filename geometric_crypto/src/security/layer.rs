use crate::core::coordinates::Coordinate3D;
use crate::compression::operations::GeometricOperation;
use super::obfuscation::CoordinateObfuscator;
use super::integrity::{IntegrityVerifier, IntegrityProof}; // IntegrityProof also from super (security/mod.rs)
use super::{SecurityError, SecurityContext}; // SecurityContext from super (security/mod.rs)

// KeyManager placeholder - can be expanded or moved to its own file later.
#[derive(Debug, Default, Clone)]
pub struct KeyManager {
    // For now, no actual key material managed here.
    // In a real system, this would hold symmetric keys, key derivation functions, etc.
    // It might be initialized with a master key or via a key exchange process.
    _placeholder: (), // To make it a struct
}

impl KeyManager {
    pub fn new() -> Self {
        Default::default()
    }
    // Placeholder for getting an encryption/obfuscation key byte.
    // This would come from the actual managed key material.
    pub fn get_obfuscation_key_byte(&self, _context: &SecurityContext) -> u8 {
        super::obfuscation::DEFAULT_OBFUSCATION_KEY_BYTE // Use the const from obfuscation.rs for now
    }
}


#[derive(Debug, Clone)] // Default is not straightforward with non-Default members if not all are wrapped in Option or have clear defaults.
pub struct SecurityLayer {
    obfuscator: CoordinateObfuscator,
    integrity_checker: IntegrityVerifier,
    key_manager: KeyManager,
    // Future: Could include configuration for security level, specific algorithms chosen, etc.
}

impl SecurityLayer {
    pub fn new() -> Self {
        Self {
            obfuscator: CoordinateObfuscator::new(),
            integrity_checker: IntegrityVerifier::new(),
            key_manager: KeyManager::new(),
        }
    }

    /// Obfuscates all coordinates within a list of geometric operations.
    pub fn obfuscate_geometric_operations(
        &self,
        operations: &mut Vec<GeometricOperation>,
        context: &SecurityContext,
    ) -> Result<(), SecurityError> {
        // In a real system, context might provide specific keys or parameters for obfuscation.
        // For now, CoordinateObfuscator uses its internal defaults or simple logic.
        for op in operations.iter_mut() {
            match op {
                GeometricOperation::RegionFill { ref mut start, ref mut end, .. } => {
                    self.obfuscator.obfuscate_coordinates(&mut [*start, *end], context)?;
                }
                GeometricOperation::PathTrace { ref mut waypoints, .. } => {
                    self.obfuscator.obfuscate_coordinates(waypoints, context)?;
                }
                GeometricOperation::PatternReference { ref mut base_coordinate, .. } => {
                    // Note: The referenced pattern itself is not obfuscated here, only its base instance coordinate.
                    // Obfuscating the pattern definition would be a separate step if patterns are stored globally.
                    self.obfuscator.obfuscate_coordinates(&mut [*base_coordinate], context)?;
                }
                GeometricOperation::SparseMapping { ref mut coordinate_deltas, .. } => {
                    // Deltas are relative; obfuscating them directly might be tricky or change their meaning.
                    // This implies that if SparseMapping refers to an origin point, that origin should be obfuscated.
                    // Or, the values themselves are obfuscated.
                    // For now, skipping delta obfuscation as it requires more design.
                    // Consider what needs to be protected in SparseMapping.
                    // If values are sensitive, they should be encrypted.
                    // If coordinates (derived from deltas) are sensitive, the base point for deltas needs obfuscation.
                    let _ = coordinate_deltas; // Avoid unused warning
                }
                GeometricOperation::FunctionGeneration { ref mut domain, .. } => {
                    // Obfuscate the coordinates defining the domain region
                    self.obfuscator.obfuscate_coordinates(&mut [domain.min_coord, domain.max_coord], context)?;
                }
            }
        }
        Ok(())
    }

    /// Deobfuscates all coordinates within a list of geometric operations.
    pub fn deobfuscate_geometric_operations(
        &self,
        operations: &mut Vec<GeometricOperation>,
        context: &SecurityContext,
    ) -> Result<(), SecurityError> {
        for op in operations.iter_mut() {
            match op {
                GeometricOperation::RegionFill { ref mut start, ref mut end, .. } => {
                    self.obfuscator.deobfuscate_coordinates(&mut [*start, *end], context)?;
                }
                GeometricOperation::PathTrace { ref mut waypoints, .. } => {
                    self.obfuscator.deobfuscate_coordinates(waypoints, context)?;
                }
                GeometricOperation::PatternReference { ref mut base_coordinate, .. } => {
                    self.obfuscator.deobfuscate_coordinates(&mut [*base_coordinate], context)?;
                }
                GeometricOperation::SparseMapping { .. } => {
                    // See notes in obfuscate_geometric_operations
                }
                GeometricOperation::FunctionGeneration { ref mut domain, .. } => {
                    self.obfuscator.deobfuscate_coordinates(&mut [domain.min_coord, domain.max_coord], context)?;
                }
            }
        }
        Ok(())
    }

    /// Generates an integrity proof for the given operations.
    pub fn generate_integrity_proof(
        &self,
        operations: &[GeometricOperation]
    ) -> Result<IntegrityProof, SecurityError> {
        self.integrity_checker.generate_integrity_proof(operations)
    }

    /// Verifies an integrity proof for the given operations.
    pub fn verify_integrity_proof(
        &self,
        operations: &[GeometricOperation],
        proof: &IntegrityProof
    ) -> Result<bool, SecurityError> {
        self.integrity_checker.verify_integrity_proof(operations, proof)
    }
    
    // Higher-level combined operations:
    /// Obfuscates operations and then generates an integrity proof.
    pub fn protect_data(
        &self,
        operations: &mut Vec<GeometricOperation>,
        context: &SecurityContext
    ) -> Result<IntegrityProof, SecurityError> {
        self.obfuscate_geometric_operations(operations, context)?;
        self.generate_integrity_proof(operations)
    }

    /// Verifies integrity and then deobfuscates operations.
    pub fn unprotect_data(
        &self,
        operations: &mut Vec<GeometricOperation>,
        proof: &IntegrityProof,
        context: &SecurityContext
    ) -> Result<(), SecurityError> {
        if !self.verify_integrity_proof(operations, proof)? {
            return Err(SecurityError::IntegrityCheckFailed);
        }
        self.deobfuscate_geometric_operations(operations, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coordinates::Coordinate3D;
    use crate::compression::operations::{GeometricOperation, InterpolationType, EncodingScheme, Matrix3x3}; // For test data
    // SecurityContext is now in super (security/mod.rs)

    fn get_sample_ops() -> Vec<GeometricOperation> {
        vec![
            GeometricOperation::RegionFill {
                start: Coordinate3D::new(10,20,30),
                end: Coordinate3D::new(15,25,35),
                fill_byte: 1,
                compression_ratio: 1.0,
            },
            GeometricOperation::PathTrace {
                waypoints: vec![Coordinate3D::new(1,2,3), Coordinate3D::new(4,5,6)],
                interpolation: InterpolationType::Linear,
                data_encoding: EncodingScheme::Raw,
            },
        ]
    }

    #[test]
    fn test_security_layer_new() {
        let _layer = SecurityLayer::new();
        // Basic instantiation check
    }

    #[test]
    fn test_obfuscate_deobfuscate_operations_reversible() {
        let layer = SecurityLayer::new();
        let context = SecurityContext::default(); // Assuming Default derive for SecurityContext
        
        let mut ops = get_sample_ops();
        let original_ops = ops.clone();

        layer.obfuscate_geometric_operations(&mut ops, &context).expect("Obfuscation failed");

        // Check that coordinates within operations actually changed
        // Comparing entire ops might be complex if some parts shouldn't change.
        // Let's check one coordinate from each op type we handle.
        if let GeometricOperation::RegionFill { start: s1, .. } = &original_ops[0] {
            if let GeometricOperation::RegionFill { start: s2, .. } = &ops[0] {
                assert_ne!(s1, s2, "RegionFill start coord should have changed after obfuscation.");
            }
        }
         if let GeometricOperation::PathTrace { waypoints: w1, .. } = &original_ops[1] {
            if let GeometricOperation::PathTrace { waypoints: w2, .. } = &ops[1] {
                if !w1.is_empty() && !w2.is_empty() {
                     assert_ne!(w1[0], w2[0], "PathTrace first waypoint should have changed after obfuscation.");
                }
            }
        }


        layer.deobfuscate_geometric_operations(&mut ops, &context).expect("Deobfuscation failed");
        assert_eq!(ops, original_ops, "Deobfuscation did not restore original operations");
    }

    #[test]
    fn test_protect_unprotect_data_flow() {
        let layer = SecurityLayer::new();
        let context = SecurityContext::default();
        let mut ops = get_sample_ops();
        let original_ops_for_value_check = ops.clone(); // For final value check

        // Protect data
        let proof_result = layer.protect_data(&mut ops, &context);
        assert!(proof_result.is_ok());
        let proof = proof_result.unwrap();

        // Ops should be obfuscated now, different from original
        // (reuse check from previous test, or simplify)
        if let GeometricOperation::RegionFill { start: s1, .. } = &original_ops_for_value_check[0] {
            if let GeometricOperation::RegionFill { start: s2, .. } = &ops[0] {
                 if *s1 != Coordinate3D::new(0,0,0) { // Avoid false positive if key makes it zero
                    assert_ne!(s1, s2, "Ops should be obfuscated by protect_data");
                 }
            }
        }
        
        // Unprotect data (should succeed)
        let unprotect_result = layer.unprotect_data(&mut ops, &proof, &context);
        assert!(unprotect_result.is_ok());
        assert_eq!(ops, original_ops_for_value_check, "Unprotect_data did not restore original operations");

        // Unprotect data with a modified proof (should fail integrity check)
        let mut tampered_proof_bytes = proof.0.clone();
        if !tampered_proof_bytes.is_empty() {
            tampered_proof_bytes[0] = tampered_proof_bytes[0].wrapping_add(1);
        } else { // Handle empty proof case if necessary, though unlikely for non-empty ops
            tampered_proof_bytes.push(1);
        }
        let tampered_proof = IntegrityProof(tampered_proof_bytes);
        
        // Before calling unprotect_data with tampered proof, ops are currently deobfuscated.
        // Re-obfuscate them to simulate the state where unprotect_data would receive them.
        layer.obfuscate_geometric_operations(&mut ops, &context).unwrap(); 

        let unprotect_fail_result = layer.unprotect_data(&mut ops, &tampered_proof, &context);
        assert!(unprotect_fail_result.is_err());
        match unprotect_fail_result.err().unwrap() {
            SecurityError::IntegrityCheckFailed => { /* Expected */ }
            e => panic!("Unexpected error type for tampered proof: {:?}", e),
        }
    }
}
