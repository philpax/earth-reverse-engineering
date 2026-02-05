//! Oriented bounding box unpacking.

use crate::OrientedBoundingBox;
use crate::error::DecodeResult;
use glam::{DVec3, Vec3};

/// Unpack a 15-byte oriented bounding box.
///
/// # Format
///
/// - Bytes 0-5: Center offset (3 × i16) relative to `head_node_center`
/// - Bytes 6-8: Extents (3 × u8)
/// - Bytes 9-14: Euler angles (3 × u16)
///
/// # Arguments
///
/// * `packed` - 15-byte packed OBB data
/// * `head_node_center` - Reference point for center offset
/// * `meters_per_texel` - Scale factor for positions
pub fn unpack_obb(
    _packed: &[u8],
    _head_node_center: Vec3,
    _meters_per_texel: f32,
) -> DecodeResult<OrientedBoundingBox> {
    // Stub - will be implemented in Commit 5
    Ok(OrientedBoundingBox {
        center: DVec3::ZERO,
        extents: DVec3::ZERO,
        orientation: glam::DMat3::IDENTITY,
    })
}
