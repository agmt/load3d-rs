# load3d's features

Each format is a feature, so that a build carries only the readers it needs. `format-fbx`, `format-obj`,
`geometry-cache` and `skinning-evaluation` are on by default, the last three after ufbx's `UFBX_ENABLE_*`; FBX, which
ufbx always builds, is a feature here too, so that a build that reads only `.obj` or `.3ds` files leaves its reader
out. A load that needs one turned off fails with "Feature disabled", its info the feature's name.
`format-3ds`, on by default too, is beyond ufbx: turned off, no file is taken for a 3DS, and the crate reads every file
as ufbx does. `flatten`, off by default, holds "The short way in".

```toml
# 3D Studio and FBX files only
load3d = { version = "0.1", default-features = false, features = ["format-fbx", "format-3ds"] }
```

## The short way in

Most programs want a file's triangles where the file places them, whatever its format. `flatten` gives one `FlatMesh`
for each node that shows a mesh: the mesh's positions placed, its faces cut into triangles, each triangle's face and
material, and the node and the mesh it came from, so that their names, animation, normals and UVs stay reachable
(`examples/flatten.rs`).

These are not ufbx's, so they keep no name of its, no oracle holds them to it, and they come with the feature
`flatten`, off by default:

- `flatten` cuts faces by `triangulate_face`; `flatten_with` takes a program's own cut. Under a mirroring node two
  corners are swapped, so that a triangle faces the way it did. `FlatMesh::material_groups` gives each material's
  triangles.
- `Material::diffuse` is the plain colour, in any format.
- `detect_format` tells a file's format by its content, in the order a load tries them.
- `SceneSettings::stated_unit` is the unit the file itself states, where `unit_meters` may be ufbx's default.

## ufbx-rust's API

A program written for the [`ufbx`](https://crates.io/crates/ufbx) crate builds on this one with the feature
`ufbx-compat`, off by default, and one line, `use load3d::compat as ufbx;`. `src/compat/` holds ufbx-rust's types,
functions and signatures. All of the crate's `unsafe` is there: in the pointers between its elements, in callbacks that
borrow, and in the C function pointers and `FILE *` that ufbx-rust's API names. The compiler refuses it anywhere else,
and anywhere at all in a build without the feature.

What has no meaning here is accepted and does nothing: the allocators and the thread pool. An allocator's
`memory_limit` is held against the call's finished result, not against each allocation. A stream is read to its end, so
its `skip` is never called. ufbx-rust's `*_raw` functions and `Raw*Opts`, C's own calling convention, are left out. A 3D
Studio file, which ufbx-rust has no name for, reads as `FileFormat::Unknown`.

## 3D Studio files, beyond ufbx

A file starting with the main chunk's id (`0x4D4D`) and taken for none of ufbx's formats, or named `.3ds`, loads as
`FileFormat::Tds` into the scene ufbx would build of an `.obj` of the same content:

- each triangle mesh (chunk `0x4100`) a node and a mesh named after its object, under the root: its vertices, its
  triangles, its texture coordinates, its smoothing groups (`face_smoothing_groups`, and `face_smoothing` where a
  face is in one), and its mesh matrix (`mesh_matrix`, the object's own axes as it was built; the vertices are stored
  already placed);
- each material one for its name, the last definition taking it, with ufbx's FBX property names: `AmbientColor`,
  `DiffuseColor` and `SpecularColor` (the plain colour, else the gamma-corrected one), `TransparencyFactor` (a whole
  percentage over 100; a float one, chunk `0x0031`, is passed over, its range not settled), and its texture, specular,
  opacity, reflection and bump maps as textures connected to `DiffuseColor`, `SpecularColor`, `TransparentColor`,
  `ReflectionColor` and `Bump`; a face's material is the last group's that names it, a group's unknown material made
  on its mention; a face in no group, beside faces of its object in one, is in a material of no colour,
  `DefaultMaterial`, as Assimp gives such a face a default material, rather than in the node's first;
- the axes 3D Studio's own, right-handed with z up (`tds_axes` sets others), the unit unknown (`tds_unit_meters` sets
  one), as `obj_axes` and `obj_unit_meters` do for an `.obj`; a corner past its vertices is fixed as
  `index_error_handling` says, and an object with triangles and no vertices fails the load, as an `.obj` with faces
  and no vertices does.

The keyframer's hierarchy and pivots (chunk `0xB000`), lights and cameras are not read.
