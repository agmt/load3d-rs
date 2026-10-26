mod ufbxt;
use ufbxt::*;

const MIRROR_AXES: [MirrorAxis; 4] = [MirrorAxis::None, MirrorAxis::X, MirrorAxis::Y, MirrorAxis::Z];
const PIVOT_HANDLINGS: [PivotHandling; 3] = [PivotHandling::Retain, PivotHandling::AdjustToPivot, PivotHandling::AdjustToRotationPivot];
const GEOMETRY_TRANSFORM_HANDLINGS: [GeometryTransformHandling; 4] = [
    GeometryTransformHandling::Preserve,
    GeometryTransformHandling::HelperNodes,
    GeometryTransformHandling::ModifyGeometry,
    GeometryTransformHandling::ModifyGeometryNoFallback,
];

file_test!(maya_pivots, |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());

    let origin_ref = Vec3::new(0.7211236250, 1.8317762500, -0.6038020000);
    assert_close_vec3(err, node.unwrap().local_transform.translation, origin_ref);
});

fn check_rotation_order(scene: &Scene, name: &str, order: RotationOrder) {
    let node = find_node(scene, name).map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let prop = node.unwrap().element.props.find("RotationOrder");
    ufbxt_assert!(prop.is_some());
    ufbxt_assert!(prop.unwrap().value_int == order as i64);
}

file_test!(maya_rotation_order, |scene, err, load_error| {
    check_rotation_order(scene, "XYZ", RotationOrder::Xyz);
    check_rotation_order(scene, "XZY", RotationOrder::Xzy);
    check_rotation_order(scene, "YZX", RotationOrder::Yzx);
    check_rotation_order(scene, "YXZ", RotationOrder::Yxz);
    check_rotation_order(scene, "ZXY", RotationOrder::Zxy);
    check_rotation_order(scene, "ZYX", RotationOrder::Zyx);
});

file_test!(maya_post_rotate_order, |scene, err, load_error| {
    check_rotation_order(scene, "pCube1", RotationOrder::Xyz);
    check_rotation_order(scene, "pCube2", RotationOrder::Zyx);
});

file_test!(synthetic_pre_post_rotate, |scene, err, load_error| {
    check_rotation_order(scene, "pCube1", RotationOrder::Xyz);
    check_rotation_order(scene, "pCube2", RotationOrder::Zyx);
});

file_test!(maya_parented_cubes, |scene, err, load_error| {});

file_test!(synthetic_geometric_squish, flags = OPT_HANDLING_IGNORE_NORMALS_IN_DIFF, |scene, err, load_error| {
    let node = find_node(scene, "pSphere1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    assert_close_real(err, node.unwrap().geometry_transform.scale.y, 0.01);
});

file_test!(synthetic_geometric_transform, flags = OPT_HANDLING_IGNORE_NORMALS_IN_DIFF, |scene, err, load_error| {
    let node = find_node(scene, "Parent").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let node = node.unwrap();
    let euler = quat_to_euler(node.geometry_transform.rotation, RotationOrder::Xyz);
    assert_close_real(err, node.geometry_transform.translation.x, -0.5);
    assert_close_real(err, node.geometry_transform.translation.y, -1.0);
    assert_close_real(err, node.geometry_transform.translation.z, -1.5);
    assert_close_real(err, euler.x, 20.0);
    assert_close_real(err, euler.y, 40.0);
    assert_close_real(err, euler.z, 60.0);
    assert_close_real(err, node.geometry_transform.scale.x, 2.0);
    assert_close_real(err, node.geometry_transform.scale.y, 3.0);
    assert_close_real(err, node.geometry_transform.scale.z, 4.0);
});

file_test!(maya_cube_hidden, |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    ufbxt_assert!(!node.unwrap().visible);
});

#[test]
fn root_transform() {
    let mut err = DiffError::default();

    let mut num_found = 0;
    for path in FileIterator::new("maya_cube") {
        num_found += 1;

        let euler = Vec3::new(90.0, 0.0, 0.0);

        let opts = LoadOpts {
            use_root_transform: true,
            root_transform: Transform {
                translation: Vec3::new(-1.0, -2.0, -3.0),
                rotation: euler_to_quat(euler, RotationOrder::Xyz),
                scale: Vec3::new(2.0, 3.0, 4.0),
            },
            ..Default::default()
        };

        let scene = load_file(&path, &opts);
        ufbxt_assert!(scene.is_ok());
        let scene = scene.unwrap();

        check_scene(&scene);

        let root = &scene.nodes[scene.root_node];
        assert_close_vec3(&mut err, root.local_transform.translation, opts.root_transform.translation);
        assert_close_quat(&mut err, root.local_transform.rotation, opts.root_transform.rotation);
        assert_close_vec3(&mut err, root.local_transform.scale, opts.root_transform.scale);

        let eval = evaluate_transform(&scene, &scene.anim, scene.root_node, 0.1);
        assert_close_vec3(&mut err, eval.translation, opts.root_transform.translation);
        assert_close_quat(&mut err, eval.rotation, opts.root_transform.rotation);
        assert_close_vec3(&mut err, eval.scale, opts.root_transform.scale);

        let state = evaluate_scene(&scene, &scene.anim, 0.1, &EvaluateOpts::default());
        ufbxt_assert!(state.is_ok());
        let state = state.unwrap();

        let state_root = &state.nodes[state.root_node];
        assert_close_vec3(&mut err, state_root.local_transform.translation, opts.root_transform.translation);
        assert_close_quat(&mut err, state_root.local_transform.rotation, opts.root_transform.rotation);
        assert_close_vec3(&mut err, state_root.local_transform.scale, opts.root_transform.scale);
    }

    ufbxt_assert!(num_found >= 8);

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn blender_axes() {
    const AXIS_NAMES: [&str; 6] = ["px", "nx", "py", "ny", "pz", "nz"];

    for fwd_ix in 0..6usize {
        for up_ix in 0..6usize {
            if (fwd_ix >> 1) == (up_ix >> 1) {
                continue;
            }

            let axis_fwd = CoordinateAxis::from_index(fwd_ix as i64);
            let axis_up = CoordinateAxis::from_index(up_ix as i64);

            let name = format!("blender_axes/axes_{}{}", AXIS_NAMES[fwd_ix], AXIS_NAMES[up_ix]);

            for path in FileIterator::new(&name) {
                let mut err = DiffError::default();

                {
                    let scene = load_file(&path, &LoadOpts::default());
                    ufbxt_assert!(scene.is_ok());
                    let scene = scene.unwrap();
                    check_scene(&scene);

                    let axis_front = CoordinateAxis::from_index(axis_fwd as i64 ^ 1);

                    ufbxt_assert!(scene.settings.axes.front == axis_front);
                    ufbxt_assert!(scene.settings.axes.up == axis_up);
                }

                for mode in 0..4 {
                    ufbxt_hintf!("mode = {mode}");
                    let mut opts = LoadOpts::default();

                    let transform_root = (mode % 2) != 0;
                    let use_adjust = (mode / 2) != 0;

                    opts.target_axes = CoordinateAxes::RIGHT_HANDED_Z_UP;
                    opts.target_unit_meters = 1.0;

                    if transform_root {
                        opts.use_root_transform = true;
                        opts.root_transform = Transform::IDENTITY;
                        opts.root_transform.translation.z = 1.0;
                    }

                    if use_adjust {
                        opts.space_conversion = SpaceConversion::AdjustTransforms;
                    } else {
                        opts.space_conversion = SpaceConversion::TransformRoot;
                    }

                    let scene = load_file(&path, &opts);
                    ufbxt_assert!(scene.is_ok());
                    let scene = scene.unwrap();
                    check_scene(&scene);

                    let plane = find_node(&scene, "Plane").map(|n| &scene.nodes[n]);
                    ufbxt_assert!(plane.is_some_and(|n| n.mesh.is_some()));
                    let plane = plane.unwrap();
                    let mesh = &scene.meshes[plane.mesh.unwrap()];

                    ufbxt_assert!(mesh.num_faces == 1);
                    let face = mesh.faces[0];
                    ufbxt_assert!(face.num_indices == 3);

                    if use_adjust && !transform_root {
                        let root = &scene.nodes[scene.root_node];

                        let identity_scale = Vec3::new(1.0, 1.0, 1.0);
                        assert_close_quat(&mut err, root.local_transform.rotation, Quat::IDENTITY);
                        assert_close_vec3(&mut err, root.local_transform.scale, identity_scale);
                    }

                    for i in 0..face.num_indices {
                        let pos = mesh.vertex_position.get((face.index_begin + i) as usize);
                        let uv = mesh.vertex_uv.get((face.index_begin + i) as usize);

                        let pos = transform_position(&plane.geometry_to_world, pos);

                        let mut ref_pos = if uv.x < 0.5 && uv.y < 0.5 {
                            Vec3::new(1.0, 1.0, 3.0)
                        } else if uv.x > 0.5 && uv.y < 0.5 {
                            Vec3::new(2.0, 2.0, 3.0)
                        } else if uv.x < 0.5 && uv.y > 0.5 {
                            Vec3::new(1.0, 2.0, 4.0)
                        } else {
                            ufbxt_assert!(false);
                            unreachable!()
                        };

                        if transform_root {
                            ref_pos.z += 1.0;
                        }

                        assert_close_vec3(&mut err, pos, ref_pos);
                    }
                }

                ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
            }
        }
    }
}

fn scale_to_cm_adjust_opts() -> LoadOpts {
    LoadOpts { target_unit_meters: 0.01f32 as Real, space_conversion: SpaceConversion::AdjustTransforms, ..Default::default() }
}
fn scale_to_cm_helper_opts() -> LoadOpts {
    LoadOpts { target_unit_meters: 0.01f32 as Real, inherit_mode_handling: InheritModeHandling::HelperNodes, ..Default::default() }
}
fn scale_to_cm_adjust_helper_opts() -> LoadOpts {
    LoadOpts {
        target_unit_meters: 0.01f32 as Real,
        space_conversion: SpaceConversion::AdjustTransforms,
        inherit_mode_handling: InheritModeHandling::HelperNodes,
        ..Default::default()
    }
}
fn scale_to_cm_compensate_opts() -> LoadOpts {
    LoadOpts { target_unit_meters: 0.01f32 as Real, inherit_mode_handling: InheritModeHandling::Compensate, ..Default::default() }
}
fn scale_to_cm_adjust_compensate_opts() -> LoadOpts {
    LoadOpts {
        target_unit_meters: 0.01f32 as Real,
        space_conversion: SpaceConversion::AdjustTransforms,
        inherit_mode_handling: InheritModeHandling::Compensate,
        ..Default::default()
    }
}

file_test!(maya_scale_no_inherit, opts = scale_to_cm_opts(), |scene, err, load_error| {
    {
        let node = find_node(scene, "joint1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        let world_transform = matrix_to_transform(&node.node_to_world);
        assert_close_vec3_xyz(err, node.local_transform.scale, 0.03, 0.03, 0.03);
        assert_close_vec3_xyz(err, world_transform.scale, 3.0, 3.0, 3.0);
        assert_close_vec3_xyz(err, world_transform.translation, 0.0, 0.0, 0.0);
    }

    {
        let node = find_node(scene, "joint2").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::IgnoreParentScale);
        let world_transform = matrix_to_transform(&node.node_to_world);
        assert_close_vec3_xyz(err, node.local_transform.scale, 1.0, 1.0, 1.0);
        assert_close_vec3_xyz(err, world_transform.scale, 100.0, 100.0, 100.0);
        assert_close_vec3_xyz(err, world_transform.translation, 0.0, 6.0, 0.0);
    }

    {
        let node = find_node(scene, "joint3").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::IgnoreParentScale);
        let world_transform = matrix_to_transform(&node.node_to_world);
        assert_close_vec3_xyz(err, node.local_transform.scale, 0.01, 0.01, 0.01);
        assert_close_vec3_xyz(err, world_transform.scale, 1.0, 1.0, 1.0);
        assert_close_vec3_xyz(err, world_transform.translation, 200.0, 206.0, 0.0);
    }

    {
        let node = find_node(scene, "joint4").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::IgnoreParentScale);
        let world_transform = matrix_to_transform(&node.node_to_world);
        assert_close_vec3_xyz(err, node.local_transform.scale, 0.015, 0.025, 0.035);
        assert_close_vec3_xyz(err, world_transform.scale, 1.5, 2.5, 3.5);
        assert_close_vec3_xyz(err, world_transform.translation, 200.0, 208.0, 0.0);
    }

    {
        let node = find_node(scene, "joint3").unwrap();

        {
            let transform = evaluate_transform(scene, &scene.anim, node, 1.0);
            assert_close_real(err, transform.scale.x * 100.0, 0.3);
            assert_close_real(err, transform.scale.y * 100.0, 0.6);
            assert_close_real(err, transform.scale.z * 100.0, 0.9);
        }

        {
            let transform = evaluate_transform(scene, &scene.anim, node, 0.5);
            assert_close_real(err, transform.scale.x * 100.0, 0.67281);
            assert_close_real(err, transform.scale.y * 100.0, 0.81304);
            assert_close_real(err, transform.scale.z * 100.0, 0.95326);
        }
    }
});

