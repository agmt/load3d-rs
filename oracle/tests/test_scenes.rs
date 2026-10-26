mod ufbxt;
use ufbxt::*;

file_test!(maya_slime, flags = HEAVY_TO_FUZZ, |scene, err, load_error| {
    let node_high = find_node(scene, "Slime_002:Slime_Body_high").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node_high.is_some());
    ufbxt_assert!(!node_high.unwrap().visible);

    ufbxt_assert!(scene.textures.len() == 1);
    ufbxt_assert!(scene.textures[0].content.len() == 306577);
});

file_test!(blender_293_barbarian, |scene, err, load_error| {});

file_test!(evaluate_alloc_fail, file = "blender_293_barbarian", |scene, err, load_error| {});

fn add_scaled(sum: &mut Matrix, m: &Matrix, weight: Real) {
    let (mut s, v) = (sum.v(), m.v());
    for i in 0..12 {
        s[i] += v[i] * weight;
    }
    *sum = Matrix::from_v(s);
}

file_test!(maya_kenney_character, |scene, err, load_error| {
    check_frame(scene, err, false, "maya_kenney_character_4", None, 4.0 / 24.0);
    check_frame(scene, err, false, "maya_kenney_character_9", None, 9.0 / 24.0);

    {
        let node = find_node(scene, "characterMedium").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
        let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
        ufbxt_assert!(mesh.skin_deformers.len() == 1);
    }

    let mut frame = 3;
    while frame <= 15 {
        let time = frame as f64 / scene.settings.frames_per_second;
        let state = evaluate_scene(scene, &scene.anim, time, &EvaluateOpts::default());
        ufbxt_assert!(state.is_ok());

        let node = find_node(scene, "characterMedium").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
        let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
        ufbxt_assert!(mesh.skin_deformers.len() == 1);

        for level in 0..=2 {
            let mut weight_limit = 0;
            while weight_limit <= 5 {
                if weight_limit == 5 {
                    weight_limit = 8;
                }

                let subdivided;
                let sub_mesh = if level > 0 {
                    let mut opts = SubdivideOpts { evaluate_source_vertices: true, evaluate_skin_weights: true, ..Default::default() };
                    if weight_limit > 0 {
                        opts.max_skin_weights = weight_limit;
                        opts.max_source_vertices = weight_limit;
                    }
                    let result = load3d::subdivide_mesh(scene, mesh, level, &opts);
                    ufbxt_assert!(result.is_ok());
                    subdivided = result.unwrap();

                    let threshold = match weight_limit {
                        1 => 0.5,
                        2 => 0.25,
                        3 => 0.125,
                        4 => 0.1,
                        8 => 0.01,
                        _ => 0.001,
                    };
                    check_source_vertices(&subdivided, mesh, err, threshold);
                    &subdivided
                } else {
                    mesh
                };

                let skin = &scene.skin_deformers[mesh.skin_deformers[0]];

                for vi in 0..sub_mesh.num_vertices {
                    let skin_pos = sub_mesh.skinned_position.values[vi];

                    if level == 0 {
                        let mat = get_skin_vertex_matrix(scene, skin, vi, None);
                        let local_pos = sub_mesh.vertices[vi];
                        let world_pos = transform_position(&mat, local_pos);
                        assert_close_vec3(err, world_pos, skin_pos);

                        let skin_vertex = skin.vertices[vi];
                        let mut sum = Matrix::ZERO;
                        for wi in 0..skin_vertex.num_weights {
                            let weight = skin.weights[(skin_vertex.weight_begin + wi) as usize];
                            let cluster = &scene.skin_clusters[skin.clusters[weight.cluster_index as usize]];
                            add_scaled(&mut sum, &cluster.geometry_to_world, weight.weight);
                        }

                        let manual_pos = transform_position(&sum, local_pos);
                        assert_close_vec3(err, manual_pos, skin_pos);
                    } else {
                        let sub_res = sub_mesh.subdivision_result.as_ref();
                        ufbxt_assert!(sub_res.is_some());
                        let sub_res = sub_res.unwrap();

                        let range = sub_res.skin_cluster_ranges[vi];
                        let mut sum = Matrix::ZERO;
                        for wi in 0..range.num_weights {
                            let weight = sub_res.skin_cluster_weights[(range.weight_begin + wi) as usize];
                            let cluster = &scene.skin_clusters[skin.clusters[weight.index as usize]];
                            add_scaled(&mut sum, &cluster.geometry_to_world, weight.weight);
                        }

                        let threshold = match weight_limit {
                            1 | 2 => 20.0,
                            _ => 10.0,
                        };

                        let local_pos = sub_mesh.vertices[vi];
                        let manual_pos = transform_position(&sum, local_pos);
                        assert_close_vec3_threshold(err, manual_pos, skin_pos, threshold);
                    }
                }
                weight_limit += 1;
            }
        }
        frame += 10;
    }
});

fn ignore_content_opts() -> LoadOpts {
    LoadOpts { ignore_all_content: true, ..Default::default() }
}

file_test!(maya_slime_ignore, file = "maya_slime", opts = ignore_content_opts(), |scene, err, load_error| {
    let node_high = find_node(scene, "Slime_002:Slime_Body_high").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node_high.is_some());
    let node_high = node_high.unwrap();
    ufbxt_assert!(!node_high.visible);

    ufbxt_assert!(node_high.mesh.is_some());
    let mesh = &scene.meshes[node_high.mesh.unwrap()];
    ufbxt_assert!(mesh.num_faces == 0);
    ufbxt_assert!(mesh.num_vertices == 0);

    ufbxt_assert!(scene.anim_curves.len() == 76);
    for curve in &scene.anim_curves {
        ufbxt_assert!(curve.keyframes.is_empty());
    }

    ufbxt_assert!(scene.textures.len() == 1);
    ufbxt_assert!(scene.textures[0].content.is_empty());
});
