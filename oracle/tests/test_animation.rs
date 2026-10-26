mod ufbxt;
use load3d::baked_key::{KEYFRAME, REDUCED, STEP_KEY, STEP_LEFT, STEP_RIGHT};
use ufbxt::*;

fn translation_curve(scene: &Scene, num_layers: usize) -> &AnimCurve {
    ufbxt_assert!(scene.anim_layers.len() == num_layers);
    let layer = &scene.anim_layers[0];
    let node = find_node(scene, "pCube1");
    ufbxt_assert!(node.is_some());
    let node = &scene.nodes[node.unwrap()];

    let aprop = find_anim_prop(layer, node.element.element_id, "Lcl Translation").unwrap();
    let curve = scene.anim_values[aprop.anim_value].curves[0];
    ufbxt_assert!(curve.is_some());
    &scene.anim_curves[curve.unwrap()]
}

fn check_curve_values(err: &mut DiffError, curve: &AnimCurve, values: &[f64], threshold: Option<Real>) {
    for (i, &ref_value) in values.iter().enumerate() {
        ufbxt_hintf!("i={i}");
        let time = i as f64 / 24.0;
        let value = evaluate_curve(curve, time, 0.0);
        match threshold {
            Some(threshold) => assert_close_real_threshold(err, value, ref_value, threshold),
            None => assert_close_real(err, value, ref_value),
        }
    }
}

file_test!(maya_interpolation_modes, |scene, err, load_error| {
    let values: &[f64] = &[
        -8.653366,
        -8.653366, -8.602998, -8.464664, -8.257528, -8.00075, -7.713489, -7.414906, -7.124163, -6.86042, -6.642837, -6.490576, -6.388305, -6.306414, -6.242637,
        -6.19471, -6.160368, -6.137348, -6.123385, -6.116215, -6.113573, -6.113196, -5.969524, -5.825851, -5.682179, -5.538507, -5.394835, -5.251163,
        -5.107491, -4.963819, -4.820146, -4.676474, -4.532802, -4.38913, -4.245458, -4.101785, -3.958113, -4.1529, -4.347686, -4.542472, -4.737258, -4.932045,
        -5.126832, -5.321618, -5.516404, -5.71119, -5.905977, -5.767788, -5.315578, -4.954943, -4.83559, -4.856855, -4.960766, -5.118543, -4.976541, -4.885909,
        -4.865979, -4.93845, -5.099224, -5.270246, -5.359269, -5.349404, -5.261964, -5.118543, -5.264501, -5.33535, -5.285445, -5.058857, -4.69383, -4.357775,
        -4.124978, -3.981697, -3.904232, -3.875225, -3.875225, -3.875225, -3.875225, -3.875225, -3.875225, -3.875225, -2.942738, -2.942738, -2.942738,
        -2.942738, -2.942738, -2.942738, -2.942738, -2.942738, -2.942738, -1.243537, -1.243537, -1.243537, -1.243537, -1.243537, -1.243537, -1.243537,
        5.603338, 5.603338, 5.603338, 5.603338, 5.603338, 5.603338, 5.603338, 5.603338, 5.603338, 5.603338, 5.603338, 5.603338, 5.603338, 5.603338, 5.603338,
        5.603338, 5.603338, 5.603338, 5.603338, 5.603338, 5.603338, 5.603338, 5.603338, 5.603338, 5.603338,
    ];

    ufbxt_assert!(scene.anim_layers.len() == 1);
    let layer = &scene.anim_layers[0];
    for &value_ix in &layer.anim_values {
        let anim_value = &scene.anim_values[value_ix];
        if anim_value.element.name != "Lcl Translation" {
            continue;
        }
        let curve = &scene.anim_curves[anim_value.curves[0].unwrap()];

        let num_keys = 12;
        ufbxt_assert!(curve.keyframes.len() == num_keys);
        let keys = &curve.keyframes;

        let key_ref: [(i32, f64); 12] = [
            (1, -8.653366),
            (11, -6.490576),
            (21, -6.113196),
            (36, -3.958113),
            (46, -5.905977),
            (53, -5.118543),
            (63, -5.118543),
            (73, -3.875225),
            (80, -2.942738),
            (89, -1.927362),
            (96, -1.243537),
            (120, 5.603338),
        ];
        ufbxt_assert!(key_ref.len() == num_keys);

        for i in 0..num_keys {
            assert_close_double(err, keys[i].time, key_ref[i].0 as f64 / 24.0);
            assert_close_real(err, keys[i].value, key_ref[i].1);
            if i > 0 {
                ufbxt_assert!(keys[i].left.dx > 0.0);
            }
            if i + 1 < num_keys {
                ufbxt_assert!(keys[i].right.dx > 0.0);
            }
        }

        ufbxt_assert!(keys[0].interpolation == Interpolation::Cubic);
        ufbxt_assert!(keys[0].right.dy == 0.0);
        ufbxt_assert!(keys[1].interpolation == Interpolation::Cubic);
        assert_close_real(err, (keys[1].left.dy / keys[1].left.dx) as Real, (keys[1].right.dy / keys[1].left.dx) as Real);
        ufbxt_assert!(keys[2].interpolation == Interpolation::Linear);
        assert_close_real(err, (keys[3].left.dy / keys[3].left.dx) as Real, (keys[2].right.dy / keys[2].right.dx) as Real);
        ufbxt_assert!(keys[3].interpolation == Interpolation::Linear);
        assert_close_real(err, (keys[4].left.dy / keys[4].left.dx) as Real, (keys[3].right.dy / keys[3].right.dx) as Real);
        ufbxt_assert!(keys[4].interpolation == Interpolation::Cubic);
        ufbxt_assert!(keys[4].right.dy == 0.0);
        ufbxt_assert!(keys[5].interpolation == Interpolation::Cubic);
        ufbxt_assert!(keys[5].left.dy < 0.0);
        ufbxt_assert!(keys[5].right.dy > 0.0);
        ufbxt_assert!(keys[6].interpolation == Interpolation::Cubic);
        ufbxt_assert!(keys[6].left.dy > 0.0);
        ufbxt_assert!(keys[6].right.dy < 0.0);
        ufbxt_assert!(keys[7].interpolation == Interpolation::ConstantPrev);
        ufbxt_assert!(keys[8].interpolation == Interpolation::ConstantPrev);
        ufbxt_assert!(keys[9].interpolation == Interpolation::ConstantNext);
        ufbxt_assert!(keys[10].interpolation == Interpolation::ConstantNext);

        for (i, &ref_value) in values.iter().enumerate() {
            // Round up to the next frame to make stepped tangents consistent
            let time = i as f64 * (1.0 / 24.0) + 0.000001;
            let value = evaluate_curve(curve, time, 0.0);
            assert_close_real(err, value, ref_value);
        }

        let num_samples = 64 * 1024;
        for i in 0..num_samples {
            let time = i as f64 * (5.0 / num_samples as f64);
            let value = evaluate_curve(curve, time, 0.0);
            ufbxt_assert!((-16.0..=16.0).contains(&value));
        }
    }
});

file_test!(maya_auto_clamp, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000, 0.000, 0.273, 0.515, 0.718, 0.868, 0.945, 0.920, 0.779, 0.611, 0.591, 0.747, 1.206, 2.059, 3.191, 4.489, 5.837, 7.121, 8.228, 9.042, 9.449,
        9.694, 10.128, 10.610, 10.873, 10.927, 10.854, 10.704, 10.502, 10.264, 10.000,
    ];

    ufbxt_assert!(scene.anim_layers.len() == 1);
    let layer = &scene.anim_layers[0];
    for &value_ix in &layer.anim_values {
        let anim_value = &scene.anim_values[value_ix];
        if anim_value.element.name != "Lcl Translation" {
            continue;
        }
        let curve = &scene.anim_curves[anim_value.curves[0].unwrap()];
        ufbxt_assert!(curve.keyframes.len() == 4);

        for (i, &ref_value) in values.iter().enumerate() {
            let time = i as f64 * (1.0 / 24.0);
            let value = evaluate_curve(curve, time, 0.0);
            assert_close_real(err, value, ref_value);
        }
    }
});

file_test!(maya_tangent_spline, |scene, err, load_error| {
    let values: &[f64] = &[
        0.00000, 3.50000, 6.00000, 6.51562, 6.37500, 6.04688, 6.00000, 6.67313, 7.71167, 8.39438, 8.00000, 3.63667, -1.04000, -1.96474, -1.68059, -1.00000,
        0.27700, 2.00000,
    ];

    let curve = translation_curve(scene, 1);
    check_curve_values(err, curve, values, None);
});

fn clamp_maya_opts() -> LoadOpts {
    LoadOpts { key_clamp_threshold: 0.05, ..Default::default() }
}

file_test!(maya_tangent_clamped, opts = clamp_maya_opts(), |scene, err, load_error| {
    let values: &[f64] = &[
        0.00000, 3.75000, 6.00000, 6.00000, 6.00000, 6.00000, 6.00000, 6.53250, 7.58667, 8.34750, 8.00000, 3.18667, -1.04000, -1.02963, -1.01037, -1.00000,
        0.12500, 2.00000,
    ];

    let curve = translation_curve(scene, 1);
    check_curve_values(err, curve, values, None);
});

file_test!(motionbuilder_tangent_linear, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000, 0.000000, 3.333333, 6.666667, 10.000000, 8.750000, 7.500000, 6.250000, 5.000000, 3.750000, 2.500000, 1.250000, 0.000000, 0.000000, 1.333333,
        2.666667, 4.000000, 0.000000, -4.000000, -4.000000, -4.000000, -4.000000, -4.000000, -1.000000, 2.000000, -1.000000, -4.000000, -4.000000,
    ];

    let curve = translation_curve(scene, 2);
    check_curve_values(err, curve, values, None);
});

file_test!(motionbuilder_tangent_auto, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000, 0.000000, 2.592592, 7.407408, 10.000000, 9.570312, 8.437500, 6.835938, 5.000000, 3.164062, 1.562500, 0.429688, 0.000000, 0.000000, 1.037037,
        2.962963, 4.000000, 0.000000, -4.000000, -4.000000, -4.000000, -4.000000, -4.000000, -1.000000, 2.000000, -1.000000, -4.000000, -4.000000,
    ];

    let curve = translation_curve(scene, 2);
    check_curve_values(err, curve, values, None);
});

file_test!(motionbuilder_tangent_spline, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000, 0.000000, 3.703704, 7.962963, 10.000000, 9.691840, 8.854167, 7.617188, 6.111111, 4.466146, 2.812500, 1.280382, 0.000000, 0.000000, 1.659259,
        3.540741, 4.000000, 0.133333, -4.000000, -4.937500, -5.166667, -4.812500, -4.000000, -0.750000, 2.000000, -0.625000, -4.000000, -4.375000,
    ];

    let curve = translation_curve(scene, 2);
    check_curve_values(err, curve, values, None);
});

file_test!(motionbuilder_tangent_spline_clamp, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000, 0.000000, 2.592592, 7.407408, 10.000000, 9.570312, 8.437500, 6.835938, 5.000000, 3.164062, 1.562500, 0.429688, 0.000000, 0.000000, 1.214815,
        3.318519, 4.000000, -0.200000, -4.000000, -4.000000, -4.000000, -4.000000, -4.000000, -1.000000, 2.000000, -1.000000, -4.000000, -4.000000,
    ];

    let curve = translation_curve(scene, 2);
    check_curve_values(err, curve, values, None);
});

file_test!(motionbuilder_tangent_smooth, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000, 2.851562, 5.937500, 8.554688, 10.000000, 10.117188, 9.375000, 8.007812, 6.250000, 4.335938, 2.500000, 0.976562, 0.000000, 0.000000, 1.148148,
        2.518519, 2.000000, -3.041667, -8.000000, -8.500000, -7.000000, -5.000000, -4.000000, -4.906250, -6.750000, -8.218750, -8.000000, -4.625000,
    ];

    let curve = translation_curve(scene, 2);
    check_curve_values(err, curve, values, None);
});

file_test!(motionbuilder_tangent_smooth_clamp, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000, 0.000000, 2.361111, 6.944445, 10.000000, 10.367839, 9.609375, 8.056641, 6.041667, 3.896484, 1.953125, 0.543620, 0.000000, 0.000000, 1.333333,
        3.555556, 4.000000, -0.333333, -4.000000, -4.000000, -4.000000, -4.000000, -4.000000, -1.000000, 2.000000, -1.000000, -4.000000, -4.000000,
    ];

    let curve = translation_curve(scene, 2);
    check_curve_values(err, curve, values, None);
});

fn check_tangent_weight(err: &mut DiffError, curve: &AnimCurve, values: &[f64]) {
    for (i, &ref_value) in values.iter().enumerate() {
        ufbxt_hintf!("i={i}");
        let time = i as f64 / 24.0;
        let value = evaluate_curve(curve, time, 0.0);
        if i == 37 {
            // Hard Bezier solve, might be an inaccuracy in MotionBuilder
            assert_close_real_threshold(err, value, ref_value, 0.01);
        } else {
            assert_close_real(err, value, ref_value);
        }
    }
}

file_test!(motionbuilder_tangent_weight, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000, 1.666723, 3.333501, 5.000291, 6.667066, 8.333776, 10.000000, 8.333776, 6.667066, 5.000291, 3.333500, 1.666723, 0.000000, -3.286567,
        -7.675018, -9.056619, -9.661684, -9.928119, -10.000000, -9.928119, -9.661685, -9.056619, -7.675017, -3.286567, 0.000000, 2.594488, 6.547710, 9.385767,
        10.524116, 10.590028, 10.000000, 8.817327, 6.850233, 3.941248, -0.101893, -5.382658, -10.000000, -2.000851, 2.786134, 5.659585, 7.615819, 9.005583,
        10.000000, 10.714197, 10.996990, 6.856248, 0.950144, 0.174010, 0.000000, 0.106695, 0.495261, 1.353501, 3.215956, 8.157769, 10.000000, 9.696111,
        7.804129, 5.550874, 3.545988, 1.713910, 0.000000,
    ];

    let curve = translation_curve(scene, 2);
    check_tangent_weight(err, curve, values);
});

file_test!(motionbuilder_tangent_weight_split, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000, 2.452671, 6.056693, 8.325726, 9.401590, 9.873834, 10.000000, 8.333776, 6.667066, 5.000291, 3.333500, 1.666723, 0.000000, -1.848064,
        -4.097735, -6.603552, -8.662155, -9.723830, -10.000000, -9.925971, -9.635168, -8.897875, -6.736949, -2.281059, 0.000000, 2.105399, 4.761521, 7.361103,
        9.316892, 10.244871, 10.000000, 4.726504, -0.810048, -5.004114, -7.855899, -9.481855, -10.000000, -2.000851, 2.786134, 5.659585, 7.615819, 9.005583,
        10.000000, 10.449002, 9.610980, 6.124925, 2.117854, 0.408753, 0.000000, 0.106695, 0.495261, 1.353501, 3.215956, 8.157769, 10.000000, 9.859852,
        9.295445, 7.872536, 5.005719, 2.033110, 0.000000,
    ];

    let curve = translation_curve(scene, 2);
    check_tangent_weight(err, curve, values);
});

file_test!(motionbuilder_tangent_velocity, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000, 1.306946, 3.600664, 6.350164, 8.505674, 9.667060, 10.000000, 9.640089, 8.020978, 4.811217, 2.299784, 0.849270, 0.000000, -0.841173,
        -2.236577, -4.544493, -7.658146, -9.554997, -10.000000, -9.524388, -7.843781, -5.036027, -2.355318, -0.707333, 0.000000, 0.699153, 2.165492, 4.372254,
        7.037397, 9.356667, 10.000000, 5.273193, -2.875481, -6.854944, -8.828015, -9.745026, -10.000000, -9.747821, -8.858396, -7.006244, -3.480528, 3.549764,
        10.000000, 8.909218, 6.263762, 3.659487, 1.646342, 0.411071, 0.000000, 0.375149, 1.510146, 3.380960, 5.851594, 8.490248, 10.000000, 7.836923, 4.849110,
        2.974404, 1.677659, 0.723459, 0.000000,
    ];

    let curve = translation_curve(scene, 2);
    check_curve_values(err, curve, values, Some(0.01));
});

