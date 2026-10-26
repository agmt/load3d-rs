//! What has no counterpart in the port is left out where it stands: the thread pool, ufbx's allocators and their counts,
//! `ufbx_retain_*` and `ufbx_free_*`.
#![allow(dead_code, unused_imports, unused_macros)]

macro_rules! ufbxt_assert {
    ($cond:expr) => {
        if !($cond) {
            $crate::ufbxt::runner::assert_fail_imp(file!(), line!(), stringify!($cond), true);
        }
    };
}

/// Counted rather than failed where a test expects its diff to fail
macro_rules! ufbxt_soft_assert {
    ($cond:expr) => {
        if !($cond) {
            $crate::ufbxt::runner::assert_fail_imp(file!(), line!(), stringify!($cond), false);
        }
    };
}

macro_rules! ufbxt_hintf {
    ($($arg:tt)*) => {
        $crate::ufbxt::runner::set_hint(format!($($arg)*))
    };
}

macro_rules! ufbxt_logf {
    ($($arg:tt)*) => {
        eprintln!($($arg)*)
    };
}

/// `file = "..."` is `_ALT` (the test's file under another name), `path = "..."` `_PATH`, `suffix = "..."` `_SUFFIX`,
/// `opts = ...` `_OPTS`, `flags = ...` `_FLAGS`. Attributes before the name (`#[ignore = "..."]`) go on the test.
macro_rules! file_test {
    ($(#[$attr:meta])* $name:ident $(, $key:ident = $value:expr)*, |$scene:ident, $err:ident, $load_error:ident| $body:block) => {
        $(#[$attr])*
        #[test]
        fn $name() {
            #[allow(unused_mut)]
            let mut spec = $crate::ufbxt::runner::FileTest::new(stringify!($name));
            $( spec.$key($value); )*
            spec.run(&|scene: Option<&$crate::ufbxt::Scene>, #[allow(unused_variables)] $err: &mut $crate::ufbxt::DiffError, #[allow(unused_variables)] $load_error: &$crate::ufbxt::Error| {
                #[allow(unused_variables)]
                let $scene: &$crate::ufbxt::Scene = scene.expect("the runner gives a scene to a test that allows no error");
                $body
            });
        }
    };
}

/// The scene is `None` where the file did not load
macro_rules! file_test_allow_error {
    ($(#[$attr:meta])* $name:ident $(, $key:ident = $value:expr)*, |$scene:ident, $err:ident, $load_error:ident| $body:block) => {
        $(#[$attr])*
        #[test]
        fn $name() {
            let mut spec = $crate::ufbxt::runner::FileTest::new(stringify!($name));
            spec.flags($crate::ufbxt::runner::ALLOW_ERROR);
            $( spec.$key($value); )*
            spec.run(&|#[allow(unused_variables)] $scene: Option<&$crate::ufbxt::Scene>, #[allow(unused_variables)] $err: &mut $crate::ufbxt::DiffError, #[allow(unused_variables)] $load_error: &$crate::ufbxt::Error| $body);
        }
    };
}

/// The body once for each of the runner's decoder settings
macro_rules! deflate_test {
    ($(#[$attr:meta])* $name:ident, |$opts:ident| $body:block) => {
        $(#[$attr])*
        #[test]
        fn $name() {
            $crate::ufbxt::runner::do_deflate_test(stringify!($name), &|$opts: &$crate::ufbxt::runner::InflateTestOpts| $body);
        }
    };
}

pub(crate) use {deflate_test, file_test, file_test_allow_error, ufbxt_assert, ufbxt_hintf, ufbxt_logf, ufbxt_soft_assert};

// after the macros, which a module sees only where they are defined above it
pub mod check_scene;
pub mod fuzz_checks;
pub mod runner;
pub mod utils;

pub use check_scene::{check_anim, check_mesh, check_scene, check_string};
pub use load3d::*;
pub use runner::*;
pub use utils::*;
