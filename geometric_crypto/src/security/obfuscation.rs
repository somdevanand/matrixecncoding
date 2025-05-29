use crate::core::coordinates::Coordinate3D;
use crate::core::transforms::Axis; // For rotation
use super::{SecurityError, SecurityContext}; // Assuming SecurityContext is defined in security/mod.rs or security/common.rs

const DEFAULT_OBFUSCATION_KEY_BYTE: u8 = 42; // Example key byte

#[derive(Debug, Default, Clone)]
pub struct CoordinateObfuscator;

impl CoordinateObfuscator {
    pub fn new() -> Self {
        Default::default()
    }

    // Method 1: Simple XOR or Wrapping Add/Sub (already partially suggested)
    fn apply_byte_op(coord: &mut Coordinate3D, key_byte: u8, op: fn(u8, u8) -> u8) {
        coord.x = op(coord.x, key_byte);
        coord.y = op(coord.y, key_byte.rotate_left(3)); // Vary key_byte slightly for different axes
        coord.z = op(coord.z, key_byte.rotate_right(3));
    }

    // Method 2: Rotation-based obfuscation
    pub fn obfuscate_by_rotation(
        &self,
        coords: &mut [Coordinate3D],
        axis: Axis,
        angle_degrees: f32,
        // _context: &SecurityContext // Context might provide axis/angle
    ) -> Result<(), SecurityError> {
        for coord in coords.iter_mut() {
            *coord = coord.rotate(axis, angle_degrees);
        }
        Ok(())
    }

    pub fn deobfuscate_by_rotation(
        &self,
        coords: &mut [Coordinate3D],
        axis: Axis,
        angle_degrees: f32,
        // _context: &SecurityContext
    ) -> Result<(), SecurityError> {
        // De-rotation is rotation by negative angle
        for coord in coords.iter_mut() {
            *coord = coord.rotate(axis, -angle_degrees);
        }
        Ok(())
    }
    
    // Method 3: Non-linear dispersion (example using bitwise ops and addition)
    // Original complex non_linear_disperse_component and its non_linear_undisperse_component
    // are kept commented for reference, as per instruction to use simpler reversible one.
    /*
    fn non_linear_disperse_component(val: u8, mix_val1: u8, mix_val2: u8) -> u8 {
        let val_rot = val.rotate_left(3);
        val_rot.wrapping_add(mix_val1 ^ val).wrapping_sub(mix_val2 & val_rot)
    }

    fn non_linear_undisperse_component(val: u8, mix_val1: u8, mix_val2: u8) -> u8 {
        // This requires finding the inverse of non_linear_disperse_component.
        // If the function is `f(v) = (v <<< 3 + (m1 ^ v)) - (m2 & (v <<< 3))`
        // This is complex to reverse directly.
        val.rotate_right( (mix_val2 % 8) as u32 ).wrapping_sub(mix_val1) // This was a placeholder, not the actual inverse
    }
    */
    
    fn apply_non_linear_dispersion(&self, coord: &mut Coordinate3D, key_byte: u8, disperse: bool) {
        // Use different mixers derived from key_byte for each component for more mixing
        let k1 = key_byte.rotate_left(1);
        let k2 = key_byte.rotate_left(2);
        let k3 = key_byte.rotate_left(3);

        if disperse {
            // Simpler reversible: val.wrapping_add(mix_val).rotate_left( (mix_val2 % 8) as u32 )
            coord.x = coord.x.wrapping_add(k1).rotate_left( (k2 % 5) + 1 ); // Ensure rotation is non-zero
            coord.y = coord.y.wrapping_add(k2).rotate_left( (k3 % 5) + 1 );
            coord.z = coord.z.wrapping_add(k3).rotate_left( (k1 % 5) + 1 );
        } else { // Undisperse
            // Inverse: val.rotate_right( (mix_val2 % 8) as u32 ).wrapping_sub(mix_val1)
            coord.x = coord.x.rotate_right( (k2 % 5) + 1 ).wrapping_sub(k1);
            coord.y = coord.y.rotate_right( (k3 % 5) + 1 ).wrapping_sub(k2);
            coord.z = coord.z.rotate_right( (k1 % 5) + 1 ).wrapping_sub(k3);
        }
    }