file_test!(motionbuilder_tangent_velocity_split, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000, 1.306946, 3.600664, 6.350164, 8.505674, 9.667060, 10.000000, 9.715650, 8.870866, 7.474936, 5.532619, 3.043366, 0.000000, -0.877294,
        -2.555327, -6.032461, -8.885489, -9.799076, -10.000000, -8.879360, -5.191613, -2.533277, -1.116049, -0.358573, 0.000000, 5.621936, 8.243435, 9.649088,
        10.286535, 10.363522, 10.000000, 8.181775, 5.144475, 1.450959, -2.605475, -6.752494, -10.000000, -9.440899, -7.391303, -3.113694, 3.258694, 8.264191,
        10.000000, 10.449370, 9.756855, 7.048309, 2.938391, 0.589423, 0.000000, 1.233396, 5.192722, 7.906461, 9.234742, 9.835874, 10.000000, 8.548979,
        5.938212, 3.775638, 2.145814, 0.920811, 0.000000,
    ];

    let curve = translation_curve(scene, 2);
    // This file contains _negative_ velocities, which don't seem to be well supported, so let's give it some leeway..
    check_curve_values(err, curve, values, Some(0.01));
});

file_test!(motionbuilder_tangent_auto_bias, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000, 0.896991, 1.898148, 2.968750, 4.074074, 5.179398, 6.250000, 7.251157, 8.148148, 8.906251, 9.490741, 9.866899, 10.000000, 8.125000, 5.000000,
        3.190000, 2.586667, 2.930000, 3.960000, 5.416667, 7.040000, 8.570000, 9.746667, 10.310000, 10.000000, 9.224537, 8.518518, 7.812500, 7.037037, 6.122685,
        5.000000, 3.146296, 1.014815, 0.000000, 1.855556, 5.927778, 10.000000, 11.805555, 11.111111, 8.750000, 5.555555, 2.361110, 0.000000,
    ];

    let curve = translation_curve(scene, 2);
    check_curve_values(err, curve, values, None);
});

file_test!(motionbuilder_tangent_auto_bias_split, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000, 0.896991, 1.898148, 2.968750, 4.074074, 5.179398, 6.250000, 7.251157, 8.148148, 8.906251, 9.490741, 9.866899, 10.000000, 6.958333, 5.000000,
        3.070000, 2.160000, 2.090000, 2.680000, 3.750000, 5.120000, 6.610000, 8.039999, 9.230000, 10.000000, 8.935184, 7.592593, 6.250000, 5.185185, 4.675926,
        5.000000, 3.705556, 0.744444, 0.000000, 1.855556, 5.927778, 10.000000, 6.597222, 4.444445, 3.125000, 2.222222, 1.319445, 0.000000,
    ];

    let curve = translation_curve(scene, 2);
    check_curve_values(err, curve, values, None);
});

file_test!(motionbuilder_tangent_auto_bias_clamp, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000, 1.019097, 2.069444, 3.140625, 4.222222, 5.303820, 6.375000, 7.425347, 8.444445, 9.421875, 10.347222, 11.210069, 12.000000, 12.160000,
        12.179999, 12.120000, 12.040000, 12.000000, 11.612245, 10.612245, 9.244898, 7.755102, 6.387755, 5.387755, 5.000000, 4.537037, 3.518518, 2.500000,
        2.037037, 2.685185, 5.000000, 4.816667, 1.300000, 0.000000, 1.855556, 5.927778, 10.000000, 6.597222, 4.444445, 3.125000, 2.222222, 1.319445, 0.000000,
    ];

    let curve = translation_curve(scene, 2);
    check_curve_values(err, curve, values, None);
});

file_test!(motionbuilder_tangent_auto_bias_absolute_simple, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000,
        12.731482,
        46.296295,
        93.750000,
        148.148148,
        202.546310,
        250.000000,
        283.564819,
        296.296295,
        281.250000,
        231.481476,
        140.046295,
        0.000000,
        -140.046310,
        -231.481476,
        -281.250000,
        -296.296295,
        -283.564819,
        -250.000000,
        -202.546310,
        -148.148148,
        -93.750008,
        -46.296303,
        -12.731483,
        0.000000,
    ];

    let curve = translation_curve(scene, 2);
    check_curve_values(err, curve, values, None);
});

file_test!(motionbuilder_tangent_auto_bias_absolute, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000, 30.833332, 41.111111, 37.500000, 26.666666, 15.277778, 10.000000, 10.856482, 12.962962, 15.624999, 18.148148, 19.837963, 20.000000,
        18.634037, 15.264155, 11.020745, 7.070707, 3.423960, 0.000000, -3.454167, -7.205388, -11.360994, -15.907764, -19.289827, -20.000000, -19.982880,
        -19.254641, -16.822596, -12.495454, -6.726660, 0.000000, 6.599604, 11.988842, 15.768107, 17.791979, 18.772791, 20.000000, 22.530613, 26.054420,
        29.469387, 31.673466, 31.564625, 28.040813, 20.000000, 16.203705, 20.407408, 24.000000, 24.065104, 23.562500, 22.695312, 21.666668, 20.679688,
        19.937500, 19.643229, 20.000000, 14.733797, 2.148147, -10.687501, -16.703705, -8.831019, 20.000000,
    ];

    let curve = translation_curve(scene, 2);
    check_curve_values(err, curve, values, None);
});

file_test!(motionbuilder_tangent_auto_bias_absolute_split, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000,
        -208.333328,
        -166.666672,
        0.000000,
        166.666672,
        208.333328,
        0.000000,
        208.333328,
        166.666672,
        0.000000,
        -166.666672,
        -208.333328,
        0.000000,
        -1134.259277,
        -907.407410,
        0.000000,
        907.407410,
        1134.259277,
        0.000000,
        -52.083332,
        -166.666672,
        -281.250000,
        -333.333344,
        -260.416656,
        0.000000,
    ];

    let curve = translation_curve(scene, 2);
    check_curve_values(err, curve, values, Some(0.01));
});

file_test!(motionbuilder_tangent_auto_bias_absolute_clamp, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000,
        -260.416656,
        -333.333344,
        -281.250000,
        -166.666672,
        -52.083328,
        0.000000,
        -52.083336,
        -166.666672,
        -281.250000,
        -333.333344,
        -260.416656,
        0.000000,
        -1417.824097,
        -1814.814819,
        -1531.250000,
        -907.407410,
        -283.564819,
        0.000000,
        -52.083332,
        -166.666672,
        -281.250000,
        -333.333344,
        -260.416656,
        0.000000,
    ];

    let curve = translation_curve(scene, 2);
    check_curve_values(err, curve, values, Some(0.01));
});

file_test!(motionbuilder_tangent_auto_bias_absolute_sign, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000, -1.145833, -4.166667, -8.437500, -13.333333, -18.229168, -22.500000, -25.520832, -26.666666, -25.312500, -20.833332, -12.604168, 0.000000,
        13.750000, 25.000000, 33.750000, 40.000000, 43.750000, 45.000000, 43.750000, 40.000000, 33.750000, 24.999998, 13.749999, 0.000000, -10.416666,
        -13.333333, -11.250000, -6.666667, -2.083333, 0.000000, -0.000001, -0.000002, 0.000000, 1.145844, 4.166698, 8.437560, 13.333425, 18.229292, 22.500154,
        25.521011, 26.666861, 25.312698, 20.833519, 12.604320, 0.000100, -13.749959, -25.000004, -33.750038, -40.000061, -43.750072, -45.000076, -43.750076,
        -40.000069, -33.750053, -25.000038, -13.750021, 0.000000, 10.416676, 13.333332, 11.249978, 6.666616, 2.083255, -0.000100,
    ];

    let curve = translation_curve(scene, 2);
    check_curve_values(err, curve, values, Some(0.01));
});

file_test!(motionbuilder_tangent_auto_bias_absolute_negative, flags = ALLOW_INVALID_UNICODE | ALLOW_WARNINGS, |scene, err, load_error| {
    ufbxt_assert!(scene.metadata.has_warning[WarningType::MissingGeometryData as usize]);

    let values: &[f64] = &[
        0.000000, 11.652200, 16.689816, 16.484375, 12.407409, 5.830441, -1.874999, -9.337385, -15.185184, -18.046875, -16.550926, -9.325809, 5.000000,
        21.818577, 35.590279, 46.289062, 53.888889, 58.363716, 59.687500, 57.834198, 52.777775, 44.492184, 32.951385, 18.129337, 0.000000, -15.677082,
        -23.666668, -24.781250, -19.833334, -9.635417, 5.000000, 20.174999, 32.466667, 41.774998, 48.000000, 51.041664, 50.800003, 47.174999, 40.066669,
        29.375004, 15.000000, -0.205000, -13.040000, -23.085001, -29.920002, -33.125000, -32.279999, -26.965002, -16.760004, -1.245000, 20.000000, 37.912502,
        44.800003, 43.137501, 35.400002, 24.062502, 11.600001, 0.487498, -6.800000, -7.787500, 0.000000,
    ];

    let curve = translation_curve(scene, 1);
    check_curve_values(err, curve, values, Some(0.01));
});

file_test!(motionbuilder_tangent_tcb, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    let values: &[f64] = &[
        0.000000, -0.839699, -2.648148, -4.546875, -5.657407, -5.101273, -2.000000, -1.805556, -1.222222, -0.250000, 1.111111, 2.861111, 5.000000, 6.862268,
        7.870370, 8.156250, 7.851851, 7.089120, 6.000000, 4.716435, 3.370370, 2.093750, 1.018518, 0.276620, 0.000000, 2.407407, 3.481481, 4.000000, 4.740741,
        6.481482, 10.000000, 7.193704, 5.340741, 4.290000, 3.890371, 3.990741, 4.440001, 5.087037, 5.780741, 6.370000, 6.703704, 6.630741, 6.000000, 4.702963,
        2.951704, 1.076000, -0.594370, -1.729629, -2.000000, -0.186167, 2.875667, 4.000000, 3.947917, 2.680556, 0.875000, -0.791667, -1.642361, -1.000000,
        -0.069444, 2.354167, 4.000000,
    ];

    let curve = translation_curve(scene, 2);
    check_curve_values(err, curve, values, None);
});

file_test!(maya_resampled, |scene, err, load_error| {
    let values6: &[f64] = &[
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.004, -0.022, -0.056, -0.104, -0.166, -0.241, -0.328, -0.427, -0.536, -0.654, -0.783, -0.919, -1.063,
        -1.214, -1.371, -1.533, -1.700, -1.871, -2.044, -2.220, -2.398, -2.577, -2.755, -2.933, -3.109, -3.283, -3.454, -3.621, -3.784, -3.941, -4.093, -4.237,
        -4.374, -4.503, -4.623, -4.733, -4.832, -4.920, -4.996, -5.059, -5.108, -5.143, -5.168, -5.186, -5.200, -5.209, -5.215, -5.218, -5.220, -5.220, -5.216,
        -5.192, -5.151, -5.091, -5.013, -4.919, -4.810, -4.686,
    ];

    let values7: &[f64] = &[
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.000, -0.004, -0.025, -0.061, -0.112, -0.176, -0.252, -0.337, -0.431, -0.533, -0.648, -0.776, -0.915, -1.064,
        -1.219, -1.378, -1.539, -1.700, -1.865, -2.037, -2.216, -2.397, -2.580, -2.761, -2.939, -3.111, -3.278, -3.447, -3.615, -3.782, -3.943, -4.098, -4.244,
        -4.379, -4.500, -4.614, -4.722, -4.821, -4.911, -4.990, -5.056, -5.107, -5.143, -5.168, -5.186, -5.200, -5.209, -5.215, -5.218, -5.220, -5.220, -5.215,
        -5.190, -5.145, -5.082, -5.002, -4.908, -4.800, -4.680, -4.550, -4.403, -4.239,
    ];

    let values = if scene.metadata.version >= 7000 { values7 } else { values6 };

    ufbxt_assert!(scene.anim_layers.len() == 1);
    let layer = &scene.anim_layers[0];
    for &value_ix in &layer.anim_values {
        let anim_value = &scene.anim_values[value_ix];
        if anim_value.element.name != "Lcl Translation" {
            continue;
        }
        let curve = &scene.anim_curves[anim_value.curves[0].unwrap()];

        for (i, &ref_value) in values.iter().enumerate() {
            let time = i as f64 * (1.0 / 200.0);
            let value = evaluate_curve(curve, time, 0.0);
            assert_close_real(err, value, ref_value);
        }
    }
});

#[derive(Clone, Copy)]
struct AnimTransformRef {
    frame: i32,
    translation: Vec3,
    rotation_euler: Vec3,
    scale: Vec3,
}

fn anim_layer_ids(scene: &Scene) -> Vec<u32> {
    scene.anim.layers.iter().map(|&layer| scene.anim_layers[layer].element.typed_id).collect()
}

