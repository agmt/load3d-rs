mod ufbxt;
use ufbxt::*;

const MIRROR_AXES: [MirrorAxis; 4] = [MirrorAxis::None, MirrorAxis::X, MirrorAxis::Y, MirrorAxis::Z];

const GEOMETRY_TRANSFORM_HANDLINGS: [GeometryTransformHandling; 4] = [
    GeometryTransformHandling::Preserve,
    GeometryTransformHandling::HelperNodes,
    GeometryTransformHandling::ModifyGeometry,
    GeometryTransformHandling::ModifyGeometryNoFallback,
];

fn ref_transform(translation: [Real; 3], rotation_euler: [Real; 3], scale: [Real; 3]) -> RefTransform {
    RefTransform { translation: Vec3::from_array(translation), rotation_euler: Vec3::from_array(rotation_euler), scale: Vec3::from_array(scale) }
}

fn geometry_transform_helper_opts() -> LoadOpts {
    LoadOpts { geometry_transform_handling: GeometryTransformHandling::HelperNodes, ..Default::default() }
}

fn geometry_transform_modify_opts() -> LoadOpts {
    LoadOpts { geometry_transform_handling: GeometryTransformHandling::ModifyGeometry, ..Default::default() }
}

fn geometry_transform_modify_no_fallback_opts() -> LoadOpts {
    LoadOpts { geometry_transform_handling: GeometryTransformHandling::ModifyGeometryNoFallback, ..Default::default() }
}

fn geometry_transform_helper_name_opts() -> LoadOpts {
    LoadOpts {
        geometry_transform_handling: GeometryTransformHandling::HelperNodes,
        geometry_transform_helper_name: "(ufbxt helper)".to_string(),
        ..Default::default()
    }
}

fn y_up_meters_root_opts() -> LoadOpts {
    LoadOpts { target_axes: CoordinateAxes::RIGHT_HANDED_Y_UP, target_unit_meters: 1.0, space_conversion: SpaceConversion::TransformRoot, ..Default::default() }
}

fn y_up_meters_adjust_opts() -> LoadOpts {
    LoadOpts {
        target_axes: CoordinateAxes::RIGHT_HANDED_Y_UP,
        target_unit_meters: 1.0,
        space_conversion: SpaceConversion::AdjustTransforms,
        ..Default::default()
    }
}

fn z_up_meters_root_opts() -> LoadOpts {
    LoadOpts { target_axes: CoordinateAxes::RIGHT_HANDED_Z_UP, target_unit_meters: 1.0, space_conversion: SpaceConversion::TransformRoot, ..Default::default() }
}

fn z_up_meters_adjust_opts() -> LoadOpts {
    LoadOpts {
        target_axes: CoordinateAxes::RIGHT_HANDED_Z_UP,
        target_unit_meters: 1.0,
        space_conversion: SpaceConversion::AdjustTransforms,
        ..Default::default()
    }
}

fn z_up_meters_modify_opts() -> LoadOpts {
    LoadOpts {
        target_axes: CoordinateAxes::RIGHT_HANDED_Z_UP,
        target_unit_meters: 1.0,
        space_conversion: SpaceConversion::ModifyGeometry,
        ..Default::default()
    }
}

fn rh_y_camera_light_axes_opts() -> LoadOpts {
    LoadOpts { target_camera_axes: CoordinateAxes::RIGHT_HANDED_Y_UP, target_light_axes: CoordinateAxes::RIGHT_HANDED_Y_UP, ..Default::default() }
}

fn blender_space_adjust_opts() -> LoadOpts {
    LoadOpts {
        target_axes: CoordinateAxes::RIGHT_HANDED_Z_UP,
        target_unit_meters: 1.0,
        target_camera_axes: CoordinateAxes::RIGHT_HANDED_Y_UP,
        target_light_axes: CoordinateAxes::RIGHT_HANDED_Y_UP,
        space_conversion: SpaceConversion::AdjustTransforms,
        ..Default::default()
    }
}

fn find<'a>(scene: &'a Scene, name: &str) -> &'a Node {
    let node = find_node(scene, name);
    ufbxt_assert!(node.is_some());
    &scene.nodes[node.unwrap()]
}

