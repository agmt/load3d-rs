mod ufbxt;
use ufbxt::*;

fn node_mesh<'a>(scene: &'a Scene, name: &str) -> &'a Mesh {
    let node = find_node(scene, name).map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    &scene.meshes[node.unwrap().mesh.unwrap()]
}

file_test!(blender_279_default, |scene, err, load_error| {
    if scene.metadata.ascii {
        ufbxt_assert!(scene.metadata.exporter == Exporter::BlenderAscii);
        ufbxt_assert!(scene.metadata.exporter_version == pack_version(2, 79, 0));
    } else {
        ufbxt_assert!(scene.metadata.exporter == Exporter::BlenderBinary);
        ufbxt_assert!(scene.metadata.exporter_version == pack_version(3, 7, 13));
    }

    let node = find_node(scene, "Lamp").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let node = node.unwrap();
    let light = node.light.map(|l| &scene.lights[l]);
    ufbxt_assert!(light.is_some());
    let light = light.unwrap();

    let color_ref = Vec3::new(1.0, 1.0, 1.0);
    let color = light.element.props.find("Color");
    ufbxt_assert!(color.is_some_and(|p| p.prop_type == PropType::Color));
    assert_close_vec3(err, color.unwrap().value_vec3(), color_ref);

    let intensity = light.element.props.find("Intensity");
    ufbxt_assert!(intensity.is_some_and(|p| p.prop_type == PropType::Number));
    assert_close_real(err, intensity.unwrap().value_real(), 100.0);

    let translation_ref = Vec3::new(4.076245307922363, 5.903861999511719, -1.0054539442062378);
    let translation = node.element.props.find("Lcl Translation");
    ufbxt_assert!(translation.is_some_and(|p| p.prop_type == PropType::Translation));
    assert_close_vec3(err, translation.unwrap().value_vec3(), translation_ref);

    let scaling_ref = Vec3::new(1.0, 1.0, 1.0);
    let scaling = node.element.props.find("GeometricScaling");
    ufbxt_assert!(scaling.is_some_and(|p| p.prop_type == PropType::Vector));
    assert_close_vec3(err, scaling.unwrap().value_vec3(), scaling_ref);
});

file_test!(blender_282_suzanne, |scene, err, load_error| {});

file_test!(blender_282_suzanne_and_transform, |scene, err, load_error| {});

file_test!(maya_cube, |scene, err, load_error| {
    ufbxt_assert!(scene.metadata.exporter == Exporter::FbxSdk);
    ufbxt_assert!(scene.metadata.exporter_version == pack_version(2019, 2, 0));

    ufbxt_assert!(scene.metadata.original_application.vendor == "Autodesk");
    ufbxt_assert!(scene.metadata.original_application.name == "Maya");
    ufbxt_assert!(scene.metadata.original_application.version == "201900");
    ufbxt_assert!(scene.metadata.latest_application.vendor == "Autodesk");
    ufbxt_assert!(scene.metadata.latest_application.name == "Maya");
    ufbxt_assert!(scene.metadata.latest_application.version == "201900");

    let mesh = node_mesh(scene, "pCube1");

    ufbxt_assert!(!mesh.generated_normals);

    for face_i in 0..mesh.num_faces {
        let face = mesh.faces[face_i];
        let begin = face.index_begin as usize;
        let num = face.num_indices as usize;
        for i in begin..begin + num {
            let n = mesh.vertex_normal.get(i);
            let b = mesh.vertex_bitangent.get(i);
            let t = mesh.vertex_tangent.get(i);
            assert_close_real(err, dot3(n, n), 1.0);
            assert_close_real(err, dot3(b, b), 1.0);
            assert_close_real(err, dot3(t, t), 1.0);
            assert_close_real(err, dot3(n, b), 0.0);
            assert_close_real(err, dot3(n, t), 0.0);
            assert_close_real(err, dot3(b, t), 0.0);

            for j in 0..num {
                let p0 = mesh.vertex_position.get(begin + j);
                let p1 = mesh.vertex_position.get(begin + (j + 1) % num);
                let edge = Vec3::new(p1.x - p0.x, p1.y - p0.y, p1.z - p0.z);
                assert_close_real(err, dot3(edge, edge), 1.0);
                assert_close_real(err, dot3(n, edge), 0.0);
            }
        }
    }
});

fn check_maya_color_sets(err: &mut DiffError, scene: &Scene) {
    let mesh = node_mesh(scene, "pCube1");

    ufbxt_assert!(mesh.color_sets.len() == 4);
    ufbxt_assert!(mesh.color_sets[0].name == "RGBCube");
    ufbxt_assert!(mesh.color_sets[1].name == "White");
    ufbxt_assert!(mesh.color_sets[2].name == "Black");
    ufbxt_assert!(mesh.color_sets[3].name == "Alpha");

    for i in 0..mesh.num_indices {
        let pos = mesh.vertex_position.get(i);
        let mut refs = [Vec4::new(0.0, 0.0, 0.0, 1.0), Vec4::new(1.0, 1.0, 1.0, 1.0), Vec4::new(0.0, 0.0, 0.0, 1.0), Vec4::new(1.0, 1.0, 1.0, 0.0)];

        refs[0].x = pos.x + 0.5;
        refs[0].y = pos.y + 0.5;
        refs[0].z = pos.z + 0.5;
        refs[3].w = (pos.x + 0.5) * 0.1 + (pos.y + 0.5) * 0.2 + (pos.z + 0.5) * 0.4;

        for set_i in 0..4 {
            let color = mesh.color_sets[set_i].vertex_color.get(i);
            assert_close_vec4(err, color, refs[set_i]);
        }
    }
}

file_test!(maya_color_sets, |scene, err, load_error| {
    check_maya_color_sets(err, scene);
});

fn flip_winding_opts() -> LoadOpts {
    LoadOpts { reverse_winding: true, ..Default::default() }
}

file_test!(maya_color_sets_winding, file = "maya_color_sets", opts = flip_winding_opts(), flags = FUZZ_ALWAYS | FUZZ_OPTS, |scene, err, load_error| {
    check_maya_color_sets(err, scene);
});

