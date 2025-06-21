use crate::core::coordinates::Coordinate3D;
use crate::compression::operations::{GeometricOperation, InterpolationType, EncodingScheme, Matrix3x3, CoordinateDelta, CompressedValues, MathFunction, CoordinateRegion, TrigFuncType}; // Added all ops types for get_sample_ops
use super::obfuscation::CoordinateObfuscator;
use super::integrity::IntegrityVerifier; // IntegrityVerifier is in integrity.rs
use super::key_manager::KeyManager; // Import KeyManager
use super::{SecurityError, SecurityContext, IntegrityProof}; // SecurityContext, SecurityError, and IntegrityProof from mod.rs

#[derive(Debug, Clone)]
pub struct SecurityLayer {
    obfuscator: CoordinateObfuscator,
    integrity_checker: IntegrityVerifier,
    pub key_manager: KeyManager, 
}

impl SecurityLayer {
    pub fn new() -> Self { 
        Self {
            obfuscator: CoordinateObfuscator::new(),
            integrity_checker: IntegrityVerifier::new(),
            key_manager: KeyManager::new(),
        }
    }
    
    pub fn set_master_key(&mut self, key: [u8; 32]) -> Result<(), SecurityError> {
        // Allow overwriting for flexibility, or add specific logic if needed
        self.key_manager.set_master_key(key);
        Ok(())
    }

    pub fn obfuscate_geometric_operations(
        &self,
        operations: &mut Vec<GeometricOperation>,
        _context: &SecurityContext,
    ) -> Result<(), SecurityError> {
        let obf_key = self.key_manager.get_derived_key(b"coord_obfuscation_v1")?;
        
        for op in operations.iter_mut() {
            match op {
                GeometricOperation::RegionFill { start, end, .. } => {
                    let mut coords = [*start, *end];
                    self.obfuscator.obfuscate_coordinates(&mut coords, &obf_key)?;
                    *start = coords[0];
                    *end = coords[1];
                }
                GeometricOperation::PathTrace { waypoints, .. } => {
                    self.obfuscator.obfuscate_coordinates(waypoints, &obf_key)?;
                }
                GeometricOperation::PatternReference { base_coordinate, .. } => {
                    let mut coords = [*base_coordinate];
                    self.obfuscator.obfuscate_coordinates(&mut coords, &obf_key)?;
                    *base_coordinate = coords[0];
                }
                GeometricOperation::SparseMapping { ref coordinate_deltas, .. } => {
                    // CoordinateDeltas are relative offsets (i8). Obfuscating them directly
                    // as if they are absolute Coordinate3D points is likely incorrect.
                    // If SparseMapping implies a base absolute coordinate, that base should be
                    // part of the struct and obfuscated. The values themselves might need
                    // encryption if sensitive, but not coordinate obfuscation via CoordinateObfuscator.
                    // No action for now on coordinate_deltas or value_encoding by CoordinateObfuscator.
                    let _ = coordinate_deltas; // Mark as used
                }
                GeometricOperation::FunctionGeneration { domain, .. } => {
                    let mut coords = [domain.min_coord, domain.max_coord];
                    self.obfuscator.obfuscate_coordinates(&mut coords, &obf_key)?;
                    domain.min_coord = coords[0];
                    domain.max_coord = coords[1];
                }
            }
        }
        Ok(())
    }

    pub fn deobfuscate_geometric_operations( &self, operations: &mut Vec<GeometricOperation>, _context: &SecurityContext ) -> Result<(), SecurityError> {
        let obf_key = self.key_manager.get_derived_key(b"coord_obfuscation_v1")?;
        for op in operations.iter_mut() {
            match op {
                GeometricOperation::RegionFill { ref mut start, ref mut end, .. } => {
                    let mut coords = [*start, *end];
                    self.obfuscator.deobfuscate_coordinates(&mut coords, &obf_key)?;
                    *start = coords[0];
                    *end = coords[1];
                }
                GeometricOperation::PathTrace { ref mut waypoints, .. } => {
                    self.obfuscator.deobfuscate_coordinates(waypoints, &obf_key)?;
                }
                GeometricOperation::PatternReference { ref mut base_coordinate, .. } => {
                    let mut coords = [*base_coordinate];
                    self.obfuscator.deobfuscate_coordinates(&mut coords, &obf_key)?;
                    *base_coordinate = coords[0];
                }
                GeometricOperation::SparseMapping { .. } => {
                    // See notes in obfuscate_geometric_operations. No action.
                }
                GeometricOperation::FunctionGeneration { ref mut domain, .. } => {
                    let mut domain_coords = [domain.min_coord, domain.max_coord];
                    self.obfuscator.deobfuscate_coordinates(&mut domain_coords, &obf_key)?;
                    domain.min_coord = domain_coords[0];
                    domain.max_coord = domain_coords[1];
                }
            }
        }
        Ok(())
    }
    
     pub fn generate_integrity_proof(&self, operations: &[GeometricOperation]) -> Result<IntegrityProof, SecurityError> { Ok(self.integrity_checker.generate_integrity_proof(operations)?) }
     pub fn verify_integrity_proof(&self, operations: &[GeometricOperation], proof: &IntegrityProof) -> Result<bool, SecurityError> { Ok(self.integrity_checker.verify_integrity_proof(operations, proof)?) }
     
