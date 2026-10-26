mod ufbxt;
use ufbxt::*;

use std::io::Read;

#[test]
fn thread_safety() {
    fn assert_thread_safe<T: Send + Sync>() {}
    assert_thread_safe::<Scene>();
}

fn strict_opts() -> LoadOpts {
    LoadOpts { strict: true, ..Default::default() }
}

file_test!(maya_cube_dom, file = "maya_cube", opts = retain_dom_opts(), flags = FUZZ_ALWAYS | FUZZ_OPTS, |scene, err, load_error| {
    ufbxt_assert!(scene.dom_root.is_some());

    let objects = scene.dom_root.as_ref().unwrap().find("Objects");
    ufbxt_assert!(objects.is_some());
    let objects = objects.unwrap();

    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let node = node.unwrap();
    let node_dom = element_dom_node(scene, &node.element);
    let mut found = false;
    for child in &objects.children {
        if node_dom.is_some_and(|d| std::ptr::eq(d, child)) {
            found = true;
            break;
        }
    }
    ufbxt_assert!(found);

    let mesh = node.attrib.and_then(|id| scene.as_mesh(id));
    ufbxt_assert!(mesh.is_some());

    let dom_mesh = element_dom_node(scene, &mesh.unwrap().element);
    ufbxt_assert!(dom_mesh.is_some());

    let dom_indices = dom_mesh.unwrap().find("PolygonVertexIndex");
    ufbxt_assert!(dom_indices.is_some());
    let dom_indices = dom_indices.unwrap();
    ufbxt_assert!(dom_indices.values.len() == 1);

    let dom_indices_value = &dom_indices.values[0];
    ufbxt_assert!(dom_indices_value.value_type == DomValueType::ArrayI32);
    ufbxt_assert!(dom_indices_value.value_int == 24);

    let count = dom_indices_value.value_int as usize;
    let data: &[i32] = match &dom_indices_value.array {
        Some(DomArray::I32(a)) => a,
        _ => &[],
    };

    let mut num_negative = 0;
    for &index in &data[..count] {
        if index < 0 {
            num_negative += 1;
        }
    }
    ufbxt_assert!(num_negative == 6);
});

file_test!(maya_cube_strict, file = "maya_cube", opts = strict_opts(), flags = FUZZ_ALWAYS | FUZZ_OPTS, |scene, err, load_error| {});

file_test!(max2009_blob_dom, file = "max2009_blob", opts = retain_dom_opts(), flags = FUZZ_ALWAYS | FUZZ_OPTS, |scene, err, load_error| {
    ufbxt_assert!(scene.dom_root.is_some());
});

file_test!(maya_leading_comma, |scene, err, load_error| {
    ufbxt_assert!(scene.metadata.creator == "FBX SDK/FBX Plugins version 2019.2");
});

file_test!(maya_zero_end, |scene, err, load_error| {
    ufbxt_assert!(scene.metadata.creator == "FBX SDK/FBX Plugins version 2019.2");
});

#[test]
fn error_format_long() {
    let data = b"Bad FBX\0";
    let scene = load_memory(data, &LoadOpts::default());
    ufbxt_assert!(scene.is_err());
    let error = scene.err().unwrap();

    // The port keeps no error stack: `Display` is the description alone, with the info after it where there is one
    let formatted = error.to_string();
    check_scene::check_string(&error.description);
    ufbxt_assert!(formatted == error.description);
}

#[test]
fn error_format_short() {
    let data = b"Bad FBX\0";
    let scene = load_memory(data, &LoadOpts::default());
    ufbxt_assert!(scene.is_err());
}

#[test]
fn error_format_hint() {
    let scene = load_file("<nonexistent>.fbx", &LoadOpts::default());
    ufbxt_assert!(scene.is_err());

    let formatted = scene.err().unwrap().to_string();
    ufbxt_assert!(formatted.contains("<nonexistent>.fbx"));
}

#[test]
fn error_format_no_error() {
}

file_test!(maya_node_attribute_zoo, |scene, err, load_error| {
    ufbxt_assert!(scene.settings.axes.right == CoordinateAxis::PositiveX);
    ufbxt_assert!(scene.settings.axes.up == CoordinateAxis::PositiveY);
    ufbxt_assert!(scene.settings.axes.front == CoordinateAxis::PositiveZ);
    ufbxt_assert!(scene.settings.time_mode == TimeMode::Fps24);
    assert_close_real(err, scene.settings.unit_meters, 0.01);
    assert_close_real(err, scene.settings.frames_per_second as Real, 24.0);

    let attrib_type = |node: &Node| node.attrib.map(|id| scene.element(id).element_type);

    let node = find_node(scene, "Null").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::Empty));
    ufbxt_assert!(attrib_type(node.unwrap()) == Some(ElementType::Empty));

    let node = find_node(scene, "Mesh").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::Mesh));
    let node = node.unwrap();
    ufbxt_assert!(attrib_type(node) == Some(ElementType::Mesh));
    ufbxt_assert!(node.mesh.map(|m| scene.meshes[m].element.element_id) == node.attrib);

    let node = find_node(scene, "Bone").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::Bone));
    let node = node.unwrap();
    ufbxt_assert!(attrib_type(node) == Some(ElementType::Bone));
    let bone = node.attrib.and_then(|id| scene.as_bone(id));
    ufbxt_assert!(bone.is_some());
    let bone = bone.unwrap();
    assert_close_real(err, bone.radius, 0.5);
    assert_close_real(err, bone.relative_length, 1.0);

    let node = find_node(scene, "Camera").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::Camera));
    let node = node.unwrap();
    ufbxt_assert!(attrib_type(node) == Some(ElementType::Camera));
    ufbxt_assert!(node.camera.map(|c| scene.cameras[c].element.element_id) == node.attrib);
    let camera = &scene.cameras[node.camera.unwrap()];
    ufbxt_assert!(camera.gate_fit == GateFit::Fill);
    assert_close_real(err, camera.field_of_view_deg.x / 10.0, 54.43 / 10.0);
    assert_close_real(err, camera.focal_length_mm, 35.0);

    let node = find_node(scene, "Light").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::Light));
    let node = node.unwrap();
    ufbxt_assert!(attrib_type(node) == Some(ElementType::Light));
    ufbxt_assert!(node.light.map(|l| scene.lights[l].element.element_id) == node.attrib);
    let light = &scene.lights[node.light.unwrap()];
    assert_close_real(err, light.intensity, 1.0);
    assert_close_real(err, light.color.x, 1.0);
    assert_close_real(err, light.color.y, 1.0);
    assert_close_real(err, light.color.z, 1.0);

    let node = find_node(scene, "StereoCamera").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::StereoCamera));
    let node = node.unwrap();
    ufbxt_assert!(attrib_type(node) == Some(ElementType::StereoCamera));
    let stereo = scene.as_stereo_camera(node.attrib.unwrap()).unwrap();
    ufbxt_assert!(stereo.left.is_some_and(|c| scene.cameras[c].element.element_type == ElementType::Camera));
    ufbxt_assert!(stereo.right.is_some_and(|c| scene.cameras[c].element.element_type == ElementType::Camera));
    let left = &scene.cameras[stereo.left.unwrap()];
    let right = &scene.cameras[stereo.right.unwrap()];
    let left_focal_prop = evaluate_prop(scene, &scene.anim, left.element.element_id, "FocalLength", 0.5);
    let right_focal_prop = evaluate_prop(scene, &scene.anim, right.element.element_id, "FocalLength", 0.5);
    assert_close_real(err, left_focal_prop.value_real(), 42.011);
    assert_close_real(err, right_focal_prop.value_real(), 42.011);

    let node = find_node(scene, "NurbsCurve").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::NurbsCurve));
    ufbxt_assert!(attrib_type(node.unwrap()) == Some(ElementType::NurbsCurve));

    let node = find_node(scene, "NurbsSurface").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::NurbsSurface));
    ufbxt_assert!(attrib_type(node.unwrap()) == Some(ElementType::NurbsSurface));

    let node = find_node(scene, "NurbsTrim").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::NurbsTrimSurface));
    ufbxt_assert!(attrib_type(node.unwrap()) == Some(ElementType::NurbsTrimSurface));

    let node = find_node(scene, "LodGroup").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::LodGroup));
    ufbxt_assert!(attrib_type(node.unwrap()) == Some(ElementType::LodGroup));
});

file_test!(synthetic_duplicate_prop, |scene, err, load_error| {
    let cube = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(cube.is_some());
    let cube = cube.unwrap();

    let prop = cube.element.props.find("Lcl Translation");
    ufbxt_assert!(prop.is_some());
    assert_close_real(err, prop.unwrap().value_vec3().x, -1.0);

    let prop = cube.element.props.find("Lcl Scaling");
    ufbxt_assert!(prop.is_some());
    assert_close_real(err, prop.unwrap().value_vec3().x, 2.0);
});

file_test!(synthetic_missing_version, flags = ALLOW_STRICT_ERROR, |scene, err, load_error| {
    ufbxt_assert!(scene.metadata.version == 6100);
    ufbxt_assert!(scene.metadata.exporter == Exporter::FbxSdk);
    ufbxt_assert!(scene.metadata.exporter_version == pack_version(2007, 2, 28));
});

file_test!(synthetic_missing_exporter, flags = ALLOW_STRICT_ERROR, |scene, err, load_error| {
    ufbxt_assert!(scene.metadata.version == 6100);
    ufbxt_assert!(scene.metadata.exporter == Exporter::Unknown);
    ufbxt_assert!(scene.metadata.exporter_version == 0);
});