fn check_uv_sets(err: &mut DiffError, mesh: &Mesh) {
    ufbxt_assert!(mesh.uv_sets.len() == 3);
    ufbxt_assert!(mesh.uv_sets[0].name == "Default");
    ufbxt_assert!(mesh.uv_sets[1].name == "PerFace");
    ufbxt_assert!(mesh.uv_sets[2].name == "Row");

    let mut counts1 = [[0usize; 2]; 2];
    let mut counts2 = [[0usize; 2]; 7];

    for i in 0..mesh.num_indices {
        let uv0 = mesh.uv_sets[0].vertex_uv.get(i);
        let uv1 = mesh.uv_sets[1].vertex_uv.get(i);
        let uv2 = mesh.uv_sets[2].vertex_uv.get(i);

        ufbxt_assert!(uv0.x > 0.05 && uv0.y > 0.05 && uv0.x < 0.95 && uv0.y < 0.95);
        let (x1, y1) = ((uv1.x + 0.5) as i32, (uv1.y + 0.5) as i32);
        let (x2, y2) = ((uv2.x + 0.5) as i32, (uv2.y + 0.5) as i32);
        assert_close_real(err, uv1.x - x1 as Real, 0.0);
        assert_close_real(err, uv1.y - y1 as Real, 0.0);
        assert_close_real(err, uv2.x - x2 as Real, 0.0);
        assert_close_real(err, uv2.y - y2 as Real, 0.0);
        ufbxt_assert!((0..=1).contains(&x1) && (0..=1).contains(&y1));
        ufbxt_assert!((0..=6).contains(&x2) && (0..=1).contains(&y2));
        counts1[x1 as usize][y1 as usize] += 1;
        counts2[x2 as usize][y2 as usize] += 1;
    }

    ufbxt_assert!(counts1[0][0] == 6);
    ufbxt_assert!(counts1[0][1] == 6);
    ufbxt_assert!(counts1[1][0] == 6);
    ufbxt_assert!(counts1[1][1] == 6);

    for (i, counts) in counts2.iter().enumerate() {
        let n = if i == 0 || i == 6 { 1 } else { 2 };
        ufbxt_assert!(counts[0] == n);
        ufbxt_assert!(counts[1] == n);
    }
}

file_test!(maya_uv_sets, |scene, err, load_error| {
    check_uv_sets(err, node_mesh(scene, "pCube1"));
});

file_test!(blender_279_color_sets, |scene, err, load_error| {
    let mesh = node_mesh(scene, "Cube");

    ufbxt_assert!(mesh.color_sets.len() == 3);
    ufbxt_assert!(mesh.color_sets[0].name == "RGBCube");
    ufbxt_assert!(mesh.color_sets[1].name == "White");
    ufbxt_assert!(mesh.color_sets[2].name == "Black");

    for i in 0..mesh.num_indices {
        let pos = mesh.vertex_position.get(i);
        let mut refs = [Vec4::new(0.0, 0.0, 0.0, 1.0), Vec4::new(1.0, 1.0, 1.0, 1.0), Vec4::new(0.0, 0.0, 0.0, 1.0)];

        refs[0].x = pos.x + 0.5;
        refs[0].y = pos.y + 0.5;
        refs[0].z = pos.z + 0.5;

        for set_i in 0..3 {
            let color = mesh.color_sets[set_i].vertex_color.get(i);
            assert_close_vec4(err, color, refs[set_i]);
        }
    }
});

file_test!(blender_279_uv_sets, |scene, err, load_error| {
    check_uv_sets(err, node_mesh(scene, "Cube"));
});

file_test!(synthetic_sets_reorder, |scene, err, load_error| {
    let mesh = node_mesh(scene, "pCube1");

    ufbxt_assert!(mesh.color_sets.len() == 4);
    ufbxt_assert!(mesh.color_sets[0].name == "RGBCube");
    ufbxt_assert!(mesh.color_sets[1].name == "White");
    ufbxt_assert!(mesh.color_sets[2].name == "Black");
    ufbxt_assert!(mesh.color_sets[3].name == "Alpha");
    ufbxt_assert!(mesh.uv_sets[0].name == "Default");
    ufbxt_assert!(mesh.uv_sets[1].name == "PerFace");
    ufbxt_assert!(mesh.uv_sets[2].name == "Row");
});

file_test!(maya_cone, |scene, err, load_error| {
    let mesh = node_mesh(scene, "pCone1");

    ufbxt_assert!(mesh.vertex_crease.exists);
    ufbxt_assert!(!mesh.edges.is_empty());
    ufbxt_assert!(!mesh.edge_crease.is_empty());
    ufbxt_assert!(!mesh.edge_smoothing.is_empty());

    ufbxt_assert!(mesh.faces[0].num_indices == 16);

    for i in 0..mesh.num_indices {
        let pos = mesh.vertex_position.get(i);
        let crease = mesh.vertex_crease.get(i);

        assert_close_real(err, crease, if pos.y > 0.0 { 0.998 } else { 0.0 });
    }

    for i in 0..mesh.num_edges {
        let edge = mesh.edges[i];
        let crease = mesh.edge_crease[i];
        let smoothing = mesh.edge_smoothing[i];
        let a = mesh.vertex_position.get(edge.a as usize);
        let b = mesh.vertex_position.get(edge.b as usize);

        if a.y < 0.0 && b.y < 0.0 {
            assert_close_real(err, crease, 0.583);
            ufbxt_assert!(!smoothing);
        } else {
            ufbxt_assert!(a.y > 0.0 || b.y > 0.0);
            assert_close_real(err, crease, 0.0);
            ufbxt_assert!(smoothing);
        }
    }
});

fn check_tangent_space(err: &mut DiffError, mesh: &Mesh) {
    for set in &mesh.uv_sets {
        ufbxt_assert!(set.vertex_uv.exists);
        ufbxt_assert!(set.vertex_bitangent.exists);
        ufbxt_assert!(set.vertex_tangent.exists);

        for face_i in 0..mesh.num_faces {
            let face = mesh.faces[face_i];
            let num = face.num_indices as usize;

            for i in 0..num {
                let a = face.index_begin as usize + i;
                let b = face.index_begin as usize + (i + 1) % num;

                let pa = mesh.vertex_position.get(a);
                let pb = mesh.vertex_position.get(b);
                let ba = set.vertex_bitangent.get(a);
                let bb = set.vertex_bitangent.get(b);
                let ta = set.vertex_tangent.get(a);
                let tb = set.vertex_tangent.get(b);
                let ua = set.vertex_uv.get(a);
                let ub = set.vertex_uv.get(b);

                let mut dp = sub3(pb, pa);
                let mut du = sub2(ua, ub);

                let dp_len = dot3(dp, dp).sqrt();
                dp.x /= dp_len;
                dp.y /= dp_len;
                dp.z /= dp_len;

                let du_len = dot2(du, du).sqrt();
                du.x /= du_len;
                du.y /= du_len;

                let dba = dot3(dp, ba);
                let dbb = dot3(dp, bb);
                let dta = dot3(dp, ta);
                let dtb = dot3(dp, tb);
                assert_close_real(err, dba, dbb);
                assert_close_real(err, dta, dtb);

                assert_close_real(err, ub.x - ua.x, dta);
                assert_close_real(err, ub.y - ua.y, dba);
            }
        }
    }
}

file_test!(maya_uv_set_tangents, |scene, err, load_error| {
    check_tangent_space(err, node_mesh(scene, "pPlane1"));
});

file_test!(blender_279_uv_set_tangents, |scene, err, load_error| {
    check_tangent_space(err, node_mesh(scene, "Plane"));
});

