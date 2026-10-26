mod ufbxt;
use ufbxt::*;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const ERROR_INFO_LENGTH: usize = 256;

/// `ufbx_panic` in ufbx.h
fn catch_panic<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|p| match p.downcast::<String>() {
        Ok(message) => *message,
        Err(p) => p.downcast_ref::<&str>().map(|s| s.to_string()).unwrap_or_default(),
    })
}

fn data_root_str() -> String {
    let mut root = data_root().to_string_lossy().into_owned();
    if !root.ends_with('/') {
        root.push('/');
    }
    root
}

// `open_file_cb` returns the bytes here, so one callback serves ufbx's three
fn do_open_memory_test(filename: &str, expected_calls_fbx: usize, expected_calls_obj: usize) {
    for path in FileIterator::new(filename) {
        for i in 0..2 {
            let num_calls = Arc::new(AtomicUsize::new(0));
            let calls = num_calls.clone();
            let mut opts = LoadOpts {
                open_file_cb: Some(Arc::new(move |path: &str, _info: &OpenFileInfo| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    read_file(path)
                })),
                load_external_files: true,
                ..Default::default()
            };
            if i == 1 {
                opts.read_buffer_size = 1;
            }

            let scene = load_file(&path, &opts);
            if let Err(e) = &scene {
                log_error(e);
            }
            ufbxt_assert!(scene.is_ok());
            let scene = scene.unwrap();

            check_scene(&scene);

            let num_calls = num_calls.load(Ordering::SeqCst);
            if scene.metadata.file_format == FileFormat::Fbx {
                ufbxt_assert!(num_calls == expected_calls_fbx);
            } else if scene.metadata.file_format == FileFormat::Obj {
                ufbxt_assert!(num_calls == expected_calls_obj);
            } else {
                ufbxt_assert!(false);
            }
        }
    }
}

#[test]
fn open_memory_default() {
    do_open_memory_test("maya_cache_sine", 5, 0);
}

#[test]
fn open_memory_ctx() {
    do_open_memory_test("maya_cache_sine", 5, 0);
}

#[test]
fn open_memory_ref() {
    do_open_memory_test("maya_cache_sine", 5, 0);
}

#[test]
fn obj_open_memory_default() {
    do_open_memory_test("blender_279_ball", 1, 2);
}

#[test]
fn obj_open_memory_ctx() {
    do_open_memory_test("blender_279_ball", 1, 2);
}

#[test]
fn obj_open_memory_ref() {
    do_open_memory_test("blender_279_ball", 1, 2);
}

#[test]
fn retain_free_null() {
}

#[test]
fn thread_memory() {
}

#[test]
fn single_thread_immediate_stream() {
    for path in FileIterator::new("blender_293_barbarian") {
        let scene = load_file(&path, &LoadOpts::default());
        if let Err(e) = &scene {
            log_error(e);
        }
        ufbxt_assert!(scene.is_ok());

        check_scene(&scene.unwrap());
    }
}

#[test]
fn single_thread_immediate_memory() {
    for path in FileIterator::new("blender_293_barbarian") {
        let data = read_file(&path);
        ufbxt_assert!(data.is_some());

        let scene = load_memory(&data.unwrap(), &LoadOpts::default());
        if let Err(e) = &scene {
            log_error(e);
        }
        ufbxt_assert!(scene.is_ok());

        check_scene(&scene.unwrap());
    }
}

#[test]
fn single_thread_deferred_stream() {
    for path in FileIterator::new("blender_293_barbarian") {
        let scene = load_file(&path, &LoadOpts::default());
        if let Err(e) = &scene {
            log_error(e);
        }
        ufbxt_assert!(scene.is_ok());

        check_scene(&scene.unwrap());
    }
}

#[test]
fn single_thread_deferred_memory() {
    for path in FileIterator::new("blender_293_barbarian") {
        let data = read_file(&path);
        ufbxt_assert!(data.is_some());

        let scene = load_memory(&data.unwrap(), &LoadOpts::default());
        if let Err(e) = &scene {
            log_error(e);
        }
        ufbxt_assert!(scene.is_ok());

        check_scene(&scene.unwrap());
    }
}