file_test!(synthetic_blender_old_exporter, flags = ALLOW_STRICT_ERROR, |scene, err, load_error| {
    ufbxt_assert!(scene.metadata.version == 6100);
    ufbxt_assert!(scene.metadata.exporter == Exporter::Unknown);
    ufbxt_assert!(scene.metadata.exporter_version == 0);
});

file_test!(blender_272_cube, |scene, err, load_error| {
    ufbxt_assert!(scene.metadata.version == 7400);
    ufbxt_assert!(scene.metadata.exporter == Exporter::BlenderBinary);
    ufbxt_assert!(scene.metadata.exporter_version == pack_version(2, 72, 0));

    ufbxt_assert!(scene.metadata.original_application.vendor == "Blender Foundation");
    ufbxt_assert!(scene.metadata.original_application.name == "Blender (stable FBX IO)");
    ufbxt_assert!(scene.metadata.original_application.version == "2.72 (sub 0)");
    ufbxt_assert!(scene.metadata.latest_application.vendor == "Blender Foundation");
    ufbxt_assert!(scene.metadata.latest_application.name == "Blender (stable FBX IO)");
    ufbxt_assert!(scene.metadata.latest_application.version == "2.72 (sub 0)");
});

#[test]
fn unicode_filename() {
    let buf = data_path("synthetic_a\u{3b2}\u{30ab}\u{1f602}_7500_ascii.fbx");

    {
        let opts = LoadOpts { ignore_geometry: true, ..Default::default() };

        let scene = load_file(&buf, &opts);
        ufbxt_assert!(scene.is_ok());
        let scene = scene.unwrap();
        ufbxt_assert!(find_node(&scene, "pCube1").is_some());
        check_scene(&scene);
    }

    {
        let scene = load_file(&buf, &LoadOpts::default());
        ufbxt_assert!(scene.is_ok());
        let scene = scene.unwrap();
        ufbxt_assert!(find_node(&scene, "pCube1").is_some());
        check_scene(&scene);
    }
}

file_test!(maya_cube_big_endian, |scene, err, load_error| {
    ufbxt_assert!(scene.metadata.big_endian);
});

file_test!(synthetic_cube_nan, flags = ALLOW_THREAD_ERROR, |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
    ufbxt_assert!(mesh.num_vertices >= 2);
    ufbxt_assert!(mesh.vertices[0].x.is_nan());
    ufbxt_assert!(mesh.vertices[0].y.is_infinite() && mesh.vertices[0].y < 0.0);
    ufbxt_assert!(mesh.vertices[0].z.is_infinite() && mesh.vertices[0].z > 0.0);
    assert_close_real(err, mesh.vertices[1].x, 0.5);
    assert_close_real(err, mesh.vertices[1].y, -0.5);
    assert_close_real(err, mesh.vertices[1].z, 0.5);
});

file_test!(synthetic_string_collision, flags = HEAVY_TO_FUZZ | SKIP_LOAD_OPTS_CHECKS, |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let props = &node.unwrap().element.props;

    let prop = props.find("BIiMTNT");
    ufbxt_assert!(prop.is_some());
    ufbxt_assert!(prop.unwrap().value_str == "BJKKUfZ");

    let prop = props.find("BbAJerP");
    ufbxt_assert!(prop.is_some());
    ufbxt_assert!(prop.unwrap().value_str == "AAAJKop");

    let prop = props.find("BpgrcZR");
    ufbxt_assert!(prop.is_some());
    ufbxt_assert!(prop.unwrap().value_str == "APGAmLj");
});

file_test!(synthetic_id_collision, flags = HEAVY_TO_FUZZ | SKIP_LOAD_OPTS_CHECKS, |scene, err, load_error| {
    ufbxt_assert!(scene.nodes.len() == 10002);
    let mut node = find_node(scene, "First");
    for depth in (0..=10000u32).rev() {
        ufbxt_assert!(node.is_some());
        ufbxt_assert!(scene.nodes[node.unwrap()].node_depth == depth);
        node = scene.nodes[node.unwrap()].parent;
    }
    ufbxt_assert!(node.is_none());
});

file_test_allow_error!(synthetic_node_depth_fail, |scene, err, load_error| {});

#[test]
fn open_file() {
    let buf = data_path("maya_cube_7500_ascii.fbx");

    let data = default_open_file(&buf, &OpenFileInfo::default());
    ufbxt_assert!(data.is_some());
    let data = data.unwrap();

    ufbxt_assert!(data.len() >= 2 + 3);
    let result = &data[2..5];
    ufbxt_assert!(result == b"FBX");
}

#[test]
fn file_not_found() {
    let buf = data_path("maya_cube_9999_emoji.fbx");

    let scene = load_file(&buf, &LoadOpts::default());
    ufbxt_assert!(scene.is_err());
    let error = scene.err().unwrap();
    ufbxt_assert!(error.kind == ErrorType::FileNotFound);
    ufbxt_assert!(!error.info.contains('\0'));
    ufbxt_assert!(error.info == buf);
}

#[test]
fn retain_scene() {
    for path in FileIterator::new("maya_cube") {
        let opts = LoadOpts::default();

        let scene = load_file(&path, &opts);
        ufbxt_assert!(scene.is_ok());
        let scene = scene.unwrap();

        check_scene(&scene);

        let eval_scene = evaluate_scene(&scene, &scene.anim, 0.1, &EvaluateOpts::default());
        ufbxt_assert!(eval_scene.is_ok());
        let eval_scene = eval_scene.unwrap();
        drop(scene);

        let node = find_node(&eval_scene, "pCube1").map(|n| &eval_scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        ufbxt_assert!(node.unwrap().element.name == "pCube1");
    }
}

file_test!(synthetic_empty_elements, |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
});

fn check_binary_prop(props: &Props) {
    let prop = props.find("Binary");
    ufbxt_assert!(prop.is_some());
    ufbxt_assert!(prop.unwrap().prop_type == PropType::Blob);
}

file_test!(synthetic_binary_props, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    check_warning(scene, WarningType::BadBase64Content, ElementType::Unknown, None, usize::MAX, None);

    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let node = node.unwrap();
    check_binary_prop(&node.element.props);
    check_binary_prop(&scene.meshes[node.mesh.unwrap()].element.props);
    check_binary_prop(&scene.settings.props);
});

file_test!(maya_unicode, |scene, err, load_error| {
    // WHAT? Maya turns U+03B2 (Greek Small Letter Beta) into U+00DF (Latin Small Letter Sharp S)
    // The larger codepoints just get turned into one or two underscores which makes a bit more sense...
    let node = find_node(scene, "a\u{df}___");
    ufbxt_assert!(node.is_some());
});

file_test!(max_unicode, |scene, err, load_error| {
    let node = find_node(scene, "a\u{3b2}\u{30ab}\u{1f602}");
    ufbxt_assert!(node.is_some());
});

file_test!(blender_279_unicode, |scene, err, load_error| {
    let node = find_node(scene, "a\u{3b2}\u{30ab}\u{1f602}");
    ufbxt_assert!(node.is_some());
});

fn decode_hex_char(c: u8) -> u32 {
    match c {
        b'0'..=b'9' => (c - b'0') as u32,
        b'a'..=b'f' => (c - b'a') as u32 + 10,
        b'A'..=b'F' => (c - b'A') as u32 + 10,
        _ => {
            ufbxt_hintf!("Bad hex character");
            ufbxt_assert!(false);
            0
        }
    }
}

fn decode_hex(dst: &mut [u8], src: &str) -> usize {
    let src = src.as_bytes();
    ufbxt_assert!(src.len() % 2 == 0);

    let num = src.len() / 2;
    ufbxt_assert!(num <= dst.len());

    for i in 0..num {
        dst[i] = (decode_hex_char(src[i * 2]) << 4 | decode_hex_char(src[i * 2 + 1])) as u8;
    }

    num
}

file_test!(synthetic_unicode, flags = ALLOW_INVALID_UNICODE | HEAVY_TO_FUZZ | SKIP_LOAD_OPTS_CHECKS | ALLOW_WARNINGS, |scene, err, load_error| {
    ufbxt_assert!(scene.metadata.warnings.len() == 1);
    ufbxt_assert!(scene.metadata.warnings[0].kind == WarningType::BadUnicode);
    ufbxt_assert!(scene.metadata.warnings[0].count >= 6000);

    let mut reference = [0u8; 128];

    {
        let node = find_node(scene, "Good").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());

        for prop in &node.unwrap().element.props.props {
            let ref_len = decode_hex(&mut reference, &prop.name);

            let src = prop.value_str.as_bytes();

            let mut src_ix = 0;
            for &rc in &reference[..ref_len] {
                if rc == 0 {
                    ufbxt_assert!(src_ix + 3 <= src.len());
                    ufbxt_assert!(src[src_ix] == 0xef);
                    ufbxt_assert!(src[src_ix + 1] == 0xbf);
                    ufbxt_assert!(src[src_ix + 2] == 0xbd);
                    src_ix += 3;
                } else {
                    ufbxt_assert!(src_ix < src.len());
                    ufbxt_assert!(src[src_ix] == rc);
                    src_ix += 1;
                }
            }
            ufbxt_assert!(src_ix == src.len());
        }
    }

    {
        let node = find_node(scene, "Ok").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());

        for prop in &node.unwrap().element.props.props {
            let ref_len = decode_hex(&mut reference, &prop.name);

            let src = prop.value_str.as_bytes();

            ufbxt_assert!(ref_len >= 1);
            ufbxt_assert!(reference[0] == 0xff);

            let mut src_ix = 0;
            ufbxt_assert!(src[src_ix] == 0xef);
            ufbxt_assert!(src[src_ix + 1] == 0xbf);
            ufbxt_assert!(src[src_ix + 2] == 0xbd);
            src_ix += 3;

            for &rc in &reference[1..ref_len] {
                if rc == 0 {
                    ufbxt_assert!(src_ix + 3 <= src.len());
                    ufbxt_assert!(src[src_ix] == 0xef);
                    ufbxt_assert!(src[src_ix + 1] == 0xbf);
                    ufbxt_assert!(src[src_ix + 2] == 0xbd);
                    src_ix += 3;
                } else {
                    ufbxt_assert!(src_ix < src.len());
                    ufbxt_assert!(src[src_ix] == rc);
                    src_ix += 1;
                }
            }
            ufbxt_assert!(src_ix == src.len());
        }
    }

    {
        let node = find_node(scene, "Bad").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());

        for prop in &node.unwrap().element.props.props {
            decode_hex(&mut reference, &prop.name);

            ufbxt_assert!(prop.value_str.contains('\u{fffd}'));
        }
    }
});

