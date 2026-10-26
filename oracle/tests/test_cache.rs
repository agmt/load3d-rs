mod ufbxt;
use std::sync::Arc;
use ufbxt::check_scene::check_string;
use ufbxt::*;

fn test_sine_cache(err: &mut DiffError, path: &str, begin: f64, end: f64, err_threshold: f64) {
    let buf = data_path(path);

    let cache = load_geometry_cache(&buf, &GeometryCacheOpts::default());
    ufbxt_assert!(cache.is_ok());
    let cache = cache.unwrap();
    ufbxt_assert!(cache.channels.len() == 2);

    let mut found_cube1 = false;
    for channel in &cache.channels {
        ufbxt_assert!(channel.interpretation == CacheInterpretation::VertexPosition);

        if channel.name == "pCubeShape1" {
            found_cube1 = true;

            let mut pos = [Vec3::default(); 64];
            let mut time = begin;
            while time <= end + 0.0001 {
                let num_verts = sample_geometry_cache_vec3(channel, time, &mut pos, &GeometryCacheDataOpts::default());
                ufbxt_assert!(num_verts == 36);

                // ufbx passes `SIZE_MAX` as the count; the count is the slice's length here, so a slice far longer
                let mut dry = vec![Vec3::default(); 4096];
                let dry_verts = sample_geometry_cache_vec3(channel, time, &mut dry, &GeometryCacheDataOpts::default());
                ufbxt_assert!(num_verts == dry_verts);

                let t = (time - 1.0 / 24.0) / (29.0 / 24.0) * 4.0;
                let pi2 = 3.141592653589793 * 2.0;
                let err_scale = 0.001 / err_threshold;

                for &v in &pos[..num_verts] {
                    let sx = ((v.y + t * 0.5) * pi2).sin() * 0.25;
                    let mut vx = v.x;
                    vx += if vx > 0.0 { -0.5 } else { 0.5 };
                    assert_close_double(err, vx * err_scale, sx * err_scale);
                }
                time += 0.1 / 24.0;
            }
        }
    }

    ufbxt_assert!(found_cube1);
}

file_test!(maya_cache_sine, |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
    ufbxt_assert!(mesh.cache_deformers.len() == 2);
    for i in 0..2 {
        let deformer = &scene.cache_deformers[mesh.cache_deformers[i]];
        ufbxt_assert!(deformer.file.is_some());
        ufbxt_assert!(scene.cache_files[deformer.file.unwrap()].format == CacheFileFormat::Mc);
    }

    check_frame(scene, err, false, "maya_cache_sine_12", None, 12.0 / 24.0);
    check_frame(scene, err, false, "maya_cache_sine_18", None, 18.0 / 24.0);
});

