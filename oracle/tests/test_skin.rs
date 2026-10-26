mod ufbxt;
use ufbxt::*;

const SPACE_CONVERSIONS: [SpaceConversion; 3] = [SpaceConversion::TransformRoot, SpaceConversion::AdjustTransforms, SpaceConversion::ModifyGeometry];
const MIRROR_AXES: [MirrorAxis; 4] = [MirrorAxis::None, MirrorAxis::X, MirrorAxis::Y, MirrorAxis::Z];

fn check_stack_times(scene: &Scene, err: &mut DiffError, stack_name: &str, begin: f64, end: f64) {
    let stack = find_element(scene, ElementType::AnimStack, stack_name).and_then(|id| scene.as_anim_stack(id));
    ufbxt_assert!(stack.is_some());
    let stack = stack.unwrap();
    ufbxt_assert!(stack.element.name == stack_name);
    assert_close_real(err, stack.time_begin, begin);
    assert_close_real(err, stack.time_end, end);
}

fn check_stack_timestamps(scene: &Scene, _err: &mut DiffError, stack_name: &str, begin: i64, end: i64) {
    let stack = find_element(scene, ElementType::AnimStack, stack_name).and_then(|id| scene.as_anim_stack(id));
    ufbxt_assert!(stack.is_some());
    let stack = stack.unwrap();
    ufbxt_assert!(stack.element.name == stack_name);

    ufbxt_assert!(stack.element.props.find_int("LocalStart", 0) == begin);
    ufbxt_assert!(stack.element.props.find_int("LocalStop", 0) == end);
    ufbxt_assert!(stack.element.props.find_int("ReferenceStart", 0) == begin);
    ufbxt_assert!(stack.element.props.find_int("ReferenceStop", 0) == end);
}