#[test]
fn thread_memory_limit() {
    for path in FileIterator::new("blender_293_barbarian") {
        let data = read_file(&path);
        ufbxt_assert!(data.is_some());

        let scene = load_memory(&data.unwrap(), &LoadOpts::default());
        if let Err(e) = &scene {
            log_error(e);
        }
        ufbxt_assert!(scene.is_ok());

        check_scene(&scene.unwrap());
    }
}

#[test]
fn single_thread_file_not_found() {
    let scene = load_file("<doesnotexist>.fbx", &LoadOpts::default());
    ufbxt_assert!(scene.is_err());
    let error = scene.unwrap_err();
    ufbxt_assert!(error.kind == ErrorType::FileNotFound);
    ufbxt_assert!(error.info.contains("<doesnotexist>.fbx"));
}

#[test]
fn empty_file_memory() {
    {
        let scene = load_memory(&[], &LoadOpts::default());
        ufbxt_assert!(scene.is_err());
        ufbxt_assert!(scene.unwrap_err().kind == ErrorType::EmptyFile);
    }

    {
        let opts = LoadOpts { file_format: FileFormat::Fbx, ..Default::default() };
        let scene = load_memory(&[], &opts);
        ufbxt_assert!(scene.is_err());
        ufbxt_assert!(scene.unwrap_err().kind == ErrorType::EmptyFile);
    }
}

#[test]
fn empty_file_stream() {
    {
        let scene = load_stream(&mut std::io::empty(), &LoadOpts::default());
        ufbxt_assert!(scene.is_err());
        ufbxt_assert!(scene.unwrap_err().kind == ErrorType::EmptyFile);
    }

    {
        let opts = LoadOpts { file_format: FileFormat::Fbx, ..Default::default() };
        let scene = load_stream(&mut std::io::empty(), &opts);
        ufbxt_assert!(scene.is_err());
        ufbxt_assert!(scene.unwrap_err().kind == ErrorType::EmptyFile);
    }
}

#[test]
fn file_format_lookahead() {
    for path in FileIterator::new("maya_cube") {
        for i in 0..=16 {
            ufbxt_hintf!("i={i}");

            let opts = LoadOpts { file_format_lookahead: i * i * i, ..Default::default() };

            let scene = load_file(&path, &opts);
            if let Err(e) = &scene {
                log_error(e);
            }
            ufbxt_assert!(scene.is_ok());

            check_scene(&scene.unwrap());
        }
    }
}

#[test]
fn multiuse_realloc() {
    for path in FileIterator::new("maya_cube") {
        let scene = load_file(&path, &LoadOpts::default());
        if let Err(e) = &scene {
            log_error(e);
        }
        ufbxt_assert!(scene.is_ok());
        check_scene(&scene.unwrap());
    }
}

#[test]
fn null_realloc() {
}

#[test]
fn null_realloc_only() {
}

#[test]
fn element_type_sizes() {
}

#[test]
fn evaluate_anim_null() {
}