file_test!(synthetic_tangents_reorder, |scene, err, load_error| {
    check_tangent_space(err, node_mesh(scene, "pPlane1"));
});

file_test!(blender_279_ball, |scene, err, load_error| {
    let red_id = find_element(scene, ElementType::Material, "Red");
    let white_id = find_element(scene, ElementType::Material, "White");
    let red = red_id.and_then(|id| scene.as_material(id));
    let white = white_id.and_then(|id| scene.as_material(id));
    ufbxt_assert!(red.is_some_and(|m| m.element.name == "Red"));
    ufbxt_assert!(white.is_some_and(|m| m.element.name == "White"));
    let (red, white) = (red.unwrap(), white.unwrap());

    let red_ref = Vec3::new(0.8, 0.0, 0.0);
    let white_ref = Vec3::new(0.8, 0.8, 0.8);
    assert_close_vec3(err, red.fbx.diffuse_color().value_vec3(), red_ref);
    assert_close_vec3(err, white.fbx.diffuse_color().value_vec3(), white_ref);

    let node = find_node(scene, "Icosphere").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let mesh = node.unwrap().mesh.map(|m| &scene.meshes[m]);
    ufbxt_assert!(mesh.is_some());
    let mesh = mesh.unwrap();
    ufbxt_assert!(!mesh.face_material.is_empty());
    ufbxt_assert!(!mesh.face_smoothing.is_empty());

    let material_id = |i: usize| mesh.materials[i].map(|m| scene.materials[m].element.element_id);
    ufbxt_assert!(mesh.materials.len() == 2);
    ufbxt_assert!(material_id(0) == red_id);
    ufbxt_assert!(material_id(1) == white_id);

    for face_i in 0..mesh.num_faces {
        let face = mesh.faces[face_i];
        let mut mid = Vec3::ZERO;
        for i in 0..face.num_indices as usize {
            mid = add3(mid, mesh.vertex_position.get(face.index_begin as usize + i));
        }
        mid.x /= face.num_indices as Real;
        mid.y /= face.num_indices as Real;
        mid.z /= face.num_indices as Real;

        let smoothing = mesh.face_smoothing[face_i];
        let material = mesh.face_material[face_i];
        ufbxt_assert!(smoothing == (mid.x > 0.0));
        ufbxt_assert!(material == if mid.z < 0.0 { 1 } else { 0 });
    }
});

file_test!(synthetic_broken_material, |scene, err, load_error| {
    let mesh = node_mesh(scene, "pCube1");

    ufbxt_assert!(mesh.materials.is_empty());
    ufbxt_assert!(mesh.face_material.is_empty());
});

file_test!(maya_uv_and_color_sets, |scene, err, load_error| {
    let mesh = node_mesh(scene, "pCube1");

    ufbxt_assert!(mesh.uv_sets.len() == 2);
    ufbxt_assert!(mesh.color_sets.len() == 2);
    ufbxt_assert!(mesh.uv_sets[0].name == "UVA");
    ufbxt_assert!(mesh.uv_sets[1].name == "UVB");
    ufbxt_assert!(mesh.color_sets[0].name == "ColorA");
    ufbxt_assert!(mesh.color_sets[1].name == "ColorB");
});

file_test!(maya_bad_face, |scene, err, load_error| {
    let mesh = node_mesh(scene, "pCube1");

    ufbxt_assert!(mesh.num_empty_faces == 0);
    ufbxt_assert!(mesh.num_point_faces == 1);
    ufbxt_assert!(mesh.num_line_faces == 1);
    ufbxt_assert!(mesh.num_triangles == 7);
    ufbxt_assert!(mesh.num_faces == 6);

    ufbxt_assert!(mesh.faces[0].num_indices == 1);
    ufbxt_assert!(mesh.faces[1].num_indices == 2);
    ufbxt_assert!(mesh.faces[2].num_indices == 3);
    ufbxt_assert!(mesh.faces[3].num_indices == 4);

    ufbxt_assert!(mesh.faces[0].index_begin == 0);
    ufbxt_assert!(mesh.faces[1].index_begin == 1);
    ufbxt_assert!(mesh.faces[2].index_begin == 3);
    ufbxt_assert!(mesh.faces[3].index_begin == 6);

    // ??? Maya exports an edge for the single point
    ufbxt_assert!(mesh.num_edges == 12);
    ufbxt_assert!(mesh.edges[0].a == 0);
    ufbxt_assert!(mesh.edges[0].b == 0);
    ufbxt_assert!(mesh.edges[1].a == 2);
    ufbxt_assert!(mesh.edges[1].b == 1);
});

file_test!(blender_279_edge_vertex, |scene, err, load_error| {
    let mesh = node_mesh(scene, "Plane");

    ufbxt_assert!(mesh.num_empty_faces == 0);
    ufbxt_assert!(mesh.num_point_faces == 0);
    ufbxt_assert!(mesh.num_line_faces == 1);
    ufbxt_assert!(mesh.num_triangles == 0);
    ufbxt_assert!(mesh.num_faces == 1);

    ufbxt_assert!(mesh.faces[0].index_begin == 0);
    ufbxt_assert!(mesh.faces[0].num_indices == 2);

    // ??? Maya exports an edge for the single point
    if scene.metadata.version == 7400 {
        ufbxt_assert!(mesh.num_edges == 1);
        ufbxt_assert!(mesh.edges[0].a == 0);
        ufbxt_assert!(mesh.edges[0].b == 1);
    } else {
        // 6100 has an edge for both directions
        ufbxt_assert!(mesh.num_edges == 2);
        ufbxt_assert!(mesh.edges[0].a == 0);
        ufbxt_assert!(mesh.edges[0].b == 1);
        ufbxt_assert!(mesh.edges[1].a == 1);
        ufbxt_assert!(mesh.edges[1].b == 0);
    }
});

file_test!(blender_279_edge_circle, |scene, err, load_error| {
    let mesh = node_mesh(scene, "Circle");

    ufbxt_assert!(mesh.num_indices == 512);

    // ??? The 7400_binary export starts off with individual edges but has a
    // massive N-gon for the rest of it.
    if scene.metadata.version == 7400 {
        ufbxt_assert!(mesh.num_line_faces == 127);
        ufbxt_assert!(mesh.num_faces == 128);

        for i in 0..127 {
            ufbxt_assert!(mesh.faces[i].num_indices == 2);
        }
        ufbxt_assert!(mesh.faces[127].num_indices == 258);

        ufbxt_assert!(mesh.num_edges == 256);
    } else {
        ufbxt_assert!(mesh.num_line_faces == 256);
        ufbxt_assert!(mesh.num_faces == 256);

        for i in 0..256 {
            ufbxt_assert!(mesh.faces[i].num_indices == 2);
        }

        // 6100 has an edge for both directions
        ufbxt_assert!(mesh.num_edges == 512);
    }
});

