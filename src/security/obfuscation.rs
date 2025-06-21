use crate::core::coordinates::Coordinate3D;
use crate::core::transforms::Axis;
use super::SecurityError; // SecurityContext no longer needed directly by these methods if key is passed

// DEFAULT_OBFUSCATION_KEY_BYTE can be removed if key is always required.
// const DEFAULT_OBFUSCATION_KEY_BYTE: u8 = 42; 

#[derive(Debug, Default, Clone)]
pub struct CoordinateObfuscator;

impl CoordinateObfuscator {
    pub fn new() -> Self { Default::default() }

    fn apply_byte_op(coord: &mut Coordinate3D, key_byte: u8, op: fn(u8, u8) -> u8) {
        // SIDE-CHANNEL NOTE: If key_byte or coordinate values were part of more complex
        // operations that might have variable timing based on data (e.g., certain multiplications,
        // or conditional branches based on secret data), care would be needed to ensure
        // constant-time execution where feasible and necessary.
        // Basic u8 arithmetic ops like wrapping_add, xor are generally constant time.
        coord.x = op(coord.x, key_byte);
        coord.y = op(coord.y, key_byte.rotate_left(3));
        coord.z = op(coord.z, key_byte.rotate_right(3));
    }

    pub fn obfuscate_by_rotation( /* ... same as before ... */ &self, coords: &mut [Coordinate3D], axis: Axis, angle_degrees: f32) -> Result<(), SecurityError> {
         for coord in coords.iter_mut() { *coord = coord.rotate(axis, angle_degrees); } Ok(())
    }
    pub fn deobfuscate_by_rotation( /* ... same as before ... */ &self, coords: &mut [Coordinate3D], axis: Axis, angle_degrees: f32) -> Result<(), SecurityError> {
         for coord in coords.iter_mut() { *coord = coord.rotate(axis, -angle_degrees); } Ok(())
    }
    
    fn apply_non_linear_dispersion(&self, coord: &mut Coordinate3D, key_byte: u8, disperse: bool) {
        // SIDE-CHANNEL NOTE: Similar to apply_byte_op, ensure any complex non-linear
        // functions used here are analyzed for data-dependent timing variations if the
        // inputs are secret and context is sensitive. Rotations by variable amounts
        // derived from data could be a concern if the rotation itself is not constant time.
        // Current u8.rotate_left/right by (const % 8) should be constant time.
         let k1 = key_byte.rotate_left(1); let k2 = key_byte.rotate_left(2); let k3 = key_byte.rotate_left(3);
         if disperse {
             coord.x = coord.x.wrapping_add(k1).rotate_left( (k2 % 8) as u32 );
             coord.y = coord.y.wrapping_add(k2).rotate_left( (k3 % 8) as u32 );
             coord.z = coord.z.wrapping_add(k3).rotate_left( (k1 % 8) as u32 );
         } else {
             coord.x = coord.x.rotate_right( (k2 % 8) as u32 ).wrapping_sub(k1);
             coord.y = coord.y.rotate_right( (k3 % 8) as u32 ).wrapping_sub(k2);
             coord.z = coord.z.rotate_right( (k1 % 8) as u32 ).wrapping_sub(k3);
         }
    }

    pub fn obfuscate_coordinates(
        &self,
        coords: &mut [Coordinate3D],
        obfuscation_key_slice: &[u8],
    ) -> Result<(), SecurityError> {
        if obfuscation_key_slice.is_empty() {
            return Err(SecurityError::ObfuscationError("Obfuscation key is empty".to_string()));
        }
        let key_byte = obfuscation_key_slice[0];

        #[cfg(test)]
        println!("Obfuscating {} coordinates with key byte: {}", coords.len(), key_byte);

        for (i, coord) in coords.iter_mut().enumerate() {
            #[cfg(test)] {
                println!("  [{}] Original: {:?}", i, coord);
            }
            
            // Apply byte operation
            let before_byte_op = *coord;
            Self::apply_byte_op(coord, key_byte, u8::wrapping_add);
            
            #[cfg(test)] {
                println!("  [{}] After byte op (key_byte={}): {:?} -> {:?}", 
                    i, key_byte, before_byte_op, coord);
            }

            // Apply non-linear dispersion
            let before_dispersion = *coord;
            let dispersion_key = key_byte.rotate_left(4);
            self.apply_non_linear_dispersion(coord, dispersion_key, true);
            
