//! Index unpacking.

use crate::error::DecodeResult;

/// Unpack varint-encoded triangle strip indices.
///
/// The indices form a triangle strip, where degenerate triangles
/// (with repeated vertices) are used for strip restarts.
pub fn unpack_indices(_packed: &[u8]) -> DecodeResult<Vec<u16>> {
    // Stub - will be implemented in Commit 4
    Ok(Vec::new())
}