#[test]
fn catch_triangulate() {
    for path in FileIterator::new("maya_cube") {
        let scene = load_file(&path, &LoadOpts::default());
        ufbxt_assert!(scene.is_ok());
        let scene = scene.unwrap();

        let node = find_node(&scene, "pCube1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
        let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
        let face = mesh.faces[0];
        ufbxt_assert!(face.index_begin == 0);
        ufbxt_assert!(face.num_indices == 4);

        // `ufbx_catch_*` is the plain function here, which panics where ufbx's sets `panic.did_panic` and returns 0
        {
            let mut indices = [0u32; 6];
            let num_tris = catch_panic(|| triangulate_face(&mut indices, mesh, face));
            ufbxt_assert!(num_tris == Ok(2));
        }

        {
            let mut indices = [0u32; 4];
            let num_tris = catch_panic(|| triangulate_face(&mut indices, mesh, face));
            ufbxt_assert!(num_tris == Err("Face needs at least 6 indices for triangles, got space for 4".to_string()));
        }

        {
            let bad_face = Face { index_begin: 100, num_indices: 4 };
            let mut indices = [0u32; 6];
            let num_tris = catch_panic(|| triangulate_face(&mut indices, mesh, bad_face));
            ufbxt_assert!(num_tris == Err("Face index begin (100) out of bounds (24)".to_string()));
        }

        {
            let bad_face = Face { index_begin: 22, num_indices: 4 };
            let mut indices = [0u32; 6];
            let num_tris = catch_panic(|| triangulate_face(&mut indices, mesh, bad_face));
            ufbxt_assert!(num_tris == Err("Face index end (22 + 4) out of bounds (24)".to_string()));
        }
    }
}

#[test]
fn catch_face_normal() {
    for path in FileIterator::new("maya_cube") {
        let scene = load_file(&path, &LoadOpts::default());
        ufbxt_assert!(scene.is_ok());
        let scene = scene.unwrap();

        let node = find_node(&scene, "pCube1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
        let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
        let face = mesh.faces[0];
        ufbxt_assert!(face.index_begin == 0);
        ufbxt_assert!(face.num_indices == 4);

        {
            let result = catch_panic(|| get_weighted_face_normal(&mesh.vertex_position, face));
            ufbxt_assert!(result.is_ok());
        }

        {
            let bad_face = Face { index_begin: 100, num_indices: 4 };
            let result = catch_panic(|| get_weighted_face_normal(&mesh.vertex_position, bad_face));
            ufbxt_assert!(result.is_err());
            ufbxt_assert!(result.unwrap_err() == "Face index begin (100) out of bounds (24)");
        }

        {
            let bad_face = Face { index_begin: 22, num_indices: 4 };
            let result = catch_panic(|| get_weighted_face_normal(&mesh.vertex_position, bad_face));
            ufbxt_assert!(result.is_err());
            ufbxt_assert!(result.unwrap_err() == "Face index end (22 + 4) out of bounds (24)");
        }
    }
}

#[test]
fn cache_fail_skip() {
    let buf = data_path("caches/sine_mxsf_regular/cache.xml");

    let cache = load_geometry_cache(&buf, &GeometryCacheOpts::default());
    ufbxt_assert!(cache.is_ok());
    let cache = cache.unwrap();

    ufbxt_assert!(cache.frames.len() > 2);
    let frame = &cache.frames[1];
    let mut data = [0.0 as Real; 256];

    {
        let num_read = read_geometry_cache_real(frame, &mut data, &GeometryCacheDataOpts::default());
        ufbxt_assert!(num_read == 108);
    }

    {
        let opts = GeometryCacheDataOpts { open_file_cb: Some(Arc::new(|_: &str, _: &OpenFileInfo| None)), ..Default::default() };
        let num_read = read_geometry_cache_real(frame, &mut data, &opts);
        ufbxt_assert!(num_read == 0);
    }
}

fn retain_w_opts() -> LoadOpts {
    LoadOpts { retain_vertex_attrib_w: true, ..Default::default() }
}

file_test!(catch_get_vertex_vec, file = "maya_color_sets", opts = retain_w_opts(), |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];

    for i in 0..mesh.num_indices {
        ufbxt_assert!(mesh.vertex_position.catch_get(i).is_ok());
        ufbxt_assert!(mesh.vertex_uv.catch_get(i).is_ok());
        ufbxt_assert!(mesh.vertex_color.catch_get(i).is_ok());
        let w = mesh.vertex_tangent.catch_get_w(i);
        ufbxt_assert!(w.is_ok());
        if scene.metadata.version >= 7000 {
            ufbxt_assert!(w.unwrap() == 1.0);
        } else {
            ufbxt_assert!(w.unwrap() == 0.0);
        }
    }

    {
        let index = 24;
        let position = mesh.vertex_position.catch_get(index);
        ufbxt_assert!(position.is_err());
        ufbxt_assert!(position.unwrap_err().description == "index (24) out of range (24)");
        let uv = mesh.vertex_uv.catch_get(index);
        ufbxt_assert!(uv.is_err());
        ufbxt_assert!(uv.unwrap_err().description == "index (24) out of range (24)");
        let color = mesh.vertex_color.catch_get(index);
        ufbxt_assert!(color.is_err());
        ufbxt_assert!(color.unwrap_err().description == "index (24) out of range (24)");
        let w = mesh.vertex_tangent.catch_get_w(index);
        ufbxt_assert!(w.is_err());
        ufbxt_assert!(w.unwrap_err().description == "index (24) out of range (24)");
    }
});

file_test!(catch_get_vertex_real, file = "maya_vertex_crease", |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];

    for i in 0..mesh.num_indices {
        ufbxt_assert!(mesh.vertex_crease.catch_get(i).is_ok());
    }

    {
        let index = 24;
        let crease = mesh.vertex_crease.catch_get(index);
        ufbxt_assert!(crease.is_err());
        ufbxt_assert!(crease.unwrap_err().description == "index (24) out of range (24)");
    }
});