            #[cfg(test)] {
                println!("  [{}] After dispersion (key={}): {:?} -> {:?}", 
                    i, dispersion_key, before_dispersion, coord);
                println!("  [{}] Final obfuscated: {:?}", i, coord);
            }
        }
        
        #[cfg(test)]
        println!("Finished obfuscating batch of {} coordinates.", coords.len());
        
        Ok(())
    }

    pub fn deobfuscate_coordinates(
        &self,
        coords: &mut [Coordinate3D],
        obfuscation_key_slice: &[u8],
    ) -> Result<(), SecurityError> {
        if obfuscation_key_slice.is_empty() {
            return Err(SecurityError::ObfuscationError("Obfuscation key is empty".to_string()));
        }
        let key_byte = obfuscation_key_slice[0];

        #[cfg(test)]
        println!("Deobfuscating {} coordinates with key byte: {}", coords.len(), key_byte);

        for (i, coord) in coords.iter_mut().enumerate() {
            #[cfg(test)] {
                println!("  [{}] Obfuscated input: {:?}", i, coord);
            }
            
            // Reverse non-linear dispersion first
            let before_reverse_dispersion = *coord;
            let dispersion_key = key_byte.rotate_left(4);
            self.apply_non_linear_dispersion(coord, dispersion_key, false);
            
            #[cfg(test)] {
                println!("  [{}] After reverse dispersion (key={}): {:?} -> {:?}", 
                    i, dispersion_key, before_reverse_dispersion, coord);
            }
            
            // Then reverse the byte operation
            let before_byte_op = *coord;
            Self::apply_byte_op(coord, key_byte, u8::wrapping_sub);
            
            #[cfg(test)] {
                println!("  [{}] After reverse byte op (key_byte={}): {:?} -> {:?}", 
                    i, key_byte, before_byte_op, coord);
                println!("  [{}] Final deobfuscated: {:?}", i, coord);
            }
        }
        
        #[cfg(test)]
        println!("Finished deobfuscating batch of {} coordinates.", coords.len());
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // SecurityContext is no longer directly used here for keying
    // const DEFAULT_OBFUSCATION_KEY_BYTE is also removed/unused

    #[test]
    fn test_obfuscate_deobfuscate_coordinates_reversible_with_key() {
        let obfuscator = CoordinateObfuscator::new();
        let key = &[42u8, 1, 2, 3]; // Example key slice

        let mut coords = vec![Coordinate3D::new(10, 20, 30)];
        let original_coords = coords.clone();

        obfuscator.obfuscate_coordinates(&mut coords, key).expect("Obfuscation failed");
        assert_ne!(coords, original_coords, "Coordinates should change");
        obfuscator.deobfuscate_coordinates(&mut coords, key).expect("Deobfuscation failed");
        assert_eq!(coords, original_coords, "Deobfuscation did not restore original");
    }
    // test_rotation_obfuscation_reversible can remain as is (it doesn't use context/key directly)
    // test_non_linear_dispersion_reversible can remain as is
     #[test]
     fn test_rotation_obfuscation_reversible() { 
        let obfuscator = CoordinateObfuscator::new();
        let mut coords_90 = vec![Coordinate3D::new(0, 0, 30)];
        let original_coords_90 = coords_90.clone();
        obfuscator.obfuscate_by_rotation(&mut coords_90, Axis::Z, 90.0).unwrap();
        obfuscator.deobfuscate_by_rotation(&mut coords_90, Axis::Z, 90.0).unwrap();
        assert_eq!(coords_90, original_coords_90, "Rotation by 90 and -90 should be reversible for these values");
     }
     impl Coordinate3D { fn x_y_z_all_zero_or_max_for_axis_test_helper(&self, _axis: Axis) -> bool { false } } // minimal stub for test if original helper was in main code
     #[test]
     fn test_non_linear_dispersion_reversible() { 
        // This test would need to be filled in if its original content was more substantial
        // For now, ensuring it compiles. Original content from Turn 63 for this test:
        let obfuscator = CoordinateObfuscator::new();
        let key_byte = 77;
        let mut coord = Coordinate3D::new(15, 25, 35);
        let original_coord = coord;
        obfuscator.apply_non_linear_dispersion(&mut coord, key_byte, true);
        if key_byte != 0 { assert_ne!(coord, original_coord, "Dispersion should change coordinate"); }
        obfuscator.apply_non_linear_dispersion(&mut coord, key_byte, false);
        assert_eq!(coord, original_coord, "Undispersion failed to restore original");
     }
}
