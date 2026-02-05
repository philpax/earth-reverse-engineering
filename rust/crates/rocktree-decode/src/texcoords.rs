//! Texture coordinate unpacking.

use crate::error::DecodeResult;
use crate::{UvTransform, Vertex};

/// Unpack texture coordinates into vertex array.
///
/// Input format: 4-byte header (`u_mod`, `v_mod`) followed by 4*N bytes of UV data.
/// The UV values are delta-encoded with modulo arithmetic.
///
/// # Arguments
///
/// * `packed` - The packed texture coordinate data
/// * `vertices` - Mutable slice of vertices to update
///
/// # Returns
///
/// The UV transform (offset and scale) for shader use.
pub fn unpack_tex_coords(_packed: &[u8], _vertices: &mut [Vertex]) -> DecodeResult<UvTransform> {
    // Stub - will be implemented in Commit 4
    Ok(UvTransform::default())
}
