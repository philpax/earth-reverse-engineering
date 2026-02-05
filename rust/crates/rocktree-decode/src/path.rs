//! Path and flags unpacking.

use crate::PathAndFlags;

/// Unpack path and flags from node metadata.
///
/// The `path_and_flags` field encodes:
/// - Lower 2 bits: Level - 1 (so level is 1-4)
/// - Next 3*level bits: Octant path digits (0-7)
/// - Remaining bits: Flags
///
/// # Arguments
///
/// * `path_and_flags` - The packed value from `NodeMetadata`
#[must_use]
pub fn unpack_path_and_flags(_path_and_flags: u32) -> PathAndFlags {
    // Stub - will be implemented in Commit 5
    PathAndFlags {
        path: String::new(),
        flags: 0,
        level: 0,
    }
}
