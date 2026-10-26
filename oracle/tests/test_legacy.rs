mod ufbxt;
use ufbxt::*;

fn diff_material_value(err: &mut DiffError, color: &MaterialMap, factor: &MaterialMap, value: Vec3) {
    assert_close_real(err, color.value_vec3().x * factor.value_vec3().x, value.x);
    assert_close_real(err, color.value_vec3().y * factor.value_vec3().x, value.y);
    assert_close_real(err, color.value_vec3().z * factor.value_vec3().x, value.z);
}

fn find_material_element<'a>(scene: &'a Scene, name: &str) -> Option<&'a Material> {
    find_element(scene, ElementType::Material, name).and_then(|id| scene.as_material(id))
}

file_test!(max7_cube, |scene, err, load_error| {});

file_test!(max7_cube_normals, |scene, err, load_error| {});

file_test!(max2009_blob, |scene, err, load_error| {
    ufbxt_assert!(scene.lights.len() == 3);
    ufbxt_assert!(scene.cameras.len() == 1);

    {
        let node = find_node(scene, "Box01").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.mesh.is_some());
        ufbxt_assert!(node.children.len() == 16);
        let mesh = &scene.meshes[node.mesh.unwrap()];

        let mut num_top = 0;
        let mut num_left = 0;
        let mut num_right = 0;
        let mut num_front = 0;

        let face_material_name = |fi: usize| &scene.materials[mesh.materials[mesh.face_material[fi] as usize].unwrap()].element.name;

        for fi in 0..mesh.num_faces {
            let face = mesh.faces[fi];
            let mut center = Vec3::ZERO;
            for i in 0..face.num_indices {
                let v = mesh.vertex_position.get((face.index_begin + i) as usize);
                center.x += v.x;
                center.y += v.y;
                center.z += v.z;
            }
            center.x /= face.num_indices as Real;
            center.y /= face.num_indices as Real;
            center.z /= face.num_indices as Real;

            if center.z >= 14.0 {
                ufbxt_assert!(face_material_name(fi) == "Top");
                num_top += 1;
            }
            if center.y <= -10.0 {
                ufbxt_assert!(face_material_name(fi) == "Right");
                num_right += 1;
            }
            if center.y >= 10.0 {
                ufbxt_assert!(face_material_name(fi) == "Left");
                num_left += 1;
            }
            if center.x >= 9.0 {
                ufbxt_assert!(face_material_name(fi) == "Front");
                num_front += 1;
            }
        }

        ufbxt_assert!(num_top >= 80);
        ufbxt_assert!(num_left >= 80);
        ufbxt_assert!(num_right >= 80);
        ufbxt_assert!(num_front >= 80);
    }

    {
        let node = find_node(scene, "Omni01").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.light.is_some()));
        let light = &scene.lights[node.unwrap().light.unwrap()];
        ufbxt_assert!(light.light_type == LightType::Point);
        if scene.metadata.version < 6000 {
            ufbxt_assert!(light.decay == LightDecay::None);
        }
        let color = Vec3::new(0.172549024224281, 0.364705890417099, 1.0);
        assert_close_vec3(err, light.color, color);
        assert_close_real(err, light.intensity, 1.0);
    }

    {
        let node = find_node(scene, "Fspot01").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.light.is_some()));
        let light = &scene.lights[node.unwrap().light.unwrap()];
        ufbxt_assert!(light.light_type == LightType::Spot);
        if scene.metadata.version < 6000 {
            ufbxt_assert!(light.decay == LightDecay::None);
        }
        let color = Vec3::new(0.972549080848694, 0.0705882385373116, 0.0705882385373116);
        assert_close_vec3(err, light.color, color);
        assert_close_real(err, light.intensity, 1.0);
        assert_close_real(err, light.outer_angle, 45.0);
    }

    {
        let node = find_node(scene, "FDirect02").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.light.is_some()));
        let light = &scene.lights[node.unwrap().light.unwrap()];
        ufbxt_assert!(light.light_type == LightType::Directional);
        if scene.metadata.version < 6000 {
            ufbxt_assert!(light.decay == LightDecay::None);
        }
        let color = Vec3::new(0.533333361148834, 0.858823597431183, 0.647058844566345);
        assert_close_vec3(err, light.color, color);
        assert_close_real(err, light.intensity, 1.0);
    }

    {
        let node = find_node(scene, "Camera01").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.camera.is_some()));
        let camera = &scene.cameras[node.unwrap().camera.unwrap()];
        ufbxt_assert!(camera.aspect_mode == AspectMode::WindowSize);
        ufbxt_assert!(camera.aperture_mode == ApertureMode::Horizontal);
        let aperture = Vec2::new(1.41732287406921, 1.06299209594727);
        assert_close_real(err, camera.focal_length_mm, 43.4558439883016);
        assert_close_vec2(err, camera.film_size_inch, aperture);
        assert_close_vec2(err, camera.aperture_size_inch, aperture);
    }

    {
        let material = find_material_element(scene, "Left");
        ufbxt_assert!(material.is_some());
        let fbx = &material.unwrap().fbx;
        let ambient = Vec3::new(0.588235318660736, 0.588235318660736, 0.588235318660736);
        let diffuse = Vec3::new(0.588235318660736, 0.588235318660736, 0.588235318660736);
        let specular = Vec3::new(0.179999984502793, 0.179999984502793, 0.179999984502793);
        let emission = Vec3::new(0.823529481887817, 0.0, 0.0);
        let shininess = 1.99999991737042;
        diff_material_value(err, fbx.ambient_color(), fbx.ambient_factor(), ambient);
        diff_material_value(err, fbx.diffuse_color(), fbx.diffuse_factor(), diffuse);
        diff_material_value(err, fbx.specular_color(), fbx.specular_factor(), specular);
        diff_material_value(err, fbx.emission_color(), fbx.emission_factor(), emission);
        assert_close_real(err, fbx.specular_exponent().value_vec3().x, shininess);
    }

    {
        let material = find_material_element(scene, "Front");
        ufbxt_assert!(material.is_some());
        let fbx = &material.unwrap().fbx;
        let ambient = Vec3::new(0.588235318660736, 0.921568691730499, 0.925490260124207);
        let diffuse = Vec3::new(0.588235318660736, 0.921568691730499, 0.925490260124207);
        let specular = Vec3::new(0.0, 0.0, 0.0);
        let emission = Vec3::new(0.0, 0.0, 0.0);
        let shininess = 1.99999991737042;
        diff_material_value(err, fbx.ambient_color(), fbx.ambient_factor(), ambient);
        diff_material_value(err, fbx.diffuse_color(), fbx.diffuse_factor(), diffuse);
        diff_material_value(err, fbx.specular_color(), fbx.specular_factor(), specular);
        diff_material_value(err, fbx.emission_color(), fbx.emission_factor(), emission);
        assert_close_real(err, fbx.specular_exponent().value_vec3().x, shininess);
    }

    {
        let material = find_material_element(scene, "Right");
        ufbxt_assert!(material.is_some());
        let fbx = &material.unwrap().fbx;
        let ambient = Vec3::new(0.588235318660736, 0.588235318660736, 0.588235318660736);
        let diffuse = Vec3::new(0.588235318660736, 0.588235318660736, 0.588235318660736);
        let specular = Vec3::new(0.0, 0.0, 0.0);
        let emission = Vec3::new(0.0, 0.803921639919281, 0.0);
        let shininess = 7.99999900844507;
        diff_material_value(err, fbx.ambient_color(), fbx.ambient_factor(), ambient);
        diff_material_value(err, fbx.diffuse_color(), fbx.diffuse_factor(), diffuse);
        diff_material_value(err, fbx.specular_color(), fbx.specular_factor(), specular);
        diff_material_value(err, fbx.emission_color(), fbx.emission_factor(), emission);
        assert_close_real(err, fbx.specular_exponent().value_vec3().x, shininess);
    }

    {
        let material = find_material_element(scene, "Top");
        ufbxt_assert!(material.is_some());
        let fbx = &material.unwrap().fbx;
        let ambient = Vec3::new(0.564705908298492, 0.603921592235565, 0.890196144580841);
        let diffuse = Vec3::new(0.564705908298492, 0.603921592235565, 0.890196144580841);
        let specular = Vec3::new(0.0, 0.0, 0.0);
        let emission = Vec3::new(0.0, 0.0, 0.0);
        let shininess = 1.99999991737042;
        diff_material_value(err, fbx.ambient_color(), fbx.ambient_factor(), ambient);
        diff_material_value(err, fbx.diffuse_color(), fbx.diffuse_factor(), diffuse);
        diff_material_value(err, fbx.specular_color(), fbx.specular_factor(), specular);
        diff_material_value(err, fbx.emission_color(), fbx.emission_factor(), emission);
        assert_close_real(err, fbx.specular_exponent().value_vec3().x, shininess);
    }

    check_frame(scene, err, false, "max2009_blob_8", None, 8.0 / 30.0);
    check_frame(scene, err, false, "max2009_blob_18", None, 18.0 / 30.0);
});

