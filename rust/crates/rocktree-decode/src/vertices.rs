//! Vertex unpacking.

use crate::Vertex;
use crate::error::DecodeResult;

/// Unpack delta-encoded vertex positions.
///
/// Input format: 3*N bytes arranged as [X0,X1,...,Xn, Y0,Y1,...,Yn, Z0,Z1,...,Zn]
/// Each component is delta-encoded (cumulative sum).
///
/// Output: N vertices with x, y, z filled in (w, u, v are zeroed).
pub fn unpack_vertices(_packed: &[u8]) -> DecodeResult<Vec<Vertex>> {
    // Stub - will be implemented in Commit 4
    Ok(Vec::new())
}