file_test!(blender_293_instancing, |scene, err, load_error| {
    ufbxt_assert!(scene.meshes.len() == 1);
    let mesh = &scene.meshes[0];
    ufbxt_assert!(mesh.element.instances.len() == 8);
});

fn check_normals_by_vertex(err: &mut DiffError, mesh: &Mesh) {
    for vi in 0..mesh.num_vertices {
        let ii = mesh.vertex_first_index[vi] as usize;
        let pos = mesh.vertex_position.values[mesh.vertex_position.indices[ii] as usize];
        let normal = mesh.vertex_normal.values[mesh.vertex_normal.indices[ii] as usize];
        let ref_normal = Vec3::new(0.0, if pos.y > 0.0 { 1.0 } else { -1.0 }, 0.0);
        assert_close_vec3(err, normal, ref_normal);
    }

    for ii in 0..mesh.num_indices {
        let pos = mesh.vertex_position.values[mesh.vertex_position.indices[ii] as usize];
        let normal = mesh.vertex_normal.values[mesh.vertex_normal.indices[ii] as usize];
        let ref_normal = Vec3::new(0.0, if pos.y > 0.0 { 1.0 } else { -1.0 }, 0.0);
        assert_close_vec3(err, normal, ref_normal);
    }
}

file_test!(synthetic_indexed_by_vertex, |scene, err, load_error| {
    ufbxt_assert!(scene.meshes.len() == 1);
    check_normals_by_vertex(err, &scene.meshes[0]);
});

file_test!(synthetic_by_vertex_bad_index, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    check_warning(scene, WarningType::IndexClamped, ElementType::Mesh, Some("#pCube1"), 9, None);

    ufbxt_assert!(scene.meshes.len() == 1);
    check_normals_by_vertex(err, &scene.meshes[0]);
});

file_test!(synthetic_by_vertex_overflow, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    check_warning(scene, WarningType::IndexClamped, ElementType::Mesh, Some("#pCube1"), 12, None);

    ufbxt_assert!(scene.meshes.len() == 1);
    let mesh = &scene.meshes[0];

    ufbxt_assert!(mesh.vertex_normal.values.len() == 4);
    for ii in 0..mesh.num_indices {
        let vertex_ix = mesh.vertex_position.indices[ii];
        let normal_ix = mesh.vertex_normal.indices[ii];
        if (vertex_ix as usize) < mesh.vertex_normal.values.len() {
            ufbxt_assert!(normal_ix == vertex_ix);
        } else {
            ufbxt_assert!(normal_ix == mesh.vertex_normal.values.len() as u32 - 1);
        }
    }
});

file_test!(maya_lod_group, |scene, err, load_error| {
    {
        let node = find_node(scene, "LOD_Group_1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.attrib_type == ElementType::LodGroup);
        let lod_group = scene.as_lod_group(node.attrib.unwrap()).unwrap();
        ufbxt_assert!(lod_group.element.element_type == ElementType::LodGroup);

        ufbxt_assert!(node.children.len() == 3);
        ufbxt_assert!(lod_group.lod_levels.len() == 3);

        ufbxt_assert!(lod_group.relative_distances);
        ufbxt_assert!(!lod_group.use_distance_limit);
        ufbxt_assert!(!lod_group.ignore_parent_transform);

        ufbxt_assert!(lod_group.lod_levels[0].display == LodDisplay::UseLod);
        ufbxt_assert!(lod_group.lod_levels[1].display == LodDisplay::UseLod);
        ufbxt_assert!(lod_group.lod_levels[2].display == LodDisplay::UseLod);

        if scene.metadata.version >= 7000 {
            assert_close_real(err, lod_group.lod_levels[0].distance, 100.0);
            assert_close_real(err, lod_group.lod_levels[1].distance, 64.0);
            assert_close_real(err, lod_group.lod_levels[2].distance, 32.0);
        }
    }

    {
        let node = find_node(scene, "LOD_Group_2").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.attrib_type == ElementType::LodGroup);
        let lod_group = scene.as_lod_group(node.attrib.unwrap()).unwrap();
        ufbxt_assert!(lod_group.element.element_type == ElementType::LodGroup);

        ufbxt_assert!(node.children.len() == 3);
        ufbxt_assert!(lod_group.lod_levels.len() == 3);

        ufbxt_assert!(!lod_group.relative_distances);
        ufbxt_assert!(!lod_group.use_distance_limit);
        ufbxt_assert!(lod_group.ignore_parent_transform);

        ufbxt_assert!(lod_group.lod_levels[0].display == LodDisplay::UseLod);
        ufbxt_assert!(lod_group.lod_levels[1].display == LodDisplay::Show);
        ufbxt_assert!(lod_group.lod_levels[2].display == LodDisplay::Hide);

        if scene.metadata.version >= 7000 {
            assert_close_real(err, lod_group.lod_levels[0].distance, 0.0);
            assert_close_real(err, lod_group.lod_levels[1].distance, 4.520276);
            assert_close_real(err, lod_group.lod_levels[2].distance, 18.081102);
        }
    }
});

file_test!(synthetic_missing_normals, |scene, err, load_error| {
    let mesh = node_mesh(scene, "pCube1");
    ufbxt_assert!(!mesh.vertex_normal.exists);
    ufbxt_assert!(!mesh.skinned_normal.exists);
});

fn generate_normals_opts() -> LoadOpts {
    LoadOpts { generate_missing_normals: true, ..Default::default() }
}

file_test!(
    synthetic_missing_normals_generated,
    file = "synthetic_missing_normals",
    opts = generate_normals_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS,
    |scene, err, load_error| {
        let mesh = node_mesh(scene, "pCube1");
        ufbxt_assert!(mesh.vertex_normal.exists);
        ufbxt_assert!(mesh.skinned_normal.exists);
        ufbxt_assert!(mesh.generated_normals);

        for &face in &mesh.faces {
            let normal = normalize(get_weighted_face_normal(&mesh.vertex_position, face));
            for i in 0..face.num_indices as usize {
                let mesh_normal = mesh.vertex_normal.get(face.index_begin as usize + i);
                let skinned_normal = mesh.skinned_normal.get(face.index_begin as usize + i);
                assert_close_vec3(err, normal, mesh_normal);
                assert_close_vec3(err, normal, skinned_normal);
            }
        }
    }
);

file_test!(blender_279_nested_meshes, |scene, err, load_error| {
});