file_test!(max7_skin, |scene, err, load_error| {
    check_frame(scene, err, false, "max7_skin_5", None, 5.0 / 30.0);
    check_frame(scene, err, false, "max7_skin_15", None, 15.0 / 30.0);
});

file_test!(max7_blend_cube, |scene, err, load_error| {
    let node = find_node(scene, "Box01").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    ufbxt_assert!(node.unwrap().mesh.is_some());
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
    ufbxt_assert!(mesh.blend_deformers.len() == 1);
    let blend = &scene.blend_deformers[mesh.blend_deformers[0]];
    ufbxt_assert!(blend.channels.len() == 2);

    check_frame(scene, err, false, "max7_blend_cube_8", None, 8.0 / 30.0);
    check_frame(scene, err, false, "max7_blend_cube_24", None, 24.0 / 30.0);
});

file_test!(max6_teapot, |scene, err, load_error| {
    let node = find_node(scene, "Teapot01").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    ufbxt_assert!(node.unwrap().mesh.is_some());
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
    ufbxt_assert!(mesh.vertex_normal.exists);
    ufbxt_assert!(mesh.vertex_uv.exists);
});

file_test!(synthetic_legacy_nonzero_material, |scene, err, load_error| {
    let node = find_node(scene, "Box01").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
    ufbxt_assert!(mesh.faces.len() == 1);
    ufbxt_assert!(mesh.num_indices == 3);
    ufbxt_assert!(mesh.materials.len() == 2);
    ufbxt_assert!(scene.materials[mesh.materials[0].unwrap()].element.name == "Right");
    ufbxt_assert!(mesh.material_parts[0].num_faces == 0);
    ufbxt_assert!(scene.materials[mesh.materials[1].unwrap()].element.name == "Left");
    ufbxt_assert!(mesh.material_parts[1].num_faces == 1);
    ufbxt_assert!(mesh.face_material.len() == 1);
    ufbxt_assert!(mesh.face_material[0] == 1);
});