file_test!(max_quote, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    if scene.metadata.ascii && scene.metadata.version == 6100 {
        check_warning_ix(scene, WarningType::DuplicateObjectId, ElementType::Node, Some("\""), 1, 1, None);
    }

    {
        let node = find_node(scene, "\"'&\"").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.mesh.is_some());
        let mesh = &scene.meshes[node.mesh.unwrap()];
        ufbxt_assert!(mesh.materials.len() == 1);
        let material = &scene.materials[mesh.materials[0].unwrap()];
        if scene.metadata.ascii {
            ufbxt_assert!(material.element.name == "&&q&qu&quo&quot\"\"");
        } else {
            ufbxt_assert!(material.element.name == "&&q&qu&quo&quot&quot;\"");
        }
    }

    {
        let node = find_node(scene, "\"");
        ufbxt_assert!(node.is_some());
    }

    {
        let node = find_node(scene, "&quot;");
        if scene.metadata.ascii {
            ufbxt_assert!(node.is_none());
        } else {
            ufbxt_assert!(node.is_some());
        }
    }
});

file_test!(max_colon_name, |scene, err, load_error| {
    let node = find_node(scene, "Cube::Model").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let node = node.unwrap();
    ufbxt_assert!(node.mesh.is_some());
    let mesh = &scene.meshes[node.mesh.unwrap()];
    ufbxt_assert!(mesh.materials.len() == 1);
    let material = &scene.materials[mesh.materials[0].unwrap()];
    ufbxt_assert!(material.element.name == "Material::Pink");
});

file_test_allow_error!(synthetic_truncated_quot_fail, |scene, err, load_error| {});

file_test!(synthetic_unicode_error_identity, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    let parent = find_node(scene, "Parent").map(|n| &scene.nodes[n]);
    ufbxt_assert!(parent.is_some());
    let parent = parent.unwrap();
    ufbxt_assert!(parent.children.len() == 2);

    ufbxt_assert!(scene.nodes[parent.children[0]].element.name == scene.nodes[parent.children[1]].element.name);

    for &child in &parent.children {
        let child = &scene.nodes[child];
        let world_transform = matrix_to_transform(&child.node_to_world);
        let left = world_transform.translation.x < 0.0;

        ufbxt_assert!(child.element.name == "Child_\u{fffd}");

        ufbxt_assert!(child.mesh.is_some());
        let mesh = &scene.meshes[child.mesh.unwrap()];
        ufbxt_assert!(mesh.materials.len() == 1);

        let material = &scene.materials[mesh.materials[0].unwrap()];
        ufbxt_assert!(material.element.name == "Material_\u{fffd}");

        if left {
            let reference = Vec3::new(1.0, 0.0, 0.0);
            assert_close_vec3(err, material.fbx.diffuse_color().value_vec3(), reference);
        } else {
            let reference = Vec3::new(0.0, 1.0, 0.0);
            assert_close_vec3(err, material.fbx.diffuse_color().value_vec3(), reference);
        }
    }
});

fn unicode_error_question_opts() -> LoadOpts {
    LoadOpts { unicode_error_handling: UnicodeErrorHandling::QuestionMark, ..Default::default() }
}

file_test!(synthetic_broken_filename, opts = unicode_error_question_opts(), flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    ufbxt_assert!(scene.textures.len() == 1);

    let abs_path_safe = "D:/Dev/clean/ufbx/data/textures/??????_diffuse.png";
    let rel_path_safe = "textures\\??????_diffuse.png";
    let abs_path_raw: &[u8] = b"D:/Dev/clean/ufbx/data/textures/\xaa\xbb\xcc\xdd\xee\xff_diffuse.png";
    let rel_path_raw: &[u8] = b"textures\\\xaa\xbb\xcc\xdd\xee\xff_diffuse.png";
    let name_path_raw: &[u8] = b"\xaa\xbb\xcc\xdd\xee\xff_diffuse.png";

    let texture = &scene.textures[0];

    ufbxt_assert!(texture.absolute_filename.len() == abs_path_safe.len());
    ufbxt_assert!(texture.relative_filename.len() == rel_path_safe.len());
    ufbxt_assert!(texture.absolute_filename == abs_path_safe);
    ufbxt_assert!(texture.relative_filename == rel_path_safe);

    ufbxt_assert!(texture.raw_absolute_filename.len() == abs_path_raw.len());
    ufbxt_assert!(texture.raw_relative_filename.len() == rel_path_raw.len());
    ufbxt_assert!(texture.raw_absolute_filename == abs_path_raw);
    ufbxt_assert!(texture.raw_relative_filename == rel_path_raw);

    {
        ufbxt_assert!(texture.filename.contains("??????_diffuse.png"));
        let begin = texture.raw_filename.iter().position(|&c| c == 0xaa);
        ufbxt_assert!(begin.is_some());
        let rest = &texture.raw_filename[begin.unwrap()..];
        ufbxt_assert!(rest.len() == name_path_raw.len());
        ufbxt_assert!(rest == name_path_raw);
    }

    let video = &scene.videos[0];

    ufbxt_assert!(video.absolute_filename.len() == abs_path_safe.len());
    ufbxt_assert!(video.relative_filename.len() == rel_path_safe.len());
    ufbxt_assert!(video.absolute_filename == abs_path_safe);
    ufbxt_assert!(video.relative_filename == rel_path_safe);

    ufbxt_assert!(video.raw_absolute_filename.len() == abs_path_raw.len());
    ufbxt_assert!(video.raw_relative_filename.len() == rel_path_raw.len());
    ufbxt_assert!(video.raw_absolute_filename == abs_path_raw);
    ufbxt_assert!(video.raw_relative_filename == rel_path_raw);

    {
        ufbxt_assert!(video.filename.contains("??????_diffuse.png"));
        let begin = video.raw_filename.iter().position(|&c| c == 0xaa);
        ufbxt_assert!(begin.is_some());
        let rest = &video.raw_filename[begin.unwrap()..];
        ufbxt_assert!(rest.len() == name_path_raw.len());
        ufbxt_assert!(rest == name_path_raw);
    }

    {
        let abs_path = video.element.props.find_string("Path", "");
        let rel_path = video.element.props.find_string("RelPath", "");
        ufbxt_assert!(abs_path.len() == abs_path_safe.len());
        ufbxt_assert!(rel_path.len() == rel_path_safe.len());
        ufbxt_assert!(abs_path == abs_path_safe);
        ufbxt_assert!(rel_path == rel_path_safe);
    }

    {
        let abs_path = video.element.props.find_blob("Path", &[]);
        let rel_path = video.element.props.find_blob("RelPath", &[]);
        ufbxt_assert!(abs_path.len() == abs_path_raw.len());
        ufbxt_assert!(rel_path.len() == rel_path_raw.len());
        ufbxt_assert!(abs_path == abs_path_raw);
        ufbxt_assert!(rel_path == rel_path_raw);
    }
});

file_test!(revit_empty, |scene, err, load_error| {
    let mut found_refs3 = false;
    let mut found_refs = false;
    for unknown in &scene.unknowns {
        if unknown.element.name == "ADSKAssetReferencesVersion3.0" {
            let prop = unknown.element.props.find("ADSKAssetReferencesBlobVersion3.0");
            let size: u32 = if scene.metadata.version == 7700 { 11228 } else { 11305 };
            let hash: u32 = if scene.metadata.version == 7700 { 0x5eee86b6 } else { 0x7f39429d };

            ufbxt_assert!(prop.is_some());
            let prop = prop.unwrap();
            ufbxt_assert!(prop.prop_type == PropType::Blob);
            ufbxt_assert!(prop.value_real() == size as Real);
            ufbxt_assert!(prop.value_int == size as i64);
            ufbxt_assert!(prop.value_blob.len() == size as usize);

            let blob = unknown.element.props.find_blob("ADSKAssetReferencesBlobVersion3.0", &[]);
            ufbxt_assert!(std::ptr::eq(blob.as_ptr(), prop.value_blob.as_ptr()));
            ufbxt_assert!(blob.len() == prop.value_blob.len());

            let hash_ref = fnv1a(blob);
            ufbxt_assert!(hash_ref == hash);

            ufbxt_assert!(!found_refs3);
            found_refs3 = true;
        } else if unknown.element.name == "ADSKAssetReferences" {
            let prop = unknown.element.props.find("ADSKAssetReferencesBlob");
            let size: u32 = if scene.metadata.version == 7700 { 7910 } else { 7885 };
            let hash: u32 = if scene.metadata.version == 7700 { 0xa081f491 } else { 0x5aaeae9a };

            ufbxt_assert!(prop.is_some());
            let prop = prop.unwrap();
            ufbxt_assert!(prop.prop_type == PropType::Blob);
            ufbxt_assert!(prop.value_real() == size as Real);
            ufbxt_assert!(prop.value_int == size as i64);
            ufbxt_assert!(prop.value_blob.len() == size as usize);

            let blob = unknown.element.props.find_blob("ADSKAssetReferencesBlob", &[]);
            ufbxt_assert!(std::ptr::eq(blob.as_ptr(), prop.value_blob.as_ptr()));
            ufbxt_assert!(blob.len() == prop.value_blob.len());

            let hash_ref = fnv1a(blob);
            ufbxt_assert!(hash_ref == hash);

            ufbxt_assert!(!found_refs);
            found_refs = true;
        }
    }

    ufbxt_assert!(found_refs);
    ufbxt_assert!(found_refs3);
});

