mod ufbxt;
use ufbxt::*;

file_test!(zbrush_vertex_color, |scene, err, load_error| {
    ufbxt_assert!(scene.nodes.len() == 2);
    let node = &scene.nodes[1];
    ufbxt_assert!(node.mesh.is_some());
    let mesh = &scene.meshes[node.mesh.unwrap()];

    ufbxt_assert!(mesh.vertex_color.exists);
    ufbxt_assert!(mesh.vertex_color.unique_per_vertex);

    ufbxt_assert!(mesh.num_vertices == 6);
    for i in 0..mesh.num_vertices {
        let mut pos = mesh.vertex_position.values[i];
        let color = mesh.vertex_color.get(mesh.vertex_first_index[i] as usize);
        let mut r = Vec4::new(0.0, 0.0, 0.0, 1.0);

        pos.y -= 1.0;

        if pos.x < -0.5 {
            r.x = 1.0;
        } else if pos.x > 0.5 {
            r.y = 1.0;
            r.z = 1.0;
        } else if pos.y > 0.5 {
            r.y = 1.0;
        } else if pos.y < -0.5 {
            r.x = 1.0;
            r.z = 1.0;
        } else if pos.z > 0.5 {
            r.z = 1.0;
        } else if pos.z < -0.5 {
            r.x = 1.0;
            r.y = 1.0;
        }

        assert_close_vec4(err, color, r);
        assert_close_vec4(err, color, r);
    }
});

file_test!(synthetic_color_suzanne, |scene, err, load_error| {
    ufbxt_assert!(scene.nodes.len() == 2);
    let node = &scene.nodes[1];
    ufbxt_assert!(node.mesh.is_some());
    let mesh = &scene.meshes[node.mesh.unwrap()];

    ufbxt_assert!(mesh.vertex_color.exists);
    ufbxt_assert!(mesh.vertex_color.unique_per_vertex);

    ufbxt_assert!(mesh.num_faces == 500);
    ufbxt_assert!(mesh.num_triangles == 968);

    for i in 0..mesh.num_indices {
        let position = mesh.vertex_position.get(i);
        let color = mesh.vertex_color.get(i);

        let col = Vec3::new(color.x, color.y, color.z);

        let r = Vec3::new(clamp(position.x * 0.5 + 0.5, 0.0, 1.0), clamp(position.y * 0.5 + 0.5, 0.0, 1.0), clamp(position.z * 0.5 + 0.5, 0.0, 1.0));

        assert_close_vec3_threshold(err, col, r, 1.0 / 256.0);
    }
});

fn check_obj_elements(err: &mut DiffError, scene: &Scene, v: i32, vt: i32, vn: i32, vc: i32, name: &str) {
    ufbxt_hintf!("name = \"{name}\"");

    let node = find_node(scene, name).map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
    ufbxt_assert!(mesh.element.name == name);

    ufbxt_assert!(mesh.num_faces == 1);
    ufbxt_assert!(mesh.num_triangles == 1);

    let face = mesh.faces[0];
    ufbxt_assert!(face.index_begin == 0);
    ufbxt_assert!(face.num_indices == 3);

    if v > 0 {
        ufbxt_assert!(mesh.vertex_position.exists);
        ufbxt_assert!(mesh.vertex_position.indices.len() == 3);
        let (v, w) = (v as Real, (v - 1) as Real);
        let refs = [Vec3::new(-v, 0.0, w), Vec3::new(v, 0.0, w), Vec3::new(0.0, v, w)];
        for (ix, r) in refs.into_iter().enumerate() {
            assert_close_vec3(err, mesh.vertex_position.get(ix), r);
        }
    } else {
        ufbxt_assert!(!mesh.vertex_position.exists);
    }

    if vt > 0 {
        ufbxt_assert!(mesh.vertex_uv.exists);
        ufbxt_assert!(mesh.vertex_uv.indices.len() == 3);
        let vt = vt as Real;
        let refs = [Vec2::new(0.0, 0.0), Vec2::new(vt, 0.0), Vec2::new(0.0, vt)];
        for (ix, r) in refs.into_iter().enumerate() {
            assert_close_vec2(err, mesh.vertex_uv.get(ix), r);
        }
    } else {
        ufbxt_assert!(!mesh.vertex_uv.exists);
    }

    if vn > 0 {
        ufbxt_assert!(mesh.vertex_normal.exists);
        ufbxt_assert!(mesh.vertex_normal.indices.len() == 3);
        let vn = vn as Real;
        let refs = [Vec3::new(0.0, -vn, 0.0), Vec3::new(0.0, -vn, 0.0), Vec3::new(0.0, vn, 0.0)];
        for (ix, r) in refs.into_iter().enumerate() {
            assert_close_vec3(err, mesh.vertex_normal.get(ix), r);
        }
    } else {
        ufbxt_assert!(!mesh.vertex_normal.exists);
    }

    if vc > 0 {
        ufbxt_assert!(mesh.vertex_color.exists);
        ufbxt_assert!(mesh.vertex_color.indices.len() == 3);
        let vc = vc as Real;
        let refs = [Vec4::new(vc, 0.0, 0.0, 1.0), Vec4::new(0.0, vc, 0.0, 1.0), Vec4::new(0.0, 0.0, vc, 1.0)];
        for (ix, r) in refs.into_iter().enumerate() {
            assert_close_vec4(err, mesh.vertex_color.get(ix), r);
        }
    } else {
        ufbxt_assert!(!mesh.vertex_color.exists);
    }
}

file_test!(synthetic_mixed_attribs, |scene, err, load_error| {
    ufbxt_assert!(scene.nodes.len() == 9);
    ufbxt_assert!(scene.meshes.len() == 8);
    check_obj_elements(err, scene, 1, 0, 0, 0, "V");
    check_obj_elements(err, scene, 2, 1, 0, 0, "VT");
    check_obj_elements(err, scene, 3, 0, 1, 0, "VN");
    check_obj_elements(err, scene, 4, 2, 2, 0, "VTN");
    check_obj_elements(err, scene, 5, 0, 0, 1, "VC");
    check_obj_elements(err, scene, 6, 3, 0, 2, "VTC");
    check_obj_elements(err, scene, 7, 0, 3, 3, "VNC");
    check_obj_elements(err, scene, 8, 4, 4, 4, "VTNC");
});

file_test!(synthetic_mixed_attribs_reverse, |scene, err, load_error| {
    ufbxt_assert!(scene.nodes.len() == 9);
    ufbxt_assert!(scene.meshes.len() == 8);
    check_obj_elements(err, scene, 1, 0, 0, 0, "V");
    check_obj_elements(err, scene, 2, 1, 0, 0, "VT");
    check_obj_elements(err, scene, 3, 0, 1, 0, "VN");
    check_obj_elements(err, scene, 4, 2, 2, 0, "VTN");
    check_obj_elements(err, scene, 5, 0, 0, 1, "VC");
    check_obj_elements(err, scene, 6, 3, 0, 2, "VTC");
    check_obj_elements(err, scene, 7, 0, 3, 3, "VNC");
    check_obj_elements(err, scene, 8, 4, 4, 4, "VTNC");
});

file_test!(synthetic_mixed_attribs_reuse, |scene, err, load_error| {
    ufbxt_assert!(scene.nodes.len() == 9);
    ufbxt_assert!(scene.meshes.len() == 8);
    check_obj_elements(err, scene, 1, 0, 0, 0, "V");
    check_obj_elements(err, scene, 1, 1, 0, 0, "VT");
    check_obj_elements(err, scene, 1, 0, 1, 0, "VN");
    check_obj_elements(err, scene, 1, 1, 1, 0, "VTN");
    check_obj_elements(err, scene, 2, 0, 0, 1, "VC");
    check_obj_elements(err, scene, 2, 1, 0, 1, "VTC");
    check_obj_elements(err, scene, 2, 0, 1, 1, "VNC");
    check_obj_elements(err, scene, 2, 1, 1, 1, "VTNC");
});

fn no_index_opts() -> LoadOpts {
    LoadOpts { index_error_handling: IndexErrorHandling::NoIndex, ..Default::default() }
}

fn abort_index_opts() -> LoadOpts {
    LoadOpts { index_error_handling: IndexErrorHandling::AbortLoading, ..Default::default() }
}

fn sentinel<T>(attrib: &VertexAttrib<T>, no_index: bool) -> u32 {
    if no_index { NO_INDEX } else { attrib.values.len() as u32 - 1 }
}