file_test!(blender_279_sausage, |scene, err, load_error| {
    if scene.metadata.ascii {
        // ???: In the 6100 ASCII file Spin Take starts from -1 frames
        check_stack_times(scene, err, "Base", 0.0, 1.0 / 24.0);
        check_stack_times(scene, err, "Spin", -1.0 / 24.0, 18.0 / 24.0);
        check_stack_times(scene, err, "Wiggle", 0.0, 19.0 / 24.0);
    } else {
        check_stack_times(scene, err, "Skeleton|Base", 0.0, 1.0 / 24.0);
        check_stack_times(scene, err, "Skeleton|Spin", 0.0, 19.0 / 24.0);
        check_stack_times(scene, err, "Skeleton|Wiggle", 0.0, 19.0 / 24.0);
    }

    let cluster_names = [["Bottom", "Cluster Skin Bottom"], ["Middle", "Cluster Skin Middle"], ["Top", "Cluster Skin Top"]];
    let cluster_bind_ref = [
        [
            Matrix::from_v([1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]),
            Matrix::from_v([0.0, -1.0, 0.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        ],
        [
            Matrix::from_v([1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0, 0.0, -1.9, 0.0]),
            Matrix::from_v([0.0, -1.0, 0.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0, -1.9, 0.0, 0.0]),
        ],
        [
            Matrix::from_v([1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0, 0.0, -3.8, 0.0]),
            Matrix::from_v([0.0, -1.0, 0.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0, -3.8, 0.0, 0.0]),
        ],
    ];

    let ascii = scene.metadata.ascii as usize;
    for i in 0..3 {
        let cluster = find_element(scene, ElementType::SkinCluster, cluster_names[i][ascii]).and_then(|id| scene.as_skin_cluster(id));
        ufbxt_assert!(cluster.is_some());
        let cluster = cluster.unwrap();
        assert_close_vec3(err, cluster.geometry_to_bone.col(0), cluster_bind_ref[i][ascii].col(0));
        assert_close_vec3(err, cluster.geometry_to_bone.col(1), cluster_bind_ref[i][ascii].col(1));
        assert_close_vec3(err, cluster.geometry_to_bone.col(2), cluster_bind_ref[i][ascii].col(2));
        assert_close_vec3(err, cluster.geometry_to_bone.col(3), cluster_bind_ref[i][ascii].col(3));
    }

    check_frame(scene, err, true, "blender_279_sausage_base_0", Some("Base"), 0.0);
    check_frame(scene, err, false, "blender_279_sausage_spin_15", Some("Spin"), 15.0 / 24.0);
    check_frame(scene, err, false, "blender_279_sausage_wiggle_20", Some("Wiggle"), 20.0 / 24.0);
});

file_test!(maya_game_sausage, |scene, err, load_error| {});

file_test!(maya_game_sausage_wiggle, file = "maya_game_sausage", suffix = "wiggle", |scene, err, load_error| {
    check_frame(scene, err, true, "maya_game_sausage_wiggle_10", None, 10.0 / 24.0);
    check_frame(scene, err, true, "maya_game_sausage_wiggle_18", None, 18.0 / 24.0);
});

file_test!(maya_game_sausage_spin, file = "maya_game_sausage", suffix = "spin", |scene, err, load_error| {
    check_frame(scene, err, false, "maya_game_sausage_spin_7", None, 27.0 / 24.0);
    check_frame(scene, err, false, "maya_game_sausage_spin_15", None, 35.0 / 24.0);
});

file_test!(maya_game_sausage_deform, file = "maya_game_sausage", suffix = "deform", |scene, err, load_error| {
    check_frame(scene, err, false, "maya_game_sausage_deform_8", None, 48.0 / 24.0);
    check_frame(scene, err, false, "maya_game_sausage_deform_15", None, 55.0 / 24.0);
});

file_test!(maya_game_sausage_combined, file = "maya_game_sausage", suffix = "combined", |scene, err, load_error| {
    check_stack_times(scene, err, "wiggle", 1.0 / 24.0, 20.0 / 24.0);
    check_stack_times(scene, err, "spin", 20.0 / 24.0, 40.0 / 24.0);
    check_stack_times(scene, err, "deform", 40.0 / 24.0, 60.0 / 24.0);

    check_stack_timestamps(scene, err, "wiggle", 1924423250, 38488465000);
    check_stack_timestamps(scene, err, "spin", 38488465000, 76976930000);
    check_stack_timestamps(scene, err, "deform", 76976930000, 115465395000);

    check_frame(scene, err, true, "maya_game_sausage_wiggle_10", Some("wiggle"), 10.0 / 24.0);
    check_frame(scene, err, true, "maya_game_sausage_wiggle_18", Some("wiggle"), 18.0 / 24.0);
    check_frame(scene, err, false, "maya_game_sausage_spin_7", Some("spin"), 27.0 / 24.0);
    check_frame(scene, err, false, "maya_game_sausage_spin_15", Some("spin"), 35.0 / 24.0);
    check_frame(scene, err, false, "maya_game_sausage_deform_8", Some("deform"), 48.0 / 24.0);
    check_frame(scene, err, false, "maya_game_sausage_deform_15", Some("deform"), 55.0 / 24.0);
});

file_test!(maya_game_sausage_combined_bake, file = "maya_game_sausage", suffix = "combined", |scene, err, load_error| {
    {
        let stack = &scene.anim_stacks[find_anim_stack(scene, "spin").unwrap()];
        let bake = ufbxt_bake_anim(scene, &stack.anim, None);
        ufbxt_assert!(bake.is_ok());
        let bake = bake.unwrap();

        ufbxt_assert!(bake.key_time_min == 20.0 / 24.0);
        ufbxt_assert!(bake.key_time_max == 40.0 / 24.0);

        ufbxt_assert!(bake.playback_time_begin == 20.0 / 24.0);
        ufbxt_assert!(bake.playback_time_end == 40.0 / 24.0);
        assert_close_double(err, bake.playback_duration, 40.0 / 24.0 - 20.0 / 24.0);
    }

    {
        let stack = &scene.anim_stacks[find_anim_stack(scene, "spin").unwrap()];

        let opts = BakeOpts { trim_start_time: true, ..Default::default() };

        let bake = ufbxt_bake_anim(scene, &stack.anim, Some(&opts));
        ufbxt_assert!(bake.is_ok());
        let bake = bake.unwrap();

        ufbxt_assert!(bake.key_time_min == 20.0 / 24.0);
        ufbxt_assert!(bake.key_time_max == 40.0 / 24.0);

        ufbxt_assert!(bake.playback_time_begin == 20.0 / 24.0);
        ufbxt_assert!(bake.playback_time_end == 40.0 / 24.0);
        assert_close_double(err, bake.playback_duration, 40.0 / 24.0 - 20.0 / 24.0);
    }
});

file_test!(synthetic_sausage_wiggle_no_link, |scene, err, load_error| {
    check_frame(scene, err, true, "maya_game_sausage_wiggle_10", None, 10.0 / 24.0);
    check_frame(scene, err, true, "maya_game_sausage_wiggle_18", None, 18.0 / 24.0);
});

file_test!(synthetic_sausage_wiggle_no_bind, |scene, err, load_error| {
    check_frame(scene, err, true, "maya_game_sausage_wiggle_10", None, 10.0 / 24.0);
    check_frame(scene, err, true, "maya_game_sausage_wiggle_18", None, 18.0 / 24.0);
});

fn check_bind_poses(scene: &Scene, err: &mut DiffError) {
    ufbxt_assert!(scene.poses.len() == 1);
    let pose = &scene.poses[0];
    ufbxt_assert!(pose.is_bind_pose);

    for cluster in &scene.skin_clusters {
        let bone_pose = pose.bone_poses.iter().find(|bone_pose| Some(bone_pose.bone_node) == cluster.bone_node);
        ufbxt_assert!(bone_pose.is_some());
        let bone_pose = bone_pose.unwrap();

        assert_close_matrix(err, bone_pose.bone_to_world, cluster.bind_to_world);
        assert_close_matrix(err, bone_pose.bone_to_world, scene.nodes[cluster.bone_node.unwrap()].node_to_world);
    }
}

fn sausage_wiggle_conversion(file: &str, bind_poses: bool) {
    let mut err = DiffError::default();

    let check = |err: &mut DiffError, path: &str, opts: &LoadOpts, frame_name: &str, flags: u32| {
        let scene = load_file(path, opts);
        if let Err(e) = &scene {
            log_error(e);
        }
        ufbxt_assert!(scene.is_ok());
        let scene = scene.unwrap();

        check_scene(&scene);
        if bind_poses {
            check_bind_poses(&scene, err);
        }

        check_frame_imp(&scene, err, true, &format!("{frame_name}_10"), None, 10.0 / 24.0, flags);
        check_frame_imp(&scene, err, true, &format!("{frame_name}_18"), None, 18.0 / 24.0, flags);
    };

    for path in FileIterator::new(file) {
        for (space_conversion_i, space_conversion) in SPACE_CONVERSIONS.into_iter().enumerate() {
            ufbxt_hintf!("space_conversion={space_conversion_i}");

            {
                let opts = LoadOpts { space_conversion, target_unit_meters: 1.0, ..Default::default() };
                check(&mut err, &path, &opts, "maya_game_sausage_wiggle", CHECK_FRAME_SCALE_100);
            }

            {
                let opts = LoadOpts { space_conversion, target_axes: CoordinateAxes::RIGHT_HANDED_Z_UP, target_unit_meters: 1.0, ..Default::default() };
                check(&mut err, &path, &opts, "maya_game_sausage_wiggle", CHECK_FRAME_SCALE_100 | CHECK_FRAME_Z_TO_Y_UP);
            }

            for (mirror_i, mirror_axis) in MIRROR_AXES.into_iter().enumerate() {
                // Cannot convert handedness without mirroring if not transforming root
                if space_conversion != SpaceConversion::TransformRoot && mirror_i == 0 {
                    continue;
                }

                let opts = LoadOpts {
                    space_conversion,
                    target_axes: CoordinateAxes::LEFT_HANDED_Y_UP,
                    target_unit_meters: 1.0,
                    handedness_conversion_axis: mirror_axis,
                    reverse_winding: mirror_i == 0,
                    ..Default::default()
                };
                check(&mut err, &path, &opts, "maya_game_sausage_wiggle_lefthanded", CHECK_FRAME_SCALE_100);
            }

            for (mirror_i, mirror_axis) in MIRROR_AXES.into_iter().enumerate() {
                // Cannot convert handedness without mirroring if not transforming root
                if space_conversion != SpaceConversion::TransformRoot && mirror_i == 0 {
                    continue;
                }

                let opts = LoadOpts {
                    space_conversion,
                    target_axes: CoordinateAxes::LEFT_HANDED_Z_UP,
                    target_unit_meters: 1.0,
                    handedness_conversion_axis: mirror_axis,
                    reverse_winding: mirror_i == 0,
                    ..Default::default()
                };
                check(&mut err, &path, &opts, "maya_game_sausage_wiggle_lefthanded", CHECK_FRAME_SCALE_100 | CHECK_FRAME_Z_TO_Y_UP);
            }
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn synthetic_sausage_wiggle_no_link_conversion() {
    sausage_wiggle_conversion("synthetic_sausage_wiggle_no_link", true);
}

#[test]
fn synthetic_sausage_wiggle_no_bind_conversion() {
    sausage_wiggle_conversion("synthetic_sausage_wiggle_no_bind", false);
}

file_test!(maya_blend_shape_cube, |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];

    ufbxt_assert!(mesh.blend_deformers.len() == 1);
    let deformer = &scene.blend_deformers[mesh.blend_deformers[0]];

    let mut top: [Option<usize>; 2] = [None, None];
    for &chan in &deformer.channels {
        let name = &scene.blend_channels[chan].element.name;
        if name.contains("TopH") {
            top[0] = Some(chan);
        }
        if name.contains("TopV") {
            top[1] = Some(chan);
        }
    }
    ufbxt_assert!(top[0].is_some() && top[1].is_some());
    let top = [top[0].unwrap(), top[1].unwrap()];

    assert_close_real(err, scene.blend_channels[top[0]].weight, 1.0);
    assert_close_real(err, scene.blend_channels[top[1]].weight, 1.0);

    let keyframes: [[f64; 3]; 4] = [[1.0 / 24.0, 1.0, 1.0], [16.0 / 24.0, 0.279, 0.670], [53.0 / 24.0, 0.901, 0.168], [120.0 / 24.0, 1.0, 1.0]];
    let v = Vec3::new;
    let ref_offsets: [[Vec3; 8]; 2] = [
        [
            v(0.0, 0.0, 0.0),
            v(0.0, 0.0, 0.0),
            v(0.317, 0.0, 0.0),
            v(-0.317, 0.0, 0.0),
            v(0.317, 0.0, 0.0),
            v(-0.317, 0.0, 0.0),
            v(0.0, 0.0, 0.0),
            v(0.0, 0.0, 0.0),
        ],
        [
            v(0.0, 0.0, 0.0),
            v(0.0, 0.0, 0.0),
            v(0.0, 0.0, -0.284),
            v(0.0, 0.0, -0.284),
            v(0.0, 0.0, 0.284),
            v(0.0, 0.0, 0.284),
            v(0.0, 0.0, 0.0),
            v(0.0, 0.0, 0.0),
        ],
    ];

    for chan_ix in 0..2 {
        let chan = &scene.blend_channels[top[chan_ix]];
        ufbxt_assert!(chan.keyframes.len() == 1);

        assert_close_real(err, chan.keyframes[0].target_weight, 1.0);
        let shape = &scene.blend_shapes[chan.keyframes[0].shape];

        let mut offsets = [Vec3::ZERO; 8];
        add_blend_shape_vertex_offsets(shape, &mut offsets, 1.0);
        for i in 0..8 {
            let r = ref_offsets[chan_ix][i];
            let off_a = offsets[i];
            let off_b = get_blend_shape_vertex_offset(shape, i);
            assert_close_vec3(err, r, off_a);
            assert_close_vec3(err, r, off_b);
        }

        for frame in &keyframes {
            let time = frame[0];
            let r = frame[1 + chan_ix];

            let weight = evaluate_blend_weight(scene, &scene.anim, top[chan_ix], time);
            assert_close_real(err, weight, r);

            let prop = evaluate_prop(scene, &scene.anim, chan.element.element_id, "DeformPercent", time);
            assert_close_real(err, prop.value_real() / 100.0, r);
        }
    }

    {
        let mut offsets = [Vec3::ZERO; 8];
        add_blend_vertex_offsets(scene, deformer, &mut offsets, 1.0);
        for i in 0..8 {
            let r = add3(ref_offsets[0][i], ref_offsets[1][i]);
            assert_close_vec3(err, r, offsets[i]);
        }
    }

    for eval_skin in [false, true] {
        for frame in &keyframes {
            let time = frame[0];

            let opts = EvaluateOpts { evaluate_skinning: eval_skin, ..Default::default() };

            let state = evaluate_scene(scene, &scene.anim, time, &opts);
            ufbxt_assert!(state.is_ok());
            let state = state.unwrap();

            let mut weights: [Real; 2] = [0.0; 2];
            for chan_ix in 0..2 {
                let r = frame[1 + chan_ix];

                let chan = &state.blend_channels[scene.blend_channels[top[chan_ix]].element.typed_id as usize];

                let prop = evaluate_prop(&state, &scene.anim, chan.element.element_id, "DeformPercent", time);

                assert_close_real(err, prop.value_real() / 100.0, r);
                assert_close_real(err, chan.weight, r);
                weights[chan_ix] = chan.weight;
            }

            if eval_skin {
                let eval_deformer = &state.blend_deformers[deformer.element.typed_id as usize];
                let eval_mesh = &state.meshes[mesh.element.typed_id as usize];
                for i in 0..8 {
                    let original_pos = eval_mesh.vertex_position.values[i];
                    let skinned_pos = eval_mesh.skinned_position.values[i];
                    let mut r = original_pos;
                    let mut blend_pos = get_blend_vertex_offset(&state, eval_deformer, i);
                    r = add3(r, mul3(ref_offsets[0][i], weights[0]));
                    r = add3(r, mul3(ref_offsets[1][i], weights[1]));
                    blend_pos = add3(original_pos, blend_pos);
                    assert_close_vec3(err, r, skinned_pos);
                    assert_close_vec3(err, r, blend_pos);
                }
            }

            check_scene(&state);
        }
    }
});

file_test!(maya_blend_inbetween, |scene, err, load_error| {
    check_frame(scene, err, false, "maya_blend_inbetween_1", None, 1.0 / 24.0);
    check_frame(scene, err, false, "maya_blend_inbetween_30", None, 30.0 / 24.0);
    check_frame(scene, err, false, "maya_blend_inbetween_60", None, 60.0 / 24.0);
    check_frame(scene, err, false, "maya_blend_inbetween_65", None, 65.0 / 24.0);
    check_frame(scene, err, false, "maya_blend_inbetween_71", None, 71.0 / 24.0);
    check_frame(scene, err, false, "maya_blend_inbetween_80", None, 80.0 / 24.0);
    check_frame(scene, err, false, "maya_blend_inbetween_89", None, 89.0 / 24.0);
    check_frame(scene, err, false, "maya_blend_inbetween_120", None, 120.0 / 24.0);
});

file_test!(synthetic_blend_shape_order, |scene, err, load_error| {
    let node = find_node(scene, "pPlatonic1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let mesh = node.unwrap().mesh.map(|m| &scene.meshes[m]);
    ufbxt_assert!(mesh.is_some());
    let mesh = mesh.unwrap();

    let v = Vec3::new;
    let r = [
        v(-0.041956, -0.004164, -1.064113),
        v(0.736301, 0.734684, -0.451944),
        v(-0.263699, 1.059604, -0.451944),
        v(-0.951595, -0.168521, -0.372847),
        v(-0.333561, -1.019172, -0.372847),
        v(1.004034, -0.525731, -0.447214),
        v(1.019802, -0.010427, 0.466597),
        v(0.289087, 1.059604, 0.442483),
        v(-0.816271, 0.564766, 0.459767),
        v(-0.870921, -0.699814, 0.400384),
        v(0.514865, -0.854815, 0.383101),
        v(0.238471, -0.004164, 0.935887),
    ];

    let shape = find_element(scene, ElementType::BlendShape, "Target").and_then(|id| scene.as_blend_shape(id));
    ufbxt_assert!(shape.is_some());
    let shape = shape.unwrap();

    let mut pos = [Vec3::ZERO; 12];
    pos.copy_from_slice(&mesh.vertices[..12]);
    add_blend_shape_vertex_offsets(shape, &mut pos, 0.5);

    for i in 0..mesh.num_vertices {
        let vert = mesh.vertices[i];
        let off = get_blend_shape_vertex_offset(shape, i);
        let res = Vec3::new(vert.x + off.x * 0.5, vert.y + off.y * 0.5, vert.z + off.z * 0.5);
        assert_close_vec3(err, res, r[i]);
        assert_close_vec3(err, pos[i], r[i]);
    }
});

file_test!(blender_293_half_skinned, |scene, err, load_error| {
    let node = find_node(scene, "Plane").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let mesh = node.unwrap().mesh.map(|m| &scene.meshes[m]);
    ufbxt_assert!(mesh.is_some());
    let mesh = mesh.unwrap();
    ufbxt_assert!(mesh.num_vertices == 4);
    ufbxt_assert!(mesh.skin_deformers.len() == 1);
    let skin = &scene.skin_deformers[mesh.skin_deformers[0]];
    ufbxt_assert!(skin.vertices.len() == 4);
    ufbxt_assert!(skin.vertices[0].num_weights == 1);
    ufbxt_assert!(skin.vertices[0].weight_begin == 0);
    ufbxt_assert!(skin.vertices[1].num_weights == 1);
    ufbxt_assert!(skin.vertices[1].weight_begin == 1);
    ufbxt_assert!(skin.vertices[2].num_weights == 0);
    ufbxt_assert!(skin.vertices[3].num_weights == 0);
});

file_test!(maya_dual_quaternion, |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
    ufbxt_assert!(mesh.skin_deformers.len() == 1);
    let skin = &scene.skin_deformers[mesh.skin_deformers[0]];
    ufbxt_assert!(skin.skinning_method == SkinningMethod::DualQuaternion);
});

file_test!(maya_dual_quaternion_scale, |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
    ufbxt_assert!(mesh.skin_deformers.len() == 1);
    let skin = &scene.skin_deformers[mesh.skin_deformers[0]];
    ufbxt_assert!(skin.skinning_method == SkinningMethod::DualQuaternion);
});

file_test!(maya_dq_weights, |scene, err, load_error| {
    check_frame(scene, err, false, "maya_dq_weights_10", None, 10.0 / 24.0);
    check_frame(scene, err, false, "maya_dq_weights_18", None, 18.0 / 24.0);
});

file_test!(maya_bone_radius, |scene, err, load_error| {
    for i in 1..scene.nodes.len() {
        let node = &scene.nodes[i];
        ufbxt_assert!(node.element.name.contains("joint"));
        let bone = node.attrib.and_then(|a| scene.as_bone(a));
        ufbxt_assert!(bone.is_some());
        let bone = bone.unwrap();
        assert_close_real(err, bone.radius, i as Real * 0.2);
        assert_close_real(err, bone.relative_length, 1.0);
    }
});

file_test!(blender_279_bone_radius, |scene, err, load_error| {
    let armature = find_node(scene, "Armature").unwrap();
    let mut node = &scene.nodes[armature];

    for i in 0.. {
        if node.children.is_empty() {
            break;
        }
        node = &scene.nodes[node.children[0]];

        ufbxt_assert!(node.element.name.contains("Bone"));
        let bone = node.attrib.and_then(|a| scene.as_bone(a));
        ufbxt_assert!(bone.is_some());
        let bone = bone.unwrap();

        // Blender end bones have some weird scaling factor
        // TODO: Add quirks mode for this?
        if node.element.name.contains("end") {
            continue;
        }

        if scene.metadata.ascii {
            assert_close_real(err, bone.radius, 1.0);
        } else {
            assert_close_real(err, bone.radius, (i + 1) as Real * 0.1);
        }
        assert_close_real(err, bone.relative_length, 1.0);
    }
});

file_test!(synthetic_broken_cluster, flags = ALLOW_STRICT_ERROR, |scene, err, load_error| {
    let cube = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(cube.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[cube.unwrap().mesh.unwrap()];
    ufbxt_assert!(mesh.skin_deformers.len() == 1);
    let skin = &scene.skin_deformers[mesh.skin_deformers[0]];
    ufbxt_assert!(skin.clusters.len() == 2);
    let bone_name = |i: usize| scene.skin_clusters[skin.clusters[i]].bone_node.map(|b| scene.nodes[b].element.name.as_str());
    ufbxt_assert!(bone_name(0) == Some("joint3"));
    ufbxt_assert!(bone_name(1) == Some("joint1"));
});

#[test]
fn synthetic_broken_cluster_connect() {
    for path in FileIterator::new("synthetic_broken_cluster") {
        let opts = LoadOpts { connect_broken_elements: true, ..Default::default() };
        let scene = load_file(&path, &opts);
        ufbxt_assert!(scene.is_ok());
        let mut scene = scene.unwrap();

        let cube = find_node(&scene, "pCube1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(cube.is_some_and(|n| n.mesh.is_some()));
        let mesh = &scene.meshes[cube.unwrap().mesh.unwrap()];
        ufbxt_assert!(mesh.skin_deformers.len() == 1);
        let skin = &scene.skin_deformers[mesh.skin_deformers[0]];
        ufbxt_assert!(skin.clusters.len() == 3);
        let bone_name = |i: usize| scene.skin_clusters[skin.clusters[i]].bone_node.map(|b| scene.nodes[b].element.name.as_str());
        ufbxt_assert!(bone_name(0) == Some("joint3"));
        ufbxt_assert!(scene.skin_clusters[skin.clusters[1]].bone_node.is_none());
        ufbxt_assert!(bone_name(2) == Some("joint1"));

        // HACK: Patch the bone of `clusters[1]` to be valid for `check_scene()`...
        let (cluster0, cluster1) = (skin.clusters[0], skin.clusters[1]);
        scene.skin_clusters[cluster1].bone_node = scene.skin_clusters[cluster0].bone_node;

        check_scene(&scene);
    }
}

#[test]
fn synthetic_broken_cluster_safe() {
    for path in FileIterator::new("synthetic_broken_cluster") {
        let scene = load_file(&path, &LoadOpts::default());
        ufbxt_assert!(scene.is_ok());
        check_scene(&scene.unwrap());
    }
}

file_test!(max_transformed_skin, |scene, err, load_error| {
    check_frame(scene, err, false, "max_transformed_skin_5", None, 5.0 / 30.0);
    check_frame(scene, err, false, "max_transformed_skin_15", None, 15.0 / 30.0);
});

#[test]
fn max_transformed_skin_lefthanded() {
    let mut err = DiffError::default();

    for path in FileIterator::new("max_transformed_skin") {
        for (i, mirror_axis) in MIRROR_AXES.into_iter().enumerate() {
            ufbxt_hintf!("mirror_axis={i}");

            let opts = LoadOpts { target_axes: CoordinateAxes::LEFT_HANDED_Z_UP, handedness_conversion_axis: mirror_axis, ..Default::default() };

            let scene = load_file(&path, &opts);
            if let Err(e) = &scene {
                log_error(e);
            }
            ufbxt_assert!(scene.is_ok());
            let scene = scene.unwrap();

            check_scene(&scene);
            check_frame(&scene, &mut err, false, "max_transformed_skin_lefthanded_5", None, 5.0 / 30.0);
            check_frame(&scene, &mut err, false, "max_transformed_skin_lefthanded_15", None, 15.0 / 30.0);

            check_transform_consistency(&mut err, &scene, 30.0, 30, 0);
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn max_transformed_skin_lefthanded_adjust() {
    let mut err = DiffError::default();

    for path in FileIterator::new("max_transformed_skin") {
        for (i, mirror_axis) in MIRROR_AXES.into_iter().enumerate().skip(1) {
            ufbxt_hintf!("mirror_axis={i}");

            let opts = LoadOpts {
                space_conversion: SpaceConversion::AdjustTransforms,
                target_axes: CoordinateAxes::LEFT_HANDED_Z_UP,
                handedness_conversion_axis: mirror_axis,
                ..Default::default()
            };

            let scene = load_file(&path, &opts);
            if let Err(e) = &scene {
                log_error(e);
            }
            ufbxt_assert!(scene.is_ok());
            let scene = scene.unwrap();

            check_scene(&scene);
            check_frame(&scene, &mut err, false, "max_transformed_skin_lefthanded_5", None, 5.0 / 30.0);
            check_frame(&scene, &mut err, false, "max_transformed_skin_lefthanded_15", None, 15.0 / 30.0);

            check_transform_consistency(&mut err, &scene, 30.0, 30, 0);
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

file_test!(synthetic_bind_to_root, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    check_warning(scene, WarningType::BadElementConnectedToRoot, ElementType::SkinDeformer, Some(""), usize::MAX, None);
    // Some unknown exporter is exporting skin deformers being parented to root
    // This test exists to check that it is handled gracefully if quirks are enabled
});

file_test!(maya_mixed_inherit_mode, |scene, err, load_error| {
    check_frame(scene, err, false, "maya_mixed_inherit_mode_0", None, 0.0 / 24.0);
    check_frame(scene, err, false, "maya_mixed_inherit_mode_1", None, 1.0 / 24.0);
    check_frame(scene, err, false, "maya_mixed_inherit_mode_2", None, 2.0 / 24.0);
    check_frame(scene, err, false, "maya_mixed_inherit_mode_3", None, 3.0 / 24.0);
    check_frame(scene, err, false, "maya_mixed_inherit_mode_4", None, 4.0 / 24.0);
    check_frame(scene, err, false, "maya_mixed_inherit_mode_5", None, 5.0 / 24.0);
    check_frame(scene, err, false, "maya_mixed_inherit_mode_6", None, 6.0 / 24.0);
    check_frame(scene, err, false, "maya_mixed_inherit_mode_7", None, 7.0 / 24.0);
});

fn check_baked_frame(scene: &Scene, err: &mut DiffError, bake: &BakedAnim, time: f64) {
    let state = evaluate_scene(scene, &scene.anim, time, &EvaluateOpts::default());
    ufbxt_assert!(state.is_ok());
    let state = state.unwrap();

    for i in 0..scene.nodes.len() {
        let state_node = &state.nodes[i];

        let transform = evaluate_transform(scene, &scene.anim, i, time);
        assert_close_vec3(err, transform.translation, state_node.local_transform.translation);
        assert_close_quat(err, transform.rotation, state_node.local_transform.rotation);
        assert_close_vec3(err, transform.scale, state_node.local_transform.scale);

        let ref_matrix = state_node.node_to_world;
        let bake_matrix = evaluate_baked_matrix(scene, bake, i, time);

        assert_close_vec3(err, ref_matrix.col(0), bake_matrix.col(0));
        assert_close_vec3(err, ref_matrix.col(1), bake_matrix.col(1));
        assert_close_vec3(err, ref_matrix.col(2), bake_matrix.col(2));
        assert_close_vec3(err, ref_matrix.col(3), bake_matrix.col(3));
    }
}

file_test!(maya_mixed_inherit_mode_helper, file = "maya_mixed_inherit_mode", opts = scale_helper_opts(), |scene, err, load_error| {
    check_frame(scene, err, false, "maya_mixed_inherit_mode_0", None, 0.0 / 24.0);
    check_frame(scene, err, false, "maya_mixed_inherit_mode_1", None, 1.0 / 24.0);
    check_frame(scene, err, false, "maya_mixed_inherit_mode_2", None, 2.0 / 24.0);
    check_frame(scene, err, false, "maya_mixed_inherit_mode_3", None, 3.0 / 24.0);
    check_frame(scene, err, false, "maya_mixed_inherit_mode_4", None, 4.0 / 24.0);
    check_frame(scene, err, false, "maya_mixed_inherit_mode_5", None, 5.0 / 24.0);
    check_frame(scene, err, false, "maya_mixed_inherit_mode_6", None, 6.0 / 24.0);
    check_frame(scene, err, false, "maya_mixed_inherit_mode_7", None, 7.0 / 24.0);

    let bake = ufbxt_bake_anim(scene, &scene.anim, None);
    ufbxt_assert!(bake.is_ok());
    let bake = bake.unwrap();

    for frame in 0..=7 {
        check_baked_frame(scene, err, &bake, frame as f64 / 24.0);
    }
});

file_test!(motionbuilder_sausage_rrss, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    check_frame(scene, err, false, "motionbuilder_sausage_rrss", None, 0.0 / 24.0);
    check_frame(scene, err, false, "motionbuilder_sausage_rrss_12", None, 12.0 / 24.0);
    check_frame(scene, err, false, "motionbuilder_sausage_rrss_24", None, 24.0 / 24.0);
    check_frame(scene, err, false, "motionbuilder_sausage_rrss_36", None, 36.0 / 24.0);
    check_frame(scene, err, false, "motionbuilder_sausage_rrss_48", None, 48.0 / 24.0);
});

file_test!(
    motionbuilder_sausage_rrss_helper,
    file = "motionbuilder_sausage_rrss",
    opts = scale_helper_opts(),
    flags = DIFF_ALWAYS | ALLOW_INVALID_UNICODE | FUZZ_ALWAYS | FUZZ_OPTS,
    |scene, err, load_error| {
        check_frame(scene, err, false, "motionbuilder_sausage_rrss", None, 0.0 / 24.0);
        check_frame(scene, err, false, "motionbuilder_sausage_rrss_12", None, 12.0 / 24.0);
        check_frame(scene, err, false, "motionbuilder_sausage_rrss_24", None, 24.0 / 24.0);
        check_frame(scene, err, false, "motionbuilder_sausage_rrss_36", None, 36.0 / 24.0);
        check_frame(scene, err, false, "motionbuilder_sausage_rrss_48", None, 48.0 / 24.0);

        let bake = ufbxt_bake_anim(scene, &scene.anim, None);
        ufbxt_assert!(bake.is_ok());
        let bake = bake.unwrap();

        for frame in (0..=48).step_by(12) {
            check_baked_frame(scene, err, &bake, frame as f64 / 24.0);
        }
    }
);

fn check_pose(scene: &Scene, err: &mut DiffError, pose_name: &str, obj_name: &str) {
    let pose = find_element(scene, ElementType::Pose, pose_name).and_then(|id| scene.as_pose(id));
    ufbxt_assert!(pose.is_some());
    let pose = pose.unwrap();

    ufbxt_assert!(pose.bone_poses.len() == 5);

    let transform_overrides: Vec<TransformOverride> = pose
        .bone_poses
        .iter()
        .map(|bone| TransformOverride { node_id: scene.nodes[bone.bone_node].element.typed_id, transform: matrix_to_transform(&bone.bone_to_parent) })
        .collect();

    let opts = AnimOpts { transform_overrides, ..Default::default() };
    let anim = create_anim(scene, &opts);
    ufbxt_assert!(anim.is_ok());
    let anim = anim.unwrap();

    let eval_opts = EvaluateOpts { evaluate_skinning: true, ..Default::default() };

    let state = evaluate_scene(scene, &anim, 0.0, &eval_opts);
    ufbxt_assert!(state.is_ok());

    let obj_file = load_obj_file(obj_name, None);

    diff_to_obj(&state.unwrap(), &obj_file, err, 0);
}

file_test!(maya_poses, |scene, err, load_error| {
    check_pose(scene, err, "skinCluster1", "maya_poses_bind1");
    check_pose(scene, err, "skinCluster2", "maya_poses_bind2");
});

fn lefthanded_y_up_z_flip_opts() -> LoadOpts {
    LoadOpts { target_axes: CoordinateAxes::LEFT_HANDED_Y_UP, handedness_conversion_axis: MirrorAxis::Z, ..Default::default() }
}

file_test!(maya_poses_lefthanded, file = "maya_poses", opts = lefthanded_y_up_z_flip_opts(), flags = FUZZ_ALWAYS | FUZZ_OPTS, |scene, err, load_error| {
    check_pose(scene, err, "skinCluster1", "maya_poses_bind1_lefthanded");
    check_pose(scene, err, "skinCluster2", "maya_poses_bind2_lefthanded");
});

file_test!(blender_331_static_blend_shape, |scene, err, load_error| {
    ufbxt_assert!(scene.anim_stacks.is_empty());
});

file_test!(maya_transformed_skin, |scene, err, load_error| {
    check_frame(scene, err, true, "maya_transformed_skin_0", None, 0.0 / 24.0);
    check_frame(scene, err, true, "maya_transformed_skin_4", None, 4.0 / 24.0);
    check_frame(scene, err, true, "maya_transformed_skin_8", None, 8.0 / 24.0);
});

file_test!(maya_instanced_skin, |scene, err, load_error| {
    // TODO: This is broken on 6100 files currently, as the skin is attached only to pCube1 but should be to pCube2 too,
    // probably somehow through `NodeAttributeName: "Geometry::pCube1_ncl1_1"`
    if scene.metadata.version >= 7000 {
        check_frame(scene, err, true, "maya_instanced_skin_0", None, 0.0 / 24.0);
        check_frame(scene, err, true, "maya_instanced_skin_4", None, 4.0 / 24.0);
        check_frame(scene, err, true, "maya_instanced_skin_8", None, 8.0 / 24.0);
    }
});

struct ShapeWeightRef {
    position: Vec3,
    weight: Real,
}

const SHAPE_WEIGHT_REF: [ShapeWeightRef; 4] = [
    ShapeWeightRef { position: Vec3 { x: -1.0, y: -1.0, z: 1.0 }, weight: 0.0 },
    ShapeWeightRef { position: Vec3 { x: 1.0, y: -1.0, z: 1.0 }, weight: 0.25 },
    ShapeWeightRef { position: Vec3 { x: 1.0, y: 1.0, z: 1.0 }, weight: 0.75 },
    ShapeWeightRef { position: Vec3 { x: -1.0, y: 1.0, z: 1.0 }, weight: 1.0 },
];

file_test!(blender_279_shape_weights, |scene, err, load_error| {
    let node = find_node(scene, "Cube").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
    ufbxt_assert!(mesh.blend_deformers.len() == 1);
    let deformer = &scene.blend_deformers[mesh.blend_deformers[0]];
    ufbxt_assert!(deformer.channels.len() == 1);
    let channel = &scene.blend_channels[deformer.channels[0]];
    ufbxt_assert!(channel.keyframes.len() == 1);
    let keyframe = &channel.keyframes[0];
    let shape = &scene.blend_shapes[keyframe.shape];

    ufbxt_assert!(channel.element.name == "Key");

    if scene.metadata.version >= 7000 {
        ufbxt_assert!(shape.num_offsets == 4);
        ufbxt_assert!(shape.offset_weights.len() == 4);

        for off_i in 0..shape.num_offsets {
            assert_vec3_equal(shape.position_offsets[off_i], 0.0, 0.0, 2.0);

            let weight = shape.offset_weights[off_i];
            let index = shape.offset_vertices[off_i];
            ufbxt_assert!((index as usize) < mesh.num_vertices);
            let vertex = mesh.vertices[index as usize];

            let mut num_found = 0;
            for r in &SHAPE_WEIGHT_REF {
                if r.position.x == vertex.x && r.position.y == vertex.y && r.position.z == vertex.z {
                    ufbxt_assert!(weight == r.weight);
                    num_found += 1;
                }
            }
            ufbxt_assert!(num_found == 1);
        }
    } else {
        ufbxt_assert!(shape.offset_weights.is_empty());
    }
});

file_test!(blender_279_shape_weights_dom, file = "blender_279_shape_weights", opts = retain_dom_opts(), |scene, err, load_error| {
    let node = find_node(scene, "Cube").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
    let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];
    ufbxt_assert!(mesh.blend_deformers.len() == 1);
    let deformer = &scene.blend_deformers[mesh.blend_deformers[0]];
    ufbxt_assert!(deformer.channels.len() == 1);
    let channel = &scene.blend_channels[deformer.channels[0]];
    ufbxt_assert!(channel.keyframes.len() == 1);
    let keyframe = &channel.keyframes[0];
    let shape = &scene.blend_shapes[keyframe.shape];

    ufbxt_assert!(channel.element.name == "Key");

    if scene.metadata.version >= 7000 {
        ufbxt_assert!(shape.num_offsets == 4);
        ufbxt_assert!(shape.offset_weights.len() == 4);

        let dom_channel = element_dom_node(scene, &channel.element);
        ufbxt_assert!(dom_channel.is_some());

        let dom_full_weights = dom_channel.unwrap().find("FullWeights");
        ufbxt_assert!(dom_full_weights.is_some());

        let full_weights = dom_full_weights.unwrap().as_real_list();
        ufbxt_assert!(full_weights.len() == 4);

        for off_i in 0..shape.num_offsets {
            assert_vec3_equal(shape.position_offsets[off_i], 0.0, 0.0, 2.0);

            let weight = shape.offset_weights[off_i];
            let dom_weight = full_weights[off_i];
            let index = shape.offset_vertices[off_i];
            ufbxt_assert!((index as usize) < mesh.num_vertices);
            let vertex = mesh.vertices[index as usize];

            ufbxt_assert!(weight == dom_weight / 100.0);

            let mut num_found = 0;
            for r in &SHAPE_WEIGHT_REF {
                if r.position.x == vertex.x && r.position.y == vertex.y && r.position.z == vertex.z {
                    ufbxt_assert!(weight == r.weight);
                    num_found += 1;
                }
            }
            ufbxt_assert!(num_found == 1);
        }
    } else {
        ufbxt_assert!(shape.offset_weights.is_empty());
    }
});

file_test!(blender440_shape_weight_anim, |scene, err, load_error| {
    check_frame(scene, err, true, "blender440_shape_weight_anim_5", None, 5.0 / 24.0);
    check_frame(scene, err, true, "blender440_shape_weight_anim_15", None, 15.0 / 24.0);
    check_frame(scene, err, true, "blender440_shape_weight_anim_25", None, 25.0 / 24.0);
});