file_test!(maya_anim_light, |scene, err, load_error| {
    let refs: [(i32, Real, Vec3); 8] = [
        (0, 3.072, Vec3::new(0.148, 0.095, 0.440)),
        (12, 1.638, Vec3::new(0.102, 0.136, 0.335)),
        (24, 1.948, Vec3::new(0.020, 0.208, 0.149)),
        (32, 3.676, Vec3::new(0.010, 0.220, 0.113)),
        (40, 4.801, Vec3::new(0.118, 0.195, 0.115)),
        (48, 3.690, Vec3::new(0.288, 0.155, 0.117)),
        (56, 1.565, Vec3::new(0.421, 0.124, 0.119)),
        (60, 1.145, Vec3::new(0.442, 0.119, 0.119)),
    ];

    for &(frame, intensity, color) in &refs {
        let time = frame as f64 * (1.0 / 24.0);
        let state = evaluate_scene(scene, &scene.anim, time, &EvaluateOpts::default());
        ufbxt_assert!(state.is_ok());
        let state = state.unwrap();

        check_scene(&state);

        let light_node = find_node(&state, "pointLight1");
        ufbxt_assert!(light_node.is_some());
        let light = state.nodes[light_node.unwrap()].light;
        ufbxt_assert!(light.is_some());
        let light = &state.lights[light.unwrap()];

        assert_close_real(err, light.intensity, intensity);
        assert_close_vec3(err, light.color, color);
    }

    {
        let node = find_node(scene, "pointLight1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.light.is_some()));
        let node = node.unwrap();
        let element_id = scene.lights[node.light.unwrap()].element.element_id;

        let overrides = vec![
            PropOverrideDesc { element_id, prop_name: "Intensity".into(), value: Vec4::new(10.0, 0.0, 0.0, 0.0), ..Default::default() },
            PropOverrideDesc { element_id, prop_name: "Color".into(), value: Vec4::new(0.3, 0.6, 0.9, 0.0), ..Default::default() },
            PropOverrideDesc {
                element_id,
                prop_name: "|NewProp".into(),
                value: Vec4::new(10.0, 20.0, 30.0, 0.0),
                value_str: "Test".into(),
                ..Default::default()
            },
            PropOverrideDesc { element_id, prop_name: "IntProp".into(), value_int: 15, ..Default::default() },
        ];

        let node_id = node.element.typed_id;
        let transform_overrides = vec![TransformOverride {
            node_id,
            transform: Transform { translation: Vec3::new(1.0, 2.0, 3.0), rotation: Quat::new(1.0, 0.0, 0.0, 0.0), scale: Vec3::new(4.0, 5.0, 6.0) },
        }];

        let opts =
            AnimOpts { layer_ids: anim_layer_ids(scene), prop_overrides: overrides, transform_overrides: transform_overrides.clone(), ..Default::default() };

        let anim = create_anim(scene, &opts);
        if let Err(e) = &anim {
            log_error(e);
        }
        ufbxt_assert!(anim.is_ok());
        let anim = anim.unwrap();

        check_anim(scene, &anim);

        let state = evaluate_scene(scene, &anim, 1.0, &EvaluateOpts::default());
        ufbxt_assert!(state.is_ok());
        let state = state.unwrap();

        check_scene(&state);

        let light_node = find_node(&state, "pointLight1");
        ufbxt_assert!(light_node.is_some());
        let light_node = &state.nodes[light_node.unwrap()];

        assert_close_vec3(err, light_node.local_transform.translation, transform_overrides[0].transform.translation);
        assert_close_quat(err, light_node.local_transform.rotation, transform_overrides[0].transform.rotation);
        assert_close_vec3(err, light_node.local_transform.scale, transform_overrides[0].transform.scale);

        ufbxt_assert!(light_node.light.is_some());
        let light = &state.lights[light_node.light.unwrap()];

        let ref_color = Vec3::new(0.3, 0.6, 0.9);
        assert_close_real(err, light.intensity, 0.1);
        assert_close_vec3(err, light.color, ref_color);

        {
            let ref_new = Vec3::new(10.0, 20.0, 30.0);
            let new_prop = light.element.props.find("|NewProp");
            ufbxt_assert!(new_prop.is_some());
            let new_prop = new_prop.unwrap();
            ufbxt_assert!(new_prop.flags & prop_flags::OVERRIDDEN != 0);
            ufbxt_assert!(new_prop.value_str == "Test");
            ufbxt_assert!(new_prop.value_int == 10);
            assert_close_vec3(err, new_prop.value_vec3(), ref_new);

            let int_prop = light.element.props.find("IntProp");
            ufbxt_assert!(int_prop.is_some());
            let int_prop = int_prop.unwrap();
            ufbxt_assert!(int_prop.flags & prop_flags::OVERRIDDEN != 0);
            assert_close_real(err, int_prop.value_real(), 15.0);
            ufbxt_assert!(int_prop.value_int == 15);
        }

        {
            let original_light = scene.lights[node.light.unwrap()].element.element_id;

            let color = evaluate_prop(scene, &anim, original_light, "Color", 1.0);
            ufbxt_assert!(color.flags & prop_flags::OVERRIDDEN != 0);
            assert_close_vec3(err, color.value_vec3(), ref_color);

            let intensity = evaluate_prop(scene, &anim, original_light, "Intensity", 1.0);
            ufbxt_assert!(intensity.flags & prop_flags::OVERRIDDEN != 0);
            assert_close_real(err, intensity.value_real(), 10.0);

            let ref_new = Vec3::new(10.0, 20.0, 30.0);
            let new_prop = evaluate_prop(scene, &anim, original_light, "|NewProp", 1.0);
            ufbxt_assert!(new_prop.flags & prop_flags::OVERRIDDEN != 0);
            ufbxt_assert!(new_prop.value_str == "Test");
            ufbxt_assert!(new_prop.value_int == 10);
            assert_close_vec3(err, new_prop.value_vec3(), ref_new);

            let int_prop = evaluate_prop(scene, &anim, original_light, "IntProp", 1.0);
            ufbxt_assert!(int_prop.flags & prop_flags::OVERRIDDEN != 0);
            assert_close_real(err, int_prop.value_real(), 15.0);
            ufbxt_assert!(int_prop.value_int == 15);
        }
    }

    {
        let layer = &scene.anim_layers[0];
        let node = find_node(scene, "pointLight1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.light.is_some()));
        let node = node.unwrap();
        let light = &scene.lights[node.light.unwrap()];

        {
            let props = find_anim_props(layer, node.element.element_id);
            ufbxt_assert!(props.len() == 3);
            ufbxt_assert!(props[0].prop_name == "Lcl Rotation");
            ufbxt_assert!(props[1].prop_name == "Lcl Scaling");
            ufbxt_assert!(props[2].prop_name == "Lcl Translation");

            let prop = find_anim_prop(layer, node.element.element_id, "Lcl Rotation");
            ufbxt_assert!(prop.is_some_and(|p| p.prop_name == "Lcl Rotation"));
            let prop = find_anim_prop(layer, node.element.element_id, "Lcl Scaling");
            ufbxt_assert!(prop.is_some_and(|p| p.prop_name == "Lcl Scaling"));
            let prop = find_anim_prop(layer, node.element.element_id, "Lcl Translation");
            ufbxt_assert!(prop.is_some_and(|p| p.prop_name == "Lcl Translation"));
        }

        {
            let props = find_anim_props(layer, light.element.element_id);
            ufbxt_assert!(props.len() == 2);
            ufbxt_assert!(props[0].prop_name == "Color");
            ufbxt_assert!(props[1].prop_name == "Intensity");

            let prop = find_anim_prop(layer, light.element.element_id, "Color");
            ufbxt_assert!(prop.is_some_and(|p| p.prop_name == "Color"));
            let prop = find_anim_prop(layer, light.element.element_id, "Intensity");
            ufbxt_assert!(prop.is_some_and(|p| p.prop_name == "Intensity"));

            let prop = find_anim_prop(layer, light.element.element_id, "Nonexistent");
            ufbxt_assert!(prop.is_none());
        }

        {
            let props = find_anim_props(layer, layer.element.element_id);
            ufbxt_assert!(props.is_empty());

            let prop = find_anim_prop(layer, layer.element.element_id, "Weight");
            ufbxt_assert!(prop.is_none());
        }
    }
});

file_test!(create_anim_alloc_fail, file = "maya_anim_light_7500_binary", |scene, err, load_error| {});

file_test!(maya_transform_animation, |scene, err, load_error| {
    let refs = [
        AnimTransformRef {
            frame: 1,
            translation: Vec3::new(0.000, 0.000, 0.000),
            rotation_euler: Vec3::new(0.000, 0.000, 0.000),
            scale: Vec3::new(1.000, 1.000, 1.000),
        },
        AnimTransformRef {
            frame: 5,
            translation: Vec3::new(0.226, 0.452, 0.677),
            rotation_euler: Vec3::new(2.258, 4.515, 6.773),
            scale: Vec3::new(1.023, 1.045, 1.068),
        },
        AnimTransformRef {
            frame: 14,
            translation: Vec3::new(1.000, 2.000, 3.000),
            rotation_euler: Vec3::new(10.000, 20.000, 30.000),
            scale: Vec3::new(1.100, 1.200, 1.300),
        },
        AnimTransformRef {
            frame: 20,
            translation: Vec3::new(-0.296, -0.592, -0.888),
            rotation_euler: Vec3::new(-2.960, -5.920, -8.880),
            scale: Vec3::new(0.970, 0.941, 0.911),
        },
        AnimTransformRef {
            frame: 24,
            translation: Vec3::new(-1.000, -2.000, -3.000),
            rotation_euler: Vec3::new(-10.000, -20.000, -30.000),
            scale: Vec3::new(0.900, 0.800, 0.700),
        },
    ];

    let node_ix = find_node(scene, "pCube1").unwrap();
    let node = &scene.nodes[node_ix];

    for r in &refs {
        let time = r.frame as f64 * (1.0 / 24.0);

        let state = evaluate_scene(scene, &scene.anim, time, &EvaluateOpts::default());
        ufbxt_assert!(state.is_ok());
        let state = state.unwrap();
        check_scene(&state);

        let t1 = state.nodes[node.element.typed_id as usize].local_transform;
        let t2 = evaluate_transform(scene, &scene.anim, node_ix, time);

        let t1_euler = quat_to_euler(t1.rotation, RotationOrder::Xyz);
        let t2_euler = quat_to_euler(t2.rotation, RotationOrder::Xyz);

        assert_close_vec3(err, r.translation, t1.translation);
        assert_close_vec3(err, r.translation, t2.translation);
        assert_close_vec3(err, r.rotation_euler, t1_euler);
        assert_close_vec3(err, r.rotation_euler, t2_euler);
        assert_close_vec3(err, r.scale, t1.scale);
        assert_close_vec3(err, r.scale, t2.scale);
    }

    {
        let element_id = node.element.element_id;
        let mut r = refs[2];
        r.translation.x -= 0.1;
        r.translation.y -= 0.2;
        r.translation.z -= 0.3;
        r.scale.x = 2.0;
        r.scale.y = 3.0;
        r.scale.z = 4.0;

        let overrides = vec![
            PropOverrideDesc { element_id, prop_name: "Color".into(), value: Vec4::new(0.3, 0.6, 0.9, 0.0), ..Default::default() },
            PropOverrideDesc {
                element_id,
                prop_name: "|NewProp".into(),
                value: Vec4::new(10.0, 20.0, 30.0, 0.0),
                value_str: "Test".into(),
                ..Default::default()
            },
            PropOverrideDesc { element_id, prop_name: "Lcl Scaling".into(), value: Vec4::new(2.0, 3.0, 4.0, 0.0), ..Default::default() },
            PropOverrideDesc { element_id, prop_name: "RotationOffset".into(), value: Vec4::new(-0.1, -0.2, -0.3, 0.0), ..Default::default() },
        ];

        let opts = AnimOpts { layer_ids: anim_layer_ids(scene), prop_overrides: overrides, ..Default::default() };

        let anim = create_anim(scene, &opts);
        if let Err(e) = &anim {
            log_error(e);
        }
        ufbxt_assert!(anim.is_ok());
        let anim = anim.unwrap();
        check_anim(scene, &anim);

        let time = 14.0 / 24.0;
        let state = evaluate_scene(scene, &anim, time, &EvaluateOpts::default());
        ufbxt_assert!(state.is_ok());
        let state = state.unwrap();
        check_scene(&state);

        let t1 = state.nodes[node.element.typed_id as usize].local_transform;
        let t2 = evaluate_transform(scene, &anim, node_ix, time);

        let t1_euler = quat_to_euler(t1.rotation, RotationOrder::Xyz);
        let t2_euler = quat_to_euler(t2.rotation, RotationOrder::Xyz);

        assert_close_vec3(err, r.translation, t1.translation);
        assert_close_vec3(err, r.translation, t2.translation);
        assert_close_vec3(err, r.rotation_euler, t1_euler);
        assert_close_vec3(err, r.rotation_euler, t2_euler);
        assert_close_vec3(err, r.scale, t1.scale);
        assert_close_vec3(err, r.scale, t2.scale);
    }
});

fn find_anim_layer<'a>(scene: &'a Scene, name: &str) -> Option<&'a AnimLayer> {
    find_element(scene, ElementType::AnimLayer, name).and_then(|id| scene.as_anim_layer(id))
}

fn check_anim_layers_compose(scene: &Scene, compose: bool) {
    let x = find_anim_layer(scene, "X");
    let y = find_anim_layer(scene, "Y");
    ufbxt_assert!(x.is_some() && y.is_some());
    let y = y.unwrap();
    ufbxt_assert!(y.compose_rotation == compose);
    ufbxt_assert!(y.compose_scale == compose);
}

file_test!(maya_anim_layers, |scene, err, load_error| {
    check_anim_layers_compose(scene, false);
});

file_test!(maya_anim_layers_acc, |scene, err, load_error| {
    check_anim_layers_compose(scene, true);
});

file_test!(maya_anim_layers_over, |scene, err, load_error| {
    check_anim_layers_compose(scene, false);
});

file_test!(maya_anim_layers_over_acc, |scene, err, load_error| {
    check_anim_layers_compose(scene, true);
});

file_test!(maya_cube_blinky, |scene, err, load_error| {
    let refs: [(f64, bool); 12] = [
        (1.0, false),
        (9.5, false),
        (10.5, true),
        (11.5, false),
        (15.0, false),
        (19.5, false),
        (20.5, false),
        (25.0, false),
        (29.5, false),
        (30.5, true),
        (40.0, true),
        (50.0, true),
    ];

    for &(frame, visible) in &refs {
        let time = frame / 24.0;
        let state = evaluate_scene(scene, &scene.anim, time, &EvaluateOpts::default());
        ufbxt_assert!(state.is_ok());
        let state = state.unwrap();

        check_scene(&state);

        let node = find_node(&state, "pCube1");
        ufbxt_assert!(node.is_some());
        ufbxt_assert!(state.nodes[node.unwrap()].visible == visible);
    }
});

file_test!(maya_anim_interpolation, |scene, err, load_error| {
    let anim_ref: [(f64, f64); 25] = [
        // Cubic
        (0.0 / 30.0, 0.0),
        (1.0 / 30.0, -0.855245),
        (2.0 / 30.0, -1.13344),
        (3.0 / 30.0, -1.17802),
        (4.0 / 30.0, -1.10882),
        (5.0 / 30.0, -0.991537),
        (6.0 / 30.0, -0.875223),
        (7.0 / 30.0, -0.808958),
        (8.0 / 30.0, -0.858419),
        (9.0 / 30.0, -1.14293),
        // Linear
        (10.0 / 30.0, -2.0),
        // Constant previous
        (20.0 / 30.0, -4.0),
        (25.0 / 30.0 - 0.001, -4.0),
        // Constant next
        (25.0 / 30.0, -6.0),
        (25.0 / 30.0 + 0.001, -8.0),
        // Constant previous
        (30.0 / 30.0, -8.0),
        (35.0 / 30.0 - 0.001, -8.0),
        // Linear
        (35.0 / 30.0, -10.0),
        // Constant next
        (40.0 / 30.0, -12.0),
        (40.0 / 30.0 + 0.001, -14.0),
        // Constant previous
        (45.0 / 30.0, -14.0),
        (50.0 / 30.0 - 0.001, -14.0),
        // Constant next
        (50.0 / 30.0, -16.0),
        (50.0 / 30.0 + 0.001, -14.0),
        // Final
        (55.0 / 30.0, -14.0),
    ];

    let node = find_node(scene, "pCube1");
    ufbxt_assert!(node.is_some());
    let node = &scene.nodes[node.unwrap()];

    for (i, &(time, value)) in anim_ref.iter().enumerate() {
        ufbxt_hintf!("{i}: {time:.6} (frame {:.2})", time * 30.0);

        let p = evaluate_prop(scene, &scene.anim, node.element.element_id, "Lcl Translation", time);
        assert_close_real(err, p.value_vec3().x, value);
    }
});

file_test!(maya_long_keyframes, |scene, err, load_error| {
    let anim_ref: [(i64, Real); 41] = [
        (-5000000, -50.0),
        (-2925270, -29.0),
        (-2925269, -28.0),
        (-2925268, -27.0),
        (-2925267, -26.0),
        (-2925266, -25.0),
        (-2925265, -24.0),
        (-2925264, -23.0),
        (-2925263, -22.0),
        (-2925262, -21.0),
        (-2000000, -20.0),
        (-599999, -5.9),
        (-500000, -5.0),
        (-49999, -4.9),
        (-40000, -4.0),
        (-3999, -3.9),
        (-3000, -3.0),
        (-299, -2.9),
        (-200, -2.0),
        (-10, -1.0),
        (0, 0.0),
        (10, 1.0),
        (200, 2.0),
        (299, 2.9),
        (3000, 3.0),
        (3999, 3.9),
        (40000, 4.0),
        (49999, 4.9),
        (500000, 5.0),
        (599999, 5.9),
        (2000000, 20.0),
        (2925262, 21.0),
        (2925263, 22.0),
        (2925264, 23.0),
        (2925265, 24.0),
        (2925266, 25.0),
        (2925267, 26.0),
        (2925268, 27.0),
        (2925269, 28.0),
        (2925270, 29.0),
        (5000000, 50.0),
    ];

    ufbxt_assert!(scene.metadata.ktime_second == 46186158000);

    let node = find_node(scene, "pCube1");
    ufbxt_assert!(node.is_some());
    let node = &scene.nodes[node.unwrap()];

    ufbxt_assert!(!scene.anim_layers.is_empty());

    let aprop = find_anim_prop(&scene.anim_layers[0], node.element.element_id, "Lcl Translation");
    ufbxt_assert!(aprop.is_some());

    let curve = scene.anim_values[aprop.unwrap().anim_value].curves[0];
    ufbxt_assert!(curve.is_some());
    let curve = &scene.anim_curves[curve.unwrap()];

    for (i, &(frame, value)) in anim_ref.iter().enumerate() {
        ufbxt_hintf!("{i}: (frame {frame})");
        ufbxt_assert!(i < curve.keyframes.len());
        let key = curve.keyframes[i];

        let mut tick_accurate = true;
        if frame.abs() > 2925270 {
            tick_accurate = false;
        }

        let ref_tick = frame * (46186158000 / 30);
        let tick = (key.time * 46186158000.0).round() as i64;
        if tick_accurate {
            ufbxt_assert!(ref_tick == tick);
        } else {
            assert_close_real(err, key.time, frame as f64 / 30.0);
        }
        assert_close_real(err, key.value, value);
    }
});