    /// Main obfuscation function, can combine techniques.
    pub fn obfuscate_coordinates(
        &self,
        coords: &mut [Coordinate3D],
        context: &SecurityContext, // Assuming SecurityContext might provide a key or parameters
    ) -> Result<(), SecurityError> {
        // For now, use a hardcoded key_byte or derive one from context if available.
        // Let's assume SecurityContext is still an empty struct for this step.
        let _ = context; // Avoid unused warning for now.
        let key_byte = DEFAULT_OBFUSCATION_KEY_BYTE; 

        for coord in coords.iter_mut() {
            // Technique 1: Byte operation (wrapping_add)
            Self::apply_byte_op(coord, key_byte, u8::wrapping_add);
            // Technique 3: Non-linear dispersion (using the simpler reversible one)
            self.apply_non_linear_dispersion(coord, key_byte.rotate_left(4), true);
        }
        // Technique 2: Rotation (example, could be conditional or use context-derived params)
        // For this example, let's apply a fixed rotation to all coordinates as part of the chain.
        // In a real scenario, context might dictate if/how this is applied.
        // self.obfuscate_by_rotation(coords, Axis::X, 30.0)?; // Commented out as per prompt
        Ok(())
    }

    pub fn deobfuscate_coordinates(
        &self,
        coords: &mut [Coordinate3D],
        context: &SecurityContext,
    ) -> Result<(), SecurityError> {
        let _ = context;
        let key_byte = DEFAULT_OBFUSCATION_KEY_BYTE;

        // Apply inverse operations in REVERSE order of obfuscation
        // Technique 2: Inverse Rotation (was applied last in obfuscate)
        // self.deobfuscate_by_rotation(coords, Axis::X, 30.0)?; // Commented out as per prompt
        
        for coord in coords.iter_mut() {
            // Technique 3: Inverse Non-linear dispersion
            self.apply_non_linear_dispersion(coord, key_byte.rotate_left(4), false);
            // Technique 1: Inverse Byte operation (wrapping_sub)
            Self::apply_byte_op(coord, key_byte, u8::wrapping_sub);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coordinates::Coordinate3D;
    // Assuming SecurityContext will be defined in security/mod.rs or a common place
    // For now, if it's not, this test might rely on a local stub or the one from `super::*`
    // if security/mod.rs defines it. Let's assume `crate::security::SecurityContext` will be valid.
    use crate::security::SecurityContext; 

    #[test]
    fn test_obfuscate_deobfuscate_coordinates_reversible() {
        let obfuscator = CoordinateObfuscator::new();
        let context = SecurityContext; // Assuming it's a unit struct or Default-constructible for now

        let mut coords = vec![
            Coordinate3D::new(10, 20, 30),
            Coordinate3D::new(0, 0, 0),
            Coordinate3D::new(255, 255, 255),
            Coordinate3D::new(123, 87, 201),
        ];
        let original_coords = coords.clone();

        obfuscator.obfuscate_coordinates(&mut coords, &context).expect("Obfuscation failed");
        
        // Check that coordinates changed (highly probable unless all operations cancel out by chance)
        if !original_coords.is_empty() && !coords.is_empty() {
            // It's possible for a specific coordinate with specific key to map to itself,
            // but unlikely for all of them if the list is diverse enough.
            if original_coords.iter().any(|c| c.x !=0 || c.y !=0 || c.z != 0) { // Avoid all-zero case if key could be zero-effect
                 assert_ne!(coords, original_coords, "Coordinates should change after obfuscation for non-trivial cases");
            }
        }

        obfuscator.deobfuscate_coordinates(&mut coords, &context).expect("Deobfuscation failed");
        assert_eq!(coords, original_coords, "Deobfuscation did not restore original coordinates");
    }

    // Helper for the rotation test to avoid panic on all zero/max for the rotation axis
    // This is defined in the main code now if it's generally useful, or can be test-local.
    // For this structure, let's assume it's test-local if not part of Coordinate3D's public API.
    impl Coordinate3D { // Test-local helper extension
        fn x_y_z_all_zero_or_max_for_axis_test_helper(&self, axis: Axis) -> bool {
            match axis {
                Axis::X => (self.y == 0 || self.y == 255) && (self.z == 0 || self.z == 255),
                Axis::Y => (self.x == 0 || self.x == 255) && (self.z == 0 || self.z == 255),
                Axis::Z => (self.x == 0 || self.x == 255) && (self.y == 0 || self.y == 255),
            }
        }
    }

    #[test]
    fn test_rotation_obfuscation_reversible() {
        let obfuscator = CoordinateObfuscator::new();
        // let context = SecurityContext; // Not used directly by rotation methods in this version

        let mut coords_45 = vec![Coordinate3D::new(10, 20, 30)];
        let original_coords_45 = coords_45.clone();
        let axis = Axis::X;
        let angle_45 = 45.0;

        obfuscator.obfuscate_by_rotation(&mut coords_45, axis, angle_45).expect("Rotation obfuscation failed");
        
        // Check if changed, with helper to avoid false positives on edge cases
        if angle_45 % 360.0 != 0.0 && !original_coords_45[0].x_y_z_all_zero_or_max_for_axis_test_helper(axis) {
             assert_ne!(coords_45, original_coords_45, "Rotation by 45 deg should change coordinates");
        }

        obfuscator.deobfuscate_by_rotation(&mut coords_45, axis, angle_45).expect("Rotation deobfuscation failed");
        // Due to f32 precision, exact reversibility for arbitrary angles is not guaranteed.
        // Test with an angle more likely to be reversible, like 90 degrees.
        // The main `test_obfuscate_deobfuscate_coordinates_reversible` covers the chained effect.
        // This specific test is more about the rotation component itself.
        // If using only 90-degree rotations, we can expect more precision.
        // The current `obfuscate_coordinates` uses 30 degrees, which will have precision issues.

        let mut coords_90 = vec![Coordinate3D::new(10,20,30)];
        let original_coords_90 = coords_90.clone();
        obfuscator.obfuscate_by_rotation(&mut coords_90, Axis::Z, 90.0).unwrap();
        obfuscator.deobfuscate_by_rotation(&mut coords_90, Axis::Z, 90.0).unwrap();
        assert_eq!(coords_90, original_coords_90, "Rotation by 90 and -90 should be reversible for these values");
    }
    
    #[test]
    fn test_non_linear_dispersion_reversible() {
        let obfuscator = CoordinateObfuscator::new();
        let key_byte = 77;
        let mut coord = Coordinate3D::new(15, 25, 35);
        let original_coord = coord;

        obfuscator.apply_non_linear_dispersion(&mut coord, key_byte, true); // Disperse
        // Check that coordinate changed (it should, unless key_byte derivatives are all zero-effect)
        if key_byte != 0 { // Simple check, real check would be on k1,k2,k3 effects
            assert_ne!(coord, original_coord, "Dispersion should change coordinate");
        }


        obfuscator.apply_non_linear_dispersion(&mut coord, key_byte, false); // Undisperse
        assert_eq!(coord, original_coord, "Undispersion failed to restore original");
        
        // Test with edge values
        let mut edge_coord = Coordinate3D::new(0, 0, 0);
        let original_edge = edge_coord;
        obfuscator.apply_non_linear_dispersion(&mut edge_coord, key_byte, true);
        obfuscator.apply_non_linear_dispersion(&mut edge_coord, key_byte, false);
        assert_eq!(edge_coord, original_edge, "Dispersion reversibility failed for (0,0,0)");

        let mut edge_coord_max = Coordinate3D::new(255, 255, 255);
        let original_edge_max = edge_coord_max;
        obfuscator.apply_non_linear_dispersion(&mut edge_coord_max, key_byte, true);
        obfuscator.apply_non_linear_dispersion(&mut edge_coord_max, key_byte, false);
        assert_eq!(edge_coord_max, original_edge_max, "Dispersion reversibility failed for (255,255,255)");
    }
}