file_test!(max_edge_visibility, |scene, err, load_error| {
    {
        let mesh = node_mesh(scene, "Box001");
        ufbxt_assert!(!mesh.edge_visibility.is_empty());

        {
            let mut num_visible = 0;

            for i in 0..mesh.num_edges {
                let edge = mesh.edges[i];
                let a = mesh.vertex_position.get(edge.a as usize);
                let b = mesh.vertex_position.get(edge.b as usize);
                let len = length3(sub3(a, b));
                let expected = len < 21.0;
                let visible = mesh.edge_visibility[i];
                ufbxt_assert!(visible == expected);
                num_visible += if visible { 1 } else { 0 };
            }

            ufbxt_assert!(mesh.num_edges == 18);
            ufbxt_assert!(num_visible == 12);
        }

        let sub_mesh = ufbxt_subdivide_mesh(scene, mesh, 2, None);
        ufbxt_assert!(sub_mesh.is_ok());
        let sub_mesh = sub_mesh.unwrap();
        check_mesh(scene, &sub_mesh);

        {
            let num_visible = sub_mesh.edge_visibility[..sub_mesh.num_edges].iter().filter(|&&v| v).count();
            ufbxt_assert!(num_visible == 12 * 4);
        }
    }

    {
        let mesh = node_mesh(scene, "Cylinder001");
        ufbxt_assert!(!mesh.edge_visibility.is_empty());
        let mut num_visible = 0;

        for i in 0..mesh.num_edges {
            let edge = mesh.edges[i];
            let a = mesh.vertex_position.get(edge.a as usize);
            let b = mesh.vertex_position.get(edge.b as usize);
            let a2 = Vec2::new(a.x, a.y);
            let b2 = Vec2::new(b.x, b.y);
            let len = length3(sub3(a, b));
            let len_a2 = length2(a2);
            let len_b2 = length2(b2);
            let expected = len < 20.7 && len_a2 > 0.1 && len_b2 > 0.1;
            let visible = mesh.edge_visibility[i];
            ufbxt_assert!(visible == expected);
            num_visible += if visible { 1 } else { 0 };
        }

        ufbxt_assert!(mesh.num_edges == 54);
        ufbxt_assert!(num_visible == 27);
    }
});

fn check_front_groups(mesh: &Mesh) {
    let mut num_front = 0;
    for i in 0..mesh.num_faces {
        let face = mesh.faces[i];
        let normal = get_weighted_face_normal(&mesh.vertex_position, face);
        let group = if normal.z < 0.0 { 9598 } else { 15349 };
        num_front += if normal.z < 0.0 { 1 } else { 0 };
        let group_ix = mesh.face_group[i];
        ufbxt_assert!(mesh.face_groups[group_ix as usize].id == group);
    }
    ufbxt_assert!(num_front == mesh.num_faces / 2);
}

file_test!(zbrush_d20, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    ufbxt_assert!(scene.nodes.len() == 3);

    {
        let mesh = node_mesh(scene, "20 Sided");
        ufbxt_assert!(mesh.num_faces == 20);
        ufbxt_assert!(mesh.face_group.len() == 20);
        for i in 0..20 {
            let group_ix = mesh.face_group[i as usize];
            let group = &mesh.face_groups[group_ix as usize];
            ufbxt_assert!(group.id == 10 + i * 5);
        }
    }

    {
        let mesh = node_mesh(scene, "PolyMesh3D1");
        ufbxt_assert!(mesh.num_faces == 24);
        ufbxt_assert!(mesh.face_group.len() == 24);

        check_front_groups(mesh);

        ufbxt_assert!(!mesh.blend_deformers.is_empty());
        let blend = &scene.blend_deformers[mesh.blend_deformers[0]];

        // WHAT? The 6100 version has duplicated blend shapes
        // and 7500 has duplicated blend deformers...
        // After e8cab0f ufbx ignores duplicated connections so we see only
        // a single blend deformer here..
        if scene.metadata.version == 6100 {
            ufbxt_assert!(mesh.blend_deformers.len() == 1);
            ufbxt_assert!(blend.channels.len() == 4);
        } else {
            ufbxt_assert!(mesh.blend_deformers.len() == 1);
            ufbxt_assert!(blend.channels.len() == 2);

            let blend_deformer = &scene.blend_deformers[mesh.blend_deformers[0]];
            let warning_substring = format!("to {}", mesh.element.element_id);
            check_warning_imp(scene, WarningType::DuplicateConnection, blend_deformer.element.element_id, 1, Some(&warning_substring));
        }

        let sub_mesh = ufbxt_subdivide_mesh(scene, mesh, 2, None);
        ufbxt_assert!(sub_mesh.is_ok());
        let sub_mesh = sub_mesh.unwrap();
        check_mesh(scene, &sub_mesh);

        check_mesh(scene, mesh);

        ufbxt_assert!(sub_mesh.face_group.len() == sub_mesh.num_faces);
        check_front_groups(&sub_mesh);
    }
});

file_test!(zbrush_d20_selection_set, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    {
        let node_ix = find_node(scene, "PolyMesh3D1");
        ufbxt_assert!(node_ix.is_some_and(|n| scene.nodes[n].mesh.is_some()));
        let mesh_ix = scene.nodes[node_ix.unwrap()].mesh;
        let mesh = &scene.meshes[mesh_ix.unwrap()];

        for i in 0..2 {
            let front = i == 0;
            let name = if front { "PolyMesh3D1_9598" } else { "PolyMesh3D1_15349" };
            let set = find_element(scene, ElementType::SelectionSet, name).and_then(|id| scene.as_selection_set(id));
            ufbxt_assert!(set.is_some());
            let set = set.unwrap();
            ufbxt_assert!(set.nodes.len() == 1);
            let sel_node = &scene.selection_nodes[set.nodes[0]];

            ufbxt_assert!(sel_node.target_node == node_ix);
            ufbxt_assert!(sel_node.target_mesh == mesh_ix);
            ufbxt_assert!(sel_node.faces.len() == 12);

            for &index in &sel_node.faces {
                ufbxt_assert!((index as usize) < mesh.faces.len());
                let face = mesh.faces[index as usize];
                let normal = get_weighted_face_normal(&mesh.vertex_position, face);
                ufbxt_assert!((normal.z < 0.0) == front);
            }
        }
    }
});

fn check_polygon_holes(mesh: &Mesh, expected_holes: usize) {
    let mut num_holes = 0;
    ufbxt_assert!(mesh.face_hole.len() == mesh.num_faces);
    for i in 0..mesh.num_faces {
        let face = mesh.faces[i];
        let mut avg_pos = Vec3::ZERO;
        for j in 0..face.num_indices as usize {
            let p = mesh.vertex_position.get(face.index_begin as usize + j);
            avg_pos = add3(avg_pos, p);
        }
        avg_pos = mul3(avg_pos, 1.0 / face.num_indices as Real);

        let hole = avg_pos.y.abs() > 0.49;
        num_holes += if hole { 1 } else { 0 };
        ufbxt_assert!(mesh.face_hole[i] == hole);
    }
    ufbxt_assert!(num_holes == expected_holes);
}

file_test!(maya_polygon_hole, |scene, err, load_error| {
    let mesh = node_mesh(scene, "pCube1");

    ufbxt_assert!(mesh.num_faces == 6);

    check_polygon_holes(mesh, 2);

    let sub_mesh = ufbxt_subdivide_mesh(scene, mesh, 2, None);
    ufbxt_assert!(sub_mesh.is_ok());
    let sub_mesh = sub_mesh.unwrap();
    check_mesh(scene, &sub_mesh);

    check_polygon_holes(&sub_mesh, 32);
});