file_test_allow_error!(synthetic_legacy_unquoted_child_fail, |scene, err, load_error| {
    ufbxt_assert!(scene.is_none());
    ufbxt_assert!(load_error.kind != ErrorType::None);
});

file_test!(max2009_cube_texture, |scene, err, load_error| {
    ufbxt_assert!(scene.videos.len() == 1);
    let video = &scene.videos[0];
    ufbxt_assert!(video.relative_filename == "..\\..\\tiny_clouds.png");
    ufbxt_assert!(video.absolute_filename == "C:\\Documents and Settings\\XP3\\My Documents\\tiny_clouds.png");
    if scene.metadata.version >= 6000 {
        ufbxt_assert!(video.element.name == "Clouds");
    } else {
        ufbxt_assert!(video.element.name == "tiny_clouds");
    }
    if !scene.metadata.ascii {
        check_blob_content(&video.content, "textures/tiny_clouds.png");
    }

    if scene.metadata.version >= 6000 {
        ufbxt_assert!(scene.textures.len() == 1);
        let texture = &scene.textures[0];
        ufbxt_assert!(texture.element.name == "Clouds");
        if !scene.metadata.ascii {
            check_blob_content(&texture.content, "textures/tiny_clouds.png");
        }
    }

    let node = find_node(scene, "Box01").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let node = node.unwrap();
    ufbxt_assert!(node.mesh.is_some());
    ufbxt_assert!(node.materials.len() == 1);

    let material = &scene.materials[node.materials[0].unwrap()];
    ufbxt_assert!(material.element.name == "Clouds");

    // The 5800 file does not seem to link the video to the material in any way??
    if scene.metadata.version >= 6000 {
        ufbxt_assert!(material.fbx.diffuse_color().texture.is_some());
    }
});

file_test!(max2009_cube_anim, |scene, err, load_error| {
    check_frame(scene, err, false, "max2009_cube_anim_15", None, 15.0 / 30.0);
    check_frame(scene, err, false, "max2009_cube_anim_45", None, 45.0 / 30.0);

    ufbxt_assert!(scene.settings.frames_per_second == 30.0);

    {
        let stack = find_anim_stack(scene, "Take 001").map(|s| &scene.anim_stacks[s]);
        ufbxt_assert!(stack.is_some());
        let stack = stack.unwrap();

        assert_close_double(err, stack.time_begin, 0.0 / 30.0);
        assert_close_double(err, stack.time_end, 60.0 / 30.0);

        ufbxt_assert!(stack.element.props.find_int("LocalStart", i64::MIN) == 0);
        ufbxt_assert!(stack.element.props.find_int("LocalStop", i64::MIN) == 92372316000);
        ufbxt_assert!(stack.element.props.find_int("ReferenceStart", i64::MIN) == 0);
        ufbxt_assert!(stack.element.props.find_int("ReferenceStop", i64::MIN) == 92372316000);
    }
});