fn check_obj_face(mesh: &Mesh, face_ix: usize, v: i32, vt: i32, vn: i32, vc: i32, no_index: bool) {
    ufbxt_hintf!("face_ix = {face_ix}");

    ufbxt_assert!(face_ix < mesh.faces.len());
    let face = mesh.faces[face_ix];
    let a = face.index_begin as usize;
    let b = a + 1;
    let c = a + 2;

    if v != 0 {
        ufbxt_assert!(mesh.vertex_position.indices[a] == (v - 1) as u32);
        ufbxt_assert!(mesh.vertex_position.indices[b] == (v - 1 + 1) as u32);
        ufbxt_assert!(mesh.vertex_position.indices[c] == (v - 1 + 2) as u32);
    } else {
        let sentinel = sentinel(&mesh.vertex_position, no_index);
        ufbxt_assert!(mesh.vertex_position.indices[a] == sentinel);
        ufbxt_assert!(mesh.vertex_position.indices[b] == sentinel);
        ufbxt_assert!(mesh.vertex_position.indices[c] == sentinel);
    }

    if vt != 0 {
        ufbxt_assert!(mesh.vertex_uv.indices[a] == (vt - 1) as u32);
        ufbxt_assert!(mesh.vertex_uv.indices[b] == (vt - 1 + 1) as u32);
        ufbxt_assert!(mesh.vertex_uv.indices[c] == (vt - 1 + 2) as u32);
    } else {
        let sentinel = sentinel(&mesh.vertex_uv, no_index);
        ufbxt_assert!(mesh.vertex_uv.indices[a] == sentinel);
        ufbxt_assert!(mesh.vertex_uv.indices[b] == sentinel);
        ufbxt_assert!(mesh.vertex_uv.indices[c] == sentinel);
    }

    if vn != 0 {
        ufbxt_assert!(mesh.vertex_normal.indices[a] == (vn - 1) as u32);
        ufbxt_assert!(mesh.vertex_normal.indices[b] == (vn - 1 + 1) as u32);
        ufbxt_assert!(mesh.vertex_normal.indices[c] == (vn - 1 + 2) as u32);
    } else {
        let sentinel = sentinel(&mesh.vertex_normal, no_index);
        ufbxt_assert!(mesh.vertex_normal.indices[a] == sentinel);
        ufbxt_assert!(mesh.vertex_normal.indices[b] == sentinel);
        ufbxt_assert!(mesh.vertex_normal.indices[c] == sentinel);
    }

    if vc != 0 {
        ufbxt_assert!(mesh.vertex_color.indices[a] == (vc - 1) as u32);
        ufbxt_assert!(mesh.vertex_color.indices[b] == (vc - 1 + 1) as u32);
        ufbxt_assert!(mesh.vertex_color.indices[c] == (vc - 1 + 2) as u32);
    } else {
        let sentinel = sentinel(&mesh.vertex_color, no_index);
        ufbxt_assert!(mesh.vertex_color.indices[a] == sentinel);
        ufbxt_assert!(mesh.vertex_color.indices[b] == sentinel);
        ufbxt_assert!(mesh.vertex_color.indices[c] == sentinel);
    }
}

fn check_obj_index(mesh: &Mesh, index: usize, v: i32, vt: i32, vn: i32, vc: i32, no_index: bool) {
    ufbxt_hintf!("index = {index}");
    ufbxt_assert!(index < mesh.num_indices);

    if v != 0 {
        ufbxt_assert!(mesh.vertex_position.indices[index] == (v - 1) as u32);
    } else {
        ufbxt_assert!(mesh.vertex_position.indices[index] == sentinel(&mesh.vertex_position, no_index));
    }

    if vt != 0 {
        ufbxt_assert!(mesh.vertex_uv.indices[index] == (vt - 1) as u32);
    } else {
        ufbxt_assert!(mesh.vertex_uv.indices[index] == sentinel(&mesh.vertex_uv, no_index));
    }

    if vn != 0 {
        ufbxt_assert!(mesh.vertex_normal.indices[index] == (vn - 1) as u32);
    } else {
        ufbxt_assert!(mesh.vertex_normal.indices[index] == sentinel(&mesh.vertex_normal, no_index));
    }

    if vc != 0 {
        ufbxt_assert!(mesh.vertex_color.indices[index] == (vc - 1) as u32);
    } else {
        ufbxt_assert!(mesh.vertex_color.indices[index] == sentinel(&mesh.vertex_color, no_index));
    }
}

fn find_mesh<'a>(scene: &'a Scene, name: &str) -> &'a Mesh {
    let node = find_node(scene, name).map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    &scene.meshes[node.unwrap().mesh.unwrap()]
}

file_test!(synthetic_partial_attrib, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    check_warning(scene, WarningType::IndexClamped, ElementType::Unknown, None, 60, None);

    let mesh = find_mesh(scene, "Mesh");

    ufbxt_assert!(mesh.num_faces == 12);
    ufbxt_assert!(mesh.num_triangles == 12);
    ufbxt_assert!(mesh.vertex_position.exists);
    ufbxt_assert!(mesh.vertex_uv.exists);
    ufbxt_assert!(mesh.vertex_normal.exists);
    ufbxt_assert!(mesh.vertex_color.exists);

    check_obj_face(mesh, 0, 1, 0, 0, 0, false);
    check_obj_face(mesh, 1, 1, 1, 0, 0, false);
    check_obj_face(mesh, 2, 1, 0, 1, 0, false);
    check_obj_face(mesh, 3, 1, 1, 1, 0, false);
    check_obj_face(mesh, 4, 4, 0, 0, 4, false);
    check_obj_face(mesh, 5, 4, 1, 0, 4, false);
    check_obj_face(mesh, 6, 4, 0, 1, 4, false);
    check_obj_face(mesh, 7, 4, 1, 1, 4, false);
    check_obj_face(mesh, 8, 0, 0, 0, 0, false);
    check_obj_face(mesh, 9, 0, 1, 0, 0, false);
    check_obj_face(mesh, 10, 0, 0, 1, 0, false);
    check_obj_face(mesh, 11, 0, 1, 1, 0, false);
});

file_test!(synthetic_partial_attrib_no_index, file = "synthetic_partial_attrib", opts = no_index_opts(), |scene, err, load_error| {
    let mesh = find_mesh(scene, "Mesh");

    ufbxt_assert!(mesh.num_faces == 12);
    ufbxt_assert!(mesh.num_triangles == 12);
    ufbxt_assert!(mesh.vertex_position.exists);
    ufbxt_assert!(mesh.vertex_uv.exists);
    ufbxt_assert!(mesh.vertex_normal.exists);
    ufbxt_assert!(mesh.vertex_color.exists);

    check_obj_face(mesh, 0, 1, 0, 0, 0, true);
    check_obj_face(mesh, 1, 1, 1, 0, 0, true);
    check_obj_face(mesh, 2, 1, 0, 1, 0, true);
    check_obj_face(mesh, 3, 1, 1, 1, 0, true);
    check_obj_face(mesh, 4, 4, 0, 0, 4, true);
    check_obj_face(mesh, 5, 4, 1, 0, 4, true);
    check_obj_face(mesh, 6, 4, 0, 1, 4, true);
    check_obj_face(mesh, 7, 4, 1, 1, 4, true);
    check_obj_face(mesh, 8, 0, 0, 0, 0, true);
    check_obj_face(mesh, 9, 0, 1, 0, 0, true);
    check_obj_face(mesh, 10, 0, 0, 1, 0, true);
    check_obj_face(mesh, 11, 0, 1, 1, 0, true);
});

file_test_allow_error!(synthetic_partial_attrib_strict, file = "synthetic_partial_attrib", opts = abort_index_opts(), |scene, err, load_error| {
    ufbxt_assert!(scene.is_none());
    ufbxt_assert!(load_error.kind == ErrorType::BadIndex);
});