file_test!(maya_scale_no_inherit_adjust, file = "maya_scale_no_inherit", opts = scale_to_cm_adjust_opts(), |scene, err, load_error| {
    {
        let node = find_node(scene, "joint1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::IgnoreParentScale);
        let world_transform = matrix_to_transform(&node.node_to_world);
        assert_close_vec3_xyz(err, node.local_transform.scale, 3.0, 3.0, 3.0);
        assert_close_vec3_xyz(err, world_transform.scale, 3.0, 3.0, 3.0);
        assert_close_vec3_xyz(err, world_transform.translation, 0.0, 0.0, 0.0);
    }

    {
        let node = find_node(scene, "joint2").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::IgnoreParentScale);
        let world_transform = matrix_to_transform(&node.node_to_world);
        assert_close_vec3_xyz(err, node.local_transform.scale, 100.0, 100.0, 100.0);
        assert_close_vec3_xyz(err, world_transform.scale, 100.0, 100.0, 100.0);
        assert_close_vec3_xyz(err, world_transform.translation, 0.0, 6.0, 0.0);
    }

    {
        let node = find_node(scene, "joint3").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::IgnoreParentScale);
        let world_transform = matrix_to_transform(&node.node_to_world);
        assert_close_vec3_xyz(err, node.local_transform.scale, 1.0, 1.0, 1.0);
        assert_close_vec3_xyz(err, world_transform.scale, 1.0, 1.0, 1.0);
        assert_close_vec3_xyz(err, world_transform.translation, 200.0, 206.0, 0.0);
    }

    {
        let node = find_node(scene, "joint4").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::IgnoreParentScale);
        let world_transform = matrix_to_transform(&node.node_to_world);
        assert_close_vec3_xyz(err, node.local_transform.scale, 1.5, 2.5, 3.5);
        assert_close_vec3_xyz(err, world_transform.scale, 1.5, 2.5, 3.5);
        assert_close_vec3_xyz(err, world_transform.translation, 200.0, 208.0, 0.0);
    }

    {
        let node = find_node(scene, "joint3").unwrap();

        {
            let transform = evaluate_transform(scene, &scene.anim, node, 1.0);
            assert_close_vec3_xyz(err, transform.scale, 0.3, 0.6, 0.9);
        }

        {
            let transform = evaluate_transform(scene, &scene.anim, node, 0.5);
            assert_close_vec3_xyz(err, transform.scale, 0.67281, 0.81304, 0.95326);
        }
    }
});

