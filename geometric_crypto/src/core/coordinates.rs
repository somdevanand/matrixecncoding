use serde::{Serialize, Deserialize};
use std::cmp::Ordering; // For manual Ord implementation if needed

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, PartialOrd, Ord)]
pub struct Coordinate3D {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl Coordinate3D {
    pub fn new(x: u8, y: u8, z: u8) -> Self {
        Self { x, y, z }
    }

    pub fn add(self, other: Self) -> Self {
        Self {
            x: self.x.wrapping_add(other.x),
            y: self.y.wrapping_add(other.y),
            z: self.z.wrapping_add(other.z),
        }
    }

    pub fn subtract(self, other: Self) -> Self {
        Self {
            x: self.x.wrapping_sub(other.x),
            y: self.y.wrapping_sub(other.y),
            z: self.z.wrapping_sub(other.z),
        }
    }

    /// Rotates the coordinate around a given axis by a specified angle in degrees.
    /// The center of rotation is assumed to be (0,0,0) for simplicity in this context.
    /// Calculations are done using f32 and results are clamped back to u8.
    pub fn rotate(self, axis: super::transforms::Axis, angle_degrees: f32) -> Self {
        let rad = super::transforms::degrees_to_radians(angle_degrees);
        let cos_a = rad.cos();
        let sin_a = rad.sin();

        let x = self.x as f32;
        let y = self.y as f32;
        let z = self.z as f32;

        let (nx, ny, nz) = match axis {
            super::transforms::Axis::X => (
                // x' = x
                // y' = y*cos(a) - z*sin(a)
                // z' = y*sin(a) + z*cos(a)
                x,
                y * cos_a - z * sin_a,
                y * sin_a + z * cos_a,
            ),
            super::transforms::Axis::Y => (
                // x' = x*cos(a) + z*sin(a)
                // y' = y
                // z' = -x*sin(a) + z*cos(a)
                x * cos_a + z * sin_a,
                y,
                -x * sin_a + z * cos_a,
            ),
            super::transforms::Axis::Z => (
                // x' = x*cos(a) - y*sin(a)
                // y' = x*sin(a) + y*cos(a)
                // z' = z
                x * cos_a - y * sin_a,
                x * sin_a + y * cos_a,
                z,
            ),
        };

        Coordinate3D {
            x: super::transforms::clamp_to_u8(nx),
            y: super::transforms::clamp_to_u8(ny),
            z: super::transforms::clamp_to_u8(nz),
        }
    }