file_test!(anim_override_utf8, file = "blender_279_default", |scene, err, load_error| {
    let cube = find_node(scene, "Cube");
    ufbxt_assert!(cube.is_some());
    let cube_id = scene.nodes[cube.unwrap()].element.element_id;

    let good_strings = ["", "", "", "a", "a"];

    // ufbx's "\xff", "a\xff" and their lengths are not UTF-8, which a Rust `String` cannot hold; "\0" remains
    let bad_strings = ["\0"];

    let string_override = |s: &str, do_value: bool| {
        let mut over = PropOverrideDesc { element_id: cube_id, prop_name: "prop".into(), ..Default::default() };
        if do_value {
            over.value_str = s.to_string();
        } else {
            over.prop_name = s.to_string();
        }
        AnimOpts { prop_overrides: vec![over], ..Default::default() }
    };

    for (i, good) in good_strings.iter().enumerate() {
        for do_value in [false, true] {
            ufbxt_hintf!("i={i}, do_value={}", do_value as i32);

            let anim = create_anim(scene, &string_override(good, do_value));
            if let Err(e) = &anim {
                log_error(e);
            }
            ufbxt_assert!(anim.is_ok());
        }
    }

    for (i, bad) in bad_strings.iter().enumerate() {
        for do_value in [false, true] {
            ufbxt_hintf!("i={i}, do_value={}", do_value as i32);

            let anim = create_anim(scene, &string_override(bad, do_value));
            ufbxt_assert!(anim.is_err());
            ufbxt_assert!(anim.unwrap_err().kind == ErrorType::InvalidUtf8);
        }
    }
});

file_test!(anim_multi_override, file = "blender_293_instancing", |scene, err, load_error| {
    let node_names = ["Suzanne", "Suzanne.001", "Suzanne.002", "Suzanne.003", "Suzanne.004", "Suzanne.005", "Suzanne.006", "Suzanne.007"];

    let num_nodes = node_names.len();
    let mut overrides = vec![PropOverrideDesc::default(); num_nodes * 3];

    for i in 0..num_nodes {
        let node = find_node(scene, node_names[i]);
        ufbxt_assert!(node.is_some());
        let element_id = scene.nodes[node.unwrap()].element.element_id;

        overrides[i * 3] =
            PropOverrideDesc { element_id, prop_name: "Lcl Translation".into(), value: Vec4::new(i as Real, 0.0, 0.0, 0.0), ..Default::default() };
        overrides[i * 3 + 1] =
            PropOverrideDesc { element_id, prop_name: "Lcl Rotation".into(), value: Vec4::new(0.0, 10.0 * i as Real, 0.0, 0.0), ..Default::default() };
        overrides[i * 3 + 2] =
            PropOverrideDesc { element_id, prop_name: "Lcl Scaling".into(), value: Vec4::new(1.0, 1.0, 1.0 + 0.1 * i as Real, 0.0), ..Default::default() };
    }

    let opts = AnimOpts { prop_overrides: overrides, ..Default::default() };

    let anim = create_anim(scene, &opts);
    if let Err(e) = &anim {
        log_error(e);
    }
    ufbxt_assert!(anim.is_ok());
    let anim = anim.unwrap();
    check_anim(scene, &anim);

    let state = evaluate_scene(scene, &anim, 0.0, &EvaluateOpts::default());
    if let Err(e) = &state {
        log_error(e);
    }
    ufbxt_assert!(state.is_ok());
    let state = state.unwrap();
    check_scene(&state);

    for i in 0..num_nodes {
        let scene_node = find_node(scene, node_names[i]).unwrap();
        let state_node = find_node(&state, node_names[i]).unwrap();

        let scene_transform = evaluate_transform(scene, &anim, scene_node, 0.0);
        let state_transform = state.nodes[state_node].local_transform;

        let mut ref_transform = REF_TRANSFORM_IDENTITY;
        ref_transform.translation.x = i as Real;
        ref_transform.rotation_euler.y = 10.0 * i as Real;
        ref_transform.scale.z = 1.0 + 0.1 * i as Real;

        check_transform(err, "scene_transform", scene_transform, ref_transform);
        check_transform(err, "state_transform", state_transform, ref_transform);
    }
});

file_test!(anim_override_duplicate, file = "blender_293_instancing", |scene, err, load_error| {
    let over =
        |element_id: u32, name: &str| PropOverrideDesc { element_id, prop_name: name.into(), value: Vec4::new(1.0, 0.0, 0.0, 0.0), ..Default::default() };
    let overrides = vec![over(1, "PropA"), over(1, "PropB"), over(1, "PropC"), over(2, "PropA"), over(2, "PropB"), over(2, "PropB"), over(2, "PropC")];

    let opts = AnimOpts { prop_overrides: overrides, ..Default::default() };

    let anim = create_anim(scene, &opts);
    ufbxt_assert!(anim.is_err());
    let error = anim.unwrap_err();
    ufbxt_assert!(error.kind == ErrorType::DuplicateOverride);
    ufbxt_assert!(error.info == "element 2 prop \"PropB\"");
});

fn check_anim_stack_times(err: &mut DiffError, scene: &Scene, name: &str, begin: f64, end: f64, start_ktime: i64, stop_ktime: i64) {
    let stack = find_anim_stack(scene, name);
    ufbxt_assert!(stack.is_some());
    let stack = &scene.anim_stacks[stack.unwrap()];
    assert_close_double(err, stack.time_begin, begin);
    assert_close_double(err, stack.time_end, end);

    let props = &stack.element.props;
    ufbxt_assert!(props.find_int("LocalStart", 0) == start_ktime);
    ufbxt_assert!(props.find_int("LocalStop", 0) == stop_ktime);
    ufbxt_assert!(props.find_int("ReferenceStart", 0) == start_ktime);
    ufbxt_assert!(props.find_int("ReferenceStop", 0) == stop_ktime);
}

file_test!(synthetic_anim_stack_no_props, |scene, err, load_error| {
    check_anim_stack_times(err, scene, "X", 1.0 / 24.0, 12.0 / 24.0, 1924423250, 23093079000);
    check_anim_stack_times(err, scene, "Y", 13.0 / 24.0, 24.0 / 24.0, 25017502250, 46186158000);
    check_anim_stack_times(err, scene, "Z", 25.0 / 24.0, 36.0 / 24.0, 48110581250, 69279237000);
});

file_test!(anim_override_int64, file = "maya_cube_7500_binary", |scene, err, load_error| {
    let node_ix = find_node(scene, "pCube1");
    ufbxt_assert!(node_ix.is_some());
    let node = &scene.nodes[node_ix.unwrap()];

    let missing_prop = node.element.props.find("NewProperty");
    ufbxt_assert!(missing_prop.is_none());

    let existing_prop = node.element.props.find("DefaultAttributeIndex");
    ufbxt_assert!(existing_prop.is_some());
    let existing_prop = existing_prop.unwrap();
    ufbxt_assert!(existing_prop.prop_type == PropType::Integer);
    ufbxt_assert!(existing_prop.flags & prop_flags::VALUE_INT != 0);
    ufbxt_assert!(existing_prop.value_int == 0);

    let element_id = node.element.element_id;
    let overrides = vec![
        PropOverrideDesc { element_id, prop_name: "NewProperty".into(), value_int: 0x4000000000000001, ..Default::default() },
        PropOverrideDesc { element_id, prop_name: "DefaultAttributeIndex".into(), value_int: 0x5000000000000001, ..Default::default() },
    ];

    let opts = AnimOpts { prop_overrides: overrides, ..Default::default() };

    let anim = create_anim(scene, &opts);
    ufbxt_assert!(anim.is_ok());
    let anim = anim.unwrap();

    let state = evaluate_scene(scene, &anim, 0.0, &EvaluateOpts::default());
    ufbxt_assert!(state.is_ok());
    let state = state.unwrap();

    let node_state = &state.nodes[node.element.typed_id as usize];

    let new_prop = node_state.element.props.find("NewProperty");
    ufbxt_assert!(new_prop.is_some());
    let new_prop = new_prop.unwrap();
    ufbxt_assert!(new_prop.flags & prop_flags::OVERRIDDEN != 0);
    ufbxt_assert!(new_prop.value_int == 0x4000000000000001);
    assert_close_real(err, new_prop.value_real(), 0x4000000000000001_i64 as Real);

    let over_prop = node_state.element.props.find("DefaultAttributeIndex");
    ufbxt_assert!(over_prop.is_some());
    let over_prop = over_prop.unwrap();
    ufbxt_assert!(over_prop.flags & prop_flags::OVERRIDDEN != 0);
    ufbxt_assert!(over_prop.value_int == 0x5000000000000001);
    assert_close_real(err, over_prop.value_real(), 0x5000000000000001_i64 as Real);
});

fn diff_baked_vec3_imp(err: &mut DiffError, refs: &[BakedVec3], list: &[BakedVec3], name: &str) {
    for (i, r) in refs.iter().enumerate() {
        ufbxt_hintf!("{name} i={i}");
        ufbxt_assert!(i < list.len());
        let key = list[i];

        if key.time != r.time {
            ufbxt_logf!("key.time={:.6}, ref.time={:.6}", key.time, r.time);
        }
        if key.flags != r.flags {
            ufbxt_logf!("key.flags=0x{:x}, ref.flags=0x{:x}", key.flags, r.flags);
        }
        ufbxt_assert!(key.time == r.time);
        assert_close_vec3(err, key.value, r.value);
        ufbxt_assert!(key.flags == r.flags);
    }
    ufbxt_assert!(list.len() == refs.len());
}

fn diff_baked_quat_imp(err: &mut DiffError, refs: &[BakedVec3], list: &[BakedQuat], name: &str) {
    for (i, r) in refs.iter().enumerate() {
        ufbxt_hintf!("{name} i={i}");
        ufbxt_assert!(i < list.len());
        let key = list[i];

        let key_euler = quat_to_euler(key.value, RotationOrder::Xyz);

        if key.time != r.time {
            ufbxt_logf!("key.time={:.6}, ref.time={:.6}", key.time, r.time);
        }
        ufbxt_assert!(key.time == r.time);
        assert_close_vec3(err, key_euler, r.value);
        ufbxt_assert!(key.flags == r.flags);
    }
    ufbxt_assert!(list.len() == refs.len());
}

macro_rules! diff_baked_vec3 {
    ($err:expr, $refs:expr, $list:expr) => {
        diff_baked_vec3_imp($err, &$refs, &$list, stringify!($list))
    };
}

macro_rules! diff_baked_quat {
    ($err:expr, $refs:expr, $list:expr) => {
        diff_baked_quat_imp($err, &$refs, &$list, stringify!($list))
    };
}

fn key(time: f64, value: [Real; 3], flags: u32) -> BakedVec3 {
    BakedVec3 { time, value: Vec3::new(value[0], value[1], value[2]), flags }
}

fn bake_scene(scene: &Scene, opts: Option<&BakeOpts>) -> BakedAnim {
    let bake = ufbxt_bake_anim(scene, &scene.anim, opts);
    if let Err(e) = &bake {
        log_error(e);
    }
    ufbxt_assert!(bake.is_ok());
    bake.unwrap()
}

fn step_opts(step_handling: BakeStepHandling) -> BakeOpts {
    BakeOpts { step_handling, ..Default::default() }
}

