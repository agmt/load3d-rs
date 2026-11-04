# load3d-rs

A reader of 3D files in Rust alone:
FBX, binary and text, of versions 3.0 to 7.7;
Wavefront `.obj` and `.mtl`;
and 3D Studio `.3ds`.
A load gives the file's scene:
its nodes and their transforms, meshes, materials and textures, lights, cameras, skinning, blend shapes and animation, with ufbx's tools over it:
animation evaluation and baking, CPU skinning, triangulation, topology, subdivision, NURBS and geometry caches (`.mc`, `.mcx`, `.pc2`).

## Why

[ufbx](https://github.com/ufbx/ufbx) is a C library that reads FBX and `.obj` files, tested on its own collection of files exported by the modelling programs.
Rust reaches it through the [`ufbx`](https://crates.io/crates/ufbx) crate, which compiles its C and calls it through `unsafe` code. load3d is ufbx 0.23.1 ported whole to Rust, function by function under ufbx's names, with no C, and no `unsafe` outside the optional layer of the `ufbx` crate's API:

- a program needs no C compiler, and builds for the browser's `wasm32-unknown-unknown` as for any other target;
- a load gives ufbx's scene, warnings and errors bit for bit, checked against ufbx's C over ufbx's own test files ([TESTING.md](TESTING.md)), so ufbx's documentation holds for it ([PORTING.md](PORTING.md) says where Rust differs);
- each format is a feature, so that a build carries only the readers it needs, and a program written for the `ufbx` crate builds on this one ([FEATURES.md](FEATURES.md)).

It is slower than ufbx:
over the 1,010 of ufbx's test files that both load, a load takes 1.6 times ufbx's time, 1.96 for binary FBX, 1.53 for text FBX and 1.22 for `.obj`.

## Using it

- [`examples/load.rs`](examples/load.rs) loads a file and lists its meshes, placed in the world:
  `cargo run --example load -- model.fbx`;
- [`examples/flatten.rs`](examples/flatten.rs) gives each mesh's triangles and their materials, whatever the format:
  `cargo run --example flatten --features flatten -- model.fbx`;
- [`examples/viewer`](examples/viewer) draws a file in the browser, at https://agmt.github.io/load3d-rs/.