file_test!(max_geometry_transform, |scene, err, load_error| {
    ufbxt_assert!(scene.nodes.len() == 3);

    {
        let node = find(scene, "Box001");
        ufbxt_assert!(!node.is_geometry_transform_helper);
        ufbxt_assert!(node.geometry_transform_helper.is_none());
        ufbxt_assert!(node.mesh.is_some());

        let local_transform = ref_transform([0.0, -10.0, 0.0], [0.0, 0.0, -90.0], [1.0, 2.0, 1.0]);
        let geometry_transform = ref_transform([0.0, 0.0, 10.0], [0.0, 90.0, 0.0], [1.0, 1.0, 2.0]);

        check_transform(err, "Box001 local", node.local_transform, local_transform);
        check_transform(err, "Box001 geometry", node.geometry_transform, geometry_transform);
    }

    {
        let node = find(scene, "Box002");
        ufbxt_assert!(!node.is_geometry_transform_helper);
        ufbxt_assert!(node.geometry_transform_helper.is_none());
        ufbxt_assert!(node.mesh.is_some());

        let local_transform = ref_transform([0.0, 0.0, 20.0], [0.0, 0.0, -180.0], [1.0, 0.5, 1.0]);
        let geometry_transform = ref_transform([10.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);

        check_transform(err, "Box002 local", node.local_transform, local_transform);
        check_transform(err, "Box002 geometry", node.geometry_transform, geometry_transform);
    }
});

file_test!(
    max_geometry_transform_helper,
    file = "max_geometry_transform",
    opts = geometry_transform_helper_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS,
    |scene, err, load_error| {
        ufbxt_assert!(scene.nodes.len() == 5);

        {
            let node = find(scene, "Box001");
            ufbxt_assert!(!node.is_geometry_transform_helper);
            ufbxt_assert!(node.geometry_transform_helper.is_some());
            ufbxt_assert!(node.mesh.is_none());

            let local_transform = ref_transform([0.0, -10.0, 0.0], [0.0, 0.0, -90.0], [1.0, 2.0, 1.0]);
            let geometry_transform = ref_transform([0.0, 0.0, 10.0], [0.0, 90.0, 0.0], [1.0, 1.0, 2.0]);

            check_transform(err, "Box001 local", node.local_transform, local_transform);
            check_transform(err, "Box001 geometry", node.geometry_transform, REF_TRANSFORM_IDENTITY);

            let node = &scene.nodes[node.geometry_transform_helper.unwrap()];
            ufbxt_assert!(node.is_geometry_transform_helper);
            ufbxt_assert!(node.geometry_transform_helper.is_none());
            ufbxt_assert!(node.mesh.is_some());

            check_transform(err, "Box001 helper local", node.local_transform, geometry_transform);
            check_transform(err, "Box001 helper geometry", node.geometry_transform, REF_TRANSFORM_IDENTITY);
        }

        {
            let node = find(scene, "Box002");
            ufbxt_assert!(!node.is_geometry_transform_helper);
            ufbxt_assert!(node.geometry_transform_helper.is_some());
            ufbxt_assert!(node.mesh.is_none());

            let local_transform = ref_transform([0.0, 0.0, 20.0], [0.0, 0.0, -180.0], [1.0, 0.5, 1.0]);
            let geometry_transform = ref_transform([10.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);

            check_transform(err, "Box002 local", node.local_transform, local_transform);
            check_transform(err, "Box002 geometry", node.geometry_transform, REF_TRANSFORM_IDENTITY);

            let node = &scene.nodes[node.geometry_transform_helper.unwrap()];
            ufbxt_assert!(node.is_geometry_transform_helper);
            ufbxt_assert!(node.geometry_transform_helper.is_none());
            ufbxt_assert!(node.mesh.is_some());

            check_transform(err, "Box002 helper local", node.local_transform, geometry_transform);
            check_transform(err, "Box002 helper geometry", node.geometry_transform, REF_TRANSFORM_IDENTITY);
        }
    }
);

file_test!(
    max_geometry_transform_modify,
    file = "max_geometry_transform",
    opts = geometry_transform_modify_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS,
    |scene, err, load_error| {
        ufbxt_assert!(scene.nodes.len() == 3);

        {
            let node = find(scene, "Box001");
            ufbxt_assert!(!node.is_geometry_transform_helper);
            ufbxt_assert!(node.geometry_transform_helper.is_none());
            ufbxt_assert!(node.mesh.is_some());

            let local_transform = ref_transform([0.0, -10.0, 0.0], [0.0, 0.0, -90.0], [1.0, 2.0, 1.0]);

            check_transform(err, "Box001 local", node.local_transform, local_transform);
            check_transform(err, "Box001 geometry", node.geometry_transform, REF_TRANSFORM_IDENTITY);
        }

        {
            let node = find(scene, "Box002");
            ufbxt_assert!(!node.is_geometry_transform_helper);
            ufbxt_assert!(node.geometry_transform_helper.is_none());
            ufbxt_assert!(node.mesh.is_some());

            let local_transform = ref_transform([0.0, 0.0, 20.0], [0.0, 0.0, -180.0], [1.0, 0.5, 1.0]);

            check_transform(err, "Box002 local", node.local_transform, local_transform);
            check_transform(err, "Box002 geometry", node.geometry_transform, REF_TRANSFORM_IDENTITY);
        }
    }
);

#[test]
fn max_geometry_transform_lefthanded() {
    let mut err = DiffError::default();

    let obj_file = load_obj_file("max_geometry_transform_lefthanded", None);

    for path in FileIterator::new("max_geometry_transform") {
        for (axis_ix, handedness_conversion_axis) in MIRROR_AXES.into_iter().enumerate() {
            for (handling_ix, geometry_transform_handling) in GEOMETRY_TRANSFORM_HANDLINGS.into_iter().enumerate() {
                ufbxt_hintf!("handedness_conversion_axis={axis_ix} geometry_transform_handling={handling_ix}");

                let opts = LoadOpts {
                    target_axes: CoordinateAxes::LEFT_HANDED_Z_UP,
                    handedness_conversion_axis,
                    geometry_transform_handling,
                    reverse_winding: axis_ix == 0,
                    ..Default::default()
                };

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

#[test]
fn max_geometry_transform_lefthanded_adjust() {
    let mut err = DiffError::default();

    let obj_file = load_obj_file("max_geometry_transform_lefthanded", None);

    for path in FileIterator::new("max_geometry_transform") {
        for (axis_ix, handedness_conversion_axis) in MIRROR_AXES.into_iter().enumerate().skip(1) {
            for (handling_ix, geometry_transform_handling) in GEOMETRY_TRANSFORM_HANDLINGS.into_iter().enumerate() {
                ufbxt_hintf!("handedness_conversion_axis={axis_ix} geometry_transform_handling={handling_ix}");

                let opts = LoadOpts {
                    target_axes: CoordinateAxes::LEFT_HANDED_Z_UP,
                    space_conversion: SpaceConversion::AdjustTransforms,
                    handedness_conversion_axis,
                    geometry_transform_handling,
                    ..Default::default()
                };

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

file_test!(no_unnecessary_geometry_helpers, file = "maya_cube", opts = geometry_transform_helper_opts(), |scene, err, load_error| {
    ufbxt_assert!(scene.nodes.len() == 2);
    let node = find(scene, "pCube1");
    ufbxt_assert!(!node.is_geometry_transform_helper);
    ufbxt_assert!(node.geometry_transform_helper.is_none());
});

file_test!(
    max_transformed_skin_helpers,
    file = "max_transformed_skin",
    opts = geometry_transform_helper_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS,
    |scene, err, load_error| {
        let node = find(scene, "Box001");
        ufbxt_assert!(node.mesh.is_none());
        ufbxt_assert!(!node.is_geometry_transform_helper);
        ufbxt_assert!(node.geometry_transform_helper.is_some());
        ufbxt_assert!(!node.has_geometry_transform);
        let geo_node = &scene.nodes[node.geometry_transform_helper.unwrap()];
        ufbxt_assert!(geo_node.mesh.is_some());
        ufbxt_assert!(geo_node.is_geometry_transform_helper);
        ufbxt_assert!(geo_node.geometry_transform_helper.is_none());
        ufbxt_assert!(!geo_node.has_geometry_transform);

        check_frame(scene, err, false, "max_transformed_skin_5", None, 5.0 / 30.0);
        check_frame(scene, err, false, "max_transformed_skin_15", None, 15.0 / 30.0);
    }
);

file_test!(
    max_transformed_skin_modify,
    file = "max_transformed_skin",
    opts = geometry_transform_modify_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS,
    |scene, err, load_error| {
        let node = find(scene, "Box001");
        ufbxt_assert!(node.mesh.is_some());
        ufbxt_assert!(!node.is_geometry_transform_helper);
        ufbxt_assert!(node.geometry_transform_helper.is_none());
        ufbxt_assert!(!node.has_geometry_transform);

        check_frame(scene, err, false, "max_transformed_skin_5", None, 5.0 / 30.0);
        check_frame(scene, err, false, "max_transformed_skin_15", None, 15.0 / 30.0);
    }
);

file_test!(max_geometry_transform_instances, |scene, err, load_error| {
    if scene.metadata.version >= 7000 {
        ufbxt_assert!(scene.meshes.len() == 1);
    } else {
        ufbxt_assert!(scene.meshes.len() == 4);
    }

    ufbxt_assert!(scene.nodes.len() == 5);
    for node in &scene.nodes[1..] {
        ufbxt_assert!(node.has_geometry_transform);
        ufbxt_assert!(node.mesh.is_some());
        ufbxt_assert!(!node.is_geometry_transform_helper);
        ufbxt_assert!(node.geometry_transform_helper.is_none());
    }
});

file_test!(
    max_geometry_transform_instances_helper,
    file = "max_geometry_transform_instances",
    opts = geometry_transform_helper_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS,
    |scene, err, load_error| {
        if scene.metadata.version >= 7000 {
            ufbxt_assert!(scene.meshes.len() == 1);
        } else {
            ufbxt_assert!(scene.meshes.len() == 4);
        }

        ufbxt_assert!(scene.nodes.len() == 9);
        for node in &scene.nodes[1..] {
            ufbxt_assert!(!node.has_geometry_transform);
            if !node.element.name.is_empty() {
                ufbxt_assert!(node.mesh.is_none());
                ufbxt_assert!(!node.is_geometry_transform_helper);
                ufbxt_assert!(node.geometry_transform_helper.is_some());
            } else {
                ufbxt_assert!(node.mesh.is_some());
                ufbxt_assert!(node.is_geometry_transform_helper);
                ufbxt_assert!(node.geometry_transform_helper.is_none());
            }
        }
    }
);

file_test!(
    max_geometry_transform_instances_modify,
    file = "max_geometry_transform_instances",
    opts = geometry_transform_modify_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS,
    |scene, err, load_error| {
        if scene.metadata.version >= 7000 {
            ufbxt_assert!(scene.meshes.len() == 1);
        } else {
            ufbxt_assert!(scene.meshes.len() == 4);
        }

        if scene.metadata.version >= 7000 {
            ufbxt_assert!(scene.nodes.len() == 9);
            for node in &scene.nodes[1..] {
                ufbxt_assert!(!node.has_geometry_transform);
                if !node.element.name.is_empty() {
                    ufbxt_assert!(node.mesh.is_none());
                    ufbxt_assert!(!node.is_geometry_transform_helper);
                    ufbxt_assert!(node.geometry_transform_helper.is_some());
                } else {
                    ufbxt_assert!(node.mesh.is_some());
                    ufbxt_assert!(node.is_geometry_transform_helper);
                    ufbxt_assert!(node.geometry_transform_helper.is_none());
                }
            }
        } else {
            ufbxt_assert!(scene.nodes.len() == 5);
            for node in &scene.nodes[1..] {
                ufbxt_assert!(!node.has_geometry_transform);
                ufbxt_assert!(node.mesh.is_some());
                ufbxt_assert!(!node.is_geometry_transform_helper);
                ufbxt_assert!(node.geometry_transform_helper.is_none());
            }
        }
    }
);

file_test!(
    max_geometry_transform_instances_modify_no_fallback,
    file = "max_geometry_transform_instances",
    opts = geometry_transform_modify_no_fallback_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS | DIFF_EXPECT_FAIL_POST_7000,
    |scene, err, load_error| {
        if scene.metadata.version >= 7000 {
            ufbxt_assert!(scene.meshes.len() == 1);
        } else {
            ufbxt_assert!(scene.meshes.len() == 4);
        }

        ufbxt_assert!(scene.nodes.len() == 5);
        for node in &scene.nodes[1..] {
            ufbxt_assert!(!node.has_geometry_transform);
            ufbxt_assert!(node.mesh.is_some());
            ufbxt_assert!(!node.is_geometry_transform_helper);
            ufbxt_assert!(node.geometry_transform_helper.is_none());
        }
    }
);

file_test!(
    max_geometry_transform_helper_names,
    file = "max_geometry_transform",
    opts = geometry_transform_helper_name_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS,
    |scene, err, load_error| {
        ufbxt_assert!(scene.nodes.len() == 5);

        {
            let node = &scene.nodes[find_node(scene, "Box001").unwrap()];
            ufbxt_assert!(node.geometry_transform_helper.is_some());
            let node = &scene.nodes[node.geometry_transform_helper.unwrap()];
            ufbxt_assert!(node.element.name == "(ufbxt helper)");
        }

        {
            let node = &scene.nodes[find_node(scene, "Box002").unwrap()];
            ufbxt_assert!(node.geometry_transform_helper.is_some());
            let node = &scene.nodes[node.geometry_transform_helper.unwrap()];
            ufbxt_assert!(node.element.name == "(ufbxt helper)");
        }
    }
);

file_test!(max_geometry_transform_types, |scene, err, load_error| {
    {
        let node = find(scene, "Plane001");
        check_transform(err, "Plane001 local", node.local_transform, REF_TRANSFORM_IDENTITY);
        ufbxt_assert!(node.has_geometry_transform);
        ufbxt_assert!(node.attrib_type == ElementType::Mesh);
    }

    {
        let node = find(scene, "Line001");
        check_transform(err, "Line001 local", node.local_transform, REF_TRANSFORM_IDENTITY);
        ufbxt_assert!(node.has_geometry_transform);
        if scene.metadata.version >= 7000 {
            ufbxt_assert!(node.attrib_type == ElementType::LineCurve);
        } else {
            ufbxt_assert!(node.attrib_type == ElementType::Empty);
        }
    }

    {
        let node = find(scene, "Curve001");
        check_transform(err, "Curve001 local", node.local_transform, REF_TRANSFORM_IDENTITY);
        ufbxt_assert!(node.has_geometry_transform);
        ufbxt_assert!(node.attrib_type == ElementType::NurbsCurve);
    }

    {
        let node = find(scene, "Surface001");
        check_transform(err, "Surface001 local", node.local_transform, REF_TRANSFORM_IDENTITY);
        ufbxt_assert!(node.has_geometry_transform);
        ufbxt_assert!(node.attrib_type == ElementType::NurbsSurface);
    }
});

file_test!(
    max_geometry_transform_types_helper,
    file = "max_geometry_transform_types",
    opts = geometry_transform_helper_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS,
    |scene, err, load_error| {
        {
            let node = find(scene, "Plane001");
            check_transform(err, "Plane001 local", node.local_transform, REF_TRANSFORM_IDENTITY);
            check_transform(err, "Plane001 geometry", node.geometry_transform, REF_TRANSFORM_IDENTITY);
            ufbxt_assert!(!node.has_geometry_transform);
            ufbxt_assert!(node.attrib.is_none());
            ufbxt_assert!(node.geometry_transform_helper.is_some());
            let node = &scene.nodes[node.geometry_transform_helper.unwrap()];
            ufbxt_assert!(node.attrib_type == ElementType::Mesh);
        }

        {
            let node = find(scene, "Line001");
            check_transform(err, "Line001 local", node.local_transform, REF_TRANSFORM_IDENTITY);
            check_transform(err, "Line001 geometry", node.geometry_transform, REF_TRANSFORM_IDENTITY);
            ufbxt_assert!(!node.has_geometry_transform);
            ufbxt_assert!(node.attrib.is_none());
            ufbxt_assert!(node.geometry_transform_helper.is_some());
            let node = &scene.nodes[node.geometry_transform_helper.unwrap()];
            if scene.metadata.version >= 7000 {
                ufbxt_assert!(node.attrib_type == ElementType::LineCurve);
            } else {
                ufbxt_assert!(node.attrib_type == ElementType::Empty);
            }
        }

        {
            let node = find(scene, "Curve001");
            check_transform(err, "Curve001 local", node.local_transform, REF_TRANSFORM_IDENTITY);
            check_transform(err, "Curve001 geometry", node.geometry_transform, REF_TRANSFORM_IDENTITY);
            ufbxt_assert!(!node.has_geometry_transform);
            ufbxt_assert!(node.attrib.is_none());
            ufbxt_assert!(node.geometry_transform_helper.is_some());
            let node = &scene.nodes[node.geometry_transform_helper.unwrap()];
            ufbxt_assert!(node.attrib_type == ElementType::NurbsCurve);
        }

        {
            let node = find(scene, "Surface001");
            check_transform(err, "Surface001 local", node.local_transform, REF_TRANSFORM_IDENTITY);
            check_transform(err, "Surface001 geometry", node.geometry_transform, REF_TRANSFORM_IDENTITY);
            ufbxt_assert!(!node.has_geometry_transform);
            ufbxt_assert!(node.attrib.is_none());
            ufbxt_assert!(node.geometry_transform_helper.is_some());
            let node = &scene.nodes[node.geometry_transform_helper.unwrap()];
            ufbxt_assert!(node.attrib_type == ElementType::NurbsSurface);
        }
    }
);

file_test!(
    max_geometry_transform_types_modify,
    file = "max_geometry_transform_types",
    opts = geometry_transform_modify_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS,
    |scene, err, load_error| {
        {
            let node = find(scene, "Plane001");
            check_transform(err, "Plane001 local", node.local_transform, REF_TRANSFORM_IDENTITY);
            check_transform(err, "Plane001 geometry", node.geometry_transform, REF_TRANSFORM_IDENTITY);
            ufbxt_assert!(!node.has_geometry_transform);

            ufbxt_assert!(node.mesh.is_some());
            let mesh = &scene.meshes[node.mesh.unwrap()];

            ufbxt_assert!(mesh.vertex_normal.exists);
            ufbxt_assert!(mesh.vertex_tangent.exists);
            ufbxt_assert!(mesh.vertex_bitangent.exists);

            let ref_positions = [Vec3::new(20.0, 10.0, 0.0), Vec3::new(0.0, 10.0, 10.0), Vec3::new(20.0, -10.0, 0.0)];

            let ref_normal = Vec3::new(0.447213, 0.0, 0.894427);
            let ref_tangent = Vec3::new(0.0, -1.0, 0.0);
            let ref_bitangent = Vec3::new(0.894427, 0.0, -0.447213);

            ufbxt_assert!(mesh.num_indices == 3);
            for (i, ref_position) in ref_positions.into_iter().enumerate() {
                ufbxt_hintf!("i={i}");

                let position = mesh.vertex_position.get(i);
                let normal = mesh.vertex_normal.get(i);
                let tangent = mesh.vertex_tangent.get(i);
                let bitangent = mesh.vertex_bitangent.get(i);

                assert_close_vec3(err, position, ref_position);
                assert_close_vec3(err, normal, ref_normal);
                assert_close_vec3(err, tangent, ref_tangent);
                assert_close_vec3(err, bitangent, ref_bitangent);
            }
        }

        {
            let node = find(scene, "Line001");
            check_transform(err, "Line001 local", node.local_transform, REF_TRANSFORM_IDENTITY);
            check_transform(err, "Line001 geometry", node.geometry_transform, REF_TRANSFORM_IDENTITY);
            ufbxt_assert!(!node.has_geometry_transform);

            let line = node.attrib.and_then(|a| scene.as_line_curve(a));
            if scene.metadata.version >= 7000 {
                ufbxt_assert!(line.is_some());
                let line = line.unwrap();

                let ref_positions = [Vec3::new(20.0, -10.0, 0.0), Vec3::new(20.0, 10.0, 0.0), Vec3::new(0.0, 10.0, 0.0)];

                ufbxt_assert!(line.point_indices.len() == 3);
                ufbxt_assert!(line.segments.len() == 1);
                ufbxt_assert!(line.segments[0].index_begin == 0);
                ufbxt_assert!(line.segments[0].num_indices == 3);
                for (i, ref_position) in ref_positions.into_iter().enumerate() {
                    ufbxt_hintf!("i={i}");

                    let position = line.control_points[line.point_indices[i] as usize];
                    assert_close_vec3(err, position, ref_position);
                }
            }
        }

        {
            let node = find(scene, "Curve001");
            check_transform(err, "Curve001 local", node.local_transform, REF_TRANSFORM_IDENTITY);
            check_transform(err, "Curve001 geometry", node.geometry_transform, REF_TRANSFORM_IDENTITY);
            ufbxt_assert!(!node.has_geometry_transform);

            let curve = node.attrib.and_then(|a| scene.as_nurbs_curve(a));
            ufbxt_assert!(curve.is_some());
            let curve = curve.unwrap();

            let ref_points = [
                Vec4::new(0.0, 10.0, 0.0, 1.0),
                Vec4::new(6.666666, 3.333333, 0.0, 1.0),
                Vec4::new(13.333333, -3.333333, 0.0, 1.0),
                Vec4::new(20.0, -10.0, 0.0, 1.0),
            ];

            ufbxt_assert!(curve.control_points.len() == 4);
            for (i, ref_point) in ref_points.into_iter().enumerate() {
                ufbxt_hintf!("i={i}");

                let point = curve.control_points[i];
                assert_close_vec4(err, point, ref_point);
            }
        }

        {
            let node = find(scene, "Surface001");
            check_transform(err, "Surface001 local", node.local_transform, REF_TRANSFORM_IDENTITY);
            check_transform(err, "Surface001 geometry", node.geometry_transform, REF_TRANSFORM_IDENTITY);
            ufbxt_assert!(!node.has_geometry_transform);

            let surface = node.attrib.and_then(|a| scene.as_nurbs_surface(a));
            ufbxt_assert!(surface.is_some());
            let surface = surface.unwrap();

            ufbxt_assert!(surface.num_control_points_u == 4);
            ufbxt_assert!(surface.num_control_points_v == 4);
            for v in 0..4 {
                for u in 0..4 {
                    ufbxt_hintf!("u={u}, v={v}");

                    let ref_point = Vec4::new(v as Real * 6.666666, 10.0 - u as Real * 6.666666, 0.0, 1.0);

                    let point = surface.control_points[v * 4 + u];
                    assert_close_vec4(err, point, ref_point);
                }
            }
        }
    }
);

file_test!(blender_340_z_up, flags = DIFF_EXPECT_FAIL, |scene, err, load_error| {});

file_test!(
    blender_340_z_up_root,
    file = "blender_340_z_up",
    opts = y_up_meters_root_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS,
    |scene, err, load_error| {
        let root = &scene.nodes[scene.root_node];
        let root_transform = ref_transform([0.0, 0.0, 0.0], [-90.0, 0.0, 0.0], [0.01, 0.01, 0.01]);
        check_transform(err, "root", root.local_transform, root_transform);
    }
);

file_test!(
    blender_340_z_up_adjust,
    file = "blender_340_z_up",
    opts = y_up_meters_adjust_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS,
    |scene, err, load_error| {
        let root = &scene.nodes[scene.root_node];
        check_transform(err, "root", root.local_transform, REF_TRANSFORM_IDENTITY);

        let mut num_adjusted = 0;
        for node in &scene.nodes {
            if node.node_depth == 1 {
                ufbxt_assert!(node.has_adjust_transform);
                let rotation = Vec3::new(-90.0, 0.0, 0.0);
                let rotation_quat = euler_to_quat(rotation, RotationOrder::Xyz);
                assert_close_quat(err, node.adjust_post_rotation, Quat::IDENTITY);
                assert_close_quat(err, node.adjust_pre_rotation, rotation_quat);
                assert_close_real(err, node.adjust_pre_scale, 0.01);
                num_adjusted += 1;
            } else {
                ufbxt_assert!(!node.has_adjust_transform);
                assert_close_quat(err, node.adjust_post_rotation, Quat::IDENTITY);
                assert_close_quat(err, node.adjust_pre_rotation, Quat::IDENTITY);
                assert_close_real(err, node.adjust_pre_scale, 1.0);
            }
        }

        {
            let node = find(scene, "Light");
            let ref_transform = ref_transform([4.0, 5.0, -1.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
            check_transform(err, "Light", node.local_transform, ref_transform);
        }

        {
            let node = find(scene, "Camera");
            let ref_transform = ref_transform([7.0, 5.0, -6.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
            check_transform(err, "Camera", node.local_transform, ref_transform);
        }

        {
            let node = find(scene, "Cube");
            let ref_transform = ref_transform([0.0, 1.0, -1.0], [-90.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
            check_transform(err, "Camera", node.local_transform, ref_transform);
        }

        ufbxt_assert!(scene.nodes.len() == 5);
        ufbxt_assert!(num_adjusted == 3);
    }
);

file_test!(blender_340_y_up, flags = DIFF_EXPECT_FAIL, |scene, err, load_error| {});

file_test!(
    blender_340_y_up_root,
    file = "blender_340_y_up",
    opts = z_up_meters_root_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS,
    |scene, err, load_error| {
        let root = &scene.nodes[scene.root_node];
        let root_transform = ref_transform([0.0, 0.0, 0.0], [90.0, 0.0, 0.0], [0.01, 0.01, 0.01]);
        check_transform(err, "root", root.local_transform, root_transform);
    }
);

file_test!(
    blender_340_y_up_adjust,
    file = "blender_340_y_up",
    opts = z_up_meters_adjust_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS,
    |scene, err, load_error| {
        let root = &scene.nodes[scene.root_node];
        check_transform(err, "root", root.local_transform, REF_TRANSFORM_IDENTITY);

        let mut num_adjusted = 0;
        for node in &scene.nodes {
            if node.node_depth == 1 {
                ufbxt_assert!(node.has_adjust_transform);
                let rotation = Vec3::new(90.0, 0.0, 0.0);
                let rotation_quat = euler_to_quat(rotation, RotationOrder::Xyz);
                assert_close_quat(err, node.adjust_post_rotation, Quat::IDENTITY);
                assert_close_quat(err, node.adjust_pre_rotation, rotation_quat);
                assert_close_real(err, node.adjust_pre_scale, 0.01);
                num_adjusted += 1;
            } else {
                ufbxt_assert!(!node.has_adjust_transform);
                assert_close_quat(err, node.adjust_post_rotation, Quat::IDENTITY);
                assert_close_quat(err, node.adjust_pre_rotation, Quat::IDENTITY);
                assert_close_real(err, node.adjust_pre_scale, 1.0);
            }
        }

        {
            let node = find(scene, "Light");
            let ref_transform = ref_transform([4.0, 1.0, 5.0], [90.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
            check_transform(err, "Light", node.local_transform, ref_transform);
        }

        {
            let node = find(scene, "Camera");
            let ref_transform = ref_transform([7.0, 6.0, 5.0], [90.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
            check_transform(err, "Camera", node.local_transform, ref_transform);
        }

        {
            let node = find(scene, "Cube");
            let ref_transform = ref_transform([0.0, 1.0, 1.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
            check_transform(err, "Camera", node.local_transform, ref_transform);
        }

        ufbxt_assert!(scene.nodes.len() == 5);
        ufbxt_assert!(num_adjusted == 3);
    }
);

file_test!(
    blender_340_y_up_modify,
    file = "blender_340_y_up",
    opts = z_up_meters_modify_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS,
    |scene, err, load_error| {
        let root = &scene.nodes[scene.root_node];
        check_transform(err, "root", root.local_transform, REF_TRANSFORM_IDENTITY);

        let mut num_adjusted = 0;
        for node in &scene.nodes {
            if node.is_root {
                ufbxt_assert!(!node.has_adjust_transform);
                assert_close_quat(err, node.adjust_post_rotation, Quat::IDENTITY);
                assert_close_quat(err, node.adjust_pre_rotation, Quat::IDENTITY);
                assert_close_real(err, node.adjust_translation_scale, 1.0);
            } else if node.node_depth == 1 {
                ufbxt_assert!(node.has_adjust_transform);
                let rotation = Vec3::new(90.0, 0.0, 0.0);
                let rotation_quat = euler_to_quat(rotation, RotationOrder::Xyz);
                assert_close_quat(err, node.adjust_post_rotation, Quat::IDENTITY);
                assert_close_quat(err, node.adjust_pre_rotation, rotation_quat);
                assert_close_real(err, node.adjust_translation_scale, 0.01);
                num_adjusted += 1;
            } else {
                ufbxt_assert!(node.has_adjust_transform);
                assert_close_quat(err, node.adjust_post_rotation, Quat::IDENTITY);
                assert_close_quat(err, node.adjust_pre_rotation, Quat::IDENTITY);
                assert_close_real(err, node.adjust_translation_scale, 0.01);
            }
        }

        {
            let node = find(scene, "Light");
            let ref_transform = ref_transform([4.0, 1.0, 5.0], [90.0, 0.0, 0.0], [100.0, 100.0, 100.0]);
            check_transform(err, "Light", node.local_transform, ref_transform);
        }

        {
            let node = find(scene, "Camera");
            let ref_transform = ref_transform([7.0, 6.0, 5.0], [90.0, 0.0, 0.0], [100.0, 100.0, 100.0]);
            check_transform(err, "Camera", node.local_transform, ref_transform);
        }

        {
            let node = find(scene, "Cube");
            let ref_transform = ref_transform([0.0, 1.0, 1.0], [0.0, 0.0, 0.0], [100.0, 100.0, 100.0]);
            check_transform(err, "Camera", node.local_transform, ref_transform);
        }

        ufbxt_assert!(scene.nodes.len() == 5);
        ufbxt_assert!(num_adjusted == 3);
    }
);

file_test!(maya_camera_light_axes_y_up, |scene, err, load_error| {
    {
        let node = find(scene, "forwardCamera");
        let ref_transform = ref_transform([0.0, 0.0, 0.0], [0.0, 90.0, 0.0], [1.0, 1.0, 1.0]);
        check_transform(err, "forwardCamera", node.local_transform, ref_transform);
    }

    {
        let node = find(scene, "forwardLight");
        let ref_transform = ref_transform([0.0, 0.0, 0.0], [90.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        check_transform(err, "forwardLight", node.local_transform, ref_transform);
    }
});

file_test!(
    maya_camera_light_axes_y_up_rh_y,
    file = "maya_camera_light_axes_y_up",
    opts = rh_y_camera_light_axes_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS,
    |scene, err, load_error| {
        {
            let node = find(scene, "forwardCamera");
            let ref_transform = ref_transform([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
            check_transform(err, "forwardCamera", node.local_transform, ref_transform);
        }

        {
            let node = find(scene, "forwardLight");
            let ref_transform = ref_transform([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
            check_transform(err, "forwardLight", node.local_transform, ref_transform);
        }
    }
);

file_test!(blender_340_default_unscaled, opts = blender_space_adjust_opts(), |scene, err, load_error| {
    {
        let node = find(scene, "Cube");
        let ref_transform = ref_transform([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        check_transform(err, "Cube", node.local_transform, ref_transform);
    }

    {
        let node = find(scene, "Camera");
        let ref_transform = ref_transform([7.3589, -6.9258, 4.9583], [63.5593, 0.0, 46.6919], [1.0, 1.0, 1.0]);
        check_transform(err, "Camera", node.local_transform, ref_transform);

        ufbxt_assert!(node.camera.is_some());
        let camera = &scene.cameras[node.camera.unwrap()];
        ufbxt_assert!(camera.projection_axes.right == CoordinateAxis::PositiveX);
        ufbxt_assert!(camera.projection_axes.up == CoordinateAxis::PositiveY);
        ufbxt_assert!(camera.projection_axes.front == CoordinateAxis::PositiveZ);
    }

    {
        let node = find(scene, "Light");
        let ref_transform = ref_transform([4.0762, 1.0055, 5.9039], [37.261, 3.16371, 106.936], [1.0, 1.0, 1.0]);
        check_transform(err, "Light", node.local_transform, ref_transform);

        ufbxt_assert!(node.light.is_some());
        let light = &scene.lights[node.light.unwrap()];

        let ref_dir = Vec3::new(0.0, 0.0, -1.0);
        assert_close_vec3(err, light.local_direction, ref_dir);
    }
});

file_test!(max_geometry_transform_order, |scene, err, load_error| {
    ufbxt_assert!(scene.nodes.len() == 3);
    let nodes = &scene.nodes;

    ufbxt_assert!(nodes[0].element.name.is_empty());
    ufbxt_assert!(nodes[0].is_root);
    ufbxt_assert!(nodes[0].node_depth == 0);
    ufbxt_assert!(nodes[0].parent.is_none());

    ufbxt_assert!(nodes[1].element.name == "Box001");
    ufbxt_assert!(nodes[1].has_geometry_transform);
    ufbxt_assert!(nodes[1].node_depth == 1);
    ufbxt_assert!(nodes[1].parent == Some(0));

    ufbxt_assert!(nodes[2].element.name == "Box002");
    ufbxt_assert!(nodes[2].has_geometry_transform);
    ufbxt_assert!(nodes[2].node_depth == 2);
    ufbxt_assert!(nodes[2].parent == Some(1));
});

file_test!(
    max_geometry_transform_order_helper,
    file = "max_geometry_transform_order",
    opts = geometry_transform_helper_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS,
    |scene, err, load_error| {
        ufbxt_assert!(scene.nodes.len() == 5);
        let nodes = &scene.nodes;

        ufbxt_assert!(nodes[0].element.name.is_empty());
        ufbxt_assert!(nodes[0].is_root);
        ufbxt_assert!(nodes[0].node_depth == 0);
        ufbxt_assert!(nodes[0].parent.is_none());

        ufbxt_assert!(nodes[1].element.name == "Box001");
        ufbxt_assert!(nodes[1].geometry_transform_helper.is_some());
        ufbxt_assert!(nodes[1].node_depth == 1);
        ufbxt_assert!(nodes[1].parent == Some(0));

        ufbxt_assert!(nodes[2].element.name.is_empty());
        ufbxt_assert!(nodes[2].is_geometry_transform_helper);
        ufbxt_assert!(nodes[2].node_depth == 2);
        ufbxt_assert!(nodes[2].parent == Some(1));

        ufbxt_assert!(nodes[3].element.name == "Box002");
        ufbxt_assert!(nodes[3].geometry_transform_helper.is_some());
        ufbxt_assert!(nodes[3].node_depth == 2);
        ufbxt_assert!(nodes[3].parent == Some(1));

        ufbxt_assert!(nodes[4].element.name.is_empty());
        ufbxt_assert!(nodes[4].is_geometry_transform_helper);
        ufbxt_assert!(nodes[4].node_depth == 3);
        ufbxt_assert!(nodes[4].parent == Some(3));
    }
);

file_test!(
    max_geometry_transform_order_modify,
    file = "max_geometry_transform_order",
    opts = geometry_transform_modify_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS,
    |scene, err, load_error| {
        ufbxt_assert!(scene.nodes.len() == 5);
        let nodes = &scene.nodes;

        ufbxt_assert!(nodes[0].element.name.is_empty());
        ufbxt_assert!(nodes[0].is_root);
        ufbxt_assert!(nodes[0].node_depth == 0);
        ufbxt_assert!(nodes[0].parent.is_none());

        ufbxt_assert!(nodes[1].element.name == "Box001");
        ufbxt_assert!(nodes[1].geometry_transform_helper.is_some());
        ufbxt_assert!(nodes[1].node_depth == 1);
        ufbxt_assert!(nodes[1].parent == Some(0));

        ufbxt_assert!(nodes[2].element.name.is_empty());
        ufbxt_assert!(nodes[2].is_geometry_transform_helper);
        ufbxt_assert!(nodes[2].node_depth == 2);
        ufbxt_assert!(nodes[2].parent == Some(1));

        ufbxt_assert!(nodes[3].element.name == "Box002");
        ufbxt_assert!(nodes[3].geometry_transform_helper.is_some());
        ufbxt_assert!(nodes[3].node_depth == 2);
        ufbxt_assert!(nodes[3].parent == Some(1));

        ufbxt_assert!(nodes[4].element.name.is_empty());
        ufbxt_assert!(nodes[4].is_geometry_transform_helper);
        ufbxt_assert!(nodes[4].node_depth == 3);
        ufbxt_assert!(nodes[4].parent == Some(3));
    }
);

file_test!(synthetic_geometry_transform_camera, flags = ALLOW_WARNINGS, |scene, err, load_error| {
    let node = find(scene, "Camera001");

    let geo_transform = ref_transform([10.0, 20.0, 30.0], [90.0, 0.0, 0.0], [1.0, 2.0, 3.0]);
    check_transform(err, "Camera geometry", node.geometry_transform, geo_transform);
});

file_test!(
    synthetic_geometry_transform_camera_helper,
    file = "synthetic_geometry_transform_camera",
    opts = geometry_transform_helper_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS | ALLOW_WARNINGS,
    |scene, err, load_error| {
        let node = find(scene, "Camera001");
        ufbxt_assert!(!node.has_geometry_transform);

        let geo_node = node.geometry_transform_helper;
        ufbxt_assert!(geo_node.is_some());
        let geo_node = &scene.nodes[geo_node.unwrap()];

        let geo_transform = ref_transform([10.0, 20.0, 30.0], [90.0, 0.0, 0.0], [1.0, 2.0, 3.0]);
        check_transform(err, "Camera geometry", geo_node.local_transform, geo_transform);
    }
);

file_test!(
    synthetic_geometry_transform_camera_modify,
    file = "synthetic_geometry_transform_camera",
    opts = geometry_transform_modify_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS | ALLOW_WARNINGS,
    |scene, err, load_error| {
        let node = find(scene, "Camera001");
        ufbxt_assert!(!node.has_geometry_transform);

        let geo_node = node.geometry_transform_helper;
        ufbxt_assert!(geo_node.is_some());
        let geo_node = &scene.nodes[geo_node.unwrap()];

        let geo_transform = ref_transform([10.0, 20.0, 30.0], [90.0, 0.0, 0.0], [1.0, 2.0, 3.0]);
        check_transform(err, "Camera geometry", geo_node.local_transform, geo_transform);
    }
);

file_test!(
    synthetic_geometry_transform_camera_modify_no_fallback,
    file = "synthetic_geometry_transform_camera",
    opts = geometry_transform_modify_no_fallback_opts(),
    flags = FUZZ_ALWAYS | FUZZ_OPTS | DIFF_ALWAYS | ALLOW_WARNINGS,
    |scene, err, load_error| {
        let node = find(scene, "Camera001");
        ufbxt_assert!(!node.has_geometry_transform);
    }
);

#[test]
fn synthetic_unnormalized() {
    let mut err = DiffError::default();

    for path in FileIterator::new("synthetic_unnormalized") {
        for normalize_normals in [false, true] {
            for normalize_tangents in [false, true] {
                ufbxt_hintf!("normalize_normals={} normalize_tangets={}", normalize_normals as i32, normalize_tangents as i32);
                let opts = LoadOpts { normalize_normals, normalize_tangents, ..Default::default() };
                let scene = load_file(&path, &opts);
                ufbxt_assert!(scene.is_ok());
                let scene = scene.unwrap();
                check_scene(&scene);

                let node = find_node(&scene, "pPlane1").map(|n| &scene.nodes[n]);
                ufbxt_assert!(node.is_some_and(|n| n.mesh.is_some()));
                let mesh = &scene.meshes[node.unwrap().mesh.unwrap()];

                ufbxt_assert!(mesh.vertex_normal.values.len() == 4);
                for i in 0..4 {
                    let mut r = Vec3::ZERO;
                    if normalize_normals {
                        r.y = if i > 0 { 1.0 } else { 0.0 };
                    } else {
                        r.y = i as Real;
                    }
                    assert_close_vec3(&mut err, mesh.vertex_normal.values[i], r);
                }

                ufbxt_assert!(mesh.vertex_tangent.values.len() == 4);
                for i in 0..4 {
                    let mut r = Vec3::ZERO;
                    if normalize_tangents {
                        r.x = if i > 0 { 1.0 } else { 0.0 };
                    } else {
                        r.x = i as Real;
                    }
                    assert_close_vec3(&mut err, mesh.vertex_tangent.values[i], r);
                }

                ufbxt_assert!(mesh.vertex_bitangent.values.len() == 4);
                for i in 0..4 {
                    let mut r = Vec3::ZERO;
                    if normalize_tangents {
                        r.z = if i > 0 { -1.0 } else { 0.0 };
                    } else {
                        r.z = -(i as Real);
                    }
                    assert_close_vec3(&mut err, mesh.vertex_bitangent.values[i], r);
                }
            }
        }
    }

    if err.num > 0 {
        let avg = err.sum / err.num as f64;
        ufbxt_logf!(".. Absolute diff: avg {avg:.3}, max {:.3} ({} tests)", err.max, err.num);
    }
}