    /// Scales the coordinate by a given factor.
    /// The center of scaling is assumed to be (0,0,0).
    /// Calculations are done using f32 and results are clamped back to u8.
    pub fn scale(self, factor: f32) -> Self {
        if factor < 0.0 {
            // Or handle error, for now, treat negative factor as 0 to avoid issues.
            // Or return self, or an error. Clamping to 0 for components.
             return Coordinate3D {
                x: super::transforms::clamp_to_u8(self.x as f32 * 0.0),
                y: super::transforms::clamp_to_u8(self.y as f32 * 0.0),
                z: super::transforms::clamp_to_u8(self.z as f32 * 0.0),
            };
        }
        Coordinate3D {
            x: super::transforms::clamp_to_u8(self.x as f32 * factor),
            y: super::transforms::clamp_to_u8(self.y as f32 * factor),
            z: super::transforms::clamp_to_u8(self.z as f32 * factor),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Imports Coordinate3D
    use serde_json; // For JSON serialization tests
    // Axis is available via super::transforms::Axis, consistent with its usage in rotate method.

    #[test]
    fn test_new_coordinate() {
        let coord = Coordinate3D::new(10, 20, 30);
        assert_eq!(coord.x, 10);
        assert_eq!(coord.y, 20);
        assert_eq!(coord.z, 30);
    }

    #[test]
    fn test_coordinate_addition() {
        let c1 = Coordinate3D::new(10, 20, 250);
        let c2 = Coordinate3D::new(5, 10, 10);
        let result = c1.add(c2);
        assert_eq!(result, Coordinate3D::new(15, 30, 4)); // 250 + 10 wraps to 4

        let c3 = Coordinate3D::new(255, 0, 100);
        let c4 = Coordinate3D::new(1, 0, 1);
        let result_wrap = c3.add(c4);
        assert_eq!(result_wrap, Coordinate3D::new(0, 0, 101));
    }

    #[test]
    fn test_coordinate_subtraction() {
        let c1 = Coordinate3D::new(10, 20, 5);
        let c2 = Coordinate3D::new(5, 30, 10); // y will underflow, z will underflow
        let result = c1.subtract(c2);
        assert_eq!(result, Coordinate3D::new(5, 246, 251)); // 20 - 30 wraps to 246, 5 - 10 wraps to 251

        let c3 = Coordinate3D::new(0, 10, 100);
        let c4 = Coordinate3D::new(1, 0, 101);
        let result_wrap = c3.subtract(c4);
        assert_eq!(result_wrap, Coordinate3D::new(255, 10, 255));
    }

    #[test]
    fn test_coordinate_equality() {
        let c1 = Coordinate3D::new(1, 2, 3);
        let c2 = Coordinate3D::new(1, 2, 3);
        let c3 = Coordinate3D::new(4, 5, 6);
        assert_eq!(c1, c2);
        assert_ne!(c1, c3);
    }

    #[test]
    fn test_coordinate_cloning_and_copying() {
        let c1 = Coordinate3D::new(1, 2, 3);
        
        // Test clone
        let c2 = c1.clone();
        assert_eq!(c1, c2);
        
        // Test copy
        let c3 = c1; // Copy occurs here because Coordinate3D implements Copy
        assert_eq!(c1, c3);

        // To further demonstrate that c2 and c3 are independent copies,
        // we can create new coordinates based on them and they should still be equal
        // to the original c1 if not modified.
        // (This doesn't modify c1, c2, or c3)
        let c1_plus_one = c1.add(Coordinate3D::new(1,1,1));
        let c2_plus_one = c2.add(Coordinate3D::new(1,1,1));
        let c3_plus_one = c3.add(Coordinate3D::new(1,1,1));

        assert_eq!(c1_plus_one, Coordinate3D::new(2,3,4));
        assert_eq!(c2_plus_one, c1_plus_one);
        assert_eq!(c3_plus_one, c1_plus_one);

        // And c1, c2, c3 remain unchanged
        assert_eq!(c1, Coordinate3D::new(1, 2, 3));
        assert_eq!(c2, Coordinate3D::new(1, 2, 3));
        assert_eq!(c3, Coordinate3D::new(1, 2, 3));
    }

    // Tests for rotate and scale methods

    #[test]
    fn test_coordinate_rotation_x_axis_90_degrees() {
        // For X-axis rotation by 90 deg:
        // x' = x
        // y' = y*cos(90) - z*sin(90) = -z
        // z' = y*sin(90) + z*cos(90) = y
        let c = Coordinate3D::new(10, 20, 30);
        // Rotate around X by 90 degrees. cos(90)=0, sin(90)=1
        // x' = 10
        // y' = 20*0 - 30*1 = -30. Clamped to 0.
        // z' = 20*1 + 30*0 = 20.
        let rotated = c.rotate(crate::core::transforms::Axis::X, 90.0);
        assert_eq!(rotated, Coordinate3D::new(10, 0, 20), "Rotation X by 90 deg failed");

        let c2 = Coordinate3D::new(10, 200, 150);
        // x' = 10
        // y' = 200*0 - 150*1 = -150. Clamped to 0.
        // z' = 200*1 + 150*0 = 200.
        let rotated2 = c2.rotate(crate::core::transforms::Axis::X, 90.0);
        assert_eq!(rotated2, Coordinate3D::new(10, 0, 200));
    }

    #[test]
    fn test_coordinate_rotation_y_axis_90_degrees() {
        // For Y-axis rotation by 90 deg:
        // x' = x*cos(90) + z*sin(90) = z
        // y' = y
        // z' = -x*sin(90) + z*cos(90) = -x
        let c = Coordinate3D::new(10, 20, 30);
        // x' = 10*0 + 30*1 = 30
        // y' = 20
        // z' = -10*1 + 30*0 = -10. Clamped to 0.
        let rotated = c.rotate(crate::core::transforms::Axis::Y, 90.0);
        assert_eq!(rotated, Coordinate3D::new(30, 20, 0), "Rotation Y by 90 deg failed");
    }

    #[test]
    fn test_coordinate_rotation_z_axis_90_degrees() {
        // For Z-axis rotation by 90 deg:
        // x' = x*cos(90) - y*sin(90) = -y
        // y' = x*sin(90) + y*cos(90) = x
        // z' = z
        let c = Coordinate3D::new(10, 20, 30);
        // x' = 10*0 - 20*1 = -20. Clamped to 0.
        // y' = 10*1 + 20*0 = 10
        // z' = 30
        let rotated = c.rotate(crate::core::transforms::Axis::Z, 90.0);
        assert_eq!(rotated, Coordinate3D::new(0, 10, 30), "Rotation Z by 90 deg failed");
    }
    
    #[test]
    fn test_coordinate_rotation_180_degrees() {
        // Rotating by 180 degrees around X: (x, -y, -z)
        let c = Coordinate3D::new(10, 20, 30);
        // x' = 10
        // y' = 20*(-1) - 30*0 = -20 -> 0
        // z' = 20*0 + 30*(-1) = -30 -> 0
        let rotated_x180 = c.rotate(crate::core::transforms::Axis::X, 180.0);
        assert_eq!(rotated_x180, Coordinate3D::new(10, 0, 0)); // Clamped due to positive coord space

        // Let's use values that stay positive after negation if we imagine a [-127, 127] space,
        // but for u8, it's always clamping or wrapping.
        // The current clamp_to_u8(val.round().max(0.0).min(255.0) as u8) means negative results become 0.
        // A rotation of (10,20,30) around X by 180 deg where center of space is e.g. (128,128,128) would be different.
        // But our rotation is around (0,0,0).
        // So, (10, 20, 30) -> (10, 20*cos(180)-30*sin(180), 20*sin(180)+30*cos(180)) = (10, -20, -30) -> (10,0,0) after clamping.
        // This is the expected behavior with the current implementation.
    }

    #[test]
    fn test_coordinate_rotation_360_degrees() {
        let c = Coordinate3D::new(10, 20, 30);
        let rotated_x360 = c.rotate(crate::core::transforms::Axis::X, 360.0);
        // cos(360)=1, sin(360)=0
        // x' = 10
        // y' = 20*1 - 30*0 = 20
        // z' = 20*0 + 30*1 = 30
        assert_eq!(rotated_x360, Coordinate3D::new(10, 20, 30), "Rotation by 360 should be identity");
    }

    #[test]
    fn test_coordinate_scaling_positive_factor() {
        let c = Coordinate3D::new(10, 20, 30);
        let scaled = c.scale(2.0);
        assert_eq!(scaled, Coordinate3D::new(20, 40, 60));

        let c2 = Coordinate3D::new(100, 150, 50);
        let scaled2 = c2.scale(2.0); // 200, 300 (clamps to 255), 100
        assert_eq!(scaled2, Coordinate3D::new(200, 255, 100));
    }

    #[test]
    fn test_coordinate_scaling_zero_factor() {
        let c = Coordinate3D::new(10, 20, 30);
        let scaled = c.scale(0.0);
        assert_eq!(scaled, Coordinate3D::new(0, 0, 0));
    }
    
    #[test]
    fn test_coordinate_scaling_one_factor() {
        let c = Coordinate3D::new(10, 20, 30);
        let scaled = c.scale(1.0);
        assert_eq!(scaled, Coordinate3D::new(10, 20, 30));
    }

    #[test]
    fn test_coordinate_scaling_fractional_factor() {
        let c = Coordinate3D::new(10, 20, 30);
        let scaled = c.scale(0.5); // 5, 10, 15
        assert_eq!(scaled, Coordinate3D::new(5, 10, 15));

        let c2 = Coordinate3D::new(11, 21, 31);
        let scaled2 = c2.scale(0.5); // 5.5->6, 10.5->11, 15.5->16 (due to round())
        assert_eq!(scaled2, Coordinate3D::new(6, 11, 16));
    }

    #[test]
    fn test_coordinate_scaling_negative_factor() {
        // Current implementation of scale for negative factor results in (0,0,0)
        let c = Coordinate3D::new(10, 20, 30);
        let scaled = c.scale(-1.0);
        assert_eq!(scaled, Coordinate3D::new(0,0,0), "Scaling by negative factor should result in (0,0,0)");
        
        let scaled_neg_two = c.scale(-2.0);
        assert_eq!(scaled_neg_two, Coordinate3D::new(0,0,0), "Scaling by negative factor should result in (0,0,0)");
    }

    #[test]
    fn test_coordinate_serialization_deserialization_json() {
        let original_coord = Coordinate3D::new(10, 20, 30);

        // Serialize to JSON string
        let json_data = serde_json::to_string(&original_coord).expect("JSON serialization failed");

        // Deserialize from JSON string
        let deserialized_coord: Coordinate3D = serde_json::from_str(&json_data).expect("JSON deserialization failed");

        assert_eq!(original_coord, deserialized_coord, "Deserialized coordinate should match original");
    }

    #[test]
    fn test_coordinate_serialization_bincode() {
        // Bincode is another common binary format. Add if desired, or stick to JSON for this test.
        // This would require `bincode` as a dev-dependency as well, though it's already a main dependency.
        let original_coord = Coordinate3D::new(101, 102, 103);
        let encoded: Vec<u8> = bincode::serialize(&original_coord).expect("Bincode serialization failed");
        let decoded: Coordinate3D = bincode::deserialize(&encoded[..]).expect("Bincode deserialization failed");
        assert_eq!(original_coord, decoded);
    }
}