file_test!(maya_anim_linear, |scene, err, load_error| {
    let opts = step_opts(BakeStepHandling::AdjacentDouble);
    let bake = bake_scene(scene, Some(&opts));

    let translation_ref = [
        key(0.0 / 24.0, [0.0, 0.0, 0.0], KEYFRAME),
        key(2.0 / 24.0, [0.5, 2.0, 0.0], KEYFRAME),
        key(8.0 / 24.0, [2.0, 0.0, 0.0], KEYFRAME),
        key((12.0_f64 / 24.0).next_down(), [2.0, 0.0, 0.0], STEP_LEFT),
        key(12.0 / 24.0, [2.0, 0.0, 2.0], KEYFRAME | STEP_KEY),
        key((12.0_f64 / 24.0).next_up(), [2.0, 0.0, 0.0], STEP_RIGHT),
        key(14.0 / 24.0, [2.0, 0.0, 0.0], KEYFRAME),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_vec3!(err, translation_ref, node.translation_keys);
});

file_test!(maya_anim_linear_default, file = "maya_anim_linear", |scene, err, load_error| {
    let bake = bake_scene(scene, None);

    let translation_ref = [
        key(0.0 / 24.0, [0.0, 0.0, 0.0], KEYFRAME),
        key(2.0 / 24.0, [0.5, 2.0, 0.0], KEYFRAME),
        key(8.0 / 24.0, [2.0, 0.0, 0.0], KEYFRAME),
        key(12.0 / 24.0 - 0.001, [2.0, 0.0, 0.0], STEP_LEFT),
        key(12.0 / 24.0, [2.0, 0.0, 2.0], KEYFRAME | STEP_KEY),
        key(12.0 / 24.0 + 0.001, [2.0, 0.0, 0.0], STEP_RIGHT),
        key(14.0 / 24.0, [2.0, 0.0, 0.0], KEYFRAME),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_vec3!(err, translation_ref, node.translation_keys);

    ufbxt_assert!(evaluate_baked_vec3(&node.translation_keys, (12.0_f64 / 24.0).next_down()).z == 0.0);
    ufbxt_assert!(evaluate_baked_vec3(&node.translation_keys, 12.0 / 24.0).z == 2.0);
    ufbxt_assert!(evaluate_baked_vec3(&node.translation_keys, (12.0_f64 / 24.0).next_up()).z == 0.0);
});

file_test!(maya_anim_linear_ignore, file = "maya_anim_linear", |scene, err, load_error| {
    let opts = step_opts(BakeStepHandling::Ignore);
    let bake = bake_scene(scene, Some(&opts));

    let translation_ref = [
        key(0.0 / 24.0, [0.0, 0.0, 0.0], KEYFRAME),
        key(2.0 / 24.0, [0.5, 2.0, 0.0], KEYFRAME),
        key(8.0 / 24.0, [2.0, 0.0, 0.0], KEYFRAME),
        key(12.0 / 24.0, [2.0, 0.0, 2.0], KEYFRAME | STEP_KEY),
        key(14.0 / 24.0, [2.0, 0.0, 0.0], KEYFRAME),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_vec3!(err, translation_ref, node.translation_keys);
});

fn check_huge_step_values(node: &BakedNode) {
    ufbxt_assert!(evaluate_baked_vec3(&node.translation_keys, 10.0_f64.next_down()).x == 0.0);
    ufbxt_assert!(evaluate_baked_vec3(&node.translation_keys, 10.0).x == 1.0);
    ufbxt_assert!(evaluate_baked_vec3(&node.translation_keys, 10.0_f64.next_up()).x == 2.0);
}

file_test!(maya_huge_stepped_tangents, |scene, err, load_error| {
    let opts = step_opts(BakeStepHandling::AdjacentDouble);
    let bake = bake_scene(scene, Some(&opts));

    let translation_ref = [
        key(0.0, [0.0, 0.0, 0.0], KEYFRAME),
        key(10.0_f64.next_down(), [0.0, 0.0, 0.0], STEP_LEFT),
        key(10.0, [1.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(10.0_f64.next_up(), [2.0, 0.0, 0.0], STEP_RIGHT),
        key(20.0, [2.0, 0.0, 0.0], KEYFRAME),
        key(100.0_f64.next_down(), [2.0, 0.0, 0.0], STEP_LEFT),
        key(100.0, [3.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(100.0_f64.next_up(), [4.0, 0.0, 0.0], STEP_RIGHT),
        key(200.0, [4.0, 0.0, 0.0], KEYFRAME),
        key(1000.0_f64.next_down(), [4.0, 0.0, 0.0], STEP_LEFT),
        key(1000.0, [5.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(1000.0_f64.next_up(), [6.0, 0.0, 0.0], STEP_RIGHT),
        key(2000.0, [6.0, 0.0, 0.0], KEYFRAME),
        key(10000.0_f64.next_down(), [6.0, 0.0, 0.0], STEP_LEFT),
        key(10000.0, [7.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(10000.0_f64.next_up(), [8.0, 0.0, 0.0], STEP_RIGHT),
        key(20000.0, [8.0, 0.0, 0.0], KEYFRAME),
        key(100000.0_f64.next_down(), [8.0, 0.0, 0.0], STEP_LEFT),
        key(100000.0, [9.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(100000.0_f64.next_up(), [10.0, 0.0, 0.0], STEP_RIGHT),
        key(200000.0, [10.0, 0.0, 0.0], KEYFRAME),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_vec3!(err, translation_ref, node.translation_keys);

    check_huge_step_values(node);
});

file_test!(maya_huge_stepped_tangents_identical, file = "maya_huge_stepped_tangents", |scene, err, load_error| {
    let opts = step_opts(BakeStepHandling::IdenticalTime);
    let bake = bake_scene(scene, Some(&opts));

    let translation_ref = [
        key(0.0, [0.0, 0.0, 0.0], KEYFRAME),
        key(10.0, [0.0, 0.0, 0.0], STEP_LEFT),
        key(10.0, [1.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(10.0, [2.0, 0.0, 0.0], STEP_RIGHT),
        key(20.0, [2.0, 0.0, 0.0], KEYFRAME),
        key(100.0, [2.0, 0.0, 0.0], STEP_LEFT),
        key(100.0, [3.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(100.0, [4.0, 0.0, 0.0], STEP_RIGHT),
        key(200.0, [4.0, 0.0, 0.0], KEYFRAME),
        key(1000.0, [4.0, 0.0, 0.0], STEP_LEFT),
        key(1000.0, [5.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(1000.0, [6.0, 0.0, 0.0], STEP_RIGHT),
        key(2000.0, [6.0, 0.0, 0.0], KEYFRAME),
        key(10000.0, [6.0, 0.0, 0.0], STEP_LEFT),
        key(10000.0, [7.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(10000.0, [8.0, 0.0, 0.0], STEP_RIGHT),
        key(20000.0, [8.0, 0.0, 0.0], KEYFRAME),
        key(100000.0, [8.0, 0.0, 0.0], STEP_LEFT),
        key(100000.0, [9.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(100000.0, [10.0, 0.0, 0.0], STEP_RIGHT),
        key(200000.0, [10.0, 0.0, 0.0], KEYFRAME),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_vec3!(err, translation_ref, node.translation_keys);

    check_huge_step_values(node);
});

file_test!(maya_huge_stepped_tangents_default, file = "maya_huge_stepped_tangents", |scene, err, load_error| {
    let bake = bake_scene(scene, None);

    let step = 0.001;
    let epsilon = 1.0 + f32::EPSILON as f64 * 4.0;
    let translation_ref = [
        key(0.0, [0.0, 0.0, 0.0], KEYFRAME),
        key(10.0 - step, [0.0, 0.0, 0.0], STEP_LEFT),
        key(10.0, [1.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(10.0 + step, [2.0, 0.0, 0.0], STEP_RIGHT),
        key(20.0, [2.0, 0.0, 0.0], KEYFRAME),
        key(100.0 - step, [2.0, 0.0, 0.0], STEP_LEFT),
        key(100.0, [3.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(100.0 + step, [4.0, 0.0, 0.0], STEP_RIGHT),
        key(200.0, [4.0, 0.0, 0.0], KEYFRAME),
        key(1000.0 - step, [4.0, 0.0, 0.0], STEP_LEFT),
        key(1000.0, [5.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(1000.0 + step, [6.0, 0.0, 0.0], STEP_RIGHT),
        key(2000.0, [6.0, 0.0, 0.0], KEYFRAME),
        key(10000.0 / epsilon, [6.0, 0.0, 0.0], STEP_LEFT),
        key(10000.0, [7.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(10000.0 * epsilon, [8.0, 0.0, 0.0], STEP_RIGHT),
        key(20000.0, [8.0, 0.0, 0.0], KEYFRAME),
        key(100000.0 / epsilon, [8.0, 0.0, 0.0], STEP_LEFT),
        key(100000.0, [9.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(100000.0 * epsilon, [10.0, 0.0, 0.0], STEP_RIGHT),
        key(200000.0, [10.0, 0.0, 0.0], KEYFRAME),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_vec3!(err, translation_ref, node.translation_keys);

    check_huge_step_values(node);
});

file_test!(maya_huge_stepped_tangents_custom, file = "maya_huge_stepped_tangents", |scene, err, load_error| {
    let opts = BakeOpts { step_handling: BakeStepHandling::CustomDuration, step_custom_duration: 0.1, step_custom_epsilon: 0.0005, ..Default::default() };
    let bake = bake_scene(scene, Some(&opts));

    let step = 0.1;
    let epsilon = 1.0 + 0.0005;
    let translation_ref = [
        key(0.0, [0.0, 0.0, 0.0], KEYFRAME),
        key(10.0 - step, [0.0, 0.0, 0.0], STEP_LEFT),
        key(10.0, [1.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(10.0 + step, [2.0, 0.0, 0.0], STEP_RIGHT),
        key(20.0, [2.0, 0.0, 0.0], KEYFRAME),
        key(100.0 - step, [2.0, 0.0, 0.0], STEP_LEFT),
        key(100.0, [3.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(100.0 + step, [4.0, 0.0, 0.0], STEP_RIGHT),
        key(200.0, [4.0, 0.0, 0.0], KEYFRAME),
        key(1000.0 / epsilon, [4.0, 0.0, 0.0], STEP_LEFT),
        key(1000.0, [5.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(1000.0 * epsilon, [6.0, 0.0, 0.0], STEP_RIGHT),
        key(2000.0, [6.0, 0.0, 0.0], KEYFRAME),
        key(10000.0 / epsilon, [6.0, 0.0, 0.0], STEP_LEFT),
        key(10000.0, [7.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(10000.0 * epsilon, [8.0, 0.0, 0.0], STEP_RIGHT),
        key(20000.0, [8.0, 0.0, 0.0], KEYFRAME),
        key(100000.0 / epsilon, [8.0, 0.0, 0.0], STEP_LEFT),
        key(100000.0, [9.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(100000.0 * epsilon, [10.0, 0.0, 0.0], STEP_RIGHT),
        key(200000.0, [10.0, 0.0, 0.0], KEYFRAME),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_vec3!(err, translation_ref, node.translation_keys);
});

file_test!(maya_huge_stepped_tangents_huge_custom, file = "maya_huge_stepped_tangents", |scene, err, load_error| {
    let opts = BakeOpts { step_handling: BakeStepHandling::CustomDuration, step_custom_duration: 100.0, ..Default::default() };
    let bake = bake_scene(scene, Some(&opts));

    let step = 100.0;
    let translation_ref = [
        key(0.0, [0.0, 0.0, 0.0], KEYFRAME),
        key(10.0, [1.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(20.0, [2.0, 0.0, 0.0], KEYFRAME),
        key(100.0, [3.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(200.0, [4.0, 0.0, 0.0], KEYFRAME),
        key(1000.0 - step, [4.0, 0.0, 0.0], STEP_LEFT),
        key(1000.0, [5.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(1000.0 + step, [6.0, 0.0, 0.0], STEP_RIGHT),
        key(2000.0, [6.0, 0.0, 0.0], KEYFRAME),
        key(10000.0 - step, [6.0, 0.0, 0.0], STEP_LEFT),
        key(10000.0, [7.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(10000.0 + step, [8.0, 0.0, 0.0], STEP_RIGHT),
        key(20000.0, [8.0, 0.0, 0.0], KEYFRAME),
        key(100000.0 - step, [8.0, 0.0, 0.0], STEP_LEFT),
        key(100000.0, [9.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(100000.0 + step, [10.0, 0.0, 0.0], STEP_RIGHT),
        key(200000.0, [10.0, 0.0, 0.0], KEYFRAME),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_vec3!(err, translation_ref, node.translation_keys);
});

file_test!(maya_huge_stepped_tangents_huge_resample, file = "maya_huge_stepped_tangents", |scene, err, load_error| {
    let opts = BakeOpts { maximum_sample_rate: 100.0, ..Default::default() };
    let bake = bake_scene(scene, Some(&opts));

    let step = 1.0 / 100.0;
    let translation_ref = [
        key(0.0, [0.0, 0.0, 0.0], KEYFRAME),
        key(10.0 - step, [0.0, 0.0, 0.0], REDUCED),
        key(10.0, [1.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(10.0 + step, [2.0, 0.0, 0.0], REDUCED),
        key(20.0, [2.0, 0.0, 0.0], KEYFRAME),
        key(100.0 - step, [2.0, 0.0, 0.0], REDUCED),
        key(100.0, [3.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(100.0 + step, [4.0, 0.0, 0.0], REDUCED),
        key(200.0, [4.0, 0.0, 0.0], KEYFRAME),
        key(1000.0 - step, [4.0, 0.0, 0.0], REDUCED),
        key(1000.0, [5.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(1000.0 + step, [6.0, 0.0, 0.0], REDUCED),
        key(2000.0, [6.0, 0.0, 0.0], KEYFRAME),
        key(10000.0 - step, [6.0, 0.0, 0.0], REDUCED),
        key(10000.0, [7.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(10000.0 + step, [8.0, 0.0, 0.0], REDUCED),
        key(20000.0, [8.0, 0.0, 0.0], KEYFRAME),
        key(100000.0 - step, [8.0, 0.0, 0.0], REDUCED),
        key(100000.0, [9.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(100000.0 + step, [10.0, 0.0, 0.0], REDUCED),
        key(200000.0, [10.0, 0.0, 0.0], KEYFRAME),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_vec3!(err, translation_ref, node.translation_keys);
});

file_test!(maya_stepped_tangent_sign, |scene, err, load_error| {
    let opts = step_opts(BakeStepHandling::AdjacentDouble);
    let bake = bake_scene(scene, Some(&opts));

    let translation_ref = [
        key(-2.0, [-3.0, 0.0, 0.0], KEYFRAME),
        key((-1.0_f64).next_down(), [-3.0, 0.0, 0.0], STEP_LEFT),
        key(-1.0, [-2.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key((-1.0_f64).next_up(), [-1.0, 0.0, 0.0], STEP_RIGHT),
        key(-0.1, [-1.0, 0.0, 0.0], KEYFRAME),
        key(0.1, [1.0, 0.0, 0.0], KEYFRAME),
        key(1.0_f64.next_down(), [1.0, 0.0, 0.0], STEP_LEFT),
        key(1.0, [2.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(1.0_f64.next_up(), [3.0, 0.0, 0.0], STEP_RIGHT),
        key(2.0, [3.0, 0.0, 0.0], KEYFRAME),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_vec3!(err, translation_ref, node.translation_keys);
});

file_test!(maya_stepped_tangent_sign_default, file = "maya_stepped_tangent_sign", |scene, err, load_error| {
    let opts = BakeOpts::default();
    let bake = bake_scene(scene, Some(&opts));

    let step = 0.001;
    let translation_ref = [
        key(-2.0, [-3.0, 0.0, 0.0], KEYFRAME),
        key(-1.0 - step, [-3.0, 0.0, 0.0], STEP_LEFT),
        key(-1.0, [-2.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(-1.0 + step, [-1.0, 0.0, 0.0], STEP_RIGHT),
        key(-0.1, [-1.0, 0.0, 0.0], KEYFRAME),
        key(0.1, [1.0, 0.0, 0.0], KEYFRAME),
        key(1.0 - step, [1.0, 0.0, 0.0], STEP_LEFT),
        key(1.0, [2.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(1.0 + step, [3.0, 0.0, 0.0], STEP_RIGHT),
        key(2.0, [3.0, 0.0, 0.0], KEYFRAME),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_vec3!(err, translation_ref, node.translation_keys);
});

file_test!(maya_stepped_tangent_sign_epsilon, file = "maya_stepped_tangent_sign", |scene, err, load_error| {
    let opts = BakeOpts { step_handling: BakeStepHandling::CustomDuration, step_custom_epsilon: 0.05, ..Default::default() };
    let bake = bake_scene(scene, Some(&opts));

    let epsilon = 1.0 + 0.05;
    let translation_ref = [
        key(-2.0, [-3.0, 0.0, 0.0], KEYFRAME),
        key(-1.0 * epsilon, [-3.0, 0.0, 0.0], STEP_LEFT),
        key(-1.0, [-2.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(-1.0 / epsilon, [-1.0, 0.0, 0.0], STEP_RIGHT),
        key(-0.1, [-1.0, 0.0, 0.0], KEYFRAME),
        key(0.1, [1.0, 0.0, 0.0], KEYFRAME),
        key(1.0 / epsilon, [1.0, 0.0, 0.0], STEP_LEFT),
        key(1.0, [2.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(1.0 * epsilon, [3.0, 0.0, 0.0], STEP_RIGHT),
        key(2.0, [3.0, 0.0, 0.0], KEYFRAME),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_vec3!(err, translation_ref, node.translation_keys);
});

file_test!(maya_stepped_tangent_sign_huge_step, file = "maya_stepped_tangent_sign", |scene, err, load_error| {
    let opts = BakeOpts { step_handling: BakeStepHandling::CustomDuration, step_custom_duration: 0.25, ..Default::default() };
    let bake = bake_scene(scene, Some(&opts));

    let step = 0.25;
    let translation_ref = [
        key(-2.0, [-3.0, 0.0, 0.0], KEYFRAME),
        key(-1.0 - step, [-3.0, 0.0, 0.0], STEP_LEFT),
        key(-1.0, [-2.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(-1.0 + step, [-1.0, 0.0, 0.0], STEP_RIGHT),
        key(-0.1, [-1.0, 0.0, 0.0], KEYFRAME),
        key(0.1, [1.0, 0.0, 0.0], KEYFRAME),
        key(1.0 - step, [1.0, 0.0, 0.0], STEP_LEFT),
        key(1.0, [2.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key(1.0 + step, [3.0, 0.0, 0.0], STEP_RIGHT),
        key(2.0, [3.0, 0.0, 0.0], KEYFRAME),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_vec3!(err, translation_ref, node.translation_keys);
});

file_test!(maya_late_stepped_tangents, |scene, err, load_error| {
    let opts = step_opts(BakeStepHandling::AdjacentDouble);
    let bake = bake_scene(scene, Some(&opts));

    let translation_ref = [
        key(80.0 / 24.0, [-3.0, 0.0, 0.0], KEYFRAME),
        key((85.0_f64 / 24.0).next_down(), [-3.0, 0.0, 0.0], STEP_LEFT),
        key(85.0 / 24.0, [-1.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key((85.0_f64 / 24.0).next_up(), [0.0, 0.0, 0.0], STEP_RIGHT),
        key(90.0 / 24.0, [0.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key((90.0_f64 / 24.0).next_up(), [1.0, 0.0, 0.0], STEP_RIGHT),
        key(95.0 / 24.0, [1.0, 0.0, 0.0], KEYFRAME),
        key((100.0_f64 / 24.0).next_down(), [1.0, 0.0, 0.0], STEP_LEFT),
        key(100.0 / 24.0, [2.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_vec3!(err, translation_ref, node.translation_keys);
});

file_test!(maya_late_stepped_tangents_trim, file = "maya_late_stepped_tangents", |scene, err, load_error| {
    let opts = BakeOpts { step_handling: BakeStepHandling::AdjacentDouble, trim_start_time: true, ..Default::default() };
    let bake = bake_scene(scene, Some(&opts));

    let translation_ref = [
        key(0.0 / 24.0, [-3.0, 0.0, 0.0], KEYFRAME),
        key((5.0_f64 / 24.0).next_down(), [-3.0, 0.0, 0.0], STEP_LEFT),
        key(5.0 / 24.0, [-1.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key((5.0_f64 / 24.0).next_up(), [0.0, 0.0, 0.0], STEP_RIGHT),
        key(10.0 / 24.0, [0.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
        key((10.0_f64 / 24.0).next_up(), [1.0, 0.0, 0.0], STEP_RIGHT),
        key(15.0 / 24.0, [1.0, 0.0, 0.0], KEYFRAME),
        key((20.0_f64 / 24.0).next_down(), [1.0, 0.0, 0.0], STEP_LEFT),
        key(20.0 / 24.0, [2.0, 0.0, 0.0], KEYFRAME | STEP_KEY),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_vec3!(err, translation_ref, node.translation_keys);
});

file_test!(maya_anim_diffuse_curve, |scene, err, load_error| {
    let opts = BakeOpts { resample_rate: 24.0, ..Default::default() };

    let bake = bake_scene(scene, Some(&opts));

    ufbxt_assert!(bake.nodes.is_empty());
    ufbxt_assert!(bake.elements.len() == 1);

    let elem = &bake.elements[0];
    let ref_elem = scene.element(elem.element_id);
    ufbxt_assert!(ref_elem.element_type == ElementType::Material);
    ufbxt_assert!(ref_elem.name == "lambert1");

    ufbxt_assert!(elem.props.len() == 1);
    let prop = &elem.props[0];

    check_string(&prop.name);
    ufbxt_assert!(prop.name == "DiffuseColor");

    let diffuse_ref = [
        key(0.0 / 24.0, [0.0, 0.5, 0.5], KEYFRAME),
        key(4.0 / 24.0, [1.0, 0.5, 0.5], KEYFRAME),
        key(6.0 / 24.0 - 0.001, [1.0, 0.5, 0.5], STEP_LEFT),
        key(6.0 / 24.0, [0.75, 0.5, 0.5], KEYFRAME | STEP_KEY),
        key(7.0 / 24.0, [0.415319, 0.5, 0.5], 0),
        key(8.0 / 24.0, [0.795, 0.5, 0.5], 0),
        key(9.0 / 24.0, [1.152243, 0.5, 0.5], 0),
        key(10.0 / 24.0, [0.75, 0.5, 0.5], KEYFRAME | STEP_KEY),
        key(10.0 / 24.0 + 0.001, [0.25, 0.5, 0.5], STEP_RIGHT),
        key(12.0 / 24.0, [0.25, 0.5, 0.5], KEYFRAME),
        key(13.0 / 24.0, [0.75, 0.5, 0.5], KEYFRAME),
        key(14.0 / 24.0, [0.0, 0.5, 0.5], KEYFRAME),
    ];

    diff_baked_vec3!(err, diffuse_ref, prop.keys);
});

file_test!(anim_bake_alloc_fail, file = "maya_anim_diffuse_curve", |scene, err, load_error| {});

file_test!(
    anim_bake_alloc_fail_alt,
    file = "motionbuilder_sausage_rrss",
    opts = scale_helper_opts(),
    flags = ALLOW_INVALID_UNICODE,
    |scene, err, load_error| {}
);

file_test!(maya_anim_pivot_rotate, |scene, err, load_error| {
    let translation_ref = [
        key(0.0 / 24.0, [0.0, 0.0, 0.0], 0),
        key(1.0 / 24.0, [0.0, 0.066987, -0.250], 0),
        key(2.0 / 24.0, [0.0, 0.250, -0.433], 0),
        key(3.0 / 24.0, [0.0, 0.5, -0.5], 0),
        key(4.0 / 24.0, [0.0, 0.75, -0.433], 0),
        key(5.0 / 24.0, [0.0, 0.933013, -0.250], 0),
        key(6.0 / 24.0, [0.0, 1.0, 0.0], 0),
    ];

    {
        let opts = BakeOpts { resample_rate: 24.0, key_reduction_enabled: true, key_reduction_rotation: true, ..Default::default() };

        let bake = bake_scene(scene, Some(&opts));

        ufbxt_assert!(bake.nodes.len() == 1);
        let node = &bake.nodes[0];

        let ref_node = &scene.nodes[node.typed_id as usize];
        ufbxt_assert!(ref_node.element.element_id == node.element_id);
        ufbxt_assert!(ref_node.element.name == "pCube1");

        let rotation_ref = [key(0.0 / 24.0, [0.0, 0.0, 0.0], KEYFRAME), key(6.0 / 24.0, [180.0, 0.0, 0.0], KEYFRAME)];

        diff_baked_vec3!(err, translation_ref, node.translation_keys);
        diff_baked_quat!(err, rotation_ref, node.rotation_keys);
    }

    {
        let opts = BakeOpts { resample_rate: 24.0, key_reduction_enabled: true, ..Default::default() };

        let bake = bake_scene(scene, Some(&opts));

        ufbxt_assert!(bake.nodes.len() == 1);
        let node = &bake.nodes[0];

        let ref_node = &scene.nodes[node.typed_id as usize];
        ufbxt_assert!(ref_node.element.element_id == node.element_id);
        ufbxt_assert!(ref_node.element.name == "pCube1");

        let rotation_ref = [
            key(0.0 / 24.0, [0.0, 0.0, 0.0], KEYFRAME),
            key(1.0 / 24.0, [30.0, 0.0, 0.0], 0),
            key(2.0 / 24.0, [60.0, 0.0, 0.0], 0),
            key(3.0 / 24.0, [90.0, 0.0, 0.0], 0),
            key(4.0 / 24.0, [120.0, 0.0, 0.0], 0),
            key(5.0 / 24.0, [150.0, 0.0, 0.0], 0),
            key(6.0 / 24.0, [180.0, 0.0, 0.0], KEYFRAME),
        ];

        diff_baked_vec3!(err, translation_ref, node.translation_keys);
        diff_baked_quat!(err, rotation_ref, node.rotation_keys);
    }
});

fn anim_scale_helper_opts() -> LoadOpts {
    LoadOpts { inherit_mode_handling: InheritModeHandling::HelperNodes, scale_helper_name: "(scale helper)".into(), ..Default::default() }
}

// Per key: the frame, then per joint its scale and its global y
const NO_INHERIT_SCALE_REF: [(f64, [(Real, Real); 4]); 5] = [
    (0.0, [(1.0, 0.0), (2.0, 4.0), (1.0, 12.0), (1.0, 16.0)]),
    (6.0, [(1.5, 0.0), (1.5, 6.0), (1.0, 12.0), (1.0, 16.0)]),
    (12.0, [(2.0, 0.0), (1.0, 8.0), (1.0, 12.0), (1.0, 16.0)]),
    (18.0, [(0.5, 0.0), (1.5, 2.0), (1.0, 8.0), (1.0, 12.0)]),
    (24.0, [(0.5, 0.0), (2.0, 2.0), (1.5, 10.0), (1.0, 16.0)]),
];

fn find_joints(scene: &Scene) -> [usize; 4] {
    let nodes = ["joint1", "joint2", "joint3", "joint4"].map(|name| find_node(scene, name));
    ufbxt_assert!(nodes.iter().all(|n| n.is_some()));
    nodes.map(|n| n.unwrap())
}

fn check_no_inherit_scale_evaluated(err: &mut DiffError, scene: &Scene, nodes: &[usize; 4]) {
    for (key_ix, &(frame, key_nodes)) in NO_INHERIT_SCALE_REF.iter().enumerate() {
        let time = frame / 24.0;

        let state = evaluate_scene(scene, &scene.anim, time, &EvaluateOpts::default());
        ufbxt_assert!(state.is_ok());
        let state = state.unwrap();

        for (node_ix, &(scale, global_y)) in key_nodes.iter().enumerate() {
            ufbxt_hintf!("key_ix={key_ix} node_ix={node_ix}");

            let mut node = &state.nodes[scene.nodes[nodes[node_ix]].element.typed_id as usize];
            if let Some(helper) = node.scale_helper {
                node = &state.nodes[helper];
            }
            let node_to_world = node.node_to_world;
            let world_transform = matrix_to_transform(&node_to_world);

            let ref_translation = Vec3::new(0.0, global_y, 0.0);
            let ref_scale = Vec3::new(scale, scale, scale);

            assert_close_vec3(err, world_transform.translation, ref_translation);
            assert_close_vec3(err, world_transform.scale, ref_scale);
        }
    }
}

file_test!(maya_anim_no_inherit_scale, |scene, err, load_error| {
    let nodes = find_joints(scene);
    check_no_inherit_scale_evaluated(err, scene, &nodes);
});

file_test!(maya_anim_no_inherit_scale_helper, file = "maya_anim_no_inherit_scale", opts = anim_scale_helper_opts(), |scene, err, load_error| {
    let nodes = find_joints(scene);
    check_no_inherit_scale_evaluated(err, scene, &nodes);

    let opts = BakeOpts::default();
    let bake = bake_scene(scene, Some(&opts));

    for (key_ix, &(frame, key_nodes)) in NO_INHERIT_SCALE_REF.iter().enumerate() {
        let time = frame / 24.0;

        for (node_ix, &(scale, global_y)) in key_nodes.iter().enumerate() {
            ufbxt_hintf!("key_ix={key_ix} node_ix={node_ix}");

            let mut node = nodes[node_ix];
            if let Some(helper) = scene.nodes[node].scale_helper {
                node = helper;
            }
            let node_to_world = evaluate_baked_matrix(scene, &bake, node, time);
            let world_transform = matrix_to_transform(&node_to_world);

            let ref_translation = Vec3::new(0.0, global_y, 0.0);
            let ref_scale = Vec3::new(scale, scale, scale);

            assert_close_vec3(err, world_transform.translation, ref_translation);
            assert_close_vec3(err, world_transform.scale, ref_scale);
        }
    }
});

fn ref_transform(translation: [Real; 3], rotation_euler: [Real; 3], scale: [Real; 3]) -> RefTransform {
    RefTransform {
        translation: Vec3::new(translation[0], translation[1], translation[2]),
        rotation_euler: Vec3::new(rotation_euler[0], rotation_euler[1], rotation_euler[2]),
        scale: Vec3::new(scale[0], scale[1], scale[2]),
    }
}

file_test!(maya_anim_layer_anim, |scene, err, load_error| {
    let opts = BakeOpts { resample_rate: 24.0, ..Default::default() };

    let bake = ufbxt_bake_anim(scene, &scene.anim, Some(&opts));
    ufbxt_assert!(bake.is_ok());
    let bake = bake.unwrap();

    ufbxt_assert!(bake.nodes.len() == 1);

    let node = &bake.nodes[0];
    ufbxt_assert!(scene.nodes[node.typed_id as usize].element.name == "pCube1");

    let one = [1.0, 1.0, 1.0];
    let r = [
        ref_transform([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], one),
        ref_transform([0.0, -0.305556, 0.0], [3.75, 0.0, 0.0], one),
        ref_transform([0.0, -0.555556, 0.0], [7.5, 0.0, 0.0], one),
        ref_transform([0.0, -0.75, 0.0], [11.25, 0.0, 0.0], one),
        ref_transform([0.0, -0.888889, 0.0], [15.0, 0.0, 0.0], one),
        ref_transform([0.0, -0.972222, 0.0], [18.75, 0.0, 0.0], one),
        ref_transform([0.0, -1.0, 0.0], [22.5, 0.0, 0.0], one),
        ref_transform([0.0, -0.972222, 0.0], [26.25, 0.0, 0.0], one),
        ref_transform([0.0, -0.888889, 0.0], [30.0, 0.0, 0.0], one),
        ref_transform([0.0, -0.75, 0.0], [33.75, 0.0, 0.0], one),
        ref_transform([0.0, -0.555556, 0.0], [37.5, 0.0, 0.0], one),
        ref_transform([0.0, -0.305556, 0.0], [41.25, 0.0, 0.0], one),
        ref_transform([0.0, 0.0, 0.0], [45.0, 0.0, 0.0], one),
        ref_transform([0.0, -0.333333, 0.0], [41.25, 0.0, 0.0], one),
        ref_transform([0.0, -0.666667, 0.0], [37.5, 0.0, 0.0], one),
        ref_transform([0.0, -1.0, 0.0], [33.75, 0.0, 0.0], one),
        ref_transform([0.0, -1.333333, 0.0], [30.0, 0.0, 0.0], one),
        ref_transform([0.0, -1.666667, 0.0], [26.25, 0.0, 0.0], one),
        ref_transform([0.0, -2.0, 0.0], [22.5, 0.0, 0.0], one),
        ref_transform([0.0, -2.333333, 0.0], [18.75, 0.0, 0.0], one),
        ref_transform([0.0, -2.666667, 0.0], [15.0, 0.0, 0.0], one),
        ref_transform([0.0, -3.0, 0.0], [11.25, 0.0, 0.0], one),
        ref_transform([0.0, -3.333333, 0.0], [7.5, 0.0, 0.0], one),
        ref_transform([0.0, -3.666667, 0.0], [3.75, 0.0, 0.0], one),
        ref_transform([0.0, -4.0, 0.0], [0.0, 0.0, 0.0], one),
    ];

    let num_keys = r.len();
    ufbxt_assert!(node.translation_keys.len() == num_keys);
    ufbxt_assert!(node.rotation_keys.len() == num_keys);
    ufbxt_assert!(node.scale_keys.len() == num_keys);

    for i in 0..num_keys {
        let time = i as f64 / 24.0;
        ufbxt_hintf!("i={i} time={time:.6}\n");

        ufbxt_assert!(node.translation_keys[i].time == time);
        ufbxt_assert!(node.rotation_keys[i].time == time);
        ufbxt_assert!(node.scale_keys[i].time == time);

        let rotation_quat = euler_to_quat(r[i].rotation_euler, RotationOrder::Xyz);

        assert_close_vec3(err, node.translation_keys[i].value, r[i].translation);
        assert_close_quat(err, node.rotation_keys[i].value, rotation_quat);
        assert_close_vec3(err, node.scale_keys[i].value, r[i].scale);

        assert_close_vec3(err, evaluate_baked_vec3(&node.translation_keys, time), r[i].translation);
        assert_close_quat(err, evaluate_baked_quat(&node.rotation_keys, time), rotation_quat);
        assert_close_vec3(err, evaluate_baked_vec3(&node.scale_keys, time), r[i].scale);
    }
});

fn check_key_times(scene: &Scene, maximum_sample_rate: f64, ref_times: &[f64]) {
    let opts = BakeOpts { maximum_sample_rate, ..Default::default() };
    let bake = ufbxt_bake_anim(scene, &scene.anim, Some(&opts));
    ufbxt_assert!(bake.is_ok());
    let bake = bake.unwrap();

    ufbxt_assert!(bake.nodes.len() == 1);
    let list = &bake.nodes[0].translation_keys;
    for (i, key) in list.iter().enumerate() {
        ufbxt_hintf!("i={i}");
        ufbxt_assert!(i < ref_times.len());
        ufbxt_hintf!("i={i}: {:.6} (ref {:.6})", key.time, ref_times[i]);
        ufbxt_assert!(key.time == ref_times[i]);
    }
    ufbxt_assert!(list.len() == ref_times.len());
}

file_test!(maya_keyframe_spacing, |scene, err, load_error| {
    check_key_times(scene, 10.0, &[0.0 / 10.0, 1.0 / 10.0, 2.0 / 10.0, 3.0 / 10.0, 4.0 / 10.0, 6.0 / 10.0, 8.0 / 10.0]);

    check_key_times(
        scene,
        20.0,
        &[0.0 / 20.0, 1.0 / 20.0, 2.0 / 20.0, 3.0 / 20.0, 4.0 / 20.0, 5.0 / 20.0, 44.0 / 120.0, 12.0 / 20.0, 15.0 / 20.0, 16.0 / 20.0],
    );

    check_key_times(
        scene,
        30.0,
        &[0.0 / 30.0, 1.0 / 30.0, 2.0 / 30.0, 3.0 / 30.0, 4.0 / 30.0, 5.0 / 30.0, 7.0 / 30.0, 8.0 / 30.0, 44.0 / 120.0, 18.0 / 30.0, 23.0 / 30.0],
    );
});

fn ufbxt_find_baked_node<'a>(bake: &'a BakedAnim, node: &Node) -> Option<&'a BakedNode> {
    bake.nodes.iter().find(|baked| baked.typed_id == node.element.typed_id)
}

fn check_scale_no_inherit_step(err: &mut DiffError, scene: &Scene, bake: &BakedAnim, scale_ref: &[BakedVec3], translation_ref: &[BakedVec3]) {
    let joint1_node = find_node(scene, "joint1").map(|n| &scene.nodes[n]);
    let joint2_node = find_node(scene, "joint2").map(|n| &scene.nodes[n]);
    ufbxt_assert!(joint1_node.is_some());
    ufbxt_assert!(joint1_node.unwrap().scale_helper.is_some());
    ufbxt_assert!(joint2_node.is_some());

    let joint1_helper = ufbxt_find_baked_node(bake, &scene.nodes[joint1_node.unwrap().scale_helper.unwrap()]);
    let joint2 = ufbxt_find_baked_node(bake, joint2_node.unwrap());
    ufbxt_assert!(joint1_helper.is_some());
    ufbxt_assert!(joint2.is_some());
    diff_baked_vec3!(err, scale_ref, joint1_helper.unwrap().scale_keys);
    diff_baked_vec3!(err, translation_ref, joint2.unwrap().translation_keys);
}

file_test!(maya_scale_no_inherit_step, opts = scale_helper_opts(), |scene, err, load_error| {
    let opts = BakeOpts::default();
    let bake = bake_scene(scene, Some(&opts));

    let scale_ref = [
        key(0.0 / 24.0, [2.0, 1.0, 1.0], KEYFRAME),
        key(5.0 / 24.0 - 0.001, [2.0, 1.0, 1.0], STEP_LEFT),
        key(5.0 / 24.0, [3.0, 1.0, 1.0], KEYFRAME | STEP_KEY),
        key(10.0 / 24.0, [4.0, 1.0, 1.0], KEYFRAME | STEP_KEY),
        key(10.0 / 24.0 + 0.001, [5.0, 1.0, 1.0], STEP_RIGHT),
        key(15.0 / 24.0, [5.0, 1.0, 1.0], KEYFRAME),
    ];

    let translation_ref = [
        key(0.0 / 24.0, [10.0, 0.0, 0.0], 0),
        key(5.0 / 24.0 - 0.001, [10.0, 0.0, 0.0], STEP_LEFT),
        key(5.0 / 24.0, [15.0, 0.0, 0.0], STEP_KEY),
        key(10.0 / 24.0, [20.0, 0.0, 0.0], STEP_KEY),
        key(10.0 / 24.0 + 0.001, [25.0, 0.0, 0.0], STEP_RIGHT),
        key(15.0 / 24.0, [25.0, 0.0, 0.0], 0),
    ];

    check_scale_no_inherit_step(err, scene, &bake, &scale_ref, &translation_ref);
});

file_test!(maya_scale_no_inherit_step_identical, file = "maya_scale_no_inherit_step", opts = scale_helper_opts(), |scene, err, load_error| {
    let opts = step_opts(BakeStepHandling::IdenticalTime);
    let bake = bake_scene(scene, Some(&opts));

    let scale_ref = [
        key(0.0 / 24.0, [2.0, 1.0, 1.0], KEYFRAME),
        key(5.0 / 24.0, [2.0, 1.0, 1.0], STEP_LEFT),
        key(5.0 / 24.0, [3.0, 1.0, 1.0], KEYFRAME | STEP_KEY),
        key(10.0 / 24.0, [4.0, 1.0, 1.0], KEYFRAME | STEP_KEY),
        key(10.0 / 24.0, [5.0, 1.0, 1.0], STEP_RIGHT),
        key(15.0 / 24.0, [5.0, 1.0, 1.0], KEYFRAME),
    ];

    let translation_ref = [
        key(0.0 / 24.0, [10.0, 0.0, 0.0], 0),
        key(5.0 / 24.0, [10.0, 0.0, 0.0], STEP_LEFT),
        key(5.0 / 24.0, [15.0, 0.0, 0.0], STEP_KEY),
        key(10.0 / 24.0, [20.0, 0.0, 0.0], STEP_KEY),
        key(10.0 / 24.0, [25.0, 0.0, 0.0], STEP_RIGHT),
        key(15.0 / 24.0, [25.0, 0.0, 0.0], 0),
    ];

    check_scale_no_inherit_step(err, scene, &bake, &scale_ref, &translation_ref);
});

file_test!(maya_scale_no_inherit_step_adjacent, file = "maya_scale_no_inherit_step", opts = scale_helper_opts(), |scene, err, load_error| {
    let opts = step_opts(BakeStepHandling::AdjacentDouble);
    let bake = bake_scene(scene, Some(&opts));

    let scale_ref = [
        key(0.0 / 24.0, [2.0, 1.0, 1.0], KEYFRAME),
        key((5.0_f64 / 24.0).next_down(), [2.0, 1.0, 1.0], STEP_LEFT),
        key(5.0 / 24.0, [3.0, 1.0, 1.0], KEYFRAME | STEP_KEY),
        key(10.0 / 24.0, [4.0, 1.0, 1.0], KEYFRAME | STEP_KEY),
        key((10.0_f64 / 24.0).next_up(), [5.0, 1.0, 1.0], STEP_RIGHT),
        key(15.0 / 24.0, [5.0, 1.0, 1.0], KEYFRAME),
    ];

    let translation_ref = [
        key(0.0 / 24.0, [10.0, 0.0, 0.0], 0),
        key((5.0_f64 / 24.0).next_down(), [10.0, 0.0, 0.0], STEP_LEFT),
        key(5.0 / 24.0, [15.0, 0.0, 0.0], STEP_KEY),
        key(10.0 / 24.0, [20.0, 0.0, 0.0], STEP_KEY),
        key((10.0_f64 / 24.0).next_up(), [25.0, 0.0, 0.0], STEP_RIGHT),
        key(15.0 / 24.0, [25.0, 0.0, 0.0], 0),
    ];

    check_scale_no_inherit_step(err, scene, &bake, &scale_ref, &translation_ref);
});

file_test!(maya_scale_no_inherit_step_resmaple, file = "maya_scale_no_inherit_step", opts = scale_helper_opts(), |scene, err, load_error| {
    let opts = BakeOpts { maximum_sample_rate: 100.0, ..Default::default() };
    let bake = bake_scene(scene, Some(&opts));

    let step = 1.0 / 100.0;
    let scale_ref = [
        key(0.0 / 24.0, [2.0, 1.0, 1.0], KEYFRAME),
        key(5.0 / 24.0 - step, [2.0, 1.0, 1.0], REDUCED),
        key(5.0 / 24.0, [3.0, 1.0, 1.0], KEYFRAME | STEP_KEY),
        key(10.0 / 24.0, [4.0, 1.0, 1.0], KEYFRAME | STEP_KEY),
        key(10.0 / 24.0 + step, [5.0, 1.0, 1.0], REDUCED),
        key(15.0 / 24.0, [5.0, 1.0, 1.0], KEYFRAME),
    ];

    let translation_ref = [
        key(0.0 / 24.0, [10.0, 0.0, 0.0], 0),
        key(5.0 / 24.0 - step, [10.0, 0.0, 0.0], 0),
        key(5.0 / 24.0, [15.0, 0.0, 0.0], STEP_KEY),
        key(10.0 / 24.0, [20.0, 0.0, 0.0], STEP_KEY),
        key(10.0 / 24.0 + step, [25.0, 0.0, 0.0], 0),
        key(15.0 / 24.0, [25.0, 0.0, 0.0], 0),
    ];

    check_scale_no_inherit_step(err, scene, &bake, &scale_ref, &translation_ref);
});

file_test!(maya_keyframe_offset, |scene, err, load_error| {
    let opts = BakeOpts { resample_rate: 30.0, ..Default::default() };
    let bake = bake_scene(scene, Some(&opts));

    let rotation_ref = [
        key(0.0 / 30.0, [0.0, 0.0, 0.0], KEYFRAME),
        key(1.0 / 30.0, [0.0, 0.0, -3.867188], 0),
        key(2.0 / 30.0, [0.0, 0.0, -14.0625], 0),
        key(3.0 / 30.0, [0.0, 0.0, -28.476563], 0),
        key(4.0 / 30.0, [0.0, 0.0, -45.0], 0),
        key(5.0 / 30.0, [0.0, 0.0, -61.523438], 0),
        key(6.0 / 30.0, [0.0, 0.0, -75.9375], 0),
        key(7.0 / 30.0, [0.0, 0.0, -86.132813], 0),
        key(8.0 / 30.0, [0.0, 0.0, -90.0], KEYFRAME),
    ];

    // FBX representation of 6401/48000
    let mid_time = 6159116611.0 / 46186158000.0;
    let translation_ref = [
        key(0.0 / 30.0, [0.000000, 2.000000, 0.000000], KEYFRAME),
        key(1.0 / 30.0, [0.134888, 1.995446, 0.249960], 0),
        key(2.0 / 30.0, [0.485960, 1.940062, 0.499921], 0),
        key(3.0 / 30.0, [0.953598, 1.758024, 0.749882], 0),
        key(mid_time, [1.414473, 1.413953, 1.000000], KEYFRAME),
        key(5.0 / 30.0, [1.758024, 0.953598, 1.499765], 0),
        key(6.0 / 30.0, [1.940062, 0.485960, 1.999843], 0),
        key(7.0 / 30.0, [1.995446, 0.134888, 2.499921], 0),
        key(8.0 / 30.0, [2.000000, 0.000000, 3.000000], KEYFRAME),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_quat!(err, rotation_ref, node.rotation_keys);
    diff_baked_vec3!(err, translation_ref, node.translation_keys);
});

file_test!(maya_keyframe_offset_step, |scene, err, load_error| {
    let opts = BakeOpts { resample_rate: 30.0, ..Default::default() };
    let bake = bake_scene(scene, Some(&opts));

    let rotation_ref = [
        key(0.0 / 30.0, [0.0, 0.0, 0.0], KEYFRAME),
        key(4.0 / 30.0 - 0.001, [0.0, 0.0, 0.0], STEP_LEFT),
        key(4.0 / 30.0, [0.0, 0.0, -45.0], KEYFRAME | STEP_KEY),
        key(5.0 / 30.0, [0.0, 0.0, -52.253807], 0),
        key(6.0 / 30.0, [0.0, 0.0, -67.697828], 0),
        key(7.0 / 30.0, [0.0, 0.0, -83.042936], 0),
        key(8.0 / 30.0, [0.0, 0.0, -90.0], KEYFRAME),
    ];

    // FBX representation of 6401/48000
    let mid_time = 6159116611.0 / 46186158000.0;
    let translation_ref = [
        key(0.0 / 30.0, [0.000000, 2.000000, 0.000000], KEYFRAME),
        key(4.0 / 30.0 - 0.001, [0.000000, 2.000000, 0.999843], STEP_LEFT),
        key(4.0 / 30.0, [1.414213, 1.414213, 0.999843], STEP_KEY),
        key(mid_time, [1.414219, 1.414207, 1.000000], KEYFRAME),
        key(5.0 / 30.0, [1.581460, 1.224329, 1.499765], 0),
        key(6.0 / 30.0, [1.850390, 0.758982, 1.999843], 0),
        key(7.0 / 30.0, [1.985274, 0.242251, 2.499921], 0),
        key(8.0 / 30.0, [2.000000, 0.000000, 3.000000], KEYFRAME),
    ];

    ufbxt_assert!(bake.nodes.len() == 1);
    let node = &bake.nodes[0];
    diff_baked_quat!(err, rotation_ref, node.rotation_keys);
    diff_baked_vec3!(err, translation_ref, node.translation_keys);
});

fn find_translation_prop(elem: &BakedElement) -> Option<&BakedProp> {
    elem.props.iter().find(|prop| prop.name == "Lcl Translation")
}

file_test!(maya_linear_spline, |scene, err, load_error| {
    {
        let opts = BakeOpts { resample_rate: 24.0, bake_transform_props: true, ..Default::default() };
        let bake = bake_scene(scene, Some(&opts));

        let translation_ref = [
            key(0.0 / 24.0, [0.0, 0.0, 0.0], KEYFRAME),
            key(1.0 / 24.0, [0.333333, 0.0, 0.0], 0),
            key(2.0 / 24.0, [0.666666, 0.0, 0.0], 0),
            key(3.0 / 24.0, [1.0, 0.0, 0.0], 0),
            key(4.0 / 24.0, [1.333333, 0.0, 0.0], 0),
            key(5.0 / 24.0, [1.666666, 0.0, 0.0], 0),
            key(6.0 / 24.0, [2.0, 0.0, 0.0], 0),
            key(7.0 / 24.0, [2.333333, 0.0, 0.0], 0),
            key(8.0 / 24.0, [2.666666, 0.0, 0.0], 0),
            key(9.0 / 24.0, [3.0, 0.0, 0.0], 0),
            key(10.0 / 24.0, [3.333333, 0.0, 0.0], 0),
            key(11.0 / 24.0, [3.666666, 0.0, 0.0], 0),
            key(12.0 / 24.0, [4.0, 0.0, 0.0], KEYFRAME),
        ];

        ufbxt_assert!(bake.nodes.len() == 1);
        let node = &bake.nodes[0];
        diff_baked_vec3!(err, translation_ref, node.translation_keys);

        ufbxt_assert!(bake.elements.len() == 1);
        let elem = &bake.elements[0];
        ufbxt_assert!(elem.element_id == node.element_id);
        let prop = find_translation_prop(elem);
        ufbxt_assert!(prop.is_some());
        diff_baked_vec3!(err, translation_ref, prop.unwrap().keys);
    }

    {
        let opts = BakeOpts { resample_rate: 24.0, bake_transform_props: true, key_reduction_enabled: true, key_reduction_passes: 16, ..Default::default() };
        let bake = bake_scene(scene, Some(&opts));

        let translation_ref = [key(0.0 / 24.0, [0.0, 0.0, 0.0], KEYFRAME), key(12.0 / 24.0, [4.0, 0.0, 0.0], KEYFRAME)];

        ufbxt_assert!(bake.nodes.len() == 1);
        let node = &bake.nodes[0];
        diff_baked_vec3!(err, translation_ref, node.translation_keys);

        ufbxt_assert!(bake.elements.len() == 1);
        let elem = &bake.elements[0];
        ufbxt_assert!(elem.element_id == node.element_id);
        let prop = find_translation_prop(elem);
        ufbxt_assert!(prop.is_some());
        diff_baked_vec3!(err, translation_ref, prop.unwrap().keys);
    }

    {
        let opts = BakeOpts { resample_rate: 24.0, bake_transform_props: true, key_reduction_enabled: true, key_reduction_passes: 1, ..Default::default() };
        let bake = bake_scene(scene, Some(&opts));

        let translation_ref = [
            key(0.0 / 24.0, [0.0, 0.0, 0.0], KEYFRAME),
            key(2.0 / 24.0, [0.666666, 0.0, 0.0], 0),
            key(4.0 / 24.0, [1.333333, 0.0, 0.0], 0),
            key(6.0 / 24.0, [2.0, 0.0, 0.0], 0),
            key(8.0 / 24.0, [2.666666, 0.0, 0.0], 0),
            key(10.0 / 24.0, [3.333333, 0.0, 0.0], 0),
            key(12.0 / 24.0, [4.0, 0.0, 0.0], KEYFRAME),
        ];

        ufbxt_assert!(bake.nodes.len() == 1);
        let node = &bake.nodes[0];
        diff_baked_vec3!(err, translation_ref, node.translation_keys);

        ufbxt_assert!(bake.elements.len() == 1);
        let elem = &bake.elements[0];
        ufbxt_assert!(elem.element_id == node.element_id);
        let prop = find_translation_prop(elem);
        ufbxt_assert!(prop.is_some());
        diff_baked_vec3!(err, translation_ref, prop.unwrap().keys);
    }

    {
        let opts = BakeOpts {
            resample_rate: 24.0,
            bake_transform_props: true,
            key_reduction_enabled: true,
            skip_node_transforms: true,
            key_reduction_passes: 16,
            ..Default::default()
        };
        let bake = bake_scene(scene, Some(&opts));

        let translation_ref = [key(0.0 / 24.0, [0.0, 0.0, 0.0], KEYFRAME), key(12.0 / 24.0, [4.0, 0.0, 0.0], KEYFRAME)];

        ufbxt_assert!(bake.nodes.is_empty());

        ufbxt_assert!(bake.elements.len() == 1);
        let elem = &bake.elements[0];
        let prop = find_translation_prop(elem);
        ufbxt_assert!(prop.is_some());
        diff_baked_vec3!(err, translation_ref, prop.unwrap().keys);
    }
});

file_test!(bake_search_node, file = "maya_anim_linear", |scene, err, load_error| {
    let bake = bake_scene(scene, None);

    let scene_node_ix = find_node(scene, "pCube1");
    ufbxt_assert!(scene_node_ix.is_some());
    let scene_node = &scene.nodes[scene_node_ix.unwrap()];

    let node = find_baked_node(&bake, scene_node_ix.unwrap());
    ufbxt_assert!(node.is_some());
    let node = node.unwrap();
    ufbxt_assert!(node.typed_id == scene_node.element.typed_id);
    ufbxt_assert!(node.element_id == scene_node.element.element_id);

    ufbxt_assert!(find_baked_node_by_typed_id(&bake, node.typed_id).is_some_and(|n| std::ptr::eq(n, node)));

    ufbxt_assert!(find_baked_node_by_typed_id(&bake, u32::MAX).is_none());
    ufbxt_assert!(find_baked_node(&bake, scene.root_node).is_none());
});

file_test!(bake_search_element, file = "maya_anim_diffuse_curve", |scene, err, load_error| {
    let bake = bake_scene(scene, None);

    let material = find_material(scene, "lambert1");
    ufbxt_assert!(material.is_some());
    let material = &scene.materials[material.unwrap()];

    let element = find_baked_element(&bake, material.element.element_id);
    ufbxt_assert!(element.is_some());
    let element = element.unwrap();
    ufbxt_assert!(element.element_id == material.element.element_id);

    ufbxt_assert!(find_baked_element_by_element_id(&bake, material.element.element_id).is_some_and(|e| std::ptr::eq(e, element)));

    ufbxt_assert!(find_baked_element_by_element_id(&bake, u32::MAX).is_none());
    ufbxt_assert!(find_baked_element(&bake, scene.nodes[scene.root_node].element.element_id).is_none());
});

fn ignore_animation_opts() -> LoadOpts {
    LoadOpts { ignore_animation: true, ..Default::default() }
}

fn assert_close_extrapolation(p_err: &mut DiffError, value: Real, r: Real) {
    let abs_error = 0.001;
    let rel_error = r.abs().max(1.0) * 1e-4;

    let threshold = f64::min(abs_error, rel_error);
    assert_close_real_threshold(p_err, value, r, threshold);
}

fn extrapolation_curve(scene: &Scene) -> &AnimCurve {
    let node = find_node(scene, "pCube1");
    ufbxt_assert!(node.is_some());
    let node = &scene.nodes[node.unwrap()];

    ufbxt_assert!(scene.anim_layers.len() == 1);
    let anim_layer = &scene.anim_layers[0];

    let anim_prop = find_anim_prop(anim_layer, node.element.element_id, "Lcl Translation");
    ufbxt_assert!(anim_prop.is_some());

    let anim_curve = scene.anim_values[anim_prop.unwrap().anim_value].curves[0];
    ufbxt_assert!(anim_curve.is_some());
    &scene.anim_curves[anim_curve.unwrap()]
}

fn check_extrapolation(err: &mut DiffError, scene: &Scene, anim_curve: &AnimCurve, refs: &[(i32, f64)]) {
    for (i, &(ref_frame, value)) in refs.iter().enumerate() {
        ufbxt_hintf!("i={i}");
        let mut ref_value = value;

        // Pre-7200 versions don't support relative repeat
        if ref_frame <= 0 && scene.metadata.version < 7200 {
            ref_value = anim_curve.keyframes[0].value;
        }

        let time = ref_frame as f64 / 24.0;

        let value = evaluate_curve(anim_curve, time, 0.0);
        assert_close_extrapolation(err, value, ref_value);
    }
}

file_test!(maya_anim_extrapolation, |scene, err, load_error| {
    let anim_curve = extrapolation_curve(scene);

    if scene.metadata.version >= 7200 {
        ufbxt_assert!(anim_curve.pre_extrapolation.mode == ExtrapolationMode::RepeatRelative);
    } else {
        ufbxt_assert!(anim_curve.pre_extrapolation.mode == ExtrapolationMode::Constant);
    }
    ufbxt_assert!(anim_curve.pre_extrapolation.repeat_count == -1);
    ufbxt_assert!(anim_curve.post_extrapolation.mode == ExtrapolationMode::Repeat);
    ufbxt_assert!(anim_curve.post_extrapolation.repeat_count == -1);
    ufbxt_assert!(anim_curve.min_time == 1.0 / 24.0);
    ufbxt_assert!(anim_curve.max_time == 1.0);
    ufbxt_assert!(anim_curve.keyframes.len() == 2);

    let refs = [
        (1, 0.0),
        (12, 4.674119),
        (24, 10.0),
        (25, 0.055067),
        (37, 5.972713),
        // ufbx keeps these three only where FLT_EVAL_METHOD is 0 (not x87), as on x86-64
        (47, 0.0),
        (5820, 0.0),
        (4459471, 0.0),
        (4459472, 0.055067),
        (9514493, 5.972713),
        (100109698, 5.325881),
        (0, -0.055067),
        (-12, -5.972713),
        (-70, -30.213693),
        (-4645, -2020.0),
        (-100509, -43700.0),
        (-100000181, -43478340.0),
        (-500000000, -217391304.674119),
    ];

    check_extrapolation(err, scene, anim_curve, &refs);
});

file_test!(maya_anim_extrapolation_ignore, file = "maya_anim_extrapolation", opts = ignore_animation_opts(), |scene, err, load_error| {
    let node = find_node(scene, "pCube1");
    ufbxt_assert!(node.is_some());
    let node = &scene.nodes[node.unwrap()];

    ufbxt_assert!(scene.anim_layers.len() == 1);
    let anim_layer = &scene.anim_layers[0];

    let anim_prop = find_anim_prop(anim_layer, node.element.element_id, "Lcl Translation");
    ufbxt_assert!(anim_prop.is_some());

    if scene.metadata.version >= 7000 {
        let anim_curve = scene.anim_values[anim_prop.unwrap().anim_value].curves[0];
        ufbxt_assert!(anim_curve.is_some());
        let anim_curve = &scene.anim_curves[anim_curve.unwrap()];

        if scene.metadata.version >= 7200 {
            ufbxt_assert!(anim_curve.pre_extrapolation.mode == ExtrapolationMode::RepeatRelative);
        } else {
            ufbxt_assert!(anim_curve.pre_extrapolation.mode == ExtrapolationMode::Constant);
        }
        ufbxt_assert!(anim_curve.pre_extrapolation.repeat_count == -1);
        ufbxt_assert!(anim_curve.post_extrapolation.mode == ExtrapolationMode::Repeat);
        ufbxt_assert!(anim_curve.post_extrapolation.repeat_count == -1);
        ufbxt_assert!(anim_curve.keyframes.is_empty());
    }
});

file_test!(motionbuilder_extrapolation_slope, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    let anim_curve = extrapolation_curve(scene);

    ufbxt_assert!(anim_curve.post_extrapolation.mode == ExtrapolationMode::Slope);
    ufbxt_assert!(anim_curve.pre_extrapolation.repeat_count == -1);
    ufbxt_assert!(anim_curve.post_extrapolation.mode == ExtrapolationMode::Slope);
    ufbxt_assert!(anim_curve.post_extrapolation.repeat_count == -1);
    ufbxt_assert!(anim_curve.min_time == 1.0 / 24.0);
    ufbxt_assert!(anim_curve.max_time == 1.0);
    ufbxt_assert!(anim_curve.keyframes.len() == 2);

    let refs = [
        (5, -2.051765),
        (20, 11.633479),
        (30, 3.016101),
        (61, -33.067375),
        (14614, -16972.513672),
        (-5, 7.743417),
        (-33, 43.879364),
        (-94, 122.604103),
        (-242, 313.608398),
        (-14301, 18457.726562),
    ];

    check_extrapolation(err, scene, anim_curve, &refs);
});

file_test!(motionbuilder_extrapolation_mirror, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    let anim_curve = extrapolation_curve(scene);

    ufbxt_assert!(anim_curve.post_extrapolation.mode == ExtrapolationMode::Mirror);
    ufbxt_assert!(anim_curve.pre_extrapolation.repeat_count == -1);
    ufbxt_assert!(anim_curve.post_extrapolation.mode == ExtrapolationMode::Mirror);
    ufbxt_assert!(anim_curve.post_extrapolation.repeat_count == -1);
    ufbxt_assert!(anim_curve.min_time == 1.0 / 24.0);
    ufbxt_assert!(anim_curve.max_time == 1.0);
    ufbxt_assert!(anim_curve.keyframes.len() == 2);

    let refs = [
        (18, 10.635811),
        (30, 10.635811),
        (42, -1.749766),
        (52, -1.749766),
        (70, 10.000000),
        (77, 9.815335),
        (355, 7.719610),
        (1706, -2.065491),
        (17357, 7.719610),
        (44874, 10.000000),
        (44897, 0.000000),
        (-24, 11.522083),
        (-52, -0.431755),
        (-414, -1.077311),
        (-2207, 0.000000),
        (-27612, 6.519766),
        (-36132, 10.000000),
        (-36155, 0.000000),
    ];

    check_extrapolation(err, scene, anim_curve, &refs);
});

file_test!(motionbuilder_extrapolation_mirror_count, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    let anim_curve = extrapolation_curve(scene);

    ufbxt_assert!(anim_curve.post_extrapolation.mode == ExtrapolationMode::Mirror);
    ufbxt_assert!(anim_curve.pre_extrapolation.repeat_count == 2);
    ufbxt_assert!(anim_curve.post_extrapolation.mode == ExtrapolationMode::Mirror);
    ufbxt_assert!(anim_curve.post_extrapolation.repeat_count == 3);
    ufbxt_assert!(anim_curve.min_time == 1.0 / 24.0);
    ufbxt_assert!(anim_curve.max_time == 1.0);
    ufbxt_assert!(anim_curve.keyframes.len() == 2);

    let refs = [
        (7, -1.197194),
        (-16, 10.635811),
        (-29, 9.815335),
        (-42, -2.065491),
        (-45, 0.000000),
        (-73, 0.000000),
        (-100000, 0.000000),
        (29, 11.254428),
        (42, -1.749766),
        (65, 11.254428),
        (74, 11.633479),
        (89, -2.051765),
        (93, 0.000000),
        (100, 0.000000),
        (100000, 0.000000),
    ];

    check_extrapolation(err, scene, anim_curve, &refs);
});

file_test!(motionbuilder_extrapolation_repeat_count, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    let anim_curve = extrapolation_curve(scene);

    ufbxt_assert!(anim_curve.post_extrapolation.mode == ExtrapolationMode::Repeat);
    ufbxt_assert!(anim_curve.pre_extrapolation.repeat_count == 2);
    ufbxt_assert!(anim_curve.post_extrapolation.mode == ExtrapolationMode::Repeat);
    ufbxt_assert!(anim_curve.post_extrapolation.repeat_count == 0);
    ufbxt_assert!(anim_curve.min_time == 1.0 / 24.0);
    ufbxt_assert!(anim_curve.max_time == 1.0);
    ufbxt_assert!(anim_curve.keyframes.len() == 2);

    let refs = [
        (16, 8.830700),
        (-5, 10.635811),
        (-26, 11.633479),
        (-43, -1.753241),
        (-44, -1.077311),
        (-45, 0.000000),
        (-46, 0.000000),
        (-100, 0.000000),
        (-10000, 0.000000),
        (24, 10.000000),
        (25, 10.000000),
        (100, 10.000000),
        (10000, 10.000000),
    ];

    check_extrapolation(err, scene, anim_curve, &refs);
});

file_test!(motionbuilder_extrapolation_relative_count, flags = ALLOW_INVALID_UNICODE, |scene, err, load_error| {
    let anim_curve = extrapolation_curve(scene);

    ufbxt_assert!(anim_curve.post_extrapolation.mode == ExtrapolationMode::RepeatRelative);
    ufbxt_assert!(anim_curve.pre_extrapolation.repeat_count == 2);
    ufbxt_assert!(anim_curve.post_extrapolation.mode == ExtrapolationMode::RepeatRelative);
    ufbxt_assert!(anim_curve.post_extrapolation.repeat_count == 1);
    ufbxt_assert!(anim_curve.min_time == 1.0 / 24.0);
    ufbxt_assert!(anim_curve.max_time == 1.0);
    ufbxt_assert!(anim_curve.keyframes.len() == 2);

    let refs = [
        (12, 4.004632),
        (-8, -2.280390),
        (-24, -8.477917),
        (-41, -22.051765),
        (-44, -21.077311),
        (-45, -20.000000),
        (-46, -20.000000),
        (-50, -20.000000),
        (-100, -20.000000),
        (-10000, -20.000000),
        (25, 8.922688),
        (30, 8.802806),
        (43, 21.633480),
        (46, 20.956230),
        (47, 20.000000),
        (50, 20.000000),
        (100, 20.000000),
        (10000, 20.000000),
    ];

    check_extrapolation(err, scene, anim_curve, &refs);
});

file_test!(synthetic_unknown_extrapolation, |scene, err, load_error| {
    let node = find_node(scene, "pCube1");
    ufbxt_assert!(node.is_some());
    let node = &scene.nodes[node.unwrap()];

    ufbxt_assert!(scene.anim_layers.len() == 1);
    let anim_layer = &scene.anim_layers[0];

    let anim_prop = find_anim_prop(anim_layer, node.element.element_id, "Lcl Translation");
    ufbxt_assert!(anim_prop.is_some());

    let anim_curve = scene.anim_values[anim_prop.unwrap().anim_value].curves[0];
    ufbxt_assert!(anim_curve.is_some());
    let anim_curve = &scene.anim_curves[anim_curve.unwrap()];

    ufbxt_assert!(anim_curve.pre_extrapolation.mode == ExtrapolationMode::Constant);
    ufbxt_assert!(anim_curve.pre_extrapolation.repeat_count == -1);
    ufbxt_assert!(anim_curve.pre_extrapolation.mode == ExtrapolationMode::Constant);
    ufbxt_assert!(anim_curve.post_extrapolation.repeat_count == -1);
});

file_test!(bake_constant_rotation, file = "maya_anim_interpolation", |scene, err, load_error| {
    let scene_node = find_node(scene, "pCube1");
    ufbxt_assert!(scene_node.is_some());
    let scene_node = &scene.nodes[scene_node.unwrap()];

    let bake = load3d::bake_anim(scene, &scene.anim, &BakeOpts::default());
    if let Err(e) = &bake {
        log_error(e);
    }
    ufbxt_assert!(bake.is_ok());
    let bake = bake.unwrap();

    ufbxt_assert!(bake.nodes.len() == 1);

    let node = &bake.nodes[0];
    ufbxt_assert!(node.typed_id == scene_node.element.typed_id);
    ufbxt_assert!(!node.constant_translation);
    ufbxt_assert!(node.constant_rotation);
    ufbxt_assert!(node.constant_scale);
});
