use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

// Helper function to clamp and cast f32 to u8
pub(crate) fn clamp_to_u8(val: f32) -> u8 {
    val.round().max(0.0).min(255.0) as u8
}

// Helper for degrees to radians, as rotations often use radians
pub(crate) fn degrees_to_radians(degrees: f32) -> f32 {
    degrees * PI / 180.0
}