#[test]
fn maya_cache_sine_caches() {
    let mut err = DiffError::default();

    test_sine_cache(&mut err, "caches/sine_mcmf_undersample/cache.xml", 1.0 / 24.0, 29.0 / 24.0, 0.04);
    test_sine_cache(&mut err, "caches/sine_mcsd_oversample/cache.xml", 1.0 / 24.0, 29.0 / 24.0, 0.003);
    test_sine_cache(&mut err, "caches/sine_mxmd_oversample/cache.xml", 11.0 / 24.0, 19.0 / 24.0, 0.003);
    test_sine_cache(&mut err, "caches/sine_mxsf_regular/cache.xml", 1.0 / 24.0, 29.0 / 24.0, 0.008);

    ufbxt_logf!(".. Absolute diff: avg {:.3e}, max {:.3e} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

file_test!(max_cache_box, |scene, err, load_error| {
    let node = find_node(scene, "Box001").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
    ufbxt_assert!(mesh.cache_deformers.len() == 1);
    let deformer = &scene.cache_deformers[mesh.cache_deformers[0]];
    ufbxt_assert!(deformer.file.is_some());
    ufbxt_assert!(scene.cache_files[deformer.file.unwrap()].format == CacheFileFormat::Pc2);
    ufbxt_assert!(deformer.external_cache.is_some());
    ufbxt_assert!(deformer.external_channel.is_some());
    let external_channel = &deformer.external_cache.as_ref().unwrap().channels[deformer.external_channel.unwrap()];
    ufbxt_assert!(external_channel.frames.len() == 11);
    for frame in &external_channel.frames {
        ufbxt_assert!(frame.file_format == CacheFileFormat::Pc2);
    }

    check_frame(scene, err, false, "max_cache_box_44", None, 44.0 / 30.0);
    check_frame(scene, err, false, "max_cache_box_48", None, 48.0 / 30.0);
});

#[test]
fn cache_xml_parse() {
    let buf = data_path("caches/sine_xml_parse/cache.xml");

    let cache = load_geometry_cache(&buf, &GeometryCacheOpts::default());
    ufbxt_assert!(cache.is_ok());
    let cache = cache.unwrap();

    ufbxt_assert!(cache.extra_info.len() == 2);
    check_string(&cache.extra_info[0]);
    check_string(&cache.extra_info[1]);

    let ex0 = "cdata! \"'&<><--&lt;&#x61;<tag></tag>-->!CDATA[]>\x61\u{3b2}\u{30ab}\u{1f602}]]";
    ufbxt_assert!(cache.extra_info[0] == ex0);

    let ex1 = "\"'&<>\x61\x61\x61\u{3b2}\u{3b2}\u{3b2}\u{30ab}\u{30ab}\u{30ab}\u{1f602}\u{1f602}\u{1f602}";
    ufbxt_assert!(cache.extra_info[1] == ex1);

    ufbxt_assert!(cache.channels.len() == 2);
    check_string(&cache.channels[0].name);
    check_string(&cache.channels[1].name);
    check_string(&cache.channels[0].interpretation_name);
    check_string(&cache.channels[1].interpretation_name);
    ufbxt_assert!(cache.channels[0].interpretation == CacheInterpretation::Unknown);
    ufbxt_assert!(cache.channels[1].interpretation == CacheInterpretation::Unknown);

    ufbxt_assert!(cache.channels[0].interpretation_name == "<!--\"positions\"-->");
    ufbxt_assert!(cache.channels[1].interpretation_name == "<![CDATA[<positions>]]>");
}

#[test]
fn cache_xml_depth() {
    let buf = data_path("cache_xml_depth.xml");

    let cache = load_geometry_cache(&buf, &GeometryCacheOpts::default());
    ufbxt_assert!(cache.is_err());
    let error = cache.unwrap_err();
    ufbxt_assert!(error.kind == ErrorType::Unknown);
}

file_test!(marvelous_quad, opts = scale_to_cm_opts(), |scene, err, load_error| {
    {
        let node = find_node(scene, "marvelous_quad").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
        let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
        ufbxt_assert!(mesh.materials.len() == 1);
        let material = &scene.materials[mesh.materials[0].unwrap()];

        // What? Marvelous writes relative filenames as absolute.
        // TODO: Quirk mode to fix this?
        let prefix = "D:\\Dev\\clean\\ufbx\\data\\";

        check_material_texture_ex(scene, material.fbx.ambient_color().texture, prefix, "marvelous_quad_diffuse_100-1.png", true);
        check_material_texture_ex(scene, material.fbx.diffuse_color().texture, prefix, "marvelous_quad_diffuse_100-1.png", true);
        check_material_texture_ex(scene, material.fbx.normal_map().texture, prefix, "marvelous_quad_normal_100-1.png", true);
        check_material_texture_ex(scene, material.pbr.base_color().texture, prefix, "marvelous_quad_diffuse_100-1.png", true);
        check_material_texture_ex(scene, material.pbr.normal_map().texture, prefix, "marvelous_quad_normal_100-1.png", true);
    }

    check_frame(scene, err, false, "marvelous_quad_12", None, 12.0 / 24.0);
    check_frame(scene, err, false, "marvelous_quad_22", None, 22.0 / 24.0);
});

file_test_allow_error!(synthetic_missing_cache_fail, |_scene, err, load_error| {
    ufbxt_assert!(load_error.kind == ErrorType::ExternalFileNotFound);
    ufbxt_assert!(load_error.info == "missing_cache.xml");
});

// The port's callback hands over the whole file, which has no skip to take away
fn open_file_no_skip(path: &str, info: &OpenFileInfo) -> Option<Vec<u8>> {
    default_open_file(path, info)
}

#[test]
fn cache_skip_read() {
    let buffer = data_path("max_cache_box_7500_binary_fpc/max_cache_box.pc2");

    let opts = GeometryCacheOpts { open_file_cb: Some(Arc::new(open_file_no_skip)), ..Default::default() };

    let cache = load_geometry_cache(&buffer, &opts);
    ufbxt_assert!(cache.is_ok());
    let cache = cache.unwrap();
    ufbxt_assert!(cache.frames.len() == 11);

    for _ in 0..cache.frames.len() {
        // sic: ufbx reads the first frame each time
        let frame = &cache.frames[0];
        ufbxt_assert!(frame.file_format == CacheFileFormat::Pc2);

        let num_vertices = frame.data_count as usize;
        ufbxt_assert!(num_vertices == 770);

        let mut vertices = vec![Vec3::default(); num_vertices];

        let data_opts = GeometryCacheDataOpts { open_file_cb: Some(Arc::new(open_file_no_skip)), ..Default::default() };
        let num_read = read_geometry_cache_vec3(frame, &mut vertices, &data_opts);
        ufbxt_assert!(num_read == num_vertices);
    }
}

#[test]
fn cache_indirect_xml() {
    let buf = data_path("caches/indirect_xml_fail/cache.xml");

    let cache = load_geometry_cache(&buf, &GeometryCacheOpts::default());
    ufbxt_assert!(cache.is_err());
}