file_test!(synthetic_partial_attrib_face, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    check_warning(scene, WarningType::IndexClamped, ElementType::Unknown, None, 20, None);

    let mesh = find_mesh(scene, "Mesh");

    ufbxt_assert!(mesh.num_faces == 1);
    ufbxt_assert!(mesh.num_triangles == 10);
    ufbxt_assert!(mesh.num_indices == 12);
    ufbxt_assert!(mesh.vertex_position.exists);
    ufbxt_assert!(mesh.vertex_uv.exists);
    ufbxt_assert!(mesh.vertex_normal.exists);
    ufbxt_assert!(mesh.vertex_color.exists);

    let face = mesh.faces[0];
    ufbxt_assert!(face.index_begin == 0);
    ufbxt_assert!(face.num_indices == 12);

    check_obj_index(mesh, 0, 1, 0, 0, 0, false);
    check_obj_index(mesh, 1, 2, 1, 0, 0, false);
    check_obj_index(mesh, 2, 3, 0, 1, 0, false);
    check_obj_index(mesh, 3, 4, 2, 2, 0, false);
    check_obj_index(mesh, 4, 5, 0, 0, 5, false);
    check_obj_index(mesh, 5, 6, 3, 0, 6, false);
    check_obj_index(mesh, 6, 7, 0, 3, 7, false);
    check_obj_index(mesh, 7, 8, 4, 4, 8, false);
    check_obj_index(mesh, 8, 0, 0, 0, 0, false);
    check_obj_index(mesh, 9, 0, 5, 0, 0, false);
    check_obj_index(mesh, 10, 0, 0, 5, 0, false);
    check_obj_index(mesh, 11, 0, 6, 6, 0, false);
});

file_test!(synthetic_partial_attrib_face_no_index, file = "synthetic_partial_attrib_face", opts = no_index_opts(), |scene, err, load_error| {
    let mesh = find_mesh(scene, "Mesh");

    ufbxt_assert!(mesh.num_faces == 1);
    ufbxt_assert!(mesh.num_triangles == 10);
    ufbxt_assert!(mesh.num_indices == 12);
    ufbxt_assert!(mesh.vertex_position.exists);
    ufbxt_assert!(mesh.vertex_uv.exists);
    ufbxt_assert!(mesh.vertex_normal.exists);
    ufbxt_assert!(mesh.vertex_color.exists);

    let face = mesh.faces[0];
    ufbxt_assert!(face.index_begin == 0);
    ufbxt_assert!(face.num_indices == 12);

    check_obj_index(mesh, 0, 1, 0, 0, 0, true);
    check_obj_index(mesh, 1, 2, 1, 0, 0, true);
    check_obj_index(mesh, 2, 3, 0, 1, 0, true);
    check_obj_index(mesh, 3, 4, 2, 2, 0, true);
    check_obj_index(mesh, 4, 5, 0, 0, 5, true);
    check_obj_index(mesh, 5, 6, 3, 0, 6, true);
    check_obj_index(mesh, 6, 7, 0, 3, 7, true);
    check_obj_index(mesh, 7, 8, 4, 4, 8, true);
    check_obj_index(mesh, 8, 0, 0, 0, 0, true);
    check_obj_index(mesh, 9, 0, 5, 0, 0, true);
    check_obj_index(mesh, 10, 0, 0, 5, 0, true);
    check_obj_index(mesh, 11, 0, 6, 6, 0, true);
});

file_test_allow_error!(synthetic_partial_attrib_face_strict, file = "synthetic_partial_attrib_face", opts = abort_index_opts(), |scene, err, load_error| {
    ufbxt_assert!(scene.is_none());
    ufbxt_assert!(load_error.kind == ErrorType::BadIndex);
});

fn find_mat<'a>(scene: &'a Scene, name: &str) -> &'a Material {
    let mat = find_material(scene, name);
    ufbxt_assert!(mat.is_some());
    &scene.materials[mat.unwrap()]
}

file_test!(synthetic_simple_materials, |scene, err, load_error| {
    ufbxt_assert!(scene.materials.len() == 3);

    {
        let mat = find_mat(scene, "RGB");
        ufbxt_assert!(mat.shader_type == ShaderType::WavefrontMtl);

        let ka = Vec3::new(1.0, 0.0, 0.0);
        let kd = Vec3::new(0.0, 1.0, 0.0);
        let ks = Vec3::new(0.0, 0.0, 1.0);
        let ke = Vec3::new(1.0, 0.0, 1.0);
        let ns = 99.0;
        let d = 0.25;

        assert_close_vec3(err, mat.fbx.ambient_color().value_vec3(), ka);
        assert_close_vec3(err, mat.fbx.diffuse_color().value_vec3(), kd);
        assert_close_vec3(err, mat.fbx.specular_color().value_vec3(), ks);
        assert_close_vec3(err, mat.fbx.emission_color().value_vec3(), ke);
        assert_close_real(err, mat.fbx.specular_exponent().value_real(), ns);
        assert_close_real(err, mat.fbx.transparency_factor().value_real(), 1.0 - d);
        ufbxt_assert!(mat.fbx.ambient_factor().value_real() == 1.0);
        ufbxt_assert!(mat.fbx.diffuse_factor().value_real() == 1.0);
        ufbxt_assert!(mat.fbx.specular_factor().value_real() == 1.0);
        ufbxt_assert!(mat.fbx.emission_factor().value_real() == 1.0);

        assert_close_vec3(err, mat.pbr.base_color().value_vec3(), kd);
        assert_close_vec3(err, mat.pbr.specular_color().value_vec3(), ks);
        assert_close_vec3(err, mat.pbr.emission_color().value_vec3(), ke);
        assert_close_real(err, mat.pbr.roughness().value_real(), 0.00501256289);
        assert_close_real(err, mat.pbr.opacity().value_real(), d);
        ufbxt_assert!(mat.pbr.base_factor().value_real() == 1.0);
        ufbxt_assert!(mat.pbr.specular_factor().value_real() == 1.0);
        ufbxt_assert!(mat.pbr.emission_factor().value_real() == 1.0);

        ufbxt_assert!(mat.features[MaterialFeature::Diffuse].enabled);
        ufbxt_assert!(mat.features[MaterialFeature::Specular].enabled);
        ufbxt_assert!(mat.features[MaterialFeature::Opacity].enabled);
        ufbxt_assert!(!mat.features[MaterialFeature::Pbr].enabled);
        ufbxt_assert!(!mat.features[MaterialFeature::Metalness].enabled);
        ufbxt_assert!(!mat.features[MaterialFeature::Sheen].enabled);
        ufbxt_assert!(!mat.features[MaterialFeature::Coat].enabled);
        ufbxt_assert!(!mat.features[MaterialFeature::Transmission].enabled);
    }

    {
        let mat = find_mat(scene, "PBR");
        ufbxt_assert!(mat.shader_type == ShaderType::WavefrontMtl);

        let pr = 0.1;
        let pm = 0.2;
        let ps = Vec3::new(0.3, 0.4, 0.5);
        let pc = 0.6;
        let pcr = 0.7;
        let ni = 1.33;
        let tf = Vec3::new(0.8, 0.9, 1.0);
        let d = 0.75;

        assert_close_real(err, mat.pbr.roughness().value_real(), pr);
        assert_close_real(err, mat.pbr.metalness().value_real(), pm);
        assert_close_vec3(err, mat.pbr.sheen_color().value_vec3(), ps);
        assert_close_real(err, mat.pbr.coat_factor().value_real(), pc);
        assert_close_real(err, mat.pbr.coat_roughness().value_real(), pcr);
        assert_close_real(err, mat.pbr.specular_ior().value_real(), ni);
        assert_close_vec3(err, mat.pbr.transmission_color().value_vec3(), tf);
        assert_close_real(err, mat.pbr.opacity().value_real(), d);

        ufbxt_assert!(mat.pbr.sheen_factor().value_real() == 1.0);
        ufbxt_assert!(mat.pbr.transmission_factor().value_real() == 1.0);

        ufbxt_assert!(mat.features[MaterialFeature::Pbr].enabled);
        ufbxt_assert!(mat.features[MaterialFeature::Metalness].enabled);
        ufbxt_assert!(mat.features[MaterialFeature::Diffuse].enabled);
        ufbxt_assert!(mat.features[MaterialFeature::Specular].enabled);
        ufbxt_assert!(mat.features[MaterialFeature::Sheen].enabled);
        ufbxt_assert!(mat.features[MaterialFeature::Coat].enabled);
        ufbxt_assert!(mat.features[MaterialFeature::Transmission].enabled);
        ufbxt_assert!(mat.features[MaterialFeature::Opacity].enabled);
    }

    {
        let mat = find_mat(scene, "Wide");
        ufbxt_assert!(mat.shader_type == ShaderType::WavefrontMtl);

        let ka = Vec3::new(0.1, 0.1, 0.1);
        let kd = Vec3::new(0.2, 0.2, 0.2);
        let ks = Vec3::new(0.3, 0.3, 0.3);
        let ke = Vec3::new(0.4, 0.4, 0.4);
        let ps = Vec3::new(0.5, 0.5, 0.5);
        let tf = Vec3::new(0.6, 0.6, 0.6);

        assert_close_vec3(err, mat.fbx.ambient_color().value_vec3(), ka);
        assert_close_vec3(err, mat.fbx.diffuse_color().value_vec3(), kd);
        assert_close_vec3(err, mat.fbx.specular_color().value_vec3(), ks);
        assert_close_vec3(err, mat.fbx.emission_color().value_vec3(), ke);

        assert_close_vec3(err, mat.pbr.base_color().value_vec3(), kd);
        assert_close_vec3(err, mat.pbr.specular_color().value_vec3(), ks);
        assert_close_vec3(err, mat.pbr.emission_color().value_vec3(), ke);
        assert_close_vec3(err, mat.pbr.sheen_color().value_vec3(), ps);
        assert_close_vec3(err, mat.pbr.transmission_color().value_vec3(), tf);
    }
});