file_test!(maya_lock_mute, |scene, err, load_error| {
    use prop_flags::*;

    let any_lock = LOCK_X | LOCK_Y | LOCK_Z | LOCK_W;
    let any_mute = MUTE_X | MUTE_Y | MUTE_Z | MUTE_W;
    let any_lock_mute = any_lock | any_mute;

    {
        let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let props = &node.unwrap().element.props;
        let prop = props.find("Lcl Translation");
        ufbxt_assert!(prop.is_some_and(|p| (p.flags & any_lock_mute) == (MUTE_X | LOCK_Z)));
        let prop = props.find("Lcl Rotation");
        ufbxt_assert!(prop.is_some_and(|p| (p.flags & any_lock_mute) == (MUTE_Y | LOCK_Y)));
        let prop = props.find("Lcl Scaling");
        ufbxt_assert!(prop.is_some_and(|p| (p.flags & any_lock_mute) == (MUTE_Z | LOCK_X)));
    }

    {
        let node = find_node(scene, "pPlane1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let props = &node.unwrap().element.props;
        let prop = props.find("Lcl Translation");
        ufbxt_assert!(prop.is_some_and(|p| (p.flags & any_lock_mute) == (MUTE_X | MUTE_Y | MUTE_Z)));
        let prop = props.find("Lcl Rotation");
        ufbxt_assert!(prop.is_some_and(|p| (p.flags & any_lock_mute) == (LOCK_X | LOCK_Y | LOCK_Z)));
        let prop = props.find("Lcl Scaling");
        ufbxt_assert!(prop.is_some_and(|p| (p.flags & any_lock_mute) == (MUTE_X | MUTE_Y | MUTE_Z | LOCK_X | LOCK_Y | LOCK_Z)));
    }
});

fn filename_load_opts() -> LoadOpts {
    LoadOpts { filename: "fake/path/to/file.fbx".to_string(), path_separator: b'/', ..Default::default() }
}

fn parent_directory_textures(scene: &Scene) -> (&Texture, &Video, &Texture, &Video) {
    let material = find_element(scene, ElementType::Material, "lambert1").and_then(|id| scene.as_material(id));
    ufbxt_assert!(material.is_some());
    let material = material.unwrap();

    let temporary = material.fbx.diffuse_color().texture.map(|t| &scene.textures[t]);
    ufbxt_assert!(temporary.is_some_and(|t| t.video.is_some()));
    let temporary = temporary.unwrap();
    let temporary_video = &scene.videos[temporary.video.unwrap()];

    let inner = material.fbx.transparency_color().texture.map(|t| &scene.textures[t]);
    ufbxt_assert!(inner.is_some_and(|t| t.video.is_some()));
    let inner = inner.unwrap();
    let inner_video = &scene.videos[inner.video.unwrap()];

    (temporary, temporary_video, inner, inner_video)
}

file_test!(synthetic_parent_directory, opts = filename_load_opts(), |scene, err, load_error| {
    let (temporary, temporary_video, inner, inner_video) = parent_directory_textures(scene);

    ufbxt_assert!(temporary.filename == "temporary.png");
    ufbxt_assert!(temporary_video.filename == "fake/path/temporary.png");
    ufbxt_assert!(inner.filename == "../directory/inner.png");
    ufbxt_assert!(inner_video.filename == "fake/path/directory/inner.png");
});

file_test!(synthetic_parent_directory_bad, opts = filename_load_opts(), |scene, err, load_error| {
    let (temporary, temporary_video, inner, inner_video) = parent_directory_textures(scene);

    ufbxt_assert!(temporary.filename == "temporary.png");
    ufbxt_assert!(temporary_video.filename == "fake/path/temporary.png");
    ufbxt_assert!(inner.filename == "../directory/inner.png");
    ufbxt_assert!(inner_video.filename == format!("{}directory/inner.png", "../".repeat(254)));
});

fn parent_dir_filename_load_opts() -> LoadOpts {
    LoadOpts { filename: "fake/../path/./file.fbx".to_string(), path_separator: b'/', ..Default::default() }
}

file_test!(synthetic_parent_directory_parent, file = "synthetic_parent_directory", opts = parent_dir_filename_load_opts(), |scene, err, load_error| {
    let (temporary, temporary_video, inner, inner_video) = parent_directory_textures(scene);

    ufbxt_assert!(temporary.filename == "fake/../../../temporary.png");
    ufbxt_assert!(temporary_video.filename == "fake/../temporary.png");
    ufbxt_assert!(inner.filename == "fake/../../../../directory/inner.png");
    ufbxt_assert!(inner_video.filename == "fake/../directory/inner.png");
});

#[test]
fn unsafe_options() {
    for path in FileIterator::new("maya_cube") {
        for mask in 0u32..0x8 {
            let allow_unsafe = (mask & 0x1) != 0;
            let unsafe_mask = mask >> 1;

            let mut opts = LoadOpts { allow_unsafe, ..Default::default() };
            if (unsafe_mask & 0x1) != 0 {
                opts.index_error_handling = IndexErrorHandling::UnsafeIgnore;
            }
            if (unsafe_mask & 0x2) != 0 {
                opts.unicode_error_handling = UnicodeErrorHandling::UnsafeIgnore;
            }
            let scene = load_file(&path, &opts);
            if allow_unsafe || unsafe_mask == 0 {
                ufbxt_assert!(scene.is_ok());
            } else {
                ufbxt_assert!(scene.is_err());
                ufbxt_assert!(scene.err().unwrap().kind == ErrorType::UnsafeOptions);
            }
        }
    }
}

fn underscore_no_index_opts() -> LoadOpts {
    LoadOpts { index_error_handling: IndexErrorHandling::NoIndex, unicode_error_handling: UnicodeErrorHandling::Underscore, ..Default::default() }
}

fn all_unsafe_opts() -> LoadOpts {
    LoadOpts {
        allow_unsafe: true,
        index_error_handling: IndexErrorHandling::UnsafeIgnore,
        unicode_error_handling: UnicodeErrorHandling::UnsafeIgnore,
        ..Default::default()
    }
}

fn fail_index_opts() -> LoadOpts {
    LoadOpts { index_error_handling: IndexErrorHandling::AbortLoading, ..Default::default() }
}

fn fail_unicode_opts() -> LoadOpts {
    LoadOpts { unicode_error_handling: UnicodeErrorHandling::AbortLoading, ..Default::default() }
}

file_test!(synthetic_unsafe_cube, flags = ALLOW_INVALID_UNICODE | ALLOW_WARNINGS, |scene, err, load_error| {
    check_warning(scene, WarningType::IndexClamped, ElementType::Mesh, Some("#pC\u{fffd}\u{fffd}e1"), 4, None);
    check_warning(scene, WarningType::BadUnicode, ElementType::Node, Some("pC\u{fffd}\u{fffd}e1"), 1, None);

    let node = find_node(scene, "pC\u{fffd}\u{fffd}e1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
    ufbxt_assert!(mesh.vertex_uv.indices[0] == 0);
    ufbxt_assert!(mesh.vertex_uv.indices[1] == 1);
    ufbxt_assert!(mesh.vertex_uv.indices[2] == 3);
    ufbxt_assert!(mesh.vertex_uv.indices[3] == 13);
    ufbxt_assert!(mesh.vertex_uv.indices[4] == 13);
    ufbxt_assert!(mesh.vertex_uv.indices[5] == 13);
    ufbxt_assert!(mesh.vertex_uv.indices[6] == 13);
    ufbxt_assert!(mesh.vertex_uv.indices[7] == 4);
});

file_test!(
    synthetic_unsafe_cube_underscore_no_index,
    file = "synthetic_unsafe_cube",
    opts = underscore_no_index_opts(),
    flags = ALLOW_INVALID_UNICODE,
    |scene, err, load_error| {
        let node = find_node(scene, "pC__e1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
        let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
        ufbxt_assert!(mesh.vertex_uv.indices[0] == 0);
        ufbxt_assert!(mesh.vertex_uv.indices[1] == 1);
        ufbxt_assert!(mesh.vertex_uv.indices[2] == 3);
        ufbxt_assert!(mesh.vertex_uv.indices[3] == NO_INDEX);
        ufbxt_assert!(mesh.vertex_uv.indices[4] == NO_INDEX);
        ufbxt_assert!(mesh.vertex_uv.indices[5] == NO_INDEX);
        ufbxt_assert!(mesh.vertex_uv.indices[6] == NO_INDEX);
        ufbxt_assert!(mesh.vertex_uv.indices[7] == 4);

        for i in 3..=6 {
            let v = mesh.vertex_uv.get(i);
            assert_close_vec2(err, v, Vec2::ZERO);

            let v = mesh.vertex_uv.catch_get(i);
            ufbxt_assert!(v.is_ok());
            assert_close_vec2(err, v.unwrap(), Vec2::ZERO);
        }

        {
            let v = mesh.vertex_uv.catch_get(4096);
            ufbxt_assert!(v.is_err());
            ufbxt_assert!(v.as_ref().unwrap_err().description == "index (4096) out of range (24)");
            assert_close_vec2(err, v.unwrap_or_default(), Vec2::ZERO);
        }
    }
);

file_test!(synthetic_unsafe_cube_unsafe, file = "synthetic_unsafe_cube", opts = all_unsafe_opts(), flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    // ufbx keeps the name's raw bytes, "pC\xff\0e1"; a `String` cannot hold the 0xff, which the port turns into U+FFFD
    let node = find_node(scene, "pC\u{fffd}\0e1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
    ufbxt_assert!(mesh.vertex_uv.indices[0] == 0);
    ufbxt_assert!(mesh.vertex_uv.indices[1] == 1);
    ufbxt_assert!(mesh.vertex_uv.indices[2] == 3);
    ufbxt_assert!(mesh.vertex_uv.indices[3] == 0xff);
    ufbxt_assert!(mesh.vertex_uv.indices[4] == 0xffff);
    ufbxt_assert!(mesh.vertex_uv.indices[5] == 0xffffff);
    ufbxt_assert!(mesh.vertex_uv.indices[6] == 0xffffffff);
    ufbxt_assert!(mesh.vertex_uv.indices[7] == 4);
});

file_test_allow_error!(
    synthetic_unsafe_cube_fail_index,
    file = "synthetic_unsafe_cube",
    opts = fail_index_opts(),
    flags = ALLOW_INVALID_UNICODE,
    |scene, err, load_error| {
        ufbxt_assert!(scene.is_none());
        ufbxt_assert!(load_error.kind == ErrorType::BadIndex);
    }
);

file_test_allow_error!(
    synthetic_unsafe_cube_fail_unicode,
    file = "synthetic_unsafe_cube",
    opts = fail_unicode_opts(),
    flags = ALLOW_INVALID_UNICODE,
    |scene, err, load_error| {
        ufbxt_assert!(scene.is_none());
        ufbxt_assert!(load_error.kind == ErrorType::InvalidUtf8);
    }
);

file_test!(find_prop_concat, file = "maya_node_attribute_zoo", |scene, err, load_error| {
    {
        let node = find_node(scene, "Null").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let props = &node.unwrap().element.props;

        {
            let prop = props.find_concat(&["Geometric", "Rotation"]);
            ufbxt_assert!(prop.is_some());
            ufbxt_assert!(prop.unwrap().name == "GeometricRotation");
        }

        {
            let prop = props.find_concat(&["Geometric", "Translation"]);
            ufbxt_assert!(prop.is_some());
            ufbxt_assert!(prop.unwrap().name == "GeometricTranslation");
        }
    }

    let is_prop = |found: Option<&Prop>, prop: &Prop| found.is_some_and(|f| std::ptr::eq(f, prop));

    for elem_ix in 0..scene.elements.len() {
        let element = scene.element(elem_ix as u32);
        let mut props = Some(&element.props);

        while let Some(p) = props {
            for prop in &p.props {
                let name = prop.name.as_str();
                ufbxt_assert!(name.len() * 2 < 512);
                // a `&str` part cannot split a character: the parts below split the name byte by byte
                ufbxt_assert!(name.is_ascii());

                ufbxt_assert!(is_prop(p.find_concat(&[name]), prop));

                let parts: Vec<&str> = (0..name.len()).map(|i| &name[i..i + 1]).collect();
                ufbxt_assert!(is_prop(p.find_concat(&parts), prop));

                let parts: Vec<&str> = (0..name.len()).flat_map(|i| [&name[i..i + 1], ""]).collect();
                ufbxt_assert!(is_prop(p.find_concat(&parts), prop));

                for step in 1..name.len() {
                    let parts: Vec<&str> = (0..name.len()).step_by(step).map(|i| &name[i..(i + step).min(name.len())]).collect();
                    ufbxt_assert!(is_prop(p.find_concat(&parts), prop));
                }
            }

            props = p.defaults.as_deref();
        }
    }
});

file_test!(synthetic_duplicate_id, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    if scene.metadata.version >= 7000 {
        check_warning_ix(scene, WarningType::DuplicateObjectId, ElementType::Node, Some("pCube2"), 0, 1, None);
        check_warning_ix(scene, WarningType::DuplicateObjectId, ElementType::Node, Some("pointLight2"), 0, 1, None);
        check_warning_ix(scene, WarningType::DuplicateObjectId, ElementType::Material, Some("lambert2"), 0, 1, None);
        check_warning_ix(scene, WarningType::DuplicateObjectId, ElementType::Mesh, Some(""), 1, 1, None);
        check_warning_ix(scene, WarningType::DuplicateObjectId, ElementType::Light, Some(""), 1, 1, None);
    } else {
        check_warning_ix(scene, WarningType::DuplicateObjectId, ElementType::Node, Some("pCube1"), 1, 1, None);
        check_warning_ix(scene, WarningType::DuplicateObjectId, ElementType::Node, Some("pointLight1"), 1, 1, None);
        check_warning_ix(scene, WarningType::DuplicateObjectId, ElementType::Material, Some("lambert1"), 1, 1, None);
    }

    // Don't make any assumptions about the scene, ufbxt_check_scene() makes sure
    // that it doesn't break any of the API guarantees.
});

file_test!(synthetic_recursive_transform, opts = retain_dom_opts(), |scene, err, load_error| {
    let takes = scene.dom_root.as_ref().and_then(|r| r.find("Takes"));
    ufbxt_assert!(takes.is_some());
    let take = takes.unwrap().find("Take");
    ufbxt_assert!(take.is_some());
    let model = take.unwrap().find("Model");
    ufbxt_assert!(model.is_some());
    let channel = model.unwrap().find("Channel");
    ufbxt_assert!(channel.is_some());
    let channel = channel.unwrap();

    ufbxt_assert!(channel.values.len() == 1);
    ufbxt_assert!(channel.values[0].value_type == DomValueType::String);
    ufbxt_assert!(channel.values[0].value_str == "Transform");

    let rec_channel = channel.find("Channel");
    ufbxt_assert!(rec_channel.is_some());
    let mut rec_channel = rec_channel.unwrap();

    let mut count = 1;
    loop {
        ufbxt_hintf!("count={count}");
        count += 1;
        ufbxt_assert!(rec_channel.values.len() == 1);
        ufbxt_assert!(rec_channel.values[0].value_type == DomValueType::String);
        ufbxt_assert!(rec_channel.values[0].value_str == "Transform");

        if rec_channel.children.is_empty() {
            break;
        }
        ufbxt_assert!(rec_channel.children.len() == 1);
        rec_channel = &rec_channel.children[0];
    }

    ufbxt_assert!(count == 29);
});

file_test!(synthetic_recursive_connections, |scene, err, load_error| {
    {
        let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();

        let prop = node.element.props.find("Lcl Translation");
        ufbxt_assert!(prop.is_some());
        ufbxt_assert!((prop.unwrap().flags & prop_flags::CONNECTED) != 0);

        let reference = Vec3::new(1.0, 0.0, 0.0);
        let value = evaluate_prop(scene, &scene.anim, node.element.element_id, "Lcl Translation", 0.5);
        assert_close_vec3(err, value.value_vec3(), reference);
    }

    {
        let node = find_node(scene, "pCube3").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();

        let prop = node.element.props.find("Lcl Translation");
        ufbxt_assert!(prop.is_some());
        ufbxt_assert!((prop.unwrap().flags & prop_flags::CONNECTED) != 0);

        let reference = Vec3::new(0.0, 0.0, 1.0);
        let value = evaluate_prop(scene, &scene.anim, node.element.element_id, "Lcl Translation", 0.5);
        assert_close_vec3(err, value.value_vec3(), reference);
    }
});

file_test!(synthetic_rotation_order_layers, |scene, err, load_error| {
    let layer = |name: &str| find_element(scene, ElementType::AnimLayer, name).and_then(|id| scene.as_anim_layer(id)).unwrap();
    let layer_base_layer = layer("BaseLayer");
    let layer_base = layer("Base");
    let layer_rotation = layer("Rotation");

    let layer_ids = vec![layer_base_layer.element.typed_id, layer_base.element.typed_id, layer_rotation.element.typed_id];

    let layer_weights = vec![1.0, 0.5, 0.25];

    let opts = AnimOpts { layer_ids, override_layer_weights: layer_weights, ..Default::default() };

    let anim = create_anim(scene, &opts);
    if let Err(e) = &anim {
        log_error(e);
    }
    ufbxt_assert!(anim.is_ok());
    let anim = anim.unwrap();
    check_scene::check_anim(scene, &anim);

    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());

    // We don't really care about the result here as runtime rotation order is not really supported,
    // just want to make sure we don't hit any infinite recursion with rotation order layers
    let _ = evaluate_prop(scene, &anim, node.unwrap().element.element_id, "Lcl Rotation", 0.2f32 as f64);
});

fn strtoul(s: &str) -> (u64, &str) {
    let t = s.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let digits = t.bytes().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 {
        return (0, s);
    }
    (t[..digits].parse().unwrap_or(u64::MAX), &t[digits..])
}

fn check_note_lines<'a>(notes: &'a str, lines: &[&str]) -> &'a str {
    let mut s = notes;
    for (i, line) in lines.iter().enumerate() {
        ufbxt_hintf!("i={i}");
        let end = match s.find('\n') {
            Some(at) => at + 1,
            None => s.len(),
        };

        let len = end;
        ufbxt_assert!(s.as_bytes()[..len] == line.as_bytes()[..len.min(line.len())]);
        ufbxt_assert!(len == line.len());
        s = &s[end..];
    }
    s
}