     pub fn protect_data(&self, operations: &mut Vec<GeometricOperation>, context: &SecurityContext) -> Result<IntegrityProof, SecurityError> {
         if !self.key_manager.is_key_set() { return Err(SecurityError::KeyExchangeError("Master key not set for protect_data".to_string())); }
         self.obfuscate_geometric_operations(operations, context)?;
         self.generate_integrity_proof(operations)
     }
     pub fn unprotect_data(&self, operations: &mut Vec<GeometricOperation>, proof: &IntegrityProof, context: &SecurityContext) -> Result<(), SecurityError> {
         if !self.key_manager.is_key_set() { return Err(SecurityError::KeyExchangeError("Master key not set for unprotect_data".to_string())); }
         if !self.verify_integrity_proof(operations, proof)? { 
             return Err(SecurityError::IntegrityCheckFailed);
         }
         self.deobfuscate_geometric_operations(operations, context)
     }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Ensure all necessary operation types are imported for get_sample_ops_for_obf_coverage
    use crate::compression::operations::{InterpolationType, EncodingScheme, Matrix3x3, CoordinateDelta, CompressedValues, MathFunction, CoordinateRegion, TrigFuncType};


    fn get_sample_ops_for_obf_coverage() -> Vec<GeometricOperation> {
        vec![
            GeometricOperation::RegionFill {
                start: Coordinate3D::new(50, 100, 150),
                end: Coordinate3D::new(55, 105, 155),
                fill_byte: 1, 
                compression_ratio: 1.0,
            },
            GeometricOperation::PathTrace {
                waypoints: vec![Coordinate3D::new(110, 120, 130), Coordinate3D::new(140, 150, 160)],
                interpolation: InterpolationType::Linear,
                data_encoding: EncodingScheme::Raw,
            },
            GeometricOperation::PatternReference {
                base_coordinate: Coordinate3D::new(200, 210, 220),
                pattern_id: 1,
                transformation_matrix: Matrix3x3::identity(),
            },
            GeometricOperation::SparseMapping {
                coordinate_deltas: vec![CoordinateDelta{dx:1,dy:1,dz:1}, CoordinateDelta{dx:-1,dy:-1,dz:-1}],
                value_encoding: CompressedValues::Bytes(vec![10, 20]),
            },
            GeometricOperation::FunctionGeneration {
                mathematical_function: MathFunction::Polynomial(vec![1.0, 2.0]),
                domain: CoordinateRegion { 
                    min_coord: Coordinate3D::new(30,30,30), 
                    max_coord: Coordinate3D::new(40,40,40) 
                },
                parameters: vec![0.1, 0.2],
            },
        ]
    }

    #[test] 
    fn test_security_layer_new() { 
        let layer = SecurityLayer::new();
        assert!(!layer.key_manager.is_key_set(), "Key should not be set on new");
    }

    #[test]
    fn test_obfuscate_deobfuscate_operations_full_coverage() { // Renamed and expanded test
        let mut layer = SecurityLayer::new();
        layer.set_master_key([2u8; 32]).unwrap();
        let context = SecurityContext::default();
        
        let mut ops = get_sample_ops_for_obf_coverage();
        let original_ops = ops.clone(); // Clone before obfuscation

        println!("Original operations: {:#?}", original_ops);
        layer.obfuscate_geometric_operations(&mut ops, &context).expect("Obfuscation failed");
        println!("Obfuscated operations: {:#?}", ops);

        // Assertions for changes (selective based on what's obfuscated)
        // RegionFill start coord
        if let (Some(GeometricOperation::RegionFill { start: s_orig, .. }), Some(GeometricOperation::RegionFill { start: s_obf, .. })) = (original_ops.get(0), ops.get(0)) {
            assert_ne!(s_orig, s_obf, "RegionFill start coord should change");
        }
        // PathTrace first waypoint
        if let (Some(GeometricOperation::PathTrace { waypoints: w_orig, .. }), Some(GeometricOperation::PathTrace { waypoints: w_obf, .. })) = (original_ops.get(1), ops.get(1)) {
            if !w_orig.is_empty() { assert_ne!(w_orig[0], w_obf[0], "PathTrace waypoint should change"); }
        }
        // PatternReference base_coordinate
        if let (Some(GeometricOperation::PatternReference { base_coordinate: b_orig, .. }), Some(GeometricOperation::PatternReference { base_coordinate: b_obf, .. })) = (original_ops.get(2), ops.get(2)) {
            assert_ne!(b_orig, b_obf, "PatternReference base_coordinate should change");
        }
        // SparseMapping deltas (should NOT change with current strategy)
        if let (Some(GeometricOperation::SparseMapping { coordinate_deltas: d_orig, .. }), Some(GeometricOperation::SparseMapping { coordinate_deltas: d_obf, .. })) = (original_ops.get(3), ops.get(3)) {
            assert_eq!(d_orig, d_obf, "SparseMapping coordinate_deltas should NOT change");
        }
        // FunctionGeneration domain min_coord (should change) and parameters (should NOT change)
        if let (Some(GeometricOperation::FunctionGeneration { domain: dom_orig, parameters: params_orig, .. }), Some(GeometricOperation::FunctionGeneration { domain: dom_obf, parameters: params_obf, .. })) = (original_ops.get(4), ops.get(4)) {
            assert_ne!(dom_orig.min_coord, dom_obf.min_coord, "FunctionGeneration domain.min_coord should change");
            assert_eq!(params_orig, params_obf, "FunctionGeneration parameters should NOT change");
        }

        layer.deobfuscate_geometric_operations(&mut ops, &context).expect("Deobfuscation failed");
        println!("Deobfuscated operations: {:#?}", ops);
        assert_eq!(ops, original_ops, "Full deobfuscation did not restore original operations");
    }