fn check_obj_texture(scene: &Scene, texture: Option<usize>, filename: &str) {
    let rel_path = format!("textures/{filename}");

    ufbxt_assert!(texture.is_some());
    ufbxt_assert!(scene.textures[texture.unwrap()].relative_filename == rel_path);
}

file_test!(synthetic_simple_textures, |scene, err, load_error| {
    ufbxt_assert!(scene.materials.len() == 3);

    {
        let mat = find_mat(scene, "RGB");
        ufbxt_assert!(mat.shader_type == ShaderType::WavefrontMtl);

        check_obj_texture(scene, mat.fbx.ambient_color().texture, "checkerboard_ambient.png");
        check_obj_texture(scene, mat.fbx.diffuse_color().texture, "checkerboard_diffuse.png");
        check_obj_texture(scene, mat.fbx.specular_color().texture, "checkerboard_specular.png");
        check_obj_texture(scene, mat.fbx.emission_color().texture, "checkerboard_emissive.png");
        check_obj_texture(scene, mat.fbx.specular_exponent().texture, "checkerboard_roughness.png");
        check_obj_texture(scene, mat.fbx.transparency_factor().texture, "checkerboard_transparency.png");
        check_obj_texture(scene, mat.fbx.bump().texture, "checkerboard_bump.png");

        check_obj_texture(scene, mat.pbr.base_color().texture, "checkerboard_diffuse.png");
        check_obj_texture(scene, mat.pbr.specular_color().texture, "checkerboard_specular.png");
        check_obj_texture(scene, mat.pbr.emission_color().texture, "checkerboard_emissive.png");
        check_obj_texture(scene, mat.pbr.roughness().texture, "checkerboard_roughness.png");
        check_obj_texture(scene, mat.pbr.normal_map().texture, "checkerboard_bump.png");
    }

    {
        let mat = find_mat(scene, "PBR");
        ufbxt_assert!(mat.shader_type == ShaderType::WavefrontMtl);

        check_obj_texture(scene, mat.pbr.roughness().texture, "checkerboard_roughness.png");
        check_obj_texture(scene, mat.pbr.metalness().texture, "checkerboard_metallic.png");
        check_obj_texture(scene, mat.pbr.sheen_color().texture, "checkerboard_reflection.png");
        check_obj_texture(scene, mat.pbr.coat_factor().texture, "checkerboard_specular.png");
        check_obj_texture(scene, mat.pbr.coat_roughness().texture, "checkerboard_weight.png");
        check_obj_texture(scene, mat.pbr.transmission_color().texture, "checkerboard_transparency.png");
        check_obj_texture(scene, mat.pbr.opacity().texture, "checkerboard_weight.png");
        check_obj_texture(scene, mat.pbr.specular_ior().texture, "checkerboard_specular.png");
        check_obj_texture(scene, mat.pbr.normal_map().texture, "checkerboard_normal.png");
        check_obj_texture(scene, mat.pbr.displacement_map().texture, "checkerboard_displacement.png");

        check_obj_texture(scene, mat.fbx.transparency_factor().texture, "checkerboard_weight.png");
        check_obj_texture(scene, mat.fbx.normal_map().texture, "checkerboard_normal.png");
        check_obj_texture(scene, mat.fbx.displacement().texture, "checkerboard_displacement.png");
    }

    {
        let mat = find_mat(scene, "NonMap");
        ufbxt_assert!(mat.shader_type == ShaderType::WavefrontMtl);

        ufbxt_assert!(mat.textures.len() == 3);

        check_obj_texture(scene, mat.pbr.normal_map().texture, "checkerboard_normal.png");
        check_obj_texture(scene, mat.pbr.displacement_map().texture, "checkerboard_displacement.png");

        check_obj_texture(scene, mat.fbx.normal_map().texture, "checkerboard_normal.png");
        check_obj_texture(scene, mat.fbx.displacement().texture, "checkerboard_displacement.png");

        check_obj_texture(scene, find_prop_texture(mat, "norm"), "checkerboard_normal.png");
        check_obj_texture(scene, find_prop_texture(mat, "disp"), "checkerboard_displacement.png");
        check_obj_texture(scene, find_prop_texture(mat, "bump"), "checkerboard_bump.png");
    }
});

#[allow(clippy::too_many_arguments)]
fn check_obj_prop(err: &mut DiffError, props: &Props, name: &str, s: Option<&str>, i: i64, x: Real, y: Real, z: Real) {
    ufbxt_hintf!("name = \"{name}\"");

    let prop = props.find(name);
    ufbxt_assert!(prop.is_some());
    let prop = prop.unwrap();
    if let Some(s) = s {
        ufbxt_assert!(prop.value_str == s);
    }

    ufbxt_assert!(prop.value_int == i);

    assert_close_real(err, prop.value_vec3().x, x);
    assert_close_real(err, prop.value_vec3().y, y);
    assert_close_real(err, prop.value_vec3().z, z);
}

file_test!(synthetic_texture_opts, |scene, err, load_error| {
    ufbxt_assert!(scene.materials.len() == 1);

    let mat = find_mat(scene, "Opts");
    ufbxt_assert!(mat.shader_type == ShaderType::WavefrontMtl);

    {
        let tex = mat.fbx.diffuse_color().texture;
        ufbxt_assert!(tex.is_some());
        let tex = &scene.textures[tex.unwrap()];
        ufbxt_assert!(tex.relative_filename == "textures/checkerboard_diffuse.png");

        let props = &tex.element.props;
        check_obj_prop(err, props, "blendu", Some("off"), 0, 0.0, 0.0, 0.0);
        check_obj_prop(err, props, "blendv", Some("on"), 1, 1.0, 0.0, 0.0);
        check_obj_prop(err, props, "clamp", Some("off"), 0, 0.0, 0.0, 0.0);
        check_obj_prop(err, props, "imfchan", Some("r"), 0, 0.0, 0.0, 0.0);
        check_obj_prop(err, props, "mm", Some("1 2"), 1, 1.0, 2.0, 0.0);
        check_obj_prop(err, props, "o", Some("0.1 0.2 0.3"), 0, 0.1, 0.2, 0.3);
        check_obj_prop(err, props, "s", Some("0.4 0.5 0.6"), 0, 0.4, 0.5, 0.6);
        check_obj_prop(err, props, "t", Some("0.7 0.8 0.9"), 0, 0.7, 0.8, 0.9);
        check_obj_prop(err, props, "texres", Some("512"), 512, 512.0, 0.0, 0.0);
    }

    {
        let tex = mat.fbx.specular_color().texture;
        ufbxt_assert!(tex.is_some());
        let tex = &scene.textures[tex.unwrap()];
        ufbxt_assert!(tex.relative_filename == "textures/checkerboard_specular.png");

        let props = &tex.element.props;
        check_obj_prop(err, props, "blendu", Some("on"), 1, 1.0, 0.0, 0.0);
        check_obj_prop(err, props, "blendv", Some("off"), 0, 0.0, 0.0, 0.0);
        check_obj_prop(err, props, "clamp", Some("on"), 1, 1.0, 0.0, 0.0);
        check_obj_prop(err, props, "imfchan", Some("g"), 0, 0.0, 0.0, 0.0);
        check_obj_prop(err, props, "mm", Some("3 4"), 3, 3.0, 4.0, 0.0);
        check_obj_prop(err, props, "o", Some("-0.1 -.2 1.3"), 0, -0.1, -0.2, 1.3);
        check_obj_prop(err, props, "s", None, 1, 1.4, 1.5, 1.6);
        check_obj_prop(err, props, "t", Some("1.7 1.8 1.9"), 1, 1.7, 1.8, 1.9);
        check_obj_prop(err, props, "texres", Some("1024"), 1024, 1024.0, 0.0, 0.0);
        check_obj_prop(err, props, "unknown", Some("hello world"), 0, 0.0, 0.0, 0.0);
        check_obj_prop(err, props, "single-unknown", Some(""), 0, 0.0, 0.0, 0.0);
    }
});

