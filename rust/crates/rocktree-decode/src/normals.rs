//! Normal vector unpacking.

use crate::error::DecodeResult;

/// Unpack normal data from `NodeData`'s `for_normals` field.
///
/// This produces a lookup table of 3-byte normals that can be
/// indexed by the mesh's normals field.
///
/// # Returns
///
/// A vector of RGB normal values (3 bytes per normal).
pub fn unpack_for_normals(_for_normals: &[u8]) -> DecodeResult<Vec<u8>> {
    // Stub - will be implemented in Commit 6
    Ok(Vec::new())
}

/// Unpack per-vertex normals using the normal lookup table.
///
/// # Arguments
///
/// * `mesh_normals` - The mesh's normals field (indices into the lookup table)
/// * `for_normals` - The unpacked normal lookup table from [`unpack_for_normals`]
/// * `vertex_count` - Number of vertices (for fallback if no normals)
///
/// # Returns
///
/// A vector of RGBA normal values (4 bytes per vertex, A is padding).
pub fn unpack_normals(
    _mesh_normals: Option<&[u8]>,
    _for_normals: Option<&[u8]>,
    _vertex_count: usize,
) -> DecodeResult<Vec<u8>> {
    // Stub - will be implemented in Commit 6
    Ok(Vec::new())
}