file_test!(maya_notes, |scene, err, load_error| {
    {
        let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let notes = node.unwrap().element.props.find("notes");
        ufbxt_assert!(notes.is_some());

        let lines = ["pCube1 notes:\n"];

        let mut s = check_note_lines(&notes.unwrap().value_str, &lines);

        for i in 0..4096u64 {
            let (l, end) = strtoul(s);
            ufbxt_assert!(l == i);
            if i + 1 < 16384 {
                ufbxt_assert!(end.starts_with(' '));
                s = &end[1..];
            }
        }
    }

    {
        let material = find_material(scene, "lambert1").map(|m| &scene.materials[m]);
        ufbxt_assert!(material.is_some());
        let notes = material.unwrap().element.props.find("notes");
        ufbxt_assert!(notes.is_some());

        let lines = ["lambert1 notes:\n", "\n", "- material\n", "- \"lambertian\"\n", "- unicode: a\u{df}???\n", "- tab\t\n", "- cr \n"];

        let s = check_note_lines(&notes.unwrap().value_str, &lines);

        ufbxt_assert!(s.is_empty());
    }
});

file_test!(motionbuilder_cube, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    ufbxt_assert!(scene.metadata.latest_application.name == "MotionBuilder");

    let node = find_node(scene, "Cube").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
});

