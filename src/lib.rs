//! A reader of 3D files in Rust: FBX, .obj and .mtl as ufbx 0.23.1 reads them, a port of it
//! (<https://github.com/ufbx/ufbx>, MIT or Unlicense), and 3D Studio (.3ds) files besides.
#![forbid(unsafe_code)]
// The FBX reader's fields stay in the types every format shares, unread without format-fbx, and a build of no format
// leaves the readers' shared helpers unused
#![cfg_attr(not(any(feature = "format-fbx", feature = "format-obj", feature = "format-3ds")), allow(dead_code))]

pub(crate) mod api;
pub(crate) mod bake;
#[cfg(feature = "geometry-cache")]
pub(crate) mod cache;
pub(crate) mod context;
pub mod error;
pub(crate) mod evaluate;
pub(crate) mod external;
pub(crate) mod finalize;
pub(crate) mod inflate;
#[cfg(feature = "format-fbx")]
pub(crate) mod legacy;
pub mod load;
pub(crate) mod matcher;
pub(crate) mod materials;
pub mod math;
pub(crate) mod mesh;
pub(crate) mod nurbs;
#[cfg(feature = "format-obj")]
pub(crate) mod obj;
pub(crate) mod parse;
pub(crate) mod paths;
pub(crate) mod prefinalize;
pub mod props;
#[cfg(feature = "format-fbx")]
pub(crate) mod read;
pub(crate) mod search;
#[cfg(feature = "flatten")]
pub(crate) mod simple;
pub(crate) mod subdivide;
#[cfg(feature = "format-3ds")]
pub(crate) mod tds;
pub(crate) mod textures;
pub(crate) mod topology;
pub mod types;
pub(crate) mod update;
#[cfg(feature = "geometry-cache")]
pub(crate) mod xml;

pub use api::{
    add_blend_shape_vertex_offsets, add_blend_vertex_offsets, find_anim_prop, find_anim_props, find_anim_stack, find_element, find_material, find_node,
    find_prop_element, find_prop_texture, find_shader_prop, find_shader_prop_bindings, find_shader_texture_input, generate_indices,
    get_blend_shape_offset_index, get_blend_shape_vertex_offset, get_blend_vertex_offset, get_bone_pose, get_compatible_matrix_for_normals, get_prop_element,
    get_skin_vertex_matrix, pack_version,
};
pub use bake::{
    bake_anim, evaluate_baked_quat, evaluate_baked_vec3, find_baked_element, find_baked_element_by_element_id, find_baked_node, find_baked_node_by_typed_id,
};
#[cfg(feature = "geometry-cache")]
pub use cache::*;
pub use error::{Error, ErrorType, Warning, WarningType};
pub use evaluate::*;
pub use inflate::{InflateInput, InflateRetain, inflate};
pub use load::{load_file, load_memory, load_stream, load_stream_prefix};
pub use math::*;
pub use nurbs::{evaluate_nurbs_basis, evaluate_nurbs_curve, evaluate_nurbs_surface, tessellate_nurbs_curve, tessellate_nurbs_surface};
pub use paths::default_open_file;
pub use subdivide::*;
pub use topology::*;
pub use types::*;

/// Looks at the first 16 KiB, ufbx's default look-ahead
pub fn is_fbx(data: &[u8]) -> bool {
    matcher::is_format(&data[..data.len().min(0x4000)], FileFormat::Fbx)
}