file_test!(tessellate_surface_overflow, file = "maya_nurbs_surface_plane", |scene, err, load_error| {
    let node = find_node(scene, "nurbsPlane1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::NurbsSurface));
    let surface = node.unwrap().attrib.and_then(|a| scene.as_nurbs_surface(a));
    ufbxt_assert!(surface.is_some());
    let surface = surface.unwrap();

    {
        let mesh = tessellate_nurbs_surface(surface, &TessellateSurfaceOpts::default());
        ufbxt_assert!(mesh.is_ok());
    }

    for i in 1..=size_of::<Real>() {
        let opts = TessellateSurfaceOpts { span_subdivision_u: usize::MAX / i, ..Default::default() };
        let mesh = tessellate_nurbs_surface(surface, &opts);
        ufbxt_assert!(mesh.is_err());
        ufbxt_assert!(mesh.unwrap_err().kind == ErrorType::Unknown);
    }

    for i in 1..=size_of::<Real>() {
        let opts = TessellateSurfaceOpts { span_subdivision_v: usize::MAX / i, ..Default::default() };
        let mesh = tessellate_nurbs_surface(surface, &opts);
        ufbxt_assert!(mesh.is_err());
        ufbxt_assert!(mesh.unwrap_err().kind == ErrorType::Unknown);
    }

    for i in 1..=size_of::<Real>() / 2 {
        let span = (usize::MAX as f64).sqrt() as usize / i;
        let opts = TessellateSurfaceOpts { span_subdivision_u: span, span_subdivision_v: span, ..Default::default() };
        let mesh = tessellate_nurbs_surface(surface, &opts);
        ufbxt_assert!(mesh.is_err());
        ufbxt_assert!(mesh.unwrap_err().kind == ErrorType::Unknown);
    }
});

file_test!(tessellate_curve_overflow, file = "max_nurbs_to_line", |scene, err, load_error| {
    let nurbs_node = find_node(scene, "Nurbs").map(|n| &scene.nodes[n]);
    ufbxt_assert!(nurbs_node.is_some());
    let nurbs = nurbs_node.unwrap().attrib.and_then(|a| scene.as_nurbs_curve(a));
    ufbxt_assert!(nurbs.is_some());
    let nurbs = nurbs.unwrap();

    {
        let tess_line = tessellate_nurbs_curve(nurbs, &TessellateCurveOpts::default());
        ufbxt_assert!(tess_line.is_ok());
    }

    for i in 1..=size_of::<Real>() {
        let opts = TessellateCurveOpts { span_subdivision: usize::MAX / i };
        let line = tessellate_nurbs_curve(nurbs, &opts);
        ufbxt_assert!(line.is_err());
        ufbxt_assert!(line.unwrap_err().kind == ErrorType::Unknown);
    }
});

#[test]
fn coordinate_axes_valid() {
    use CoordinateAxis::{PositiveX, PositiveY, PositiveZ};

    // `(ufbx_coordinate_axis)-1` and `100` are the axis ufbx reads for an index out of range
    let bad_low = CoordinateAxis::from_index(-1);
    let bad_high = CoordinateAxis::from_index(100);

    {
        let axes = CoordinateAxes { right: PositiveX, up: PositiveY, front: PositiveZ };
        ufbxt_assert!(load3d::coordinate_axes_valid(axes));
    }

    {
        let axes = CoordinateAxes { right: PositiveX, up: PositiveX, front: PositiveZ };
        ufbxt_assert!(!load3d::coordinate_axes_valid(axes));
    }

    {
        let axes = CoordinateAxes { right: bad_low, up: PositiveY, front: PositiveZ };
        ufbxt_assert!(!load3d::coordinate_axes_valid(axes));
    }

    {
        let axes = CoordinateAxes { right: bad_high, up: PositiveY, front: PositiveZ };
        ufbxt_assert!(!load3d::coordinate_axes_valid(axes));
    }

    {
        let axes = CoordinateAxes { right: PositiveX, up: bad_low, front: PositiveZ };
        ufbxt_assert!(!load3d::coordinate_axes_valid(axes));
    }

    {
        let axes = CoordinateAxes { right: PositiveX, up: bad_high, front: PositiveZ };
        ufbxt_assert!(!load3d::coordinate_axes_valid(axes));
    }

    {
        let axes = CoordinateAxes { right: PositiveX, up: PositiveY, front: bad_low };
        ufbxt_assert!(!load3d::coordinate_axes_valid(axes));
    }

    {
        let axes = CoordinateAxes { right: PositiveX, up: PositiveY, front: bad_high };
        ufbxt_assert!(!load3d::coordinate_axes_valid(axes));
    }
}

/// `ufbxt_fail_read_fn` in test/test_api.h
struct FailRead;

impl std::io::Read for FailRead {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("read failed"))
    }
}