file_test!(blender_331_space_texture, path = "blender_331_space texture", |scene, err, load_error| {
    ufbxt_assert!(scene.materials.len() == 1);
    let material = find_material(scene, "Material");
    ufbxt_assert!(material.is_some());

    let texture = scene.materials[material.unwrap()].fbx.diffuse_color().texture;
    ufbxt_assert!(texture.is_some());
    ufbxt_assert!(scene.textures[texture.unwrap()].relative_filename == "space dir/space tex.png");

    if scene.metadata.file_format == FileFormat::Obj {
        let node = find_node(scene, "Cube").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
        let node = node.unwrap();
        ufbxt_assert!(node.materials.len() == 1);
        ufbxt_assert!(node.materials[0] == material);
    }
});

file_test!(synthetic_obj_zoo, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    check_warning(scene, WarningType::UnknownObjDirective, ElementType::Unknown, None, 24, Some("Unknown .obj directive, skipped line"));

    let mesh = find_mesh(scene, "Object");

    ufbxt_assert!(mesh.num_faces == 7);
    ufbxt_assert!(mesh.num_line_faces == 2);
    ufbxt_assert!(mesh.num_point_faces == 4);
});

// `ufbxt_mesh_material` and `ufbxt_mesh_group` in test/test_obj.h, as tuples
fn find_mesh_material<'a>(scene: &Scene, mesh: &'a Mesh, name: &str) -> (Option<usize>, Option<&'a MeshPart>) {
    for (i, &material) in mesh.materials.iter().enumerate() {
        if material.is_some_and(|m| scene.materials[m].element.name == name) {
            return (material, mesh.material_parts.get(i));
        }
    }
    (None, None)
}

fn find_face_group<'a>(mesh: &'a Mesh, name: &str) -> (Option<&'a FaceGroup>, Option<&'a MeshPart>) {
    let mut group = (None, None);
    for (i, face_group) in mesh.face_groups.iter().enumerate() {
        if face_group.name == name {
            group = (Some(face_group), mesh.face_group_parts.get(i));
        }
    }
    group
}

fn find_mesh_material_part<'a>(scene: &Scene, mesh: &'a Mesh, name: &str) -> Option<&'a MeshPart> {
    find_mesh_material(scene, mesh, name).1
}

fn find_face_group_part<'a>(mesh: &'a Mesh, name: &str) -> Option<&'a MeshPart> {
    find_face_group(mesh, name).1
}

fn check_planar_face(err: &mut DiffError, mesh: &Mesh, face: Face, normal: Vec3, w: Real) {
    for i in 0..face.num_indices {
        let pos = mesh.vertex_position.get((face.index_begin + i) as usize);
        let r = dot3(normal, pos);
        assert_close_real(err, r, w);
    }
}

fn check_planar_face_ix(err: &mut DiffError, mesh: &Mesh, face_ix: usize, normal: Vec3, w: Real) {
    ufbxt_assert!(face_ix < mesh.faces.len());
    check_planar_face(err, mesh, mesh.faces[face_ix], normal, w);
}

const POS_X: Vec3 = Vec3::new(1.0, 0.0, 0.0);
const NEG_X: Vec3 = Vec3::new(-1.0, 0.0, 0.0);
const POS_Y: Vec3 = Vec3::new(0.0, 1.0, 0.0);
const NEG_Y: Vec3 = Vec3::new(0.0, -1.0, 0.0);
const POS_Z: Vec3 = Vec3::new(0.0, 0.0, 1.0);
const NEG_Z: Vec3 = Vec3::new(0.0, 0.0, -1.0);

file_test!(synthetic_face_groups, |scene, err, load_error| {
    let mesh = find_mesh(scene, "Cube");

    {
        let group = find_face_group_part(mesh, "");
        ufbxt_assert!(group.is_some());
        let group = group.unwrap();
        ufbxt_assert!(group.num_faces == 2);
        ufbxt_assert!(group.face_indices[0] == 0);
        ufbxt_assert!(group.face_indices[1] == 5);
        check_planar_face_ix(err, mesh, group.face_indices[0] as usize, POS_Y, 1.0);
        check_planar_face_ix(err, mesh, group.face_indices[1] as usize, NEG_Y, 1.0);
    }

    {
        let group = find_face_group_part(mesh, "Front");
        ufbxt_assert!(group.is_some());
        let group = group.unwrap();
        ufbxt_assert!(group.num_faces == 1);
        ufbxt_assert!(group.face_indices[0] == 1);
        check_planar_face_ix(err, mesh, group.face_indices[0] as usize, NEG_Z, 1.0);
    }

    {
        let group = find_face_group_part(mesh, "Sides");
        ufbxt_assert!(group.is_some());
        let group = group.unwrap();
        ufbxt_assert!(group.num_faces == 2);
        ufbxt_assert!(group.face_indices[0] == 2);
        ufbxt_assert!(group.face_indices[1] == 4);
        check_planar_face_ix(err, mesh, group.face_indices[0] as usize, NEG_X, 1.0);
        check_planar_face_ix(err, mesh, group.face_indices[1] as usize, POS_X, 1.0);
    }

    {
        let group = find_face_group_part(mesh, "Back");
        ufbxt_assert!(group.is_some());
        let group = group.unwrap();
        ufbxt_assert!(group.num_faces == 1);
        ufbxt_assert!(group.face_indices[0] == 3);
        check_planar_face_ix(err, mesh, group.face_indices[0] as usize, POS_Z, 1.0);
    }

    {
        let mat = find_mesh_material_part(scene, mesh, "A");
        ufbxt_assert!(mat.is_some());
        let mat = mat.unwrap();
        ufbxt_assert!(mat.num_faces == 3);
        ufbxt_assert!(mat.face_indices[0] == 0);
        ufbxt_assert!(mat.face_indices[1] == 1);
        ufbxt_assert!(mat.face_indices[2] == 2);
    }

    {
        let mat = find_mesh_material_part(scene, mesh, "B");
        ufbxt_assert!(mat.is_some());
        let mat = mat.unwrap();
        ufbxt_assert!(mat.num_faces == 3);
        ufbxt_assert!(mat.face_indices[0] == 3);
        ufbxt_assert!(mat.face_indices[1] == 4);
        ufbxt_assert!(mat.face_indices[2] == 5);
    }
});

fn split_groups_opts() -> LoadOpts {
    LoadOpts { obj_split_groups: true, ..Default::default() }
}

fn merge_objects_opts() -> LoadOpts {
    LoadOpts { obj_merge_objects: true, ..Default::default() }
}

file_test!(synthetic_face_groups_split, file = "synthetic_face_groups", opts = split_groups_opts(), |scene, err, load_error| {
    let mat_a = find_material(scene, "A");
    let mat_b = find_material(scene, "B");

    let mut num_a_found = 0;
    let mut num_b_found = 0;
    for node in &scene.nodes {
        if node.element.name != "Cube" {
            continue;
        }
        ufbxt_assert!(node.mesh.is_some());
        let mesh = &scene.meshes[node.mesh.unwrap()];

        ufbxt_assert!(mesh.materials.len() == 1);
        ufbxt_assert!(mesh.num_faces == 1);

        let material = mesh.materials[0];
        if material == mat_a {
            check_planar_face_ix(err, mesh, 0, POS_Y, 1.0);
            num_a_found += 1;
        } else {
            ufbxt_assert!(material == mat_b);
            check_planar_face_ix(err, mesh, 0, NEG_Y, 1.0);
            num_b_found += 1;
        }
    }

    ufbxt_assert!(num_a_found == 1);
    ufbxt_assert!(num_b_found == 1);

    {
        let mesh = find_mesh(scene, "Front");

        ufbxt_assert!(mesh.num_faces == 1);
        check_planar_face_ix(err, mesh, 0, NEG_Z, 1.0);

        ufbxt_assert!(mesh.materials.len() == 1);
        ufbxt_assert!(mesh.materials[0] == mat_a);
    }

    {
        let mesh = find_mesh(scene, "Back");

        ufbxt_assert!(mesh.num_faces == 1);
        check_planar_face_ix(err, mesh, 0, POS_Z, 1.0);

        ufbxt_assert!(mesh.materials.len() == 1);
        ufbxt_assert!(mesh.materials[0] == mat_b);
    }

    num_a_found = 0;
    num_b_found = 0;
    for node in &scene.nodes {
        if node.element.name != "Sides" {
            continue;
        }
        ufbxt_assert!(node.mesh.is_some());
        let mesh = &scene.meshes[node.mesh.unwrap()];

        ufbxt_assert!(mesh.materials.len() == 1);
        ufbxt_assert!(mesh.num_faces == 1);

        let material = mesh.materials[0];
        if material == mat_a {
            check_planar_face_ix(err, mesh, 0, NEG_X, 1.0);
            num_a_found += 1;
        } else {
            ufbxt_assert!(material == mat_b);
            check_planar_face_ix(err, mesh, 0, POS_X, 1.0);
            num_b_found += 1;
        }
    }

    ufbxt_assert!(num_a_found == 1);
    ufbxt_assert!(num_b_found == 1);
});

