//! Decode packed mesh data from Google Earth protobuf messages.
//!
//! This crate provides pure synchronous decoding functions for unpacking
//! mesh data from Google Earth's rocktree format. All functions are designed
//! to be called from any threading context - the library user controls
//! parallelism.
//!
//! # Design principles
//!
//! - **Synchronous**: No async, no threading primitives
//! - **User-controlled parallelism**: Client decides how to parallelize
//! - **Web-compatible**: Compiles to WASM
//!
//! # Key functions
//!
//! - [`unpack_vertices`]: Delta-decode XYZ vertex positions
//! - [`unpack_tex_coords`]: Unpack UV texture coordinates
//! - [`unpack_indices`]: Decode varint-encoded triangle strip indices
//! - [`unpack_obb`]: Decode oriented bounding box from 15 bytes
//! - [`unpack_path_and_flags`]: Extract octant path and flags from metadata

mod error;
mod varint;

pub mod indices;
pub mod normals;
pub mod obb;
pub mod octants;
pub mod path;
pub mod texcoords;
pub mod vertices;

pub use error::{DecodeError, DecodeResult};
pub use indices::unpack_indices;
pub use normals::{unpack_for_normals, unpack_normals};
pub use obb::unpack_obb;
pub use octants::unpack_octant_mask_and_layer_bounds;
pub use path::unpack_path_and_flags;
pub use texcoords::unpack_tex_coords;
pub use varint::read_varint;
pub use vertices::unpack_vertices;

/// Maximum octree depth level.
pub const MAX_LEVEL: usize = 20;

/// Packed vertex structure (8 bytes per vertex).
///
/// This matches the GPU vertex format used for rendering:
/// - `x`, `y`, `z`: 8-bit position components (delta-decoded)
/// - `w`: Octant mask (which of 8 sub-octants this vertex belongs to)
/// - `u`, `v`: 16-bit texture coordinates
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C, packed)]
pub struct Vertex {
    pub x: u8,
    pub y: u8,
    pub z: u8,
    pub w: u8,
    pub u: u16,
    pub v: u16,
}

const _: () = assert!(std::mem::size_of::<Vertex>() == 8);

/// UV offset and scale for texture coordinate mapping.
#[derive(Debug, Clone, Copy, Default)]
pub struct UvTransform {
    pub offset: glam::Vec2,
    pub scale: glam::Vec2,
}

/// Oriented bounding box for frustum culling.
#[derive(Debug, Clone, Copy)]
pub struct OrientedBoundingBox {
    pub center: glam::DVec3,
    pub extents: glam::DVec3,
    pub orientation: glam::DMat3,
}

/// Result of unpacking path and flags from node metadata.
#[derive(Debug, Clone)]
pub struct PathAndFlags {
    /// Octant path string (e.g., "01234567").
    pub path: String,
    /// Flags from the node metadata.
    pub flags: u32,
    /// Path level (1-4 for relative paths).
    pub level: usize,
}