file_test!(motionbuilder_thumbnail, flags = ALLOW_INVALID_UNICODE | ALLOW_FEWER_PROGRESS_CALLS | ALLOW_WARNINGS, |scene, err, load_error| {
    let thumbnail = &scene.metadata.thumbnail;
    ufbxt_assert!(thumbnail.props.find_int("CustomWidth", 0) == 100);
    ufbxt_assert!(thumbnail.props.find_int("CustomHeight", 0) == 100);
    ufbxt_assert!(thumbnail.format == ThumbnailFormat::Rgba32);
    ufbxt_assert!(thumbnail.width == 128);
    ufbxt_assert!(thumbnail.height == 128);
    ufbxt_assert!(fnv1a(&thumbnail.data) == 0xc9d71343);

    let num_pixels = (thumbnail.width * thumbnail.height) as usize;
    ufbxt_assert!(thumbnail.data.len() == num_pixels * 4);
    let pixels = &thumbnail.data;
    for i in 0..num_pixels {
        let a = pixels[i * 4 + 3];
        ufbxt_assert!(a == 255);
    }
});

/// `ufbxt_read_byte` in test/test_parse.h
struct ByteReader(std::fs::File);

impl Read for ByteReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.0.read(&mut buf[..1])
    }
}

#[test]
fn single_byte_stream() {
    for path in FileIterator::new("maya_cube") {
        let f = std::fs::File::open(&path);
        ufbxt_assert!(f.is_ok());

        let mut stream = ByteReader(f.unwrap());

        let scene = load_stream(&mut stream, &LoadOpts::default());
        if let Err(e) = &scene {
            log_error(e);
        }
        ufbxt_assert!(scene.is_ok());
    }
}

fn depth_limit_3_opts() -> LoadOpts {
    LoadOpts { node_depth_limit: 3, ..Default::default() }
}

fn depth_limit_4_opts() -> LoadOpts {
    LoadOpts { node_depth_limit: 4, ..Default::default() }
}

file_test_allow_error!(node_depth_limit_fail, file = "maya_game_sausage", opts = depth_limit_3_opts(), |scene, err, load_error| {
    ufbxt_assert!(scene.is_none());
    ufbxt_assert!(load_error.kind != ErrorType::None);
    ufbxt_assert!(load_error.kind == ErrorType::NodeDepthLimit);
});

file_test_allow_error!(node_depth_limit_ok, file = "maya_game_sausage", opts = depth_limit_4_opts(), |scene, err, load_error| {
    ufbxt_assert!(scene.is_some());
});

#[test]
fn filename_option_unicode() {
    for path in FileIterator::new("maya_textured_cube") {
        let data = read_file(&path);
        ufbxt_assert!(data.is_some());
        let data = data.unwrap();

        {
            let opts = LoadOpts { filename: "path\0/file.fbx".to_string(), path_separator: b'/', ..Default::default() };

            let scene = load_memory(&data, &opts);
            if let Err(e) = &scene {
                log_error(e);
            }
            ufbxt_assert!(scene.is_ok());
            let scene = scene.unwrap();
            check_scene(&scene);

            ufbxt_assert!(scene.metadata.filename == "path\u{fffd}/file.fbx");
            ufbxt_assert!(scene.metadata.raw_filename.len() == 14);
            ufbxt_assert!(scene.metadata.raw_filename[..9] == b"path\0/file.fbx"[..9]);

            let material = find_material(&scene, "phong1").map(|m| &scene.materials[m]);
            ufbxt_assert!(material.is_some());
            let texture = material.unwrap().fbx.diffuse_color().texture.map(|t| &scene.textures[t]);
            ufbxt_assert!(texture.is_some());
            let texture = texture.unwrap();

            ufbxt_assert!(texture.filename == "path\u{fffd}/textures/checkerboard_diffuse.png");
            ufbxt_assert!(texture.raw_filename.len() == 39);
            ufbxt_assert!(texture.raw_filename == b"path\0/textures/checkerboard_diffuse.png");
        }

        {
            let opts = LoadOpts { raw_filename: b"path\0/file.fbx".to_vec(), path_separator: b'\\', ..Default::default() };

            let scene = load_memory(&data, &opts);
            if let Err(e) = &scene {
                log_error(e);
            }
            ufbxt_assert!(scene.is_ok());
            let scene = scene.unwrap();
            check_scene(&scene);

            ufbxt_assert!(scene.metadata.filename == "path\u{fffd}/file.fbx");
            ufbxt_assert!(scene.metadata.raw_filename.len() == 14);
            ufbxt_assert!(scene.metadata.raw_filename == b"path\0/file.fbx");

            let material = find_material(&scene, "phong1").map(|m| &scene.materials[m]);
            ufbxt_assert!(material.is_some());
            let texture = material.unwrap().fbx.diffuse_color().texture.map(|t| &scene.textures[t]);
            ufbxt_assert!(texture.is_some());
            let texture = texture.unwrap();

            ufbxt_assert!(texture.filename == "path\u{fffd}\\textures\\checkerboard_diffuse.png");
            ufbxt_assert!(texture.raw_filename.len() == 39);
            ufbxt_assert!(texture.raw_filename == b"path\0\\textures\\checkerboard_diffuse.png");
        }
    }
}

const HOLE_REF: [i32; 16] = [0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 0, 0, 0];

file_test!(synthetic_integer_holes, |scene, err, load_error| {
    let node = find_node(scene, "pPlane1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];

    ufbxt_assert!(mesh.face_hole.len() == 16);
    for i in 0..16 {
        let hole = mesh.face_hole[i];
        let reference = HOLE_REF[i] != 0;
        ufbxt_assert!(hole == reference);
    }
});

file_test!(synthetic_integer_holes_threads, file = "synthetic_integer_holes", flags = FUZZ_ALWAYS | FUZZ_OPTS, |scene, err, load_error| {
    let node = find_node(scene, "pPlane1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];

    ufbxt_assert!(mesh.face_hole.len() == 16);
    for i in 0..16 {
        let hole = mesh.face_hole[i];
        let reference = HOLE_REF[i] != 0;
        ufbxt_assert!(hole == reference);
    }
});

file_test!(synthetic_comment_cube, |scene, err, load_error| {});

file_test!(synthetic_comment_cube_threads, file = "synthetic_comment_cube", flags = DIFF_ALWAYS | FUZZ_ALWAYS | FUZZ_OPTS, |scene, err, load_error| {});

file_test!(maya_cube_threads, file = "maya_cube", flags = DIFF_ALWAYS | FUZZ_ALWAYS | FUZZ_OPTS, |scene, err, load_error| {});

fn check_base64_len(scene: &Scene, name: &str, content: &[u8], length: usize) {
    let video = find_element(scene, ElementType::Video, name).and_then(|id| scene.as_video(id));
    ufbxt_assert!(video.is_some());
    let video = video.unwrap();

    ufbxt_assert!(video.content.len() == length);
    if length > 0 {
        ufbxt_assert!(video.content[..] == content[..length]);
    }
}

fn check_base64(scene: &Scene, name: &str, content: &str) {
    check_base64_len(scene, name, content.as_bytes(), content.len());
}

fn check_base64_error(scene: &Scene, name: &str) {
    check_base64_len(scene, name, b"", 0);
    check_warning(scene, WarningType::BadBase64Content, ElementType::Video, Some(name), 1, None);
}

file_test!(synthetic_base64_parse, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    check_base64(scene, "Hello", "Hello world");
    check_base64(scene, "Ufbx", "Hello ufbx");
    check_base64(scene, "World", "Hello world!");
    check_base64(scene, "Empty", "");

    check_base64_error(scene, "MidPad");
    check_base64_error(scene, "Len1");
    check_base64_error(scene, "Len2");
    check_base64_error(scene, "Len3");
    check_base64_error(scene, "Len5");
    check_base64_error(scene, "BadChar");
    check_base64_error(scene, "BadCharEnd");

    check_base64_len(scene, "Pad00", b"Hello wor\0\0\0\0", 12);
    check_base64_error(scene, "Pad01");
    check_base64_error(scene, "Pad02");
    check_base64_error(scene, "Pad03");
    check_base64_error(scene, "Pad04");
    check_base64_error(scene, "Pad05");
    check_base64_error(scene, "Pad06");
    check_base64_error(scene, "Pad07");
    check_base64_len(scene, "Pad08", b"Hello wor\0\0\0\0", 11);
    check_base64_error(scene, "Pad09");
    check_base64_error(scene, "Pad10");
    check_base64_error(scene, "Pad11");
    check_base64_len(scene, "Pad12", b"Hello wor\0\0\0\0", 10);
    check_base64_error(scene, "Pad13");
    check_base64_error(scene, "Pad14");
    check_base64_error(scene, "Pad15");
});

fn check_tcdefinition(scene: &Scene) {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());

    ufbxt_assert!(scene.anim.layers.len() == 1);
    let layer = &scene.anim_layers[scene.anim.layers[0]];

    let anim_prop = find_anim_prop(layer, node.unwrap().element.element_id, "Lcl Translation");
    ufbxt_assert!(anim_prop.is_some());

    let anim_curve = scene.anim_values[anim_prop.unwrap().anim_value].curves[0].map(|c| &scene.anim_curves[c]);
    ufbxt_assert!(anim_curve.is_some());
    let anim_curve = anim_curve.unwrap();

    ufbxt_assert!(anim_curve.keyframes.len() == 2);
    ufbxt_assert!(anim_curve.keyframes[0].time == 0.0);
    ufbxt_assert!(anim_curve.keyframes[1].time == 1.0);
}

