#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    // To test rotate and scale, we would need to make Axis and helpers available here.
    // For now, these tests will be added in a subsequent step when module linking is addressed.
    // If geometric_crypto/core/mod.rs existed and declared `pub mod transforms;`,
    // then `use super::super::transforms::Axis;` might be needed in tests,
    // or `use crate::core::transforms::Axis;` if lib.rs is set up.

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
}
