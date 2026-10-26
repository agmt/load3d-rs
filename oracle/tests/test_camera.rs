mod ufbxt;
use ufbxt::*;

const SPACE_CONVERSIONS: [SpaceConversion; 3] = [SpaceConversion::TransformRoot, SpaceConversion::AdjustTransforms, SpaceConversion::ModifyGeometry];

fn check_ortho_camera(err: &mut DiffError, scene: &Scene, name: &str, gate_fit: GateFit, extent: Real, width: Real, height: Real) {
    ufbxt_hintf!("Cameara {name}");
    let node = find_node(scene, name).map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.camera.is_some()));
    let camera = &scene.cameras[node.unwrap().camera.unwrap()];

    ufbxt_assert!(camera.projection_mode == ProjectionMode::Orthographic);
    ufbxt_assert!(camera.gate_fit == gate_fit);
    assert_close_real(err, camera.orthographic_extent, extent);
    assert_close_real(err, camera.orthographic_size.x, width);
    assert_close_real(err, camera.orthographic_size.y, height);
}

file_test!(maya_ortho_camera_400x200, |scene, err, load_error| {
    check_ortho_camera(err, scene, "Fill", GateFit::Fill, 30.0, 30.0, 15.0);
    check_ortho_camera(err, scene, "Horizontal", GateFit::Horizontal, 30.0, 30.0, 15.0);
    check_ortho_camera(err, scene, "Vertical", GateFit::Vertical, 30.0, 60.0, 30.0);
    check_ortho_camera(err, scene, "Overscan", GateFit::Overscan, 30.0, 60.0, 30.0);
});

file_test!(maya_ortho_camera_200x300, |scene, err, load_error| {
    check_ortho_camera(err, scene, "Fill", GateFit::Fill, 30.0, 20.0, 30.0);
    check_ortho_camera(err, scene, "Horizontal", GateFit::Horizontal, 30.0, 30.0, 45.0);
    check_ortho_camera(err, scene, "Vertical", GateFit::Vertical, 30.0, 20.0, 30.0);
    check_ortho_camera(err, scene, "Overscan", GateFit::Overscan, 30.0, 30.0, 45.0);
});

file_test!(maya_ortho_camera_size, |scene, err, load_error| {
    check_ortho_camera(err, scene, "Ortho_10", GateFit::Fill, 10.0, 10.0, 10.0);
    check_ortho_camera(err, scene, "Ortho_30", GateFit::Fill, 30.0, 30.0, 30.0);
    check_ortho_camera(err, scene, "Ortho_35", GateFit::Fill, 35.0, 35.0, 35.0);
    check_ortho_camera(err, scene, "Ortho_100", GateFit::Fill, 100.0, 100.0, 100.0);
});

file_test!(blender_402_ortho_scale_local, |scene, err, load_error| {
    check_ortho_camera(err, scene, "Ortho_1", GateFit::Horizontal, 1.0, 1.0, 1.0);
    check_ortho_camera(err, scene, "Ortho_2", GateFit::Horizontal, 2.0, 2.0, 2.0);
    check_ortho_camera(err, scene, "Ortho_3", GateFit::Horizontal, 3.0, 3.0, 3.0);
});

fn ortho_scale_conversion(file: &str) {
    let mut err = DiffError::default();
    for path in FileIterator::new(file) {
        for space_conversion in SPACE_CONVERSIONS {
            let opts = LoadOpts { space_conversion, target_unit_meters: 1.0, ..Default::default() };
            let scene = load_file(&path, &opts);
            ufbxt_assert!(scene.is_ok());
            let scene = scene.unwrap();
            check_ortho_camera(&mut err, &scene, "Ortho_1", GateFit::Horizontal, 1.0, 1.0, 1.0);
            check_ortho_camera(&mut err, &scene, "Ortho_2", GateFit::Horizontal, 2.0, 2.0, 2.0);
            check_ortho_camera(&mut err, &scene, "Ortho_3", GateFit::Horizontal, 3.0, 3.0, 3.0);
        }
    }
    ufbxt_logf!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
}

#[test]
fn blender_402_ortho_scale_local_conversion() {
    ortho_scale_conversion("blender_402_ortho_scale_local");
}

file_test!(blender_402_ortho_scale_unit, |scene, err, load_error| {
    check_ortho_camera(err, scene, "Ortho_1", GateFit::Horizontal, 1.0, 1.0, 1.0);
    check_ortho_camera(err, scene, "Ortho_2", GateFit::Horizontal, 2.0, 2.0, 2.0);
    check_ortho_camera(err, scene, "Ortho_3", GateFit::Horizontal, 3.0, 3.0, 3.0);
});

#[test]
fn blender_402_ortho_scale_unit_conversion() {
    ortho_scale_conversion("blender_402_ortho_scale_unit");
}