file_test!(synthetic_tcdefinition_127, |scene, err, load_error| {
    check_tcdefinition(scene);
});

file_test!(synthetic_tcdefinition_0, |scene, err, load_error| {
    check_tcdefinition(scene);
});

file_test!(synthetic_tcdefinition_64, |scene, err, load_error| {
    check_tcdefinition(scene);
});

file_test!(synthetic_tcdefinition_0_old_header, |scene, err, load_error| {
    check_tcdefinition(scene);
});

file_test!(synthetic_unsupported_cube, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    if scene.metadata.version == 2000 {
        ufbxt_assert!(!scene.metadata.ascii);
        check_warning(scene, WarningType::UnsupportedVersion, ElementType::Unknown, None, 1, Some("(2000)"));
    } else if scene.metadata.version == 8000 {
        ufbxt_assert!(scene.metadata.ascii);
        check_warning(scene, WarningType::UnsupportedVersion, ElementType::Unknown, None, 1, Some("(8000)"));
    } else {
        ufbxt_assert!(false);
    }
});

file_test_allow_error!(synthetic_unsupported_cube_fail, |scene, err, load_error| {
    ufbxt_assert!(scene.is_none());
    ufbxt_assert!(load_error.description == "Unsupported version");
    ufbxt_assert!(!load_error.description.contains('\0'));
    ufbxt_assert!(load_error.kind == ErrorType::UnsupportedVersion);
    ufbxt_assert!(load_error.info == "2000" || load_error.info == "8000");
});

fn check_user_prop_int(props: &Props, name: &str, prop_type: PropType, a: i64, b: i64, c: i64) {
    ufbxt_hintf!("prop={name}");

    let prop = props.find(name);
    ufbxt_assert!(prop.is_some());
    let prop = prop.unwrap();

    ufbxt_assert!(prop.prop_type == prop_type);
    ufbxt_assert!(prop.value_int == a);
    ufbxt_assert!(prop.flags & prop_flags::ANIMATABLE != 0);
    ufbxt_assert!(prop.flags & prop_flags::VALUE_INT != 0);
    ufbxt_assert!(prop.flags & prop_flags::VALUE_VEC3 != 0);
    ufbxt_assert!(prop.value_real_arr[1] == b as Real);
    ufbxt_assert!(prop.value_real_arr[2] == c as Real);
}

fn check_user_prop_float(err: &mut DiffError, props: &Props, name: &str, prop_type: PropType, a: f64, b: f64, c: f64) {
    ufbxt_hintf!("prop={name}");

    let prop = props.find(name);
    ufbxt_assert!(prop.is_some());
    let prop = prop.unwrap();

    ufbxt_assert!(prop.prop_type == prop_type);
    assert_close_real(err, prop.value_real(), a as Real);
    ufbxt_assert!(prop.flags & prop_flags::ANIMATABLE != 0);
    ufbxt_assert!(prop.flags & prop_flags::VALUE_VEC3 != 0);
    assert_close_real(err, prop.value_real_arr[1], b as Real);
    assert_close_real(err, prop.value_real_arr[2], c as Real);
}

fn check_user_prop_bool(props: &Props, name: &str, prop_type: PropType, value: bool) {
    ufbxt_hintf!("prop={name}");

    let prop = props.find(name);
    ufbxt_assert!(prop.is_some());
    let prop = prop.unwrap();

    ufbxt_assert!(prop.prop_type == prop_type);
    ufbxt_assert!(prop.value_int == value as i64);
    ufbxt_assert!(prop.flags & prop_flags::ANIMATABLE != 0);
    ufbxt_assert!(prop.flags & prop_flags::VALUE_INT != 0);
    ufbxt_assert!(prop.flags & prop_flags::VALUE_REAL != 0);
}

fn check_user_prop_string(props: &Props, name: &str, prop_type: PropType, s: &str) {
    ufbxt_hintf!("prop={name}");

    let prop = props.find(name);
    ufbxt_assert!(prop.is_some());
    let prop = prop.unwrap();

    ufbxt_assert!(prop.prop_type == prop_type);
    ufbxt_assert!(prop.value_str == s);
    ufbxt_assert!((prop.flags & prop_flags::ANIMATABLE) == 0);
    ufbxt_assert!(prop.flags & prop_flags::VALUE_STR != 0);
}

fn check_user_prop_enum(props: &Props, name: &str, prop_type: PropType, value: i64, _s: &str) {
    ufbxt_hintf!("prop={name}");

    let prop = props.find(name);
    ufbxt_assert!(prop.is_some());
    let prop = prop.unwrap();

    ufbxt_assert!(prop.prop_type == prop_type);
    ufbxt_assert!(prop.value_int == value);
    ufbxt_assert!(prop.flags & prop_flags::ANIMATABLE != 0);
    ufbxt_assert!(prop.flags & prop_flags::VALUE_INT != 0);
    ufbxt_assert!(prop.flags & prop_flags::VALUE_REAL != 0);
    ufbxt_assert!(prop.flags & prop_flags::VALUE_STR != 0);
}

file_test!(maya_user_props, |scene, err, load_error| {
    {
        let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());

        let props = &node.unwrap().element.props;

        check_user_prop_int(props, "User_Int_Unbounded", PropType::Integer, 1, 1, 1);
        check_user_prop_int(props, "User_Int_Lower", PropType::Integer, 2, 1, 1);
        check_user_prop_int(props, "User_Int_Upper", PropType::Integer, 3, 10, 10);
        check_user_prop_int(props, "User_Int_Bounded", PropType::Integer, 4, 1, 10);

        check_user_prop_float(err, props, "User_Float_Unbounded", PropType::Number, 0.1, 0.1, 0.1);
        check_user_prop_float(err, props, "User_Float_Lower", PropType::Number, 0.2, 0.1, 0.1);
        check_user_prop_float(err, props, "User_Float_Upper", PropType::Number, 0.3, 1.0, 1.0);
        check_user_prop_float(err, props, "User_Float_Bounded", PropType::Number, 0.4, 0.0, 1.0);

        check_user_prop_float(err, props, "User_Vector", PropType::Vector, 0.1, 0.2, 0.3);
        check_user_prop_bool(props, "User_Bool", PropType::Boolean, true);
        check_user_prop_string(props, "User_String", PropType::String, "Hello");
        check_user_prop_enum(props, "User_Enum", PropType::Integer, 1, "ValueA~ValueB~ValueC");
    }

    {
        let state = evaluate_scene(scene, &scene.anim, 1.0, &EvaluateOpts::default());
        ufbxt_assert!(state.is_ok());
        let state = state.unwrap();

        let node = find_node(&state, "pCube1").map(|n| &state.nodes[n]);
        ufbxt_assert!(node.is_some());

        let props = &node.unwrap().element.props;

        check_user_prop_int(props, "User_Int_Unbounded", PropType::Integer, 2, 0, 0);
        check_user_prop_int(props, "User_Int_Lower", PropType::Integer, 3, 0, 0);
        check_user_prop_int(props, "User_Int_Upper", PropType::Integer, 4, 0, 0);
        check_user_prop_int(props, "User_Int_Bounded", PropType::Integer, 5, 0, 0);

        check_user_prop_float(err, props, "User_Float_Unbounded", PropType::Number, 1.0, 0.0, 0.0);
        check_user_prop_float(err, props, "User_Float_Lower", PropType::Number, 2.0, 0.0, 0.0);
        check_user_prop_float(err, props, "User_Float_Upper", PropType::Number, -1.0, 0.0, 0.0);
        check_user_prop_float(err, props, "User_Float_Bounded", PropType::Number, 0.5, 0.0, 0.0);

        check_user_prop_float(err, props, "User_Vector", PropType::Vector, -0.1, 0.4, -0.6);
        check_user_prop_bool(props, "User_Bool", PropType::Boolean, false);
        check_user_prop_string(props, "User_String", PropType::String, "Hello");
        check_user_prop_enum(props, "User_Enum", PropType::Integer, 2, "ValueA~ValueB~ValueC");
    }
});

fn check_dom_id(scene: &Scene, node: &Node, id: i64) {
    ufbxt_hintf!("node={}", node.element.name);
    let dom_node = element_dom_node(scene, &node.element);
    ufbxt_assert!(dom_node.is_some());
    let dom_node = dom_node.unwrap();
    ufbxt_assert!(dom_node.values[0].value_type == DomValueType::Number);
    ufbxt_assert!(dom_node.values[0].value_int == id);
}