file_test!(synthetic_cursed_geometry, opts = generate_normals_opts(), |scene, err, load_error| {
    let mesh = node_mesh(scene, "pDisc1");

    let mut indices = [0u32; 64];

    for &face in &mesh.faces {
        let num_tris = triangulate_face(&mut indices, mesh, face);
        ufbxt_assert!(num_tris == 0 || num_tris == face.num_indices - 2);
    }

    let _sub_mesh = ufbxt_subdivide_mesh(scene, mesh, 2, None);
});

file_test!(synthetic_vertex_gaps, |scene, err, load_error| {
    let mesh = node_mesh(scene, "pPlane1");
    ufbxt_assert!(mesh.vertices.len() == 7);

    let gap_values = [Vec3::new(-1.0, -1.1, -1.2), Vec3::new(-2.0, -2.1, -2.2), Vec3::new(-3.0, -3.1, -3.2), Vec3::new(-4.0, -4.1, -4.2)];

    assert_close_vec3(err, mesh.vertices[0], gap_values[0]);
    assert_close_vec3(err, mesh.vertices[3], gap_values[1]);
    assert_close_vec3(err, mesh.vertices[6], gap_values[2]);

    ufbxt_assert!(mesh.vertex_normal.values.len() == 8);
    assert_close_vec3(err, mesh.vertex_normal.values[0], gap_values[0]);
    assert_close_vec3(err, mesh.vertex_normal.values[3], gap_values[1]);
    assert_close_vec3(err, mesh.vertex_normal.values[6], gap_values[2]);
    assert_close_vec3(err, mesh.vertex_normal.values[7], gap_values[3]);

    let gap_uvs = [Vec2::new(-1.3, -1.4), Vec2::new(-2.3, -2.4), Vec2::new(-3.3, -3.4), Vec2::new(-4.3, -4.4)];

    ufbxt_assert!(mesh.vertex_uv.values.len() == 8);
    assert_close_vec2(err, mesh.vertex_uv.values[0], gap_uvs[0]);
    assert_close_vec2(err, mesh.vertex_uv.values[2], gap_uvs[1]);
    assert_close_vec2(err, mesh.vertex_uv.values[5], gap_uvs[2]);
    assert_close_vec2(err, mesh.vertex_uv.values[7], gap_uvs[3]);
});

struct GroupsPerFaceCount {
    num_faces: usize,
    num_groups: u32,
}

file_test!(zbrush_polygroup_mess, |scene, err, load_error| {
    ufbxt_assert!(scene.meshes.len() == 1);
    let mesh = &scene.meshes[0];

    let groups_per_face_count_ref = [
        (1, 2),
        (2, 1164),
        (3, 1),
        (4, 482),
        (6, 181),
        (8, 90),
        (10, 26),
        (12, 21),
        (14, 8),
        (16, 10),
        (18, 5),
        (20, 6),
        (22, 4),
        (24, 7),
        (26, 1),
        (28, 1),
        (30, 1),
        (32, 5),
        (34, 2),
        (40, 1),
        (48, 3),
        (50, 1),
        (60, 1),
        (64, 2),
        (72, 1),
        (104, 1),
        (128, 1),
        (328, 1),
    ]
    .map(|(num_faces, num_groups)| GroupsPerFaceCount { num_faces, num_groups });

    let mut groups_per_face_count = [0u32; 512];

    ufbxt_assert!(mesh.face_groups.len() == 2029);
    for i in 0..mesh.face_groups.len() {
        let part = &mesh.face_group_parts[i];
        ufbxt_assert!(part.num_faces < groups_per_face_count.len());
        groups_per_face_count[part.num_faces] += 1;
    }

    for r in &groups_per_face_count_ref {
        ufbxt_assert!(groups_per_face_count[r.num_faces] == r.num_groups);
    }
});

file_test!(zbrush_cut_sphere, |scene, err, load_error| {});

struct FaceGroupRef {
    id: i32,
    face_count: usize,
}

file_test!(synthetic_face_group_id, |scene, err, load_error| {
    let ref_groups =
        [(i32::MIN, 1), (-2000, 2), (-2, 1), (-1, 1), (0, 3), (1, 2), (2, 2), (10, 1), (20, 1), (30, 1), (40, 1), (50, 1), (3000, 2), (i32::MAX, 1)]
            .map(|(id, face_count)| FaceGroupRef { id, face_count });

    let mesh = node_mesh(scene, "20 Sided");

    for (i, r) in ref_groups.iter().enumerate() {
        ufbxt_hintf!("i = {i}");
        ufbxt_assert!(i < mesh.face_groups.len());
        let group = &mesh.face_groups[i];
        let part = &mesh.face_group_parts[i];
        ufbxt_assert!(group.id == r.id);
        ufbxt_assert!(part.num_faces == r.face_count);
    }
});

file_test!(blender_279_empty_cube, |scene, err, load_error| {
    let mesh = node_mesh(scene, "Cube");

    ufbxt_assert!(mesh.num_faces == 0);
    ufbxt_assert!(mesh.num_indices == 0);
    ufbxt_assert!(mesh.num_vertices == 0);

    ufbxt_assert!(mesh.materials.len() == 1);
    ufbxt_assert!(mesh.materials[0].is_some());
    ufbxt_assert!(mesh.material_parts[0].num_faces == 0);
});

file_test!(synthetic_truncated_crease_partial, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    check_warning(scene, WarningType::TruncatedArray, ElementType::Mesh, Some("#pPlane1"), 1, Some("Truncated array: EdgeCrease"));

    let mesh = node_mesh(scene, "pPlane1");
    ufbxt_assert!(mesh.edge_crease.len() == 4);
    assert_close_real(err, mesh.edge_crease[0], 0.25);
    assert_close_real(err, mesh.edge_crease[1], 0.5);
    assert_close_real(err, mesh.edge_crease[2], 0.5);
    assert_close_real(err, mesh.edge_crease[3], 0.5);
});

file_test!(synthetic_truncated_crease_full, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    check_warning(scene, WarningType::TruncatedArray, ElementType::Mesh, Some("#pPlane1"), 1, Some("Truncated array: EdgeCrease"));

    let mesh = node_mesh(scene, "pPlane1");
    ufbxt_assert!(mesh.edge_crease.len() == 4);
    assert_close_real(err, mesh.edge_crease[0], 0.0);
    assert_close_real(err, mesh.edge_crease[1], 0.0);
    assert_close_real(err, mesh.edge_crease[2], 0.0);
    assert_close_real(err, mesh.edge_crease[3], 0.0);
});