#[test]
fn io_error() {
    let scene = load_stream(&mut FailRead, &LoadOpts::default());
    ufbxt_assert!(scene.is_err());
    ufbxt_assert!(scene.unwrap_err().kind == ErrorType::Io);
}

#[test]
fn huge_filename() {
    let filename_size = 2048;
    let mut filename = data_root_str();
    ufbxt_assert!(filename.len() < filename_size / 2);

    let path = "maya_cube_7500_ascii.fbx";
    while filename.len() + path.len() < filename_size - 8 {
        filename.push_str("./");
    }
    filename.push_str(path);

    let scene = load_file(&filename, &LoadOpts::default());
    if let Err(e) = &scene {
        log_error(e);
    }
    ufbxt_assert!(scene.is_ok());

    check_scene(&scene.unwrap());
}

#[test]
fn huge_filename_not_found() {
    let filename_size = 2048;
    let mut filename = data_root_str();
    ufbxt_assert!(filename.len() < filename_size / 2);

    let path = "does_not_exist.fbx";
    while filename.len() + path.len() < filename_size - 8 {
        filename.push_str("./");
    }
    filename.push_str(path);

    let scene = load_file(&filename, &LoadOpts::default());
    ufbxt_assert!(scene.is_err());
    let error = scene.unwrap_err();

    ufbxt_assert!(error.kind == ErrorType::FileNotFound);
    ufbxt_assert!(!error.info.contains('\0'));
    // ToDo: expected: ERROR_INFO_LENGTH - 1
    ufbxt_assert!(error.info.len() == filename.len());
}

fn missing_obj_mtl_path_opts() -> LoadOpts {
    LoadOpts { obj_mtl_path: "nonexistent_path.mtl".to_string(), ignore_missing_external_files: true, ..Default::default() }
}

file_test!(
    missing_explicit_mtl,
    file = "blender_279_ball",
    opts = missing_obj_mtl_path_opts(),
    flags = FUZZ_OPTS | FUZZ_ALWAYS | ALLOW_WARNINGS,
    |scene, err, load_error| {
        if scene.metadata.file_format == FileFormat::Obj {
            check_warning(scene, WarningType::MissingExternalFile, ElementType::Unknown, None, 1, Some("nonexistent_path.mtl"));
        }
    }
);

#[test]
fn load_scene_bad_opts() {
}

#[test]
fn subdivide_bad_opts() {
    for path in FileIterator::new("maya_cube") {
        let scene = load_file(&path, &LoadOpts::default());
        ufbxt_assert!(scene.is_ok());
        let scene = scene.unwrap();
        ufbxt_assert!(!scene.meshes.is_empty());
    }
}

#[test]
fn evaluate_bad_opts() {
    for path in FileIterator::new("maya_cube") {
        let scene = load_file(&path, &LoadOpts::default());
        ufbxt_assert!(scene.is_ok());
    }
}

#[test]
fn bake_bad_opts() {
    for path in FileIterator::new("maya_cube") {
        let scene = load_file(&path, &LoadOpts::default());
        ufbxt_assert!(scene.is_ok());
    }
}

#[test]
fn tessellate_curve_bad_opts() {
    for path in FileIterator::new("max_nurbs_to_line") {
        let scene = load_file(&path, &LoadOpts::default());
        ufbxt_assert!(scene.is_ok());
        let scene = scene.unwrap();
        ufbxt_assert!(!scene.nurbs_curves.is_empty());
    }
}

#[test]
fn tessellate_surface_bad_opts() {
    for path in FileIterator::new("maya_nurbs_surface_plane") {
        let scene = load_file(&path, &LoadOpts::default());
        ufbxt_assert!(scene.is_ok());
        let scene = scene.unwrap();
        ufbxt_assert!(!scene.nurbs_surfaces.is_empty());
    }
}