file_test!(synthetic_negative_fbx_id, opts = retain_dom_opts(), |scene, err, load_error| {
    let root = scene.root_node;
    let minus_one = find_node(scene, "MinusOne");
    let min_node = find_node(scene, "Min");
    let max_node = find_node(scene, "Max");
    let one = find_node(scene, "One");

    ufbxt_assert!(minus_one.is_some());
    ufbxt_assert!(min_node.is_some());
    ufbxt_assert!(max_node.is_some());
    ufbxt_assert!(one.is_some());

    ufbxt_assert!(scene.nodes[minus_one.unwrap()].parent == Some(root));
    ufbxt_assert!(scene.nodes[min_node.unwrap()].parent == minus_one);
    ufbxt_assert!(scene.nodes[max_node.unwrap()].parent == min_node);
    ufbxt_assert!(scene.nodes[one.unwrap()].parent == max_node);

    check_dom_id(scene, &scene.nodes[minus_one.unwrap()], -1);
    check_dom_id(scene, &scene.nodes[min_node.unwrap()], i64::MIN);
    check_dom_id(scene, &scene.nodes[max_node.unwrap()], i64::MAX);
    check_dom_id(scene, &scene.nodes[one.unwrap()], 1);
});

fn check_inf_nan(scene: &Scene) {
    {
        let inf_node = find_node(scene, "INF").map(|n| &scene.nodes[n]);
        ufbxt_assert!(inf_node.is_some());
        let inf_node = inf_node.unwrap();
        let scaling_max = inf_node.element.props.find_vec3("ScalingMax", Vec3::ZERO);
        assert_vec3_equal(scaling_max, Real::INFINITY, -Real::INFINITY, Real::INFINITY);

        let inf = &scene.meshes[inf_node.mesh.unwrap()];
        let values: Vec<Real> = inf.vertex_position.values.iter().flat_map(|v| [v.x, v.y, v.z]).collect();
        ufbxt_assert!(inf.vertex_position.values.len() == 4);
        for i in 0..12 {
            ufbxt_assert!(values[i].is_infinite());
            ufbxt_assert!((values[i] < 0.0) == (i >= 6));
        }
    }

    {
        let nan_node = find_node(scene, "NAN").map(|n| &scene.nodes[n]);
        ufbxt_assert!(nan_node.is_some());
        let nan_node = nan_node.unwrap();
        let scaling_max = nan_node.element.props.find_vec3("ScalingMax", Vec3::ZERO);
        assert_vec3_equal(scaling_max, Real::NAN, Real::NAN, Real::NAN);

        let nan = &scene.meshes[nan_node.mesh.unwrap()];
        let values: Vec<Real> = nan.vertex_position.values.iter().flat_map(|v| [v.x, v.y, v.z]).collect();
        ufbxt_assert!(nan.vertex_position.values.len() == 4);
        for value in &values[..12] {
            ufbxt_assert!(value.is_nan());
        }
    }
}

file_test!(synthetic_inf_nan, |scene, err, load_error| {
    check_inf_nan(scene);
});

file_test!(synthetic_inf_nan_threaded, file = "synthetic_inf_nan", |scene, err, load_error| {
    check_inf_nan(scene);
});

file_test!(synthetic_bad_inf_nan, flags = ALLOW_THREAD_ERROR, |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];

    let pos = &mesh.vertices;
    ufbxt_assert!(mesh.vertices.len() == 8);
    assert_vec3_equal(pos[0], 1.0, 2.0, b's' as Real);
    assert_vec3_equal(pos[1], 1.0, 2.0, b'i' as Real);
    assert_vec3_equal(pos[2], 1.0, 2.0, b'i' as Real);
    assert_vec3_equal(pos[3], 1.0, 2.0, b'n' as Real);
    assert_vec3_equal(pos[4], 1.0, 2.0, b'n' as Real);
    assert_vec3_equal(pos[5], 1.0, 2.0, Real::NAN);
    assert_vec3_equal(pos[6], 1.0, 2.0, b'N' as Real);
    assert_vec3_equal(pos[7], 1.0, 2.0, Real::INFINITY);
});

file_test_allow_error!(synthetic_bad_inf_nan_fail, |scene, err, load_error| {
    ufbxt_assert!(scene.is_none());
    ufbxt_assert!(load_error.kind == ErrorType::Unknown);
});

file_test!(dom_get_array, file = "maya_anim_extrapolation", opts = retain_dom_opts(), |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let node = node.unwrap();

    let dom_node = element_dom_node(scene, &node.element);
    ufbxt_assert!(dom_node.is_some());

    let dom_node_not_found = dom_node.unwrap().find("Non-Existent");
    ufbxt_assert!(dom_node_not_found.is_none());

    ufbxt_assert!(scene.anim_layers.len() == 1);
    let anim_layer = &scene.anim_layers[0];

    let anim_prop = find_anim_prop(anim_layer, node.element.element_id, "Lcl Translation");
    ufbxt_assert!(anim_prop.is_some());

    let anim_curve = scene.anim_values[anim_prop.unwrap().anim_value].curves[0].map(|c| &scene.anim_curves[c]);
    ufbxt_assert!(anim_curve.is_some());

    let dom_curve = element_dom_node(scene, &anim_curve.unwrap().element);
    ufbxt_assert!(dom_curve.is_some());
    let dom_curve = dom_curve.unwrap();

    if scene.metadata.version >= 7000 {
        let attr_data = dom_curve.find("KeyAttrDataFloat");
        let attr_data_int32 = attr_data.map_or(&[][..], |n| n.as_int32_list());
        let attr_data_float = attr_data.map_or(&[][..], |n| n.as_float_list());

        let mut attr_int = [0i32; 8];
        if scene.metadata.ascii && scene.metadata.version >= 7200 {
            ufbxt_assert!(attr_data_int32.len() == 8);
            ufbxt_assert!(attr_data_float.is_empty());
            attr_int.copy_from_slice(attr_data_int32);
        } else {
            ufbxt_assert!(attr_data_float.len() == 8);
            ufbxt_assert!(attr_data_int32.is_empty());
            for (dst, src) in attr_int.iter_mut().zip(attr_data_float) {
                *dst = src.to_bits() as i32;
            }
        }

        let attr_ref = [0, 0, 218434821, 0, 0, 0, 218434821, 0];
        for i in 0..attr_ref.len() {
            ufbxt_assert!(attr_int[i] == attr_ref[i]);
        }

        let key_time = dom_curve.find("KeyTime").map_or(&[][..], |n| n.as_int64_list());
        ufbxt_assert!(key_time.len() == 2);
        ufbxt_assert!(key_time[0] == 1924423250);
        ufbxt_assert!(key_time[1] == 46186158000);

        let key_value = dom_curve.find("KeyValueFloat").map_or(&[][..], |n| n.as_real_list());
        ufbxt_assert!(key_value.len() == 2);
        ufbxt_assert!(key_value[0] == 0.0);
        ufbxt_assert!(key_value[1] == 10.0);
    } else {
        let dom_key = dom_curve.find("Key");
        ufbxt_assert!(dom_key.is_some());
        let key = dom_key.unwrap().as_double_list();

        let key_ref =
            [1924423250.0, 0.0, b'U' as f64, b's' as f64, 0.0, 0.0, b'n' as f64, 46186158000.0, 10.0, b'U' as f64, b's' as f64, 0.0, 0.0, b'n' as f64];
        ufbxt_assert!(key.len() == key_ref.len());
        for i in 0..key_ref.len() {
            ufbxt_assert!(key[i] == key_ref[i]);
        }
    }
});

file_test_allow_error!(synthetic_truncated_compressed_fail, |scene, err, load_error| {
    ufbxt_assert!(scene.is_none());
});

#[test]
fn synthetic_truncated_compressed_thread_memory() {
    for path in FileIterator::new("synthetic_truncated_compressed_fail") {
        let data = read_file(&path);
        ufbxt_assert!(data.is_some());

        let opts = LoadOpts::default();

        let scene = load_memory(&data.unwrap(), &opts);
        ufbxt_assert!(scene.is_err());
    }
}

fn unicode_remove_opts() -> LoadOpts {
    LoadOpts { unicode_error_handling: UnicodeErrorHandling::Remove, ..Default::default() }
}

file_test!(synthetic_full_bad_unicode, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    ufbxt_assert!(scene.metadata.original_file_path.len() == 9);
    ufbxt_assert!(scene.metadata.original_file_path == "\u{fffd}\u{fffd}\u{fffd}");

    ufbxt_assert!(scene.metadata.raw_original_file_path.len() == 3);
    ufbxt_assert!(scene.metadata.raw_original_file_path == b"\xff\xff\xff");

    let prop = scene.metadata.scene_props.find("SrcDocumentUrl");
    ufbxt_assert!(prop.is_some());
    let prop = prop.unwrap();

    ufbxt_assert!(prop.value_str.len() == 18);
    ufbxt_assert!(prop.value_str == "\u{fffd}\u{fffd}\u{fffd}\u{fffd}\u{fffd}\u{fffd}");

    ufbxt_assert!(prop.value_blob.len() == 6);
    ufbxt_assert!(prop.value_blob == b"\xfc\x80\x80\x80\x80\x80");
});

file_test!(
    synthetic_full_bad_unicode_remove,
    file = "synthetic_full_bad_unicode",
    opts = unicode_remove_opts(),
    flags = ALLOW_INVALID_UNICODE | FUZZ_OPTS,
    |scene, err, load_error| {
        ufbxt_assert!(scene.metadata.original_file_path.is_empty());
        ufbxt_assert!(scene.metadata.original_file_path == "");

        ufbxt_assert!(scene.metadata.raw_original_file_path.len() == 3);
        ufbxt_assert!(scene.metadata.raw_original_file_path == b"\xff\xff\xff");

        let prop = scene.metadata.scene_props.find("SrcDocumentUrl");
        ufbxt_assert!(prop.is_some());
        let prop = prop.unwrap();

        ufbxt_assert!(prop.value_str.is_empty());
        ufbxt_assert!(prop.value_str == "");

        ufbxt_assert!(prop.value_blob.len() == 6);
        ufbxt_assert!(prop.value_blob == b"\xfc\x80\x80\x80\x80\x80");
    }
);