fn material_name(scene: &Scene, material: Option<usize>) -> &str {
    &scene.materials[material.unwrap()].element.name
}

file_test!(synthetic_partial_material, |scene, err, load_error| {
    ufbxt_assert!(scene.materials.len() == 8);
    ufbxt_assert!(find_material(scene, "A").is_some());
    ufbxt_assert!(find_material(scene, "B").is_some());
    ufbxt_assert!(find_material(scene, "C").is_some());
    ufbxt_assert!(find_material(scene, "D").is_some());
    ufbxt_assert!(find_material(scene, "E").is_some());
    ufbxt_assert!(find_material(scene, "F").is_some());
    ufbxt_assert!(find_material(scene, "G").is_some());
    ufbxt_assert!(find_material(scene, "H").is_some());

    {
        let mesh = find_mesh(scene, "First");
        ufbxt_assert!(mesh.face_material.is_empty());
        ufbxt_assert!(mesh.materials.is_empty());
        ufbxt_assert!(mesh.material_parts.len() == 1);
    }

    {
        let mesh = find_mesh(scene, "Second");

        ufbxt_assert!(mesh.materials.len() == 3);
        ufbxt_assert!(material_name(scene, mesh.materials[0]) == "A");
        ufbxt_assert!(material_name(scene, mesh.materials[1]) == "B");
        ufbxt_assert!(material_name(scene, mesh.materials[2]) == "D");

        ufbxt_assert!(mesh.face_material.len() == 5);
        ufbxt_assert!(mesh.face_material[0] == 0);
        ufbxt_assert!(mesh.face_material[1] == 0);
        ufbxt_assert!(mesh.face_material[2] == 1);
        ufbxt_assert!(mesh.face_material[3] == 2);
        ufbxt_assert!(mesh.face_material[4] == 0);
    }

    {
        let mesh = find_mesh(scene, "Third");

        ufbxt_assert!(mesh.materials.len() == 2);
        ufbxt_assert!(material_name(scene, mesh.materials[0]) == "F");
        ufbxt_assert!(material_name(scene, mesh.materials[1]) == "A");

        ufbxt_assert!(mesh.face_material.len() == 2);
        ufbxt_assert!(mesh.face_material[0] == 0);
        ufbxt_assert!(mesh.face_material[1] == 1);
    }
});

file_test!(synthetic_partial_material_merged, file = "synthetic_partial_material", opts = merge_objects_opts(), |scene, err, load_error| {
    ufbxt_assert!(scene.materials.len() == 8);
    ufbxt_assert!(find_material(scene, "A").is_some());
    ufbxt_assert!(find_material(scene, "B").is_some());
    ufbxt_assert!(find_material(scene, "C").is_some());
    ufbxt_assert!(find_material(scene, "D").is_some());
    ufbxt_assert!(find_material(scene, "E").is_some());
    ufbxt_assert!(find_material(scene, "F").is_some());
    ufbxt_assert!(find_material(scene, "G").is_some());
    ufbxt_assert!(find_material(scene, "H").is_some());

    ufbxt_assert!(scene.nodes.len() == 2);
    let node = &scene.nodes[1];
    ufbxt_assert!(node.mesh.is_some());
    let mesh = &scene.meshes[node.mesh.unwrap()];

    ufbxt_assert!(mesh.materials.len() == 4);
    ufbxt_assert!(material_name(scene, mesh.materials[0]) == "A");
    ufbxt_assert!(material_name(scene, mesh.materials[1]) == "B");
    ufbxt_assert!(material_name(scene, mesh.materials[2]) == "D");
    ufbxt_assert!(material_name(scene, mesh.materials[3]) == "F");

    ufbxt_assert!(mesh.face_material.len() == 8);
    ufbxt_assert!(mesh.face_material[0] == 0);
    ufbxt_assert!(mesh.face_material[1] == 0);
    ufbxt_assert!(mesh.face_material[2] == 0);
    ufbxt_assert!(mesh.face_material[3] == 1);
    ufbxt_assert!(mesh.face_material[4] == 2);
    ufbxt_assert!(mesh.face_material[5] == 0);
    ufbxt_assert!(mesh.face_material[6] == 3);
    ufbxt_assert!(mesh.face_material[7] == 0);
});

fn search_mtl_by_filename_opts() -> LoadOpts {
    LoadOpts { obj_search_mtl_by_filename: true, ..Default::default() }
}

file_test!(synthetic_filename_mtl, opts = search_mtl_by_filename_opts(), flags = FUZZ_ALWAYS | FUZZ_OPTS | ALLOW_WARNINGS, |scene, err, load_error| {
    check_warning(scene, WarningType::MissingExternalFile, ElementType::Unknown, None, 1, Some("materials.mtl"));
    check_warning(scene, WarningType::ImplicitMtl, ElementType::Unknown, None, 1, Some("synthetic_filename_mtl_0_obj.mtl"));

    {
        let ka = Vec3::new(1.0, 0.0, 0.0);
        let kd = Vec3::new(0.0, 1.0, 0.0);
        let ks = Vec3::new(0.0, 0.0, 1.0);

        let material = find_mat(scene, "Material");
        assert_close_vec3(err, material.fbx.ambient_color().value_vec3(), ka);
        assert_close_vec3(err, material.fbx.diffuse_color().value_vec3(), kd);
        assert_close_vec3(err, material.fbx.specular_color().value_vec3(), ks);
    }

    {
        let material = find_material(scene, "Other");
        ufbxt_assert!(material.is_some());
    }
});

file_test_allow_error!(synthetic_filename_mtl_not_found, file = "synthetic_filename_mtl", |scene, err, load_error| {
    ufbxt_assert!(load_error.kind == ErrorType::ExternalFileNotFound);
    ufbxt_assert!(load_error.info == "materials.mtl");
});

file_test_allow_error!(synthetic_bad_unicode_mtl, |scene, err, load_error| {
    ufbxt_assert!(load_error.kind == ErrorType::ExternalFileNotFound);
    ufbxt_assert!(load_error.info == "material?.mtl");
});

file_test_allow_error!(synthetic_missing_position_fail, |scene, err, load_error| {});

fn log_diff(err: &DiffError) {
    if err.num > 0 {
        let avg = err.sum / err.num as f64;
        ufbxt_logf!(".. Absolute diff: avg {avg:.3}, max {:.3} ({} tests)", err.max, err.num);
    }
}

#[test]
fn obj_opts_mtl_data() {
    let obj = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl Material\nf 0 1 2\n";

    let mtl = b"newmtl Material\nKa 1 0 0\nKd 0 1 0\nKs 0 0 1\n";

    let opts = LoadOpts { obj_mtl_data: mtl.to_vec(), ..Default::default() };
    let scene = load_memory(obj, &opts);
    ufbxt_assert!(scene.is_ok());
    let scene = scene.unwrap();
    ufbxt_assert!(scene.meshes.len() == 1);
    let mesh = &scene.meshes[0];
    ufbxt_assert!(mesh.faces.len() == 1);
    ufbxt_assert!(mesh.materials.len() == 1);
    let material = &scene.materials[mesh.materials[0].unwrap()];
    ufbxt_assert!(material.element.name == "Material");

    let ka = Vec3::new(1.0, 0.0, 0.0);
    let kd = Vec3::new(0.0, 1.0, 0.0);
    let ks = Vec3::new(0.0, 0.0, 1.0);

    let mut err = DiffError::default();
    assert_close_vec3(&mut err, material.fbx.ambient_color().value_vec3(), ka);
    assert_close_vec3(&mut err, material.fbx.diffuse_color().value_vec3(), kd);
    assert_close_vec3(&mut err, material.fbx.specular_color().value_vec3(), ks);
    log_diff(&err);
}