file_test!(maya_scale_no_inherit_helper, file = "maya_scale_no_inherit", opts = scale_to_cm_helper_opts(), |scene, err, load_error| {
    {
        let node = find_node(scene, "joint1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        let helper = node.scale_helper.map(|n| &scene.nodes[n]);
        ufbxt_assert!(helper.is_some());
        let helper = helper.unwrap();
        ufbxt_assert!(helper.inherit_mode == InheritMode::Normal);
        let world_transform = matrix_to_transform(&helper.node_to_world);

        assert_close_vec3_xyz(err, node.local_transform.scale, 1.0, 1.0, 1.0);
        assert_close_vec3_xyz(err, helper.local_transform.scale, 0.03, 0.03, 0.03);
        assert_close_vec3_xyz(err, world_transform.scale, 3.0, 3.0, 3.0);
        assert_close_vec3_xyz(err, world_transform.translation, 0.0, 0.0, 0.0);
    }

    {
        let node = find_node(scene, "joint2").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        ufbxt_assert!(node.scale_helper.is_none());
        let world_transform = matrix_to_transform(&node.node_to_world);

        assert_close_vec3_xyz(err, node.local_transform.scale, 1.0, 1.0, 1.0);
        assert_close_vec3_xyz(err, world_transform.scale, 100.0, 100.0, 100.0);
        assert_close_vec3_xyz(err, world_transform.translation, 0.0, 6.0, 0.0);
    }

    {
        let node = find_node(scene, "joint3").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        let helper = node.scale_helper.map(|n| &scene.nodes[n]);
        ufbxt_assert!(helper.is_some());
        let helper = helper.unwrap();
        ufbxt_assert!(helper.inherit_mode == InheritMode::Normal);
        let world_transform = matrix_to_transform(&helper.node_to_world);

        assert_close_vec3_xyz(err, node.local_transform.scale, 1.0, 1.0, 1.0);
        assert_close_vec3_xyz(err, helper.local_transform.scale, 0.01, 0.01, 0.01);
        assert_close_vec3_xyz(err, world_transform.scale, 1.0, 1.0, 1.0);
        assert_close_vec3_xyz(err, world_transform.translation, 200.0, 206.0, 0.0);
    }

    {
        let node = find_node(scene, "joint4").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        ufbxt_assert!(node.scale_helper.is_none());
        let world_transform = matrix_to_transform(&node.node_to_world);

        assert_close_vec3_xyz(err, node.local_transform.scale, 0.015, 0.025, 0.035);
        assert_close_vec3_xyz(err, world_transform.scale, 1.5, 2.5, 3.5);
        assert_close_vec3_xyz(err, world_transform.translation, 200.0, 208.0, 0.0);
    }

    {
        let node = &scene.nodes[find_node(scene, "joint3").unwrap()];
        let helper = node.scale_helper;
        ufbxt_assert!(helper.is_some());
        let helper = helper.unwrap();

        {
            let transform = evaluate_transform(scene, &scene.anim, helper, 1.0);
            assert_close_real(err, transform.scale.x * 100.0, 0.3);
            assert_close_real(err, transform.scale.y * 100.0, 0.6);
            assert_close_real(err, transform.scale.z * 100.0, 0.9);
        }

        {
            let transform = evaluate_transform(scene, &scene.anim, helper, 0.5);
            assert_close_real(err, transform.scale.x * 100.0, 0.67281);
            assert_close_real(err, transform.scale.y * 100.0, 0.81304);
            assert_close_real(err, transform.scale.z * 100.0, 0.95326);
        }
    }
});

file_test!(maya_scale_no_inherit_adjust_helper, file = "maya_scale_no_inherit", opts = scale_to_cm_adjust_helper_opts(), |scene, err, load_error| {
    {
        let node = find_node(scene, "joint1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        let helper = node.scale_helper.map(|n| &scene.nodes[n]);
        ufbxt_assert!(helper.is_some());
        let helper = helper.unwrap();
        ufbxt_assert!(helper.inherit_mode == InheritMode::Normal);
        let world_transform = matrix_to_transform(&helper.node_to_world);

        assert_close_vec3_xyz(err, node.local_transform.scale, 100.0, 100.0, 100.0);
        assert_close_vec3_xyz(err, helper.local_transform.scale, 0.03, 0.03, 0.03);
        assert_close_vec3_xyz(err, world_transform.scale, 3.0, 3.0, 3.0);
        assert_close_vec3_xyz(err, world_transform.translation, 0.0, 0.0, 0.0);
    }

    {
        let node = find_node(scene, "joint2").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        ufbxt_assert!(node.scale_helper.is_none());
        let world_transform = matrix_to_transform(&node.node_to_world);

        assert_close_vec3_xyz(err, node.local_transform.scale, 1.0, 1.0, 1.0);
        assert_close_vec3_xyz(err, world_transform.scale, 100.0, 100.0, 100.0);
        assert_close_vec3_xyz(err, world_transform.translation, 0.0, 6.0, 0.0);
    }

    {
        let node = find_node(scene, "joint3").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        let helper = node.scale_helper.map(|n| &scene.nodes[n]);
        ufbxt_assert!(helper.is_some());
        let helper = helper.unwrap();
        ufbxt_assert!(helper.inherit_mode == InheritMode::Normal);
        let world_transform = matrix_to_transform(&helper.node_to_world);

        assert_close_vec3_xyz(err, node.local_transform.scale, 1.0, 1.0, 1.0);
        assert_close_vec3_xyz(err, helper.local_transform.scale, 0.01, 0.01, 0.01);
        assert_close_vec3_xyz(err, world_transform.scale, 1.0, 1.0, 1.0);
        assert_close_vec3_xyz(err, world_transform.translation, 200.0, 206.0, 0.0);
    }

    {
        let node = find_node(scene, "joint4").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        ufbxt_assert!(node.scale_helper.is_none());
        let world_transform = matrix_to_transform(&node.node_to_world);

        assert_close_vec3_xyz(err, node.local_transform.scale, 0.015, 0.025, 0.035);
        assert_close_vec3_xyz(err, world_transform.scale, 1.5, 2.5, 3.5);
        assert_close_vec3_xyz(err, world_transform.translation, 200.0, 208.0, 0.0);
    }

    {
        let node = &scene.nodes[find_node(scene, "joint3").unwrap()];
        let helper = node.scale_helper;
        ufbxt_assert!(helper.is_some());
        let helper = helper.unwrap();

        {
            let transform = evaluate_transform(scene, &scene.anim, helper, 1.0);
            assert_close_real(err, transform.scale.x * 100.0, 0.3);
            assert_close_real(err, transform.scale.y * 100.0, 0.6);
            assert_close_real(err, transform.scale.z * 100.0, 0.9);
        }

        {
            let transform = evaluate_transform(scene, &scene.anim, helper, 0.5);
            assert_close_real(err, transform.scale.x * 100.0, 0.67281);
            assert_close_real(err, transform.scale.y * 100.0, 0.81304);
            assert_close_real(err, transform.scale.z * 100.0, 0.95326);
        }
    }
});

file_test!(maya_scale_no_inherit_compensate, file = "maya_scale_no_inherit", opts = scale_to_cm_compensate_opts(), |scene, err, load_error| {
    {
        let node = find_node(scene, "joint1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        ufbxt_assert!(node.scale_helper.is_none());
        let world_transform = matrix_to_transform(&node.node_to_world);

        assert_close_vec3_xyz(err, node.local_transform.scale, 0.03, 0.03, 0.03);
        assert_close_vec3_xyz(err, world_transform.scale, 3.0, 3.0, 3.0);
        assert_close_vec3_xyz(err, world_transform.translation, 0.0, 0.0, 0.0);
    }

    {
        let node = find_node(scene, "joint2").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        ufbxt_assert!(node.scale_helper.is_none());
        let world_transform = matrix_to_transform(&node.node_to_world);

        assert_close_vec3_xyz(err, node.local_transform.scale, 33.33333, 33.33333, 33.33333);
        assert_close_vec3_xyz(err, world_transform.scale, 100.0, 100.0, 100.0);
        assert_close_vec3_xyz(err, world_transform.translation, 0.0, 6.0, 0.0);
    }

    {
        let node = find_node(scene, "joint3").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        let helper = node.scale_helper.map(|n| &scene.nodes[n]);
        ufbxt_assert!(helper.is_some());
        let helper = helper.unwrap();
        ufbxt_assert!(helper.inherit_mode == InheritMode::Normal);
        let world_transform = matrix_to_transform(&helper.node_to_world);

        assert_close_vec3_xyz(err, node.local_transform.scale, 1.0, 1.0, 1.0);
        assert_close_vec3_xyz(err, helper.local_transform.scale, 0.01, 0.01, 0.01);
        assert_close_vec3_xyz(err, world_transform.scale, 1.0, 1.0, 1.0);
        assert_close_vec3_xyz(err, world_transform.translation, 200.0, 206.0, 0.0);
    }

    {
        let node = find_node(scene, "joint4").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        ufbxt_assert!(node.scale_helper.is_none());
        let world_transform = matrix_to_transform(&node.node_to_world);

        assert_close_vec3_xyz(err, node.local_transform.scale, 0.015, 0.025, 0.035);
        assert_close_vec3_xyz(err, world_transform.scale, 1.5, 2.5, 3.5);
        assert_close_vec3_xyz(err, world_transform.translation, 200.0, 208.0, 0.0);
    }

    {
        let node = &scene.nodes[find_node(scene, "joint3").unwrap()];
        let helper = node.scale_helper;
        ufbxt_assert!(helper.is_some());
        let helper = helper.unwrap();

        {
            let transform = evaluate_transform(scene, &scene.anim, helper, 1.0);
            assert_close_real(err, transform.scale.x * 100.0, 0.3);
            assert_close_real(err, transform.scale.y * 100.0, 0.6);
            assert_close_real(err, transform.scale.z * 100.0, 0.9);
        }

        {
            let transform = evaluate_transform(scene, &scene.anim, helper, 0.5);
            assert_close_real(err, transform.scale.x * 100.0, 0.67281);
            assert_close_real(err, transform.scale.y * 100.0, 0.81304);
            assert_close_real(err, transform.scale.z * 100.0, 0.95326);
        }
    }
});

file_test!(maya_scale_no_inherit_adjust_compensate, file = "maya_scale_no_inherit", opts = scale_to_cm_adjust_compensate_opts(), |scene, err, load_error| {
    {
        let node = find_node(scene, "joint1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        ufbxt_assert!(node.scale_helper.is_none());
        let world_transform = matrix_to_transform(&node.node_to_world);

        assert_close_vec3_xyz(err, node.local_transform.scale, 3.0, 3.0, 3.0);
        assert_close_vec3_xyz(err, world_transform.scale, 3.0, 3.0, 3.0);
        assert_close_vec3_xyz(err, world_transform.translation, 0.0, 0.0, 0.0);
    }

    {
        let node = find_node(scene, "joint2").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        ufbxt_assert!(node.scale_helper.is_none());
        let world_transform = matrix_to_transform(&node.node_to_world);

        assert_close_vec3_xyz(err, node.local_transform.scale, 33.33333, 33.33333, 33.33333);
        assert_close_vec3_xyz(err, world_transform.scale, 100.0, 100.0, 100.0);
        assert_close_vec3_xyz(err, world_transform.translation, 0.0, 6.0, 0.0);
    }

    {
        let node = find_node(scene, "joint3").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        let helper = node.scale_helper.map(|n| &scene.nodes[n]);
        ufbxt_assert!(helper.is_some());
        let helper = helper.unwrap();
        ufbxt_assert!(helper.inherit_mode == InheritMode::Normal);
        let world_transform = matrix_to_transform(&helper.node_to_world);

        assert_close_vec3_xyz(err, node.local_transform.scale, 1.0, 1.0, 1.0);
        assert_close_vec3_xyz(err, helper.local_transform.scale, 0.01, 0.01, 0.01);
        assert_close_vec3_xyz(err, world_transform.scale, 1.0, 1.0, 1.0);
        assert_close_vec3_xyz(err, world_transform.translation, 200.0, 206.0, 0.0);
    }

    {
        let node = find_node(scene, "joint4").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.inherit_mode == InheritMode::Normal);
        ufbxt_assert!(node.scale_helper.is_none());
        let world_transform = matrix_to_transform(&node.node_to_world);

        assert_close_vec3_xyz(err, node.local_transform.scale, 0.015, 0.025, 0.035);
        assert_close_vec3_xyz(err, world_transform.scale, 1.5, 2.5, 3.5);
        assert_close_vec3_xyz(err, world_transform.translation, 200.0, 208.0, 0.0);
    }

    {
        let node = &scene.nodes[find_node(scene, "joint3").unwrap()];
        let helper = node.scale_helper;
        ufbxt_assert!(helper.is_some());
        let helper = helper.unwrap();

        {
            let transform = evaluate_transform(scene, &scene.anim, helper, 1.0);
            assert_close_real(err, transform.scale.x * 100.0, 0.3);
            assert_close_real(err, transform.scale.y * 100.0, 0.6);
            assert_close_real(err, transform.scale.z * 100.0, 0.9);
        }

        {
            let transform = evaluate_transform(scene, &scene.anim, helper, 0.5);
            assert_close_real(err, transform.scale.x * 100.0, 0.67281);
            assert_close_real(err, transform.scale.y * 100.0, 0.81304);
            assert_close_real(err, transform.scale.z * 100.0, 0.95326);
        }
    }
});

file_test!(synthetic_node_dag, |scene, err, load_error| {
    let root = scene.root_node;
    let a = find_node(scene, "A");
    let b = find_node(scene, "B");
    let c = find_node(scene, "C");
    let d = find_node(scene, "D");

    ufbxt_assert!(a.is_some() && b.is_some() && c.is_some() && d.is_some());
    let (a, b, c, d) = (a.unwrap(), b.unwrap(), c.unwrap(), d.unwrap());
    ufbxt_assert!(scene.nodes[root].children.len() == 1);
    ufbxt_assert!(scene.nodes[root].children[0] == a);

    ufbxt_assert!(scene.nodes[a].parent == Some(root));
    ufbxt_assert!(scene.nodes[a].children.len() == 1);
    ufbxt_assert!(scene.nodes[a].children[0] == b);

    ufbxt_assert!(scene.nodes[b].parent == Some(a));
    ufbxt_assert!(scene.nodes[b].children.len() == 1);
    ufbxt_assert!(scene.nodes[b].children[0] == c);

    ufbxt_assert!(scene.nodes[c].parent == Some(b));
    ufbxt_assert!(scene.nodes[c].children.len() == 1);
    ufbxt_assert!(scene.nodes[c].children[0] == d);

    ufbxt_assert!(scene.nodes[d].parent == Some(c));
    ufbxt_assert!(scene.nodes[d].children.is_empty());
});

file_test_allow_error!(synthetic_node_cycle_fail, |scene, err, load_error| {
    ufbxt_assert!(scene.is_none());
});

file_test!(maya_static_no_inherit_scale, |scene, err, load_error| {
    {
        let node = find_node(scene, "joint1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();

        let ref_translation = Vec3::new(0.0, 0.0, 0.0);
        let ref_scale = Vec3::new(2.0, 2.0, 2.0);
        let transform = matrix_to_transform(&node.node_to_world);

        assert_close_vec3(err, ref_translation, transform.translation);
        assert_close_vec3(err, ref_scale, transform.scale);
        assert_close_vec3(err, ref_scale, node.local_transform.scale);
    }

    {
        let node = find_node(scene, "joint2").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();

        let ref_translation = Vec3::new(0.0, 8.0, 0.0);
        let ref_scale = Vec3::new(1.0, 2.0, 2.0);
        let transform = matrix_to_transform(&node.node_to_world);

        assert_close_vec3(err, ref_translation, transform.translation);
        assert_close_vec3(err, ref_scale, transform.scale);
        assert_close_vec3(err, ref_scale, node.local_transform.scale);
    }

    {
        let node = find_node(scene, "joint3").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();

        let ref_translation = Vec3::new(0.0, 12.0, 0.0);
        let ref_scale = Vec3::new(1.0, 1.0, 1.0);
        let transform = matrix_to_transform(&node.node_to_world);

        assert_close_vec3(err, ref_translation, transform.translation);
        assert_close_vec3(err, ref_scale, transform.scale);
        assert_close_vec3(err, ref_scale, node.local_transform.scale);
    }
});

fn scale_compensate_opts() -> LoadOpts {
    LoadOpts { inherit_mode_handling: InheritModeHandling::Compensate, ..Default::default() }
}

file_test!(
    maya_static_no_inherit_scale_helper,
    file = "maya_static_no_inherit_scale",
    opts = scale_helper_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS,
    |scene, err, load_error| {
        let ref_scale_one = Vec3::new(1.0, 1.0, 1.0);

        {
            let node = find_node(scene, "joint1").map(|n| &scene.nodes[n]);
            ufbxt_assert!(node.is_some());
            let node = node.unwrap();
            ufbxt_assert!(node.scale_helper.is_some());
            let scale_helper = &scene.nodes[node.scale_helper.unwrap()];

            let ref_translation = Vec3::new(0.0, 0.0, 0.0);
            let ref_scale = Vec3::new(2.0, 2.0, 2.0);
            let transform = matrix_to_transform(&scale_helper.node_to_world);

            assert_close_vec3(err, ref_translation, transform.translation);
            assert_close_vec3(err, ref_scale, transform.scale);
            assert_close_vec3(err, ref_scale, scale_helper.local_transform.scale);
            assert_close_vec3(err, ref_scale_one, node.local_transform.scale);
        }

        {
            let node = find_node(scene, "joint2").map(|n| &scene.nodes[n]);
            ufbxt_assert!(node.is_some());
            let node = node.unwrap();
            ufbxt_assert!(node.scale_helper.is_some());
            let scale_helper = &scene.nodes[node.scale_helper.unwrap()];

            let ref_translation = Vec3::new(0.0, 8.0, 0.0);
            let ref_scale = Vec3::new(1.0, 2.0, 2.0);
            let transform = matrix_to_transform(&scale_helper.node_to_world);

            assert_close_vec3(err, ref_translation, transform.translation);
            assert_close_vec3(err, ref_scale, transform.scale);
            assert_close_vec3(err, ref_scale, scale_helper.local_transform.scale);
            assert_close_vec3(err, ref_scale_one, node.local_transform.scale);
        }

        {
            let node = find_node(scene, "joint3").map(|n| &scene.nodes[n]);
            ufbxt_assert!(node.is_some());
            let node = node.unwrap();
            ufbxt_assert!(node.scale_helper.is_none());

            let ref_translation = Vec3::new(0.0, 12.0, 0.0);
            let ref_scale = Vec3::new(1.0, 1.0, 1.0);
            let transform = matrix_to_transform(&node.node_to_world);

            assert_close_vec3(err, ref_translation, transform.translation);
            assert_close_vec3(err, ref_scale, transform.scale);
        }
    }
);

file_test!(
    maya_static_no_inherit_scale_compensate,
    file = "maya_static_no_inherit_scale",
    opts = scale_compensate_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS,
    |scene, err, load_error| {
        {
            let node = find_node(scene, "joint1").map(|n| &scene.nodes[n]);
            ufbxt_assert!(node.is_some());
            let node = node.unwrap();

            let ref_translation = Vec3::new(0.0, 0.0, 0.0);
            let ref_scale = Vec3::new(2.0, 2.0, 2.0);
            let transform = matrix_to_transform(&node.node_to_world);

            assert_close_vec3(err, ref_translation, transform.translation);
            assert_close_vec3(err, ref_scale, transform.scale);
            assert_close_vec3(err, ref_scale, node.local_transform.scale);
        }

        {
            let node = find_node(scene, "joint2").map(|n| &scene.nodes[n]);
            ufbxt_assert!(node.is_some());
            let node = node.unwrap();
            ufbxt_assert!(node.scale_helper.is_some());
            assert_close_real(err, node.adjust_post_scale, 0.5);
            let scale_helper = &scene.nodes[node.scale_helper.unwrap()];

            let ref_translation = Vec3::new(0.0, 8.0, 0.0);
            let ref_scale = Vec3::new(1.0, 2.0, 2.0);
            let local_scale = Vec3::new(0.5, 0.5, 0.5);
            let transform = matrix_to_transform(&scale_helper.node_to_world);

            assert_close_vec3(err, ref_translation, transform.translation);
            assert_close_vec3(err, ref_scale, transform.scale);
            assert_close_vec3(err, ref_scale, scale_helper.local_transform.scale);
            assert_close_vec3(err, local_scale, node.local_transform.scale);
        }

        {
            let node = find_node(scene, "joint3").map(|n| &scene.nodes[n]);
            ufbxt_assert!(node.is_some());
            let node = node.unwrap();
            ufbxt_assert!(node.scale_helper.is_none());

            let ref_translation = Vec3::new(0.0, 12.0, 0.0);
            let ref_scale = Vec3::new(1.0, 1.0, 1.0);
            let transform = matrix_to_transform(&node.node_to_world);

            assert_close_vec3(err, ref_translation, transform.translation);
            assert_close_vec3(err, ref_scale, transform.scale);
        }
    }
);

file_test!(synthetic_geometry_transform_inherit_mode, |scene, err, load_error| {});

file_test!(
    synthetic_geometry_transform_inherit_mode_scale_helper,
    file = "synthetic_geometry_transform_inherit_mode",
    opts = scale_helper_opts(),
    flags = DIFF_ALWAYS | FUZZ_ALWAYS | FUZZ_OPTS,
    |scene, err, load_error| {
        for (i, node) in scene.nodes.iter().enumerate() {
            let transform = evaluate_transform(scene, &scene.anim, i, 0.0);
            assert_close_vec3(err, node.local_transform.translation, transform.translation);
            assert_close_quat(err, node.local_transform.rotation, transform.rotation);
            assert_close_vec3(err, node.local_transform.scale, transform.scale);
        }
    }
);

file_test!(maya_child_pivots, |scene, err, load_error| {
    check_frame(scene, err, true, "maya_child_pivots", None, 0.0 / 24.0);
    check_frame(scene, err, true, "maya_child_pivots_6", None, 6.0 / 24.0);
    check_frame(scene, err, true, "maya_child_pivots_12", None, 12.0 / 24.0);
});

fn lefthanded_y_up_opts() -> LoadOpts {
    LoadOpts { target_axes: CoordinateAxes::LEFT_HANDED_Y_UP, handedness_conversion_axis: MirrorAxis::X, ..Default::default() }
}

file_test!(maya_axes, |scene, err, load_error| {});

fn modify_meter_opts() -> LoadOpts {
    LoadOpts { target_unit_meters: 1.0, space_conversion: SpaceConversion::ModifyGeometry, ..Default::default() }
}

file_test!(
    maya_axes_modify,
    file = "maya_axes",
    opts = modify_meter_opts(),
    flags = DIFF_ALWAYS | FUZZ_ALWAYS | FUZZ_OPTS | DIFF_SCALE_100,
    |scene, err, load_error| {}
);

#[test]
fn maya_axes_lefthanded() {
    let mut err = DiffError::default();

    let obj_file = load_obj_file("maya_axes_lefthanded", None);

    for path in FileIterator::new("maya_axes") {
        for (i, &mirror_axis) in MIRROR_AXES.iter().enumerate() {
            ufbxt_hintf!("mirror_axis={i}");

            let opts = LoadOpts {
                target_axes: CoordinateAxes::LEFT_HANDED_Y_UP,
                handedness_conversion_axis: mirror_axis,
                reverse_winding: i == 0,
                ..Default::default()
            };

            let scene = load_file(&path, &opts);
            if let Err(error) = &scene {
                log_error(error);
            }
            ufbxt_assert!(scene.is_ok());
            let scene = scene.unwrap();

            check_transform_consistency(&mut err, &scene, 24.0, 1, 0);

            check_scene(&scene);
            diff_to_obj(&scene, &obj_file, &mut err, 0);
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

file_test!(maya_axes_lefthanded_file, file = "maya_axes", opts = lefthanded_y_up_opts(), flags = FUZZ_ALWAYS | FUZZ_OPTS, |scene, err, load_error| {});

#[test]
fn maya_axes_lefthanded_adjust() {
    let mut err = DiffError::default();

    let obj_file = load_obj_file("maya_axes_lefthanded", None);

    for path in FileIterator::new("maya_axes") {
        for (i, &mirror_axis) in MIRROR_AXES.iter().enumerate().skip(1) {
            ufbxt_hintf!("mirror_axis={i}");

            let opts = LoadOpts {
                target_axes: CoordinateAxes::LEFT_HANDED_Y_UP,
                handedness_conversion_axis: mirror_axis,
                space_conversion: SpaceConversion::AdjustTransforms,
                ..Default::default()
            };

            let scene = load_file(&path, &opts);
            if let Err(error) = &scene {
                log_error(error);
            }
            ufbxt_assert!(scene.is_ok());
            let scene = scene.unwrap();

            check_transform_consistency(&mut err, &scene, 24.0, 1, 0);

            check_scene(&scene);
            diff_to_obj(&scene, &obj_file, &mut err, 0);
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn maya_axes_lefthanded_modify() {
    let mut err = DiffError::default();

    let mut obj_file = load_obj_file("maya_axes_lefthanded", None);
    obj_file.fbx_position_scale = 100.0;

    for path in FileIterator::new("maya_axes") {
        for (i, &mirror_axis) in MIRROR_AXES.iter().enumerate().skip(1) {
            ufbxt_hintf!("mirror_axis={i}");

            let opts = LoadOpts {
                target_axes: CoordinateAxes::LEFT_HANDED_Y_UP,
                handedness_conversion_axis: mirror_axis,
                space_conversion: SpaceConversion::ModifyGeometry,
                target_unit_meters: 1.0,
                ..Default::default()
            };

            let scene = load_file(&path, &opts);
            if let Err(error) = &scene {
                log_error(error);
            }
            ufbxt_assert!(scene.is_ok());
            let scene = scene.unwrap();

            check_transform_consistency(&mut err, &scene, 24.0, 1, 0);

            check_scene(&scene);
            diff_to_obj(&scene, &obj_file, &mut err, 0);
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

file_test!(maya_axes_anim, |scene, err, load_error| {
    check_frame(scene, err, false, "maya_axes_anim_0", None, 0.0 / 24.0);
    check_frame(scene, err, false, "maya_axes_anim_5", None, 5.0 / 24.0);
    check_frame(scene, err, false, "maya_axes_anim_8", None, 8.0 / 24.0);

    ufbxt_assert!(scene.poses.len() == 1);
    let pose = &scene.poses[0];
    ufbxt_assert!(pose.is_bind_pose);
    ufbxt_assert!(pose.bone_poses.len() == 5);
    for bone_pose in &pose.bone_poses {
        let bone_node = &scene.nodes[bone_pose.bone_node];

        assert_close_vec3(err, bone_node.node_to_world.col(0), bone_pose.bone_to_world.col(0));
        assert_close_vec3(err, bone_node.node_to_world.col(1), bone_pose.bone_to_world.col(1));
        assert_close_vec3(err, bone_node.node_to_world.col(2), bone_pose.bone_to_world.col(2));
        assert_close_vec3(err, bone_node.node_to_world.col(3), bone_pose.bone_to_world.col(3));
    }
});

file_test!(maya_axes_anim_modify, file = "maya_axes_anim", opts = modify_meter_opts(), flags = FUZZ_ALWAYS | FUZZ_OPTS, |scene, err, load_error| {
    check_frame_imp(scene, err, false, "maya_axes_anim_0", None, 0.0 / 24.0, CHECK_FRAME_SCALE_100);
    check_frame_imp(scene, err, false, "maya_axes_anim_5", None, 5.0 / 24.0, CHECK_FRAME_SCALE_100);
    check_frame_imp(scene, err, false, "maya_axes_anim_8", None, 8.0 / 24.0, CHECK_FRAME_SCALE_100);
});

#[test]
fn maya_axes_anim_lefthanded() {
    let mut err = DiffError::default();

    for path in FileIterator::new("maya_axes_anim") {
        for (i, &mirror_axis) in MIRROR_AXES.iter().enumerate() {
            ufbxt_hintf!("mirror_axis={i}");

            let opts = LoadOpts {
                target_axes: CoordinateAxes::LEFT_HANDED_Y_UP,
                handedness_conversion_axis: mirror_axis,
                load_external_files: true,
                reverse_winding: i == 0,
                ..Default::default()
            };

            let scene = load_file(&path, &opts);
            if let Err(error) = &scene {
                log_error(error);
            }
            ufbxt_assert!(scene.is_ok());
            let scene = scene.unwrap();

            check_scene(&scene);
            check_frame(&scene, &mut err, false, "maya_axes_anim_lefthanded_0", None, 0.0 / 24.0);
            check_frame(&scene, &mut err, false, "maya_axes_anim_lefthanded_5", None, 5.0 / 24.0);
            check_frame(&scene, &mut err, false, "maya_axes_anim_lefthanded_8", None, 8.0 / 24.0);

            check_transform_consistency(&mut err, &scene, 24.0, 24, 0);
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

file_test!(
    maya_axes_anim_lefthanded_file,
    file = "maya_axes_anim",
    opts = lefthanded_y_up_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS,
    |scene, err, load_error| {}
);

#[test]
fn maya_axes_anim_lefthanded_adjust() {
    let mut err = DiffError::default();

    for path in FileIterator::new("maya_axes_anim") {
        for (i, &mirror_axis) in MIRROR_AXES.iter().enumerate().skip(1) {
            ufbxt_hintf!("mirror_axis={i}");

            let opts = LoadOpts {
                space_conversion: SpaceConversion::AdjustTransforms,
                target_axes: CoordinateAxes::LEFT_HANDED_Y_UP,
                handedness_conversion_axis: mirror_axis,
                load_external_files: true,
                ..Default::default()
            };

            let scene = load_file(&path, &opts);
            if let Err(error) = &scene {
                log_error(error);
            }
            ufbxt_assert!(scene.is_ok());
            let scene = scene.unwrap();

            check_scene(&scene);
            check_frame(&scene, &mut err, false, "maya_axes_anim_lefthanded_0", None, 0.0 / 24.0);
            check_frame(&scene, &mut err, false, "maya_axes_anim_lefthanded_5", None, 5.0 / 24.0);
            check_frame(&scene, &mut err, false, "maya_axes_anim_lefthanded_8", None, 8.0 / 24.0);

            check_transform_consistency(&mut err, &scene, 24.0, 24, 0);
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn maya_axes_anim_lefthanded_modify() {
    let mut err = DiffError::default();

    for path in FileIterator::new("maya_axes_anim") {
        for (i, &mirror_axis) in MIRROR_AXES.iter().enumerate().skip(1) {
            ufbxt_hintf!("mirror_axis={i}");

            let opts = LoadOpts {
                space_conversion: SpaceConversion::ModifyGeometry,
                target_unit_meters: 1.0,
                target_axes: CoordinateAxes::LEFT_HANDED_Y_UP,
                handedness_conversion_axis: mirror_axis,
                load_external_files: true,
                ..Default::default()
            };

            let scene = load_file(&path, &opts);
            if let Err(error) = &scene {
                log_error(error);
            }
            ufbxt_assert!(scene.is_ok());
            let scene = scene.unwrap();

            check_scene(&scene);
            check_frame_imp(&scene, &mut err, false, "maya_axes_anim_lefthanded_0", None, 0.0 / 24.0, CHECK_FRAME_SCALE_100);
            check_frame_imp(&scene, &mut err, false, "maya_axes_anim_lefthanded_5", None, 5.0 / 24.0, CHECK_FRAME_SCALE_100);
            check_frame_imp(&scene, &mut err, false, "maya_axes_anim_lefthanded_8", None, 8.0 / 24.0, CHECK_FRAME_SCALE_100);

            check_transform_consistency(&mut err, &scene, 24.0, 24, 0);
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

fn pivot_adjust_opts() -> LoadOpts {
    LoadOpts { pivot_handling: PivotHandling::AdjustToPivot, ..Default::default() }
}

file_test!(maya_equal_pivot, |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let node = node.unwrap();

    let prop = node.element.props.find("RotationPivot");
    ufbxt_assert!(prop.is_some());
    assert_close_vec3_xyz(err, prop.unwrap().value_vec3(), 2.0, 0.0, 0.0);

    let prop = node.element.props.find("ScalingPivot");
    ufbxt_assert!(prop.is_some());
    assert_close_vec3_xyz(err, prop.unwrap().value_vec3(), 2.0, 0.0, 0.0);

    let prop = node.element.props.find("GeometricTranslation");
    assert_close_vec3_xyz(err, prop.unwrap().value_vec3(), 0.0, 0.0, 0.0);
});

file_test!(maya_equal_pivot_adjust, file = "maya_equal_pivot", opts = pivot_adjust_opts(), flags = DIFF_ALWAYS, |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let node = node.unwrap();

    let prop = node.element.props.find("RotationPivot");
    ufbxt_assert!(prop.is_some());
    assert_close_vec3_xyz(err, prop.unwrap().value_vec3(), 0.0, 0.0, 0.0);

    let prop = node.element.props.find("ScalingPivot");
    ufbxt_assert!(prop.is_some());
    assert_close_vec3_xyz(err, prop.unwrap().value_vec3(), 0.0, 0.0, 0.0);

    let prop = node.element.props.find("GeometricTranslation");
    ufbxt_assert!(prop.is_some());
    assert_close_vec3_xyz(err, prop.unwrap().value_vec3(), -2.0, 0.0, 0.0);
});

file_test!(maya_equal_pivot_scale, |scene, err, load_error| {
    let node = find_node(scene, "Middle").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let node = node.unwrap();

    let prop = node.element.props.find("RotationPivot");
    ufbxt_assert!(prop.is_some());
    assert_close_vec3_xyz(err, prop.unwrap().value_vec3(), 2.0, 0.0, 0.0);

    let prop = node.element.props.find("ScalingPivot");
    ufbxt_assert!(prop.is_some());
    assert_close_vec3_xyz(err, prop.unwrap().value_vec3(), 2.0, 0.0, 0.0);

    let prop = node.element.props.find("GeometricTranslation");
    assert_close_vec3_xyz(err, prop.unwrap().value_vec3(), 0.0, 0.0, 0.0);

    check_frame_imp(scene, err, false, "maya_equal_pivot_scale_6", None, 6.0 / 24.0, 0);
    check_frame_imp(scene, err, false, "maya_equal_pivot_scale_12", None, 12.0 / 24.0, 0);
});

file_test!(maya_equal_pivot_scale_adjust, file = "maya_equal_pivot_scale", opts = pivot_adjust_opts(), flags = DIFF_ALWAYS, |scene, err, load_error| {
    let node = find_node(scene, "Middle").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let node = node.unwrap();

    let prop = node.element.props.find("RotationPivot");
    ufbxt_assert!(prop.is_some());
    assert_close_vec3_xyz(err, prop.unwrap().value_vec3(), 0.0, 0.0, 0.0);

    let prop = node.element.props.find("ScalingPivot");
    ufbxt_assert!(prop.is_some());
    assert_close_vec3_xyz(err, prop.unwrap().value_vec3(), 0.0, 0.0, 0.0);

    let prop = node.element.props.find("GeometricTranslation");
    ufbxt_assert!(prop.is_some());
    assert_close_vec3_xyz(err, prop.unwrap().value_vec3(), -2.0, 0.0, 0.0);

    check_frame_imp(scene, err, false, "maya_equal_pivot_scale_6", None, 6.0 / 24.0, 0);
    check_frame_imp(scene, err, false, "maya_equal_pivot_scale_12", None, 12.0 / 24.0, 0);
});

#[test]
fn maya_equal_pivot_handling() {
    let mut err = DiffError::default();

    let obj_file = load_obj_file("maya_equal_pivot", None);

    for path in FileIterator::new("maya_equal_pivot") {
        for (pv, &pivot_handling) in PIVOT_HANDLINGS.iter().enumerate() {
            for (gh, &geometry_transform_handling) in GEOMETRY_TRANSFORM_HANDLINGS.iter().enumerate() {
                ufbxt_hintf!("pivot_handling={pv}, geometry_transform_handling={gh}");

                let opts = LoadOpts { pivot_handling, geometry_transform_handling, ..Default::default() };

                let scene = load_file(&path, &opts);
                if let Err(error) = &scene {
                    log_error(error);
                }
                ufbxt_assert!(scene.is_ok());
                let scene = scene.unwrap();
                check_scene(&scene);

                let node = find_node(&scene, "pCube1").map(|n| &scene.nodes[n]);
                ufbxt_assert!(node.is_some());
                let node = node.unwrap();
                if pivot_handling == PivotHandling::AdjustToPivot {
                    assert_close_vec3_xyz(&mut err, node.element.props.find_vec3("RotationPivot", Vec3::ZERO), 0.0, 0.0, 0.0);
                    assert_close_vec3_xyz(&mut err, node.element.props.find_vec3("ScalingPivot", Vec3::ZERO), 0.0, 0.0, 0.0);
                }

                diff_to_obj(&scene, &obj_file, &mut err, 0);
            }
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn maya_equal_pivot_scale_handling() {
    let mut err = DiffError::default();

    let obj_file = load_obj_file("maya_equal_pivot_scale", None);

    for path in FileIterator::new("maya_equal_pivot_scale") {
        for (pv, &pivot_handling) in PIVOT_HANDLINGS.iter().enumerate() {
            for (gh, &geometry_transform_handling) in GEOMETRY_TRANSFORM_HANDLINGS.iter().enumerate() {
                ufbxt_hintf!("pivot_handling={pv}, geometry_transform_handling={gh}");

                let opts = LoadOpts { pivot_handling, geometry_transform_handling, ..Default::default() };

                let scene = load_file(&path, &opts);
                if let Err(error) = &scene {
                    log_error(error);
                }
                ufbxt_assert!(scene.is_ok());
                let scene = scene.unwrap();
                check_scene(&scene);

                let node = find_node(&scene, "Middle").map(|n| &scene.nodes[n]);
                ufbxt_assert!(node.is_some());
                let node = node.unwrap();
                if pivot_handling == PivotHandling::AdjustToPivot {
                    assert_close_vec3_xyz(&mut err, node.element.props.find_vec3("RotationPivot", Vec3::ZERO), 0.0, 0.0, 0.0);
                    assert_close_vec3_xyz(&mut err, node.element.props.find_vec3("ScalingPivot", Vec3::ZERO), 0.0, 0.0, 0.0);
                }

                diff_to_obj(&scene, &obj_file, &mut err, 0);
                check_frame_imp(&scene, &mut err, false, "maya_equal_pivot_scale_6", None, 6.0 / 24.0, 0);
                check_frame_imp(&scene, &mut err, false, "maya_equal_pivot_scale_12", None, 12.0 / 24.0, 0);
            }
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn maya_instanced_pivots() {
    let mut err = DiffError::default();

    let obj_file = load_obj_file("maya_instanced_pivots", None);

    for path in FileIterator::new("maya_instanced_pivots") {
        for (pv, &pivot_handling) in PIVOT_HANDLINGS.iter().enumerate() {
            for (gh, &geometry_transform_handling) in GEOMETRY_TRANSFORM_HANDLINGS.iter().enumerate() {
                ufbxt_hintf!("pivot_handling={pv}, geometry_transform_handling={gh}");

                let opts = LoadOpts { pivot_handling, geometry_transform_handling, ..Default::default() };

                let scene = load_file(&path, &opts);
                if let Err(error) = &scene {
                    log_error(error);
                }
                ufbxt_assert!(scene.is_ok());
                let scene = scene.unwrap();

                check_scene(&scene);
                diff_to_obj(&scene, &obj_file, &mut err, 0);

                let cube1 = find_node(&scene, "pCube1").map(|n| &scene.nodes[n]);
                let cube2 = find_node(&scene, "pCube2").map(|n| &scene.nodes[n]);
                let light = find_node(&scene, "pointLight1").map(|n| &scene.nodes[n]);
                ufbxt_assert!(cube1.is_some());
                ufbxt_assert!(cube2.is_some());
                ufbxt_assert!(light.is_some());
                let (cube1, cube2, light) = (cube1.unwrap(), cube2.unwrap(), light.unwrap());

                ufbxt_assert!(scene.lights.len() == 1);
                let light_attr = &scene.lights[0];
                ufbxt_assert!(light_attr.element.instances.len() == 1);
                let light_parent = &scene.nodes[light_attr.element.instances[0]];
                assert_close_vec3_xyz(&mut err, light_parent.geometry_to_world.col(3), 0.0, 1.414, 0.586);

                if (pivot_handling == PivotHandling::AdjustToPivot || pivot_handling == PivotHandling::AdjustToRotationPivot)
                    && geometry_transform_handling != GeometryTransformHandling::ModifyGeometryNoFallback
                {
                    assert_close_vec3_xyz(&mut err, cube1.element.props.find_vec3("RotationPivot", Vec3::ZERO), 0.0, 0.0, 0.0);
                    assert_close_vec3_xyz(&mut err, cube1.element.props.find_vec3("ScalingPivot", Vec3::ZERO), 0.0, 0.0, 0.0);
                    if geometry_transform_handling == GeometryTransformHandling::Preserve {
                        assert_close_vec3_xyz(&mut err, cube1.element.props.find_vec3("GeometricTranslation", Vec3::ZERO), -2.0, 0.0, 0.0);
                    } else {
                        assert_close_vec3_xyz(&mut err, cube1.element.props.find_vec3("GeometricTranslation", Vec3::ZERO), 0.0, 0.0, 0.0);
                        ufbxt_assert!(cube1.geometry_transform_helper.is_some());
                    }

                    assert_close_vec3_xyz(&mut err, cube2.element.props.find_vec3("RotationPivot", Vec3::ZERO), 0.0, 0.0, 0.0);
                    assert_close_vec3_xyz(&mut err, cube2.element.props.find_vec3("ScalingPivot", Vec3::ZERO), 0.0, 0.0, 0.0);
                    if geometry_transform_handling == GeometryTransformHandling::Preserve {
                        assert_close_vec3_xyz(&mut err, cube2.element.props.find_vec3("GeometricTranslation", Vec3::ZERO), 2.0, 0.0, 0.0);
                    } else {
                        assert_close_vec3_xyz(&mut err, cube2.element.props.find_vec3("GeometricTranslation", Vec3::ZERO), 0.0, 0.0, 0.0);
                        ufbxt_assert!(cube2.geometry_transform_helper.is_some());
                    }

                    assert_close_vec3_xyz(&mut err, light.element.props.find_vec3("RotationPivot", Vec3::ZERO), 0.0, 0.0, 0.0);
                    assert_close_vec3_xyz(&mut err, light.element.props.find_vec3("ScalingPivot", Vec3::ZERO), 0.0, 0.0, 0.0);
                    if geometry_transform_handling == GeometryTransformHandling::Preserve {
                        assert_close_vec3_xyz(&mut err, light.element.props.find_vec3("GeometricTranslation", Vec3::ZERO), 0.0, 0.0, -2.0);
                    } else {
                        assert_close_vec3_xyz(&mut err, light.element.props.find_vec3("GeometricTranslation", Vec3::ZERO), 0.0, 0.0, 0.0);
                        ufbxt_assert!(cube2.geometry_transform_helper.is_some());
                    }
                } else {
                    assert_close_vec3_xyz(&mut err, cube1.element.props.find_vec3("RotationPivot", Vec3::ZERO), 2.0, 0.0, 0.0);
                    assert_close_vec3_xyz(&mut err, cube1.element.props.find_vec3("ScalingPivot", Vec3::ZERO), 2.0, 0.0, 0.0);
                    assert_close_vec3_xyz(&mut err, cube1.element.props.find_vec3("GeometricTranslation", Vec3::ZERO), 0.0, 0.0, 0.0);

                    assert_close_vec3_xyz(&mut err, cube2.element.props.find_vec3("RotationPivot", Vec3::ZERO), -2.0, 0.0, 0.0);
                    assert_close_vec3_xyz(&mut err, cube2.element.props.find_vec3("ScalingPivot", Vec3::ZERO), -2.0, 0.0, 0.0);
                    assert_close_vec3_xyz(&mut err, cube2.element.props.find_vec3("GeometricTranslation", Vec3::ZERO), 0.0, 0.0, 0.0);

                    assert_close_vec3_xyz(&mut err, light.element.props.find_vec3("RotationPivot", Vec3::ZERO), 0.0, 0.0, 2.0);
                    assert_close_vec3_xyz(&mut err, light.element.props.find_vec3("ScalingPivot", Vec3::ZERO), 0.0, 0.0, 2.0);
                    assert_close_vec3_xyz(&mut err, light.element.props.find_vec3("GeometricTranslation", Vec3::ZERO), 0.0, 0.0, 0.0);
                }
            }
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn motionbuilder_pivot() {
    let mut err = DiffError::default();

    let obj_file = load_obj_file("motionbuilder_pivot", None);

    for path in FileIterator::new("motionbuilder_pivot") {
        for (pv, &pivot_handling) in PIVOT_HANDLINGS.iter().enumerate() {
            for (gh, &geometry_transform_handling) in GEOMETRY_TRANSFORM_HANDLINGS.iter().enumerate() {
                ufbxt_hintf!("pivot_handling={pv}, geometry_transform_handling={gh}");

                let opts = LoadOpts { pivot_handling, geometry_transform_handling, ..Default::default() };

                let scene = load_file(&path, &opts);
                if let Err(error) = &scene {
                    log_error(error);
                }
                ufbxt_assert!(scene.is_ok());
                let scene = scene.unwrap();
                check_scene(&scene);
                diff_to_obj(&scene, &obj_file, &mut err, 0);
            }
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

fn adjust_to_pivot_opts() -> LoadOpts {
    LoadOpts { pivot_handling: PivotHandling::AdjustToPivot, ..Default::default() }
}

file_test!(maya_skinned_pivot, opts = adjust_to_pivot_opts(), |scene, err, load_error| {});

#[test]
fn maya_skinned_pivot_opts() {
    let mut err = DiffError::default();

    let obj_file = load_obj_file("maya_skinned_pivot", None);

    for path in FileIterator::new("maya_skinned_pivot") {
        for (pv, &pivot_handling) in PIVOT_HANDLINGS.iter().enumerate() {
            for (gh, &geometry_transform_handling) in GEOMETRY_TRANSFORM_HANDLINGS.iter().enumerate() {
                ufbxt_hintf!("pivot_handling={pv}, geometry_transform_handling={gh}");

                let opts = LoadOpts { evaluate_skinning: true, pivot_handling, geometry_transform_handling, ..Default::default() };

                let scene = load_file(&path, &opts);
                if let Err(error) = &scene {
                    log_error(error);
                }
                ufbxt_assert!(scene.is_ok());
                let scene = scene.unwrap();
                check_scene(&scene);
                diff_to_obj(&scene, &obj_file, &mut err, 0);
            }
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

file_test!(maya_advanced_skinned_pivot, opts = adjust_to_pivot_opts(), |scene, err, load_error| {});

#[test]
fn maya_advanced_skinned_pivot_opts() {
    let mut err = DiffError::default();

    let obj_file = load_obj_file("maya_advanced_skinned_pivot", None);

    for path in FileIterator::new("maya_advanced_skinned_pivot") {
        for (pv, &pivot_handling) in PIVOT_HANDLINGS.iter().enumerate() {
            for (gh, &geometry_transform_handling) in GEOMETRY_TRANSFORM_HANDLINGS.iter().enumerate() {
                ufbxt_hintf!("pivot_handling={pv}, geometry_transform_handling={gh}");

                let opts = LoadOpts { evaluate_skinning: true, pivot_handling, geometry_transform_handling, ..Default::default() };

                let scene = load_file(&path, &opts);
                if let Err(error) = &scene {
                    log_error(error);
                }
                ufbxt_assert!(scene.is_ok());
                let scene = scene.unwrap();
                check_scene(&scene);
                diff_to_obj(&scene, &obj_file, &mut err, 0);
            }
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

file_test!(blender_340_mirrored_normals, |scene, err, load_error| {});

file_test!(maya_extrapolate_cube, |scene, err, load_error| {
    check_frame(scene, err, true, "maya_extrapolate_cube_26", None, 26.0 / 24.0);
    check_frame(scene, err, true, "maya_extrapolate_cube_60", None, 60.0 / 24.0);
    check_frame(scene, err, true, "maya_extrapolate_cube_80", None, 80.0 / 24.0);
    check_frame(scene, err, true, "maya_extrapolate_cube_100", None, 100.0 / 24.0);

    check_frame_imp(scene, err, true, "maya_extrapolate_cube_noex_28", None, 28.0 / 24.0, CHECK_FRAME_NO_EXTRAPOLATE);
    check_frame_imp(scene, err, true, "maya_extrapolate_cube_noex_34", None, 34.0 / 24.0, CHECK_FRAME_NO_EXTRAPOLATE);
    check_frame_imp(scene, err, true, "maya_extrapolate_cube_noex_44", None, 44.0 / 24.0, CHECK_FRAME_NO_EXTRAPOLATE);
    check_frame_imp(scene, err, true, "maya_extrapolate_cube_noex_68", None, 68.0 / 24.0, CHECK_FRAME_NO_EXTRAPOLATE);
    check_frame_imp(scene, err, true, "maya_extrapolate_cube_noex_68", None, 100.0 / 24.0, CHECK_FRAME_NO_EXTRAPOLATE);

    check_transform_consistency(err, scene, 24.0, 120, CHECK_TRANSFORM_CONSISTENCY_NO_BAKE);
    check_transform_consistency(err, scene, 24.0, 120, CHECK_TRANSFORM_CONSISTENCY_NO_EXTRAPOLATION);
});

file_test!(maya_split_pivot, |scene, err, load_error| {
    let node = find_node(scene, "pCube1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some());
    let node = node.unwrap();

    let rotation_pivot = node.element.props.find_vec3("RotationPivot", Vec3::ZERO);
    let scaling_pivot = node.element.props.find_vec3("ScalingPivot", Vec3::ZERO);

    assert_vec3_equal(rotation_pivot, 2.0, 0.0, 0.0);
    assert_vec3_equal(scaling_pivot, -0.5, 0.0, 0.0);

    check_frame(scene, err, true, "maya_split_pivot", None, 1.0 / 24.0);
    check_frame(scene, err, true, "maya_split_pivot_6", None, 6.0 / 24.0);
    check_frame(scene, err, true, "maya_split_pivot_12", None, 12.0 / 24.0);
});

fn adjust_to_rotation_pivot_opts() -> LoadOpts {
    LoadOpts { pivot_handling: PivotHandling::AdjustToRotationPivot, ..Default::default() }
}

file_test!(maya_split_pivot_adjust, file = "maya_split_pivot", opts = adjust_to_rotation_pivot_opts(), |scene, err, load_error| {
    let node = find_node(scene, "pCube1");
    ufbxt_assert!(node.is_some());
    let node = node.unwrap();

    {
        let transform = scene.nodes[node].local_transform;
        let ref_translation = Vec3::new(2.0, 0.0, 0.0);
        assert_close_vec3(err, transform.translation, ref_translation);
    }

    {
        let transform = evaluate_transform(scene, &scene.anim, node, 1.0 / 24.0);
        let ref_translation = Vec3::new(2.0, 0.0, 0.0);
        assert_close_vec3(err, transform.translation, ref_translation);
    }

    {
        let transform = evaluate_transform(scene, &scene.anim, node, 6.0 / 24.0);
        let ref_translation = Vec3::new(2.0, 0.0, 0.0);
        assert_close_vec3(err, transform.translation, ref_translation);
    }

    {
        let transform = evaluate_transform(scene, &scene.anim, node, 12.0 / 24.0);
        let ref_translation = Vec3::new(2.0, 3.0, 0.0);
        assert_close_vec3(err, transform.translation, ref_translation);
    }

    check_frame(scene, err, true, "maya_split_pivot", None, 1.0 / 24.0);
    check_frame(scene, err, true, "maya_split_pivot_6", None, 6.0 / 24.0);
    check_frame(scene, err, true, "maya_split_pivot_12", None, 12.0 / 24.0);
});

file_test!(maya_pivot_offset, |scene, err, load_error| {
    check_frame(scene, err, true, "maya_pivot_offset", None, 1.0 / 24.0);
    check_frame(scene, err, true, "maya_pivot_offset_8", None, 8.0 / 24.0);
    check_frame(scene, err, true, "maya_pivot_offset_12", None, 12.0 / 24.0);

    {
        let node = find_node(scene, "ScalingOffset").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());

        let ref_translation = Vec3::new(20.0, 0.0, 0.0);
        assert_close_vec3(err, node.unwrap().local_transform.translation, ref_translation);
    }

    {
        let node = find_node(scene, "RotationOffset").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());

        let ref_translation = Vec3::new(10.0, 0.0, 10.0);
        assert_close_vec3(err, node.unwrap().local_transform.translation, ref_translation);
    }

    {
        let node = find_node(scene, "BothOffsets").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());

        let ref_translation = Vec3::new(20.0, 0.0, 0.0);
        assert_close_vec3(err, node.unwrap().local_transform.translation, ref_translation);
    }

    {
        let node = find_node(scene, "PivotOffsets").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());

        let ref_translation = Vec3::new(10.0, 0.0, 30.0);
        assert_close_vec3(err, node.unwrap().local_transform.translation, ref_translation);
    }
});

file_test!(maya_pivot_offset_legacy_adjust, file = "maya_pivot_offset", opts = adjust_to_pivot_opts(), |scene, err, load_error| {
    check_frame(scene, err, true, "maya_pivot_offset", None, 1.0 / 24.0);
    check_frame(scene, err, true, "maya_pivot_offset_8", None, 8.0 / 24.0);
    check_frame(scene, err, true, "maya_pivot_offset_12", None, 12.0 / 24.0);
});

file_test!(maya_pivot_offset_rotation_adjust, file = "maya_pivot_offset", opts = adjust_to_rotation_pivot_opts(), |scene, err, load_error| {
    check_frame(scene, err, true, "maya_pivot_offset", None, 1.0 / 24.0);
    check_frame(scene, err, true, "maya_pivot_offset_8", None, 8.0 / 24.0);
    check_frame(scene, err, true, "maya_pivot_offset_12", None, 12.0 / 24.0);

    {
        let node = find_node(scene, "ScalingOffset").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());

        let ref_translation = Vec3::new(0.0, 0.0, 0.0);
        assert_close_vec3(err, node.unwrap().local_transform.translation, ref_translation);
    }

    {
        let node = find_node(scene, "RotationOffset").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());

        let ref_translation = Vec3::new(10.0, 0.0, 10.0);
        assert_close_vec3(err, node.unwrap().local_transform.translation, ref_translation);
    }

    {
        let node = find_node(scene, "BothOffsets").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());

        let ref_translation = Vec3::new(20.0, 0.0, 20.0);
        assert_close_vec3(err, node.unwrap().local_transform.translation, ref_translation);
    }

    {
        let node = find_node(scene, "PivotOffsets").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());

        let ref_translation = Vec3::new(10.0, 0.0, 20.0);
        assert_close_vec3(err, node.unwrap().local_transform.translation, ref_translation);
    }
});

file_test!(maya_zero_scale_pivot, |scene, err, load_error| {
    check_frame(scene, err, true, "maya_zero_scale_pivot", None, 1.0 / 24.0);
    check_frame(scene, err, true, "maya_zero_scale_pivot_6", None, 6.0 / 24.0);
    check_frame(scene, err, true, "maya_zero_scale_pivot_12", None, 12.0 / 24.0);
});

file_test!(maya_zero_scale_pivot_legacy_adjust, file = "maya_zero_scale_pivot", opts = adjust_to_pivot_opts(), |scene, err, load_error| {
    check_frame(scene, err, true, "maya_zero_scale_pivot", None, 1.0 / 24.0);
    check_frame(scene, err, true, "maya_zero_scale_pivot_6", None, 6.0 / 24.0);
    check_frame(scene, err, true, "maya_zero_scale_pivot_12", None, 12.0 / 24.0);
});

file_test!(maya_zero_scale_pivot_rotation_adjust, file = "maya_zero_scale_pivot", opts = adjust_to_rotation_pivot_opts(), |scene, err, load_error| {
    check_frame(scene, err, true, "maya_zero_scale_pivot", None, 1.0 / 24.0);
    check_frame(scene, err, true, "maya_zero_scale_pivot_6", None, 6.0 / 24.0);
    check_frame(scene, err, true, "maya_zero_scale_pivot_12", None, 12.0 / 24.0);

    {
        let node = find_node(scene, "Ortho_Parent");
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();

        let begin_ref = Vec3::new(10.0, 0.0, 0.0);
        let end_ref = Vec3::new(10.0, 0.0, 0.0);
        let begin_pos = scene.nodes[node].local_transform.translation;
        let end_pos = evaluate_transform(scene, &scene.anim, node, 1.0).translation;
        assert_close_vec3(err, begin_pos, begin_ref);
        assert_close_vec3(err, end_pos, end_ref);
    }

    {
        let node = find_node(scene, "Parallel_Parent");
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();

        let begin_ref = Vec3::new(10.0, 0.0, 0.0); // incorrect due to zero scale
        let end_ref = Vec3::new(-10.0, 0.0, 0.0);
        let begin_pos = scene.nodes[node].local_transform.translation;
        let end_pos = evaluate_transform(scene, &scene.anim, node, 1.0).translation;
        assert_close_vec3_threshold(err, begin_pos, begin_ref, 0.01);
        assert_close_vec3(err, end_pos, end_ref);
    }

    {
        let node = find_node(scene, "Align_Parent");
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();

        let begin_ref = Vec3::new(0.0, 0.0, 10.0);
        let end_ref = Vec3::new(0.0, 0.0, 10.0);
        let begin_pos = scene.nodes[node].local_transform.translation;
        let end_pos = evaluate_transform(scene, &scene.anim, node, 1.0).translation;
        assert_close_vec3(err, begin_pos, begin_ref);
        assert_close_vec3(err, end_pos, end_ref);
    }

    {
        let node = find_node(scene, "Offset_Parent");
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();

        let begin_ref = Vec3::new(0.0, 0.0, 0.0); // incorrect due to zero scale
        let end_ref = Vec3::new(0.0, 0.0, -10.0);
        let begin_pos = scene.nodes[node].local_transform.translation;
        let end_pos = evaluate_transform(scene, &scene.anim, node, 1.0).translation;
        assert_close_vec3(err, begin_pos, begin_ref);
        assert_close_vec3(err, end_pos, end_ref);
    }
});

file_test!(maya_null_pivots, |scene, err, load_error| {
    check_frame(scene, err, true, "maya_null_pivots", None, 1.0 / 24.0);
    check_frame(scene, err, true, "maya_null_pivots_6", None, 6.0 / 24.0);
    check_frame(scene, err, true, "maya_null_pivots_12", None, 12.0 / 24.0);
});

file_test!(maya_null_pivots_adjust, file = "maya_null_pivots", opts = adjust_to_rotation_pivot_opts(), |scene, err, load_error| {
    check_frame(scene, err, true, "maya_null_pivots", None, 1.0 / 24.0);
    check_frame(scene, err, true, "maya_null_pivots_6", None, 6.0 / 24.0);
    check_frame(scene, err, true, "maya_null_pivots_12", None, 12.0 / 24.0);
});

#[test]
fn maya_null_pivots_opts() {
    let mut err = DiffError::default();

    let obj_file = load_obj_file("maya_null_pivots", None);

    for path in FileIterator::new("maya_null_pivots") {
        for (pv, &pivot_handling) in PIVOT_HANDLINGS.iter().enumerate() {
            for (gh, &geometry_transform_handling) in GEOMETRY_TRANSFORM_HANDLINGS.iter().enumerate() {
                for re in 0..2 {
                    ufbxt_hintf!("pivot_handling={pv}, geometry_transform_handling={gh}, pivot_handling_retain_empties={re}");

                    let opts = LoadOpts { pivot_handling, pivot_handling_retain_empties: re != 0, geometry_transform_handling, ..Default::default() };

                    let mut top_adjusted = false;
                    let mut mid_adjusted = false;
                    let mut top_helper = false;
                    let mid_helper = false;
                    match opts.pivot_handling {
                        PivotHandling::Retain => {}

                        PivotHandling::AdjustToPivot => match opts.geometry_transform_handling {
                            GeometryTransformHandling::Preserve => {
                                top_adjusted = true;
                            }
                            GeometryTransformHandling::HelperNodes | GeometryTransformHandling::ModifyGeometry => {
                                top_adjusted = true;
                                top_helper = true;
                            }
                            GeometryTransformHandling::ModifyGeometryNoFallback => {}
                        },

                        PivotHandling::AdjustToRotationPivot => {
                            if !opts.pivot_handling_retain_empties {
                                top_adjusted = true;
                                mid_adjusted = true;
                            }
                        }
                    }

                    let scene = load_file(&path, &opts);
                    if let Err(error) = &scene {
                        log_error(error);
                    }
                    ufbxt_assert!(scene.is_ok());
                    let scene = scene.unwrap();
                    check_scene(&scene);

                    {
                        let node = find_node(&scene, "Top").map(|n| &scene.nodes[n]);
                        ufbxt_assert!(node.is_some());
                        let node = node.unwrap();
                        if top_helper {
                            ufbxt_assert!(node.attrib_type == ElementType::Unknown);
                            ufbxt_assert!(node.geometry_transform_helper.is_some());
                        } else {
                            ufbxt_assert!(node.attrib_type == ElementType::Empty);
                            ufbxt_assert!(node.geometry_transform_helper.is_none());
                        }

                        if top_adjusted {
                            let ref_translation = Vec3::new(10.0, 0.0, 0.0);
                            assert_close_vec3(&mut err, node.local_transform.translation, ref_translation);
                        } else {
                            let ref_translation = Vec3::new(0.0, 0.0, 0.0);
                            assert_close_vec3(&mut err, node.local_transform.translation, ref_translation);
                        }
                    }

                    {
                        let node = find_node(&scene, "Mid").map(|n| &scene.nodes[n]);
                        ufbxt_assert!(node.is_some());
                        let node = node.unwrap();
                        if mid_helper {
                            ufbxt_assert!(node.attrib_type == ElementType::Unknown);
                            ufbxt_assert!(node.geometry_transform_helper.is_some());
                        } else {
                            ufbxt_assert!(node.attrib_type == ElementType::Empty);
                            ufbxt_assert!(node.geometry_transform_helper.is_none());
                        }

                        if mid_adjusted {
                            let ref_translation = Vec3::new(if top_adjusted { -20.0 } else { -10.0 }, 0.0, 0.0);
                            assert_close_vec3(&mut err, node.local_transform.translation, ref_translation);
                        } else {
                            let ref_translation = Vec3::new(if top_adjusted { -5.0 } else { 5.0 }, 0.0, 0.0);
                            assert_close_vec3(&mut err, node.local_transform.translation, ref_translation);
                        }
                    }

                    diff_to_obj(&scene, &obj_file, &mut err, 0);
                    check_frame(&scene, &mut err, true, "maya_null_pivots", None, 1.0 / 24.0);
                    check_frame(&scene, &mut err, true, "maya_null_pivots_6", None, 6.0 / 24.0);
                    check_frame(&scene, &mut err, true, "maya_null_pivots_12", None, 12.0 / 24.0);
                }
            }
        }
    }

    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

file_test!(casegen_rotation_order, |scene, err, load_error| {});

file_test!(casegen_rotation_space, |scene, err, load_error| {});