file_test!(motionbuilder_smoothing, flags = ALLOW_INVALID_UNICODE | ALLOW_WARNINGS, |scene, err, load_error| {
    check_warning(scene, WarningType::MissingGeometryData, ElementType::Mesh, Some("#pCube1"), 1, Some("Missing geometry data: Smoothing"));

    let mesh = node_mesh(scene, "pCube1");

    ufbxt_assert!(mesh.edge_smoothing.is_empty());
});

fn skip_mesh_parts_opts() -> LoadOpts {
    LoadOpts { skip_mesh_parts: true, ..Default::default() }
}

file_test!(maya_cube_skip_mesh_parts, file = "maya_cube", opts = skip_mesh_parts_opts(), |scene, err, load_error| {
    let mesh = node_mesh(scene, "pCube1");
    ufbxt_assert!(mesh.materials.len() == 1);
    ufbxt_assert!(mesh.material_parts.is_empty());
    ufbxt_assert!(mesh.face_groups.is_empty());
    ufbxt_assert!(mesh.face_group_parts.is_empty());
});

file_test!(synthetic_face_group_id_skip_mesh_parts, file = "synthetic_face_group_id", opts = skip_mesh_parts_opts(), |scene, err, load_error| {
    let mesh = node_mesh(scene, "20 Sided");
    ufbxt_assert!(mesh.materials.len() == 1);
    ufbxt_assert!(mesh.material_parts.is_empty());
    ufbxt_assert!(mesh.face_groups.len() == 14);
    ufbxt_assert!(mesh.face_group_parts.is_empty());
});

file_test!(synthetic_face_groups_skip_mesh_parts, file = "synthetic_face_groups", opts = skip_mesh_parts_opts(), |scene, err, load_error| {
    let mesh = node_mesh(scene, "Cube");
    ufbxt_assert!(mesh.materials.len() == 2);
    ufbxt_assert!(mesh.material_parts.is_empty());
    ufbxt_assert!(mesh.face_groups.len() == 4);
    ufbxt_assert!(mesh.face_group_parts.is_empty());
});

fn cm_y_up_opts() -> LoadOpts {
    LoadOpts { target_unit_meters: 0.01, target_axes: CoordinateAxes::RIGHT_HANDED_Y_UP, ..Default::default() }
}

fn cm_y_up_abort_opts() -> LoadOpts {
    LoadOpts {
        target_unit_meters: 0.01,
        target_axes: CoordinateAxes::RIGHT_HANDED_Y_UP,
        index_error_handling: IndexErrorHandling::AbortLoading,
        ..Default::default()
    }
}

#[allow(dead_code)]
fn find_metadata_object<'a>(scene: &'a Scene, element: &Element) -> Option<&'a MetadataObject> {
    for conn in &element.connections_dst {
        if !conn.src_prop.is_empty() || !conn.dst_prop.is_empty() {
            continue;
        }
        if scene.element(conn.src).element_type == ElementType::MetadataObject {
            return scene.as_metadata_object(conn.src);
        }
    }
    None
}

file_test!(revit_wall_square, opts = cm_y_up_opts(), |scene, err, load_error| {});

file_test!(revit_wall_square_abort, file = "revit_wall_square", opts = cm_y_up_abort_opts(), |scene, err, load_error| {});