#[test]
fn obj_opts_mtl_path() {
    let obj = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl RGB\nf 0 1 2\n";

    let opts = LoadOpts { obj_mtl_path: data_path("synthetic_simple_materials_0_mtl.mtl"), ..Default::default() };
    let scene = load_memory(obj, &opts);
    ufbxt_assert!(scene.is_ok());
    let scene = scene.unwrap();
    ufbxt_assert!(scene.meshes.len() == 1);
    let mesh = &scene.meshes[0];
    ufbxt_assert!(mesh.faces.len() == 1);
    ufbxt_assert!(mesh.materials.len() == 1);
    let material = &scene.materials[mesh.materials[0].unwrap()];
    ufbxt_assert!(material.element.name == "RGB");

    let ka = Vec3::new(1.0, 0.0, 0.0);
    let kd = Vec3::new(0.0, 1.0, 0.0);
    let ks = Vec3::new(0.0, 0.0, 1.0);

    let mut err = DiffError::default();
    assert_close_vec3(&mut err, material.fbx.ambient_color().value_vec3(), ka);
    assert_close_vec3(&mut err, material.fbx.diffuse_color().value_vec3(), kd);
    assert_close_vec3(&mut err, material.fbx.specular_color().value_vec3(), ks);
    log_diff(&err);
}

#[test]
fn obj_opts_no_extrnal_files() {
    let mut err = DiffError::default();

    for path in FileIterator::new("blender_279_ball") {
        for load_external in [false, true] {
            let opts = LoadOpts { load_external_files: load_external, ..Default::default() };
            let scene = load_file(&path, &opts);
            ufbxt_assert!(scene.is_ok());
            let scene = scene.unwrap();
            check_scene(&scene);

            if scene.metadata.file_format == FileFormat::Obj {
                let red = find_material(&scene, "Red");
                let white = find_material(&scene, "White");
                ufbxt_assert!(red.is_some());
                ufbxt_assert!(white.is_some());
                let (red, white) = (&scene.materials[red.unwrap()], &scene.materials[white.unwrap()]);

                if load_external {
                    let ref_red = Vec3::new(0.8, 0.0, 0.0);
                    let ref_white = Vec3::new(0.8, 0.8, 0.8);
                    assert_close_vec3(&mut err, red.fbx.diffuse_color().value_vec3(), ref_red);
                    assert_close_vec3(&mut err, white.fbx.diffuse_color().value_vec3(), ref_white);
                } else {
                    ufbxt_assert!(red.element.props.props.is_empty());
                    ufbxt_assert!(white.element.props.props.is_empty());
                }
            }
        }
    }

    log_diff(&err);
}

#[test]
fn obj_opts_no_extrnal_files_by_filename() {
    let mut err = DiffError::default();

    for path in FileIterator::new("synthetic_filename_mtl") {
        for load_by_filename in [false, true] {
            for load_external in [false, true] {
                let opts = LoadOpts {
                    load_external_files: load_external,
                    obj_search_mtl_by_filename: load_by_filename,
                    ignore_missing_external_files: true,
                    ..Default::default()
                };
                let scene = load_file(&path, &opts);
                ufbxt_assert!(scene.is_ok());
                let scene = scene.unwrap();
                check_scene(&scene);

                if scene.metadata.file_format == FileFormat::Obj {
                    let material = find_mat(&scene, "Material");

                    if load_external && load_by_filename {
                        check_warning(&scene, WarningType::ImplicitMtl, ElementType::Unknown, None, 1, Some("synthetic_filename_mtl_0_obj.mtl"));
                        let ka = Vec3::new(1.0, 0.0, 0.0);
                        let kd = Vec3::new(0.0, 1.0, 0.0);
                        let ks = Vec3::new(0.0, 0.0, 1.0);
                        assert_close_vec3(&mut err, material.fbx.ambient_color().value_vec3(), ka);
                        assert_close_vec3(&mut err, material.fbx.diffuse_color().value_vec3(), kd);
                        assert_close_vec3(&mut err, material.fbx.specular_color().value_vec3(), ks);
                    } else {
                        if load_external && !load_by_filename {
                            check_warning(&scene, WarningType::MissingExternalFile, ElementType::Unknown, None, 1, Some("materials.mtl"));
                        }
                        ufbxt_assert!(material.element.props.props.is_empty());
                    }
                }
            }
        }
    }

    log_diff(&err);
}

fn check_only_texture(scene: &Scene, map: &MaterialMap, filename: &str) {
    ufbxt_assert!(!map.has_value);
    ufbxt_assert!(map.texture.is_some());
    ufbxt_assert!(scene.textures[map.texture.unwrap()].relative_filename == filename);
}

file_test!(synthetic_map_feature, |scene, err, load_error| {
    {
        let material = &scene.materials[find_material(scene, "Phong").unwrap()];
        ufbxt_assert!(material.shader_type == ShaderType::WavefrontMtl);

        check_only_texture(scene, material.fbx.diffuse_color(), "diffuse.png");
        check_only_texture(scene, material.fbx.specular_color(), "specular.png");

        ufbxt_assert!(!material.features[MaterialFeature::Pbr].enabled);
        ufbxt_assert!(!material.features[MaterialFeature::Sheen].enabled);
        ufbxt_assert!(!material.features[MaterialFeature::Coat].enabled);
        ufbxt_assert!(!material.features[MaterialFeature::Metalness].enabled);
        ufbxt_assert!(!material.features[MaterialFeature::Ior].enabled);
        ufbxt_assert!(!material.features[MaterialFeature::Opacity].enabled);
        ufbxt_assert!(!material.features[MaterialFeature::Transmission].enabled);
        ufbxt_assert!(!material.features[MaterialFeature::Emission].enabled);
    }

    {
        let material = &scene.materials[find_material(scene, "PBR").unwrap()];
        ufbxt_assert!(material.shader_type == ShaderType::WavefrontMtl);

        check_only_texture(scene, material.pbr.roughness(), "roughness.png");
        check_only_texture(scene, material.pbr.metalness(), "metalness.png");

        ufbxt_assert!(material.features[MaterialFeature::Pbr].enabled);
        ufbxt_assert!(!material.features[MaterialFeature::Sheen].enabled);
        ufbxt_assert!(!material.features[MaterialFeature::Coat].enabled);
        ufbxt_assert!(material.features[MaterialFeature::Metalness].enabled);
        ufbxt_assert!(!material.features[MaterialFeature::Ior].enabled);
        ufbxt_assert!(!material.features[MaterialFeature::Opacity].enabled);
        ufbxt_assert!(!material.features[MaterialFeature::Transmission].enabled);
        ufbxt_assert!(!material.features[MaterialFeature::Emission].enabled);
    }

    {
        let material = &scene.materials[find_material(scene, "Extended").unwrap()];
        ufbxt_assert!(material.shader_type == ShaderType::WavefrontMtl);

        check_only_texture(scene, material.pbr.sheen_color(), "sheen.png");
        check_only_texture(scene, material.pbr.coat_factor(), "coat.png");
        check_only_texture(scene, material.pbr.metalness(), "metalness.png");
        check_only_texture(scene, material.pbr.specular_ior(), "ior.png");
        check_only_texture(scene, material.pbr.opacity(), "opacity.png");
        check_only_texture(scene, material.pbr.transmission_color(), "transmission.png");
        check_only_texture(scene, material.pbr.emission_color(), "emission.png");

        ufbxt_assert!(material.features[MaterialFeature::Pbr].enabled);
        ufbxt_assert!(material.features[MaterialFeature::Sheen].enabled);
        ufbxt_assert!(material.features[MaterialFeature::Coat].enabled);
        ufbxt_assert!(material.features[MaterialFeature::Metalness].enabled);
        ufbxt_assert!(material.features[MaterialFeature::Ior].enabled);
        ufbxt_assert!(material.features[MaterialFeature::Opacity].enabled);
        ufbxt_assert!(material.features[MaterialFeature::Transmission].enabled);
        ufbxt_assert!(material.features[MaterialFeature::Emission].enabled);
    }
});

file_test!(blender_340_line_point, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    ufbxt_assert!(scene.metadata.has_warning[WarningType::IndexClamped as usize]);

    let mesh = find_mesh(scene, "Cube");

    ufbxt_assert!(mesh.num_faces == 6);
    ufbxt_assert!(mesh.num_empty_faces == 0);
    ufbxt_assert!(mesh.num_point_faces == 0);
    ufbxt_assert!(mesh.num_line_faces == 5);
});

