// ufbx's own `Error` is the large one
#![allow(clippy::result_large_err)]

pub mod api;
pub mod bake;
pub mod cache;
pub mod common;
pub mod evaluate;
pub mod inflate;
pub mod io;
pub mod runner;
pub mod scene;
pub mod subdivide;

pub use scene::{compare_scenes, convert_scene};

use runner::Section;

pub const SECTIONS: &[Section] = &[
    scene::SCENE,
    scene::MUTANTS,
    io::IO,
    inflate::INFLATE,
    api::API,
    bake::BAKE,
    cache::CACHE,
    evaluate::EVALUATE,
    scene::OBJ,
    subdivide::SUBDIVIDE,
    scene::UPDATE,
    scene::LEGACY,
];