#[test]
fn load_cache_bad_opts() {
}

#[test]
fn read_cache_bad_opts() {
    let buf = data_path("caches/sine_mxsf_regular/cache.xml");

    let cache = load_geometry_cache(&buf, &GeometryCacheOpts::default());
    ufbxt_assert!(cache.is_ok());
    let cache = cache.unwrap();
    ufbxt_assert!(!cache.frames.is_empty());
    let mut data = [Vec3::ZERO; 512];

    let result_ok = read_geometry_cache_vec3(&cache.frames[0], &mut data, &GeometryCacheDataOpts::default());
    ufbxt_assert!(result_ok == 36);
}

#[test]
fn sample_cache_bad_opts() {
    let buf = data_path("caches/sine_mxsf_regular/cache.xml");

    let cache = load_geometry_cache(&buf, &GeometryCacheOpts::default());
    ufbxt_assert!(cache.is_ok());
    let cache = cache.unwrap();
    ufbxt_assert!(!cache.channels.is_empty());
    let mut data = [Vec3::ZERO; 512];

    let result_ok = sample_geometry_cache_vec3(&cache.channels[0], 0.0, &mut data, &GeometryCacheDataOpts::default());
    ufbxt_assert!(result_ok == 36);
}

#[test]
fn retain_free_garbage() {
}

file_test!(dom_helper_null, file = "maya_cube", opts = retain_dom_opts(), flags = NO_FUZZ, |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let node = node.unwrap();

    let dom_node = element_dom_node(scene, &node.element);
    ufbxt_assert!(dom_node.is_some());
    let dom_node = dom_node.unwrap();

    ufbxt_assert!(!dom_node.is_array());
    ufbxt_assert!(dom_node.array_size() == 0);
    ufbxt_assert!(dom_node.as_int32_list().is_empty());
    ufbxt_assert!(dom_node.as_int64_list().is_empty());
    ufbxt_assert!(dom_node.as_float_list().is_empty());
    ufbxt_assert!(dom_node.as_double_list().is_empty());
    ufbxt_assert!(dom_node.as_real_list().is_empty());
    ufbxt_assert!(dom_node.as_blob_list().is_empty());

    let dom_version = dom_node.find("Version");
    ufbxt_assert!(dom_version.is_some());
    let dom_version = dom_version.unwrap();

    ufbxt_assert!(!dom_version.is_array());
    ufbxt_assert!(dom_version.array_size() == 0);
    ufbxt_assert!(dom_version.as_int32_list().is_empty());
    ufbxt_assert!(dom_version.as_int64_list().is_empty());
    ufbxt_assert!(dom_version.as_float_list().is_empty());
    ufbxt_assert!(dom_version.as_double_list().is_empty());
    ufbxt_assert!(dom_version.as_real_list().is_empty());
    ufbxt_assert!(dom_version.as_blob_list().is_empty());

    ufbxt_assert!(node.mesh.is_some());
    let mesh = &scene.meshes[node.mesh.unwrap()];

    let dom_mesh = element_dom_node(scene, &mesh.element);
    ufbxt_assert!(dom_mesh.is_some());
    let dom_mesh = dom_mesh.unwrap();

    if scene.metadata.version < 7000 {
        ufbxt_assert!(std::ptr::eq(dom_mesh, dom_node));
    } else {
        ufbxt_assert!(!std::ptr::eq(dom_mesh, dom_node));
    }

    let dom_vertices = dom_mesh.find("Vertices");
    ufbxt_assert!(dom_vertices.is_some_and(|d| d.is_array()));
    ufbxt_assert!(dom_vertices.is_some_and(|d| d.array_size() == 24));
});

fn allow_invalid_unicode_opts() -> LoadOpts {
    LoadOpts { allow_unsafe: true, unicode_error_handling: UnicodeErrorHandling::UnsafeIgnore, ..Default::default() }
}

#[test]
fn bad_path() {
}

file_test_allow_error!(casegen_bad_path, opts = allow_invalid_unicode_opts(), flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    ufbxt_assert!(load_error.kind == ErrorType::ExternalFileNotFound);
    ufbxt_assert!(scene.is_none());
});

file_test_allow_error!(synthetic_bad_path, |scene, err, load_error| {
    ufbxt_assert!(load_error.kind == ErrorType::ExternalFileNotFound);
    ufbxt_assert!(scene.is_none());
});