file_test!(synthetic_extended_line, |scene, err, load_error| {
    let mesh = find_mesh(scene, "Line");

    ufbxt_assert!(mesh.num_faces == 4);
    ufbxt_assert!(mesh.num_empty_faces == 0);
    ufbxt_assert!(mesh.num_point_faces == 0);
    ufbxt_assert!(mesh.num_line_faces == 4);
    ufbxt_assert!(mesh.num_indices == 8);

    let pos_ref = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 0.0),
    ];

    let uv_ref = [
        Vec2::new(0.0, 0.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(0.0, 2.0),
        Vec2::new(0.0, 2.0),
        Vec2::new(2.0, 2.0),
        Vec2::new(2.0, 2.0),
        Vec2::new(0.0, 0.0),
    ];

    let col_ref = [
        Vec4::new(1.0, 0.0, 0.0, 1.0),
        Vec4::new(0.0, 1.0, 0.0, 1.0),
        Vec4::new(0.0, 1.0, 0.0, 1.0),
        Vec4::new(0.0, 0.0, 1.0, 1.0),
        Vec4::new(0.0, 0.0, 1.0, 1.0),
        Vec4::new(1.0, 1.0, 1.0, 1.0),
        Vec4::new(1.0, 1.0, 1.0, 1.0),
        Vec4::new(1.0, 0.0, 0.0, 1.0),
    ];

    for i in 0..mesh.num_indices {
        ufbxt_hintf!("i={i}");
        assert_close_vec3(err, pos_ref[i], mesh.vertex_position.get(i));
        assert_close_vec2(err, uv_ref[i], mesh.vertex_uv.get(i));
        assert_close_vec4(err, col_ref[i], mesh.vertex_color.get(i));
    }
});

file_test!(synthetic_extended_points, |scene, err, load_error| {
    let mesh = find_mesh(scene, "Points");

    ufbxt_assert!(mesh.num_faces == 3);
    ufbxt_assert!(mesh.num_empty_faces == 0);
    ufbxt_assert!(mesh.num_point_faces == 3);
    ufbxt_assert!(mesh.num_line_faces == 0);
    ufbxt_assert!(mesh.num_indices == 3);

    let pos_ref = [Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)];

    let col_ref = [Vec4::new(1.0, 0.0, 0.0, 1.0), Vec4::new(0.0, 1.0, 0.0, 1.0), Vec4::new(0.0, 0.0, 1.0, 1.0)];

    for i in 0..mesh.num_indices {
        ufbxt_hintf!("i={i}");
        assert_close_vec3(err, pos_ref[i], mesh.vertex_position.get(i));
        assert_close_vec4(err, col_ref[i], mesh.vertex_color.get(i));
    }
});

file_test!(synthetic_empty_face, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    check_warning(scene, WarningType::EmptyFaceRemoved, ElementType::Unknown, None, 1, None);

    let mesh = find_mesh(scene, "Object");

    ufbxt_assert!(mesh.num_faces == 0);
    ufbxt_assert!(mesh.num_empty_faces == 0);
});

fn allow_empty_faces_opts() -> LoadOpts {
    LoadOpts { allow_empty_faces: true, ..Default::default() }
}

file_test!(synthetic_empty_face_allow, file = "synthetic_empty_face", opts = allow_empty_faces_opts(), |scene, err, load_error| {
    let mesh = find_mesh(scene, "Object");

    ufbxt_assert!(mesh.num_faces == 1);
    ufbxt_assert!(mesh.num_empty_faces == 1);

    ufbxt_assert!(mesh.faces[0].index_begin == 0);
    ufbxt_assert!(mesh.faces[0].num_indices == 0);
});

file_test!(obj_space_default, file = "blender_279_ball", flags = FUZZ_ALWAYS, |scene, err, load_error| {
    if scene.metadata.file_format == FileFormat::Obj {
        ufbxt_assert!(scene.settings.axes.right == CoordinateAxis::Unknown);
        ufbxt_assert!(scene.settings.axes.up == CoordinateAxis::Unknown);
        ufbxt_assert!(scene.settings.axes.front == CoordinateAxis::Unknown);
        ufbxt_assert!(scene.settings.unit_meters == 0.0);
        ufbxt_assert!(scene.settings.original_unit_meters == 0.0);
    } else {
        ufbxt_assert!(scene.settings.axes.right == CoordinateAxis::PositiveX);
        ufbxt_assert!(scene.settings.axes.up == CoordinateAxis::PositiveY);
        ufbxt_assert!(scene.settings.axes.front == CoordinateAxis::PositiveZ);
        ufbxt_assert!(scene.settings.unit_meters == 0.01);
        ufbxt_assert!(scene.settings.original_unit_meters == 0.01);
    }
});

fn obj_axes_opts() -> LoadOpts {
    LoadOpts { obj_unit_meters: 0.1, obj_axes: CoordinateAxes::RIGHT_HANDED_Z_UP, ..Default::default() }
}

file_test!(obj_space_manual_axes, file = "blender_279_ball", opts = obj_axes_opts(), flags = FUZZ_ALWAYS, |scene, err, load_error| {
    if scene.metadata.file_format == FileFormat::Obj {
        ufbxt_assert!(scene.settings.axes.right == CoordinateAxis::PositiveX);
        ufbxt_assert!(scene.settings.axes.up == CoordinateAxis::PositiveZ);
        ufbxt_assert!(scene.settings.axes.front == CoordinateAxis::NegativeY);
        ufbxt_assert!(scene.settings.unit_meters == 0.1);
        ufbxt_assert!(scene.settings.original_unit_meters == 0.1);
    } else {
        ufbxt_assert!(scene.settings.axes.right == CoordinateAxis::PositiveX);
        ufbxt_assert!(scene.settings.axes.up == CoordinateAxis::PositiveY);
        ufbxt_assert!(scene.settings.axes.front == CoordinateAxis::PositiveZ);
        ufbxt_assert!(scene.settings.unit_meters == 0.01);
        ufbxt_assert!(scene.settings.original_unit_meters == 0.01);
    }
});

fn obj_axes_convert_opts() -> LoadOpts {
    LoadOpts {
        obj_unit_meters: 0.1,
        obj_axes: CoordinateAxes::RIGHT_HANDED_Z_UP,
        target_unit_meters: 1.0,
        target_axes: CoordinateAxes::RIGHT_HANDED_Y_UP,
        ..Default::default()
    }
}

file_test!(obj_space_manual_convert, file = "blender_279_ball", opts = obj_axes_convert_opts(), flags = FUZZ_ALWAYS, |scene, err, load_error| {
    if scene.metadata.file_format == FileFormat::Obj {
        ufbxt_assert!(scene.settings.axes.right == CoordinateAxis::PositiveX);
        ufbxt_assert!(scene.settings.axes.up == CoordinateAxis::PositiveZ);
        ufbxt_assert!(scene.settings.axes.front == CoordinateAxis::NegativeY);
        ufbxt_assert!(scene.settings.unit_meters == 0.1);
        ufbxt_assert!(scene.settings.original_unit_meters == 0.1);

        let node = &scene.nodes[scene.root_node];
        let scale = Vec3::new(0.1, 0.1, 0.1);
        let euler = Vec3::new(-90.0, 0.0, 0.0);
        let rotation = euler_to_quat(euler, RotationOrder::Xyz);

        assert_close_vec3(err, node.local_transform.scale, scale);
        assert_close_quat(err, node.local_transform.rotation, rotation);
    } else {
        ufbxt_assert!(scene.settings.axes.right == CoordinateAxis::PositiveX);
        ufbxt_assert!(scene.settings.axes.up == CoordinateAxis::PositiveY);
        ufbxt_assert!(scene.settings.axes.front == CoordinateAxis::PositiveZ);
        ufbxt_assert!(scene.settings.unit_meters == 0.01);
        ufbxt_assert!(scene.settings.original_unit_meters == 0.01);
    }
});

file_test!(blender_441_empty_material, |scene, err, load_error| {
    let mesh = find_mesh(scene, "Cube_Cube");

    ufbxt_assert!(mesh.materials.len() == 1);
    for &index in &mesh.face_material {
        ufbxt_assert!(index == 0);
    }
});

file_test!(synthetic_empty_material, |scene, err, load_error| {
    let mesh = find_mesh(scene, "Cube");

    ufbxt_assert!(mesh.materials.is_empty());
    ufbxt_assert!(mesh.face_material.is_empty());
});