    #[test]
    fn test_protect_unprotect_data_flow_with_key_integration() { 
        // Enable test logging
        std::env::set_var("RUST_LOG", "debug");
        env_logger::builder().is_test(true).init();
        
        log::info!("Starting test_protect_unprotect_data_flow_with_key_integration");
        
        // Initialize security layer and set master key
        let mut layer = SecurityLayer::new();
        let master_key = [0x5u8; 32];
        log::info!("Setting master key: {:?}", master_key);
        layer.set_master_key(master_key).unwrap();
        
        let context = SecurityContext::default();
        
        // Get sample operations and make a clone for comparison
        let mut ops = get_sample_ops_for_obf_coverage();
        let original_ops_for_value_check = ops.clone();
        
        log::info!("\nOriginal operations (before protect):");
        for (i, op) in original_ops_for_value_check.iter().enumerate() {
            log::info!("  [{}] {:?}", i, op);
        }

        // Protect the operations (obfuscate + generate proof)
        log::info!("\nCalling protect_data...");
        let protect_result = layer.protect_data(&mut ops, &context);
        
        if let Err(ref e) = protect_result {
            log::error!("Protect failed: {:?}", e);
            log::debug!("Operations after failed protect: {:?}", ops);
        }
        assert!(protect_result.is_ok(), "Protect failed: {:?}", protect_result.err());
        let proof = protect_result.unwrap();
        
        // Verify operations were obfuscated
        log::info!("\nOperations after protect (should be obfuscated):");
        for (i, op) in ops.iter().enumerate() {
            log::info!("  [{}] {:?}", i, op);
        }
        
        assert_ne!(
            ops, original_ops_for_value_check, 
            "Operations should be obfuscated by protect_data"
        );

        // Unprotect the operations (deobfuscate + verify proof)
        log::info!("\nCalling unprotect_data with valid proof...");
        let unprotect_result = layer.unprotect_data(&mut ops, &proof, &context);
        
        if let Err(e) = &unprotect_result {
            log::error!("Unprotect failed: {:?}", e);
            log::info!("Current operations state when unprotect failed:");
            for (i, op) in ops.iter().enumerate() {
                log::info!("  [{}] {:?}", i, op);
            }
        }
        
        assert!(unprotect_result.is_ok(), "Unprotect failed: {:?}", unprotect_result.err());
        
        log::info!("\nOperations after unprotect (should match original):");
        for (i, op) in ops.iter().enumerate() {
            log::info!("  [{}] {:?}", i, op);
        }
        
        // Verify operations were restored to original
        assert_eq!(
            ops, original_ops_for_value_check, 
            "Unprotect_data did not restore original operations"
        );

        // Test tampering detection
        log::info!("\nTesting tampering detection...");
        
        // Re-obfuscate for tampering test
        if let Err(e) = layer.obfuscate_geometric_operations(&mut ops, &context) {
            log::error!("Failed to re-obfuscate for tampering test: {:?}", e);
            panic!("Failed to re-obfuscate for tampering test: {:?}", e);
        }
        
        // Tamper with the proof
        let mut tampered_proof_bytes = proof.0.clone();
        if !tampered_proof_bytes.is_empty() { 
            tampered_proof_bytes[0] ^= 0xFF; 
            log::debug!("Tampered with proof byte 0: {:02x} -> {:02x}", proof.0[0], tampered_proof_bytes[0]);
        } else { 
            tampered_proof_bytes.push(1);
            log::debug!("Added tampering byte to empty proof");
        }
        let tampered_proof = IntegrityProof(tampered_proof_bytes);
        
        // Verify tampered proof is rejected
        log::info!("Attempting to unprotect with tampered proof...");
        let unprotect_fail_result = layer.unprotect_data(&mut ops, &tampered_proof, &context);
        log::info!("Tamper test result: {:?}", unprotect_fail_result);
        
        assert!(unprotect_fail_result.is_err(), "Tampered proof should be rejected");
        assert!(
            matches!(unprotect_fail_result.as_ref().err().unwrap(), SecurityError::IntegrityCheckFailed),
            "Expected IntegrityCheckFailed error for tampered proof, got: {:?}", 
            unprotect_fail_result.err()
        );
        
        log::info!("\nTest completed successfully!");
    }
}