file_test!(synthetic_direct_by_polygon, |scene, err, load_error| {
    let node = find_node(scene, "pPlane1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());

    let mesh = node.unwrap().mesh.map(|m| &scene.meshes[m]);
    ufbxt_assert!(mesh.is_some());
    let mesh = mesh.unwrap();

    ufbxt_assert!(mesh.faces.len() == 2);
    ufbxt_assert!(!mesh.vertex_normal.unique_per_vertex);
    ufbxt_assert!(mesh.vertex_normal.values.len() == 2);

    let ref_normals = [Vec3::new(-0.707106769, 0.707106769, 0.0), Vec3::new(0.707106769, 0.707106769, 0.0)];
    for (face_ix, &face) in mesh.faces.iter().enumerate() {
        for i in 0..face.num_indices as usize {
            let normal = mesh.vertex_normal.get(face.index_begin as usize + i);
            assert_close_vec3(err, normal, ref_normals[face_ix]);
        }
    }
});

file_test!(blender_292_circle, flags = ALLOW_STRICT_ERROR, |scene, err, load_error| {
    let node = find_node(scene, "Circle").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    ufbxt_assert!(node.unwrap().mesh.is_some());
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
    ufbxt_assert!(mesh.num_vertices == 8);

    for i in 0..mesh.num_vertices {
        ufbxt_assert!(mesh.vertex_first_index[i] == NO_INDEX);
    }
});

fn retain_vertex_w_opts() -> LoadOpts {
    LoadOpts { retain_vertex_attrib_w: true, ..Default::default() }
}

fn get_vertex_w_vec3(v: &VertexVec3, index: usize) -> Real {
    v.catch_get_w(index).unwrap()
}

fn check_tangent_sign(scene: &Scene, mesh: &Mesh, ref_sign: Real) {
    if scene.metadata.version < 7000 {
        ufbxt_assert!(mesh.vertex_normal.values_w.is_empty());
        ufbxt_assert!(mesh.vertex_tangent.values_w.is_empty());
        ufbxt_assert!(mesh.vertex_bitangent.values_w.is_empty());
        return;
    }

    for ix in 0..mesh.num_indices {
        let normal = mesh.vertex_normal.get(ix);
        let tangent = mesh.vertex_tangent.get(ix);
        let bitangent = mesh.vertex_bitangent.get(ix);

        let b = cross3(normal, tangent);
        let dot = dot3(bitangent, b);
        let sign = if dot > 0.0 { 1.0 } else { -1.0 };

        let tangent_w = get_vertex_w_vec3(&mesh.vertex_tangent, ix);
        ufbxt_assert!(tangent_w == sign);
        if ref_sign != 0.0 {
            ufbxt_assert!(sign == ref_sign);
        }
    }
}

fn check_tangent_sign_set(scene: &Scene, mesh: &Mesh, set: &UvSet, ref_sign: Real) {
    if scene.metadata.version < 7000 {
        ufbxt_assert!(mesh.vertex_normal.values_w.is_empty());
        ufbxt_assert!(set.vertex_tangent.values_w.is_empty());
        ufbxt_assert!(set.vertex_bitangent.values_w.is_empty());
        return;
    }

    for ix in 0..mesh.num_indices {
        let normal = mesh.vertex_normal.get(ix);
        let tangent = set.vertex_tangent.get(ix);
        let bitangent = set.vertex_bitangent.get(ix);

        let b = cross3(normal, tangent);
        let dot = dot3(bitangent, b);
        let sign = if dot > 0.0 { 1.0 } else { -1.0 };

        let tangent_w = get_vertex_w_vec3(&set.vertex_tangent, ix);
        ufbxt_assert!(tangent_w == sign);
        if ref_sign != 0.0 {
            ufbxt_assert!(sign == ref_sign);
        }
    }
}

fn check_vertex_w(scene: &Scene, mesh: &Mesh, attrib: &VertexVec3, r: Real) {
    if scene.metadata.version < 7000 {
        ufbxt_assert!(attrib.values_w.is_empty());
        return;
    }

    for ix in 0..mesh.num_indices {
        let value_w = get_vertex_w_vec3(attrib, ix);
        ufbxt_assert!(value_w == r);
    }
}

fn check_tangent_sign_nodes(scene: &Scene, names: [&str; 4], signs: [Real; 4], normal_w: Real) {
    for (name, sign) in names.into_iter().zip(signs) {
        let mesh = node_mesh(scene, name);
        check_tangent_sign(scene, mesh, sign);
        check_vertex_w(scene, mesh, &mesh.vertex_normal, normal_w);
        check_vertex_w(scene, mesh, &mesh.vertex_bitangent, 1.0);
    }
}

fn check_no_vertex_w(scene: &Scene, names: [&str; 4]) {
    for name in names {
        let mesh = node_mesh(scene, name);
        ufbxt_assert!(mesh.vertex_normal.values_w.is_empty());
        ufbxt_assert!(mesh.vertex_tangent.values_w.is_empty());
        ufbxt_assert!(mesh.vertex_bitangent.values_w.is_empty());
    }
}

file_test!(maya_tangent_sign, opts = retain_vertex_w_opts(), flags = FUZZ_ALWAYS | FUZZ_OPTS, |scene, err, load_error| {
    check_tangent_sign_nodes(scene, ["Positive", "FlipX", "FlipY", "FlipBoth"], [1.0, -1.0, -1.0, 1.0], 1.0);
});

file_test!(max_tangent_sign, opts = retain_vertex_w_opts(), |scene, err, load_error| {
    check_tangent_sign_nodes(scene, ["Positive", "FlipX", "FlipY", "FlipBoth"], [1.0, -1.0, -1.0, 1.0], 0.0);
});

file_test!(blender340_tangent_sign, opts = retain_vertex_w_opts(), |scene, err, load_error| {
    check_no_vertex_w(scene, ["Positive", "FlipX", "FlipY", "FlipXY"]);
});

file_test!(maya_tangent_sign_default, file = "maya_tangent_sign", |scene, err, load_error| {
    check_no_vertex_w(scene, ["Positive", "FlipX", "FlipY", "FlipBoth"]);
});

file_test!(maya_tangent_sign_mixed_split, opts = retain_vertex_w_opts(), |scene, err, load_error| {
    let mesh = node_mesh(scene, "polySurface2");
    check_vertex_w(scene, mesh, &mesh.vertex_normal, 1.0);
    check_vertex_w(scene, mesh, &mesh.vertex_bitangent, 1.0);
    check_tangent_sign(scene, mesh, 0.0);
});

file_test!(maya_tangent_sign_mixed, opts = retain_vertex_w_opts(), |scene, err, load_error| {
    let mesh = node_mesh(scene, "pPlane1");
    check_vertex_w(scene, mesh, &mesh.vertex_normal, 1.0);
    check_vertex_w(scene, mesh, &mesh.vertex_bitangent, 1.0);

    // ???: The tangent sign seems absolutely arbitrary here
    let tangent_w_ref = [-1.0, -1.0, 1.0, -1.0, -1.0, -1.0, 1.0, 1.0];
    for i in 0..mesh.num_indices {
        let w = get_vertex_w_vec3(&mesh.vertex_tangent, i);
        ufbxt_assert!(w == tangent_w_ref[i]);
    }
});

file_test!(maya_uv_set_tangent_w, opts = retain_vertex_w_opts(), |scene, err, load_error| {
    let mesh = node_mesh(scene, "pPlane1");
    ufbxt_assert!(mesh.uv_sets.len() == 3);
    check_vertex_w(scene, mesh, &mesh.vertex_normal, 1.0);

    let uv_map1 = &mesh.uv_sets[0];
    ufbxt_assert!(uv_map1.name == "map1");
    check_tangent_sign_set(scene, mesh, uv_map1, 1.0);
    check_vertex_w(scene, mesh, &uv_map1.vertex_bitangent, 1.0);

    let uv_left = &mesh.uv_sets[1];
    ufbxt_assert!(uv_left.name == "FlipLeft");
    check_vertex_w(scene, mesh, &uv_left.vertex_bitangent, 1.0);

    // ???: The tangent sign seems absolutely arbitrary here
    let tangent_w_left = [-1.0, -1.0, 1.0, -1.0, -1.0, -1.0, 1.0, 1.0];
    for i in 0..mesh.num_indices {
        let w = get_vertex_w_vec3(&uv_left.vertex_tangent, i);
        if scene.metadata.version >= 7000 {
            ufbxt_assert!(w == tangent_w_left[i]);
        } else {
            ufbxt_assert!(w == 0.0);
        }
    }

    let uv_right = &mesh.uv_sets[2];
    ufbxt_assert!(uv_right.name == "FlipRight");
    check_vertex_w(scene, mesh, &uv_right.vertex_bitangent, 1.0);

    // The tangent sign seems arbitrary here, though inverted compared to `uv_left`
    let tangent_w_right = [1.0, 1.0, -1.0, 1.0, 1.0, 1.0, -1.0, -1.0];
    for i in 0..mesh.num_indices {
        let w = get_vertex_w_vec3(&uv_right.vertex_tangent, i);
        if scene.metadata.version >= 7000 {
            ufbxt_assert!(w == tangent_w_right[i]);
        } else {
            ufbxt_assert!(w == 0.0);
        }
    }
});

file_test!(synthetic_missing_mapping, flags = ALLOW_STRICT_ERROR | ALLOW_WARNINGS, |scene, err, load_error| {
    let mesh = node_mesh(scene, "pCube1");

    let ref_warnings = [
        "Ignoring geometry 'Normals' with bad mapping mode ''",
        "Ignoring geometry 'Binormals' with bad mapping mode ''",
        "Ignoring geometry 'Tangents' with bad mapping mode 'Missing'",
        "Ignoring geometry 'UV' with bad mapping mode ''",
        "Ignoring geometry 'Smoothing' with bad mapping mode ''",
        "Ignoring geometry 'Materials' with bad mapping mode ''",
    ];
    let mut found = [false; 6];

    ufbxt_assert!(scene.metadata.warnings.len() == 6);
    for warning in &scene.metadata.warnings {
        ufbxt_assert!(warning.kind == WarningType::MissingPolygonMapping);
        ufbxt_assert!(warning.element_id == mesh.element.element_id);
        if let Some(j) = ref_warnings.iter().position(|&r| warning.description == r) {
            found[j] = true;
        }
    }

    for (i, &f) in found.iter().enumerate() {
        ufbxt_hintf!("i={i}");
        ufbxt_assert!(f);
    }
});
