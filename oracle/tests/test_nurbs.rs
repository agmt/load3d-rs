mod ufbxt;
use ufbxt::*;

/// u, position, derivative
type CurveSample = (Real, Vec3, Vec3);

/// u, v, position, derivative_u, derivative_v
type SurfaceSample = (Real, Real, Vec3, Vec3, Vec3);

fn check_curve_samples(err: &mut DiffError, curve: &NurbsCurve, samples: &[CurveSample]) {
    for (i, sample) in samples.iter().enumerate() {
        ufbxt_hintf!("i: {i}");
        let p = evaluate_nurbs_curve(curve, sample.0);
        assert_close_vec3(err, p.position, sample.1);
        assert_close_vec3(err, p.derivative, sample.2);
    }
}

fn nurbs_curve<'a>(scene: &'a Scene, node: &Node) -> &'a NurbsCurve {
    scene.as_nurbs_curve(node.attrib.unwrap()).unwrap()
}

fn nurbs_surface<'a>(scene: &'a Scene, node: &Node) -> &'a NurbsSurface {
    scene.as_nurbs_surface(node.attrib.unwrap()).unwrap()
}

file_test!(maya_nurbs_curve_form, |scene, err, load_error| {
    let node_open = find_node(scene, "circleOpen").map(|n| &scene.nodes[n]);
    let node_closed = find_node(scene, "circleClosed").map(|n| &scene.nodes[n]);
    let node_periodic = find_node(scene, "circlePeriodic").map(|n| &scene.nodes[n]);

    ufbxt_assert!(node_open.is_some_and(|n| n.attrib_type == ElementType::NurbsCurve));
    ufbxt_assert!(node_closed.is_some_and(|n| n.attrib_type == ElementType::NurbsCurve));
    ufbxt_assert!(node_periodic.is_some_and(|n| n.attrib_type == ElementType::NurbsCurve));

    let open = nurbs_curve(scene, node_open.unwrap());
    let closed = nurbs_curve(scene, node_closed.unwrap());
    let periodic = nurbs_curve(scene, node_periodic.unwrap());

    ufbxt_assert!(open.basis.valid);
    ufbxt_assert!(closed.basis.valid);
    ufbxt_assert!(periodic.basis.valid);

    ufbxt_assert!(open.basis.topology == NurbsTopology::Open);
    ufbxt_assert!(closed.basis.topology == NurbsTopology::Closed);
    ufbxt_assert!(periodic.basis.topology == NurbsTopology::Periodic);

    ufbxt_assert!(open.basis.order == 4);
    ufbxt_assert!(closed.basis.order == 4);
    ufbxt_assert!(periodic.basis.order == 4);

    ufbxt_assert!(!open.basis.is_2d);
    ufbxt_assert!(!closed.basis.is_2d);
    ufbxt_assert!(!periodic.basis.is_2d);

    {
        assert_close_real(err, open.basis.t_min, 0.0);
        assert_close_real(err, open.basis.t_max, 1.0);
        ufbxt_assert!(open.basis.knot_vector.len() == 8);
        ufbxt_assert!(open.basis.spans.len() == 2);
        ufbxt_assert!(open.control_points.len() == 4);

        {
            let mut weights = [0.0; 16];
            let knot = evaluate_nurbs_basis(&open.basis, 0.0, &mut weights, &mut []);
            ufbxt_assert!(knot == 0);
            assert_close_real(err, weights[0], 1.0);
            assert_close_real(err, weights[1], 0.0);
            assert_close_real(err, weights[2], 0.0);
            assert_close_real(err, weights[3], 0.0);
        }

        {
            let mut weights = [0.0; 16];
            let knot = evaluate_nurbs_basis(&open.basis, 0.5, &mut weights, &mut []);
            ufbxt_assert!(knot == 0);
            assert_close_real(err, weights[0], 0.125);
            assert_close_real(err, weights[1], 0.375);
            assert_close_real(err, weights[2], 0.375);
            assert_close_real(err, weights[3], 0.125);
        }

        {
            let mut weights = [0.0; 16];
            let knot = evaluate_nurbs_basis(&open.basis, 1.0, &mut weights, &mut []);
            ufbxt_assert!(knot == 0);
            assert_close_real(err, weights[0], 0.0);
            assert_close_real(err, weights[1], 0.0);
            assert_close_real(err, weights[2], 0.0);
            assert_close_real(err, weights[3], 1.0);
        }

        {
            let samples: [CurveSample; 11] = [
                (0.00, Vec3::new(0.000000, 0.0, -1.000000), Vec3::new(-1.500000, 0.0, 0.000000)),
                (0.10, Vec3::new(-0.149500, 0.0, -0.985500), Vec3::new(-1.485000, 0.0, 0.285000)),
                (0.20, Vec3::new(-0.296000, 0.0, -0.944000), Vec3::new(-1.440000, 0.0, 0.540000)),
                (0.30, Vec3::new(-0.436500, 0.0, -0.878500), Vec3::new(-1.365000, 0.0, 0.765000)),
                (0.40, Vec3::new(-0.568000, 0.0, -0.792000), Vec3::new(-1.260000, 0.0, 0.960000)),
                (0.50, Vec3::new(-0.687500, 0.0, -0.687500), Vec3::new(-1.125000, 0.0, 1.125000)),
                (0.60, Vec3::new(-0.792000, 0.0, -0.568000), Vec3::new(-0.960000, 0.0, 1.260000)),
                (0.70, Vec3::new(-0.878500, 0.0, -0.436500), Vec3::new(-0.765000, 0.0, 1.365000)),
                (0.80, Vec3::new(-0.944000, 0.0, -0.296000), Vec3::new(-0.540000, 0.0, 1.440000)),
                (0.90, Vec3::new(-0.985500, 0.0, -0.149500), Vec3::new(-0.285000, 0.0, 1.485000)),
                (1.00, Vec3::new(-1.000000, 0.0, -0.000000), Vec3::new(0.000000, 0.0, 1.500000)),
            ];
            check_curve_samples(err, open, &samples);
        }
    }

    {
        assert_close_real(err, closed.basis.t_min, 1.0);
        assert_close_real(err, closed.basis.t_max, 5.0);
        ufbxt_assert!(closed.basis.knot_vector.len() == 11);
        ufbxt_assert!(closed.basis.spans.len() == 5);
        ufbxt_assert!(closed.control_points.len() == 6);

        {
            let mut weights = [0.0; 16];
            let knot = evaluate_nurbs_basis(&closed.basis, 3.14, &mut weights, &mut []);
            ufbxt_assert!(knot == 2);
            assert_close_real(err, weights[0], 0.106009);
            assert_close_real(err, weights[1], 0.648438);
            assert_close_real(err, weights[2], 0.244866);
            assert_close_real(err, weights[3], 0.000686);
        }

        {
            let samples: [CurveSample; 14] = [
                (1.00, Vec3::new(-1.000000, 0.0, 0.000000), Vec3::new(0.000000, 0.0, 1.500000)),
                (1.30, Vec3::new(-0.878500, 0.0, 0.436500), Vec3::new(0.765000, 0.0, 1.365000)),
                (1.60, Vec3::new(-0.568000, 0.0, 0.792000), Vec3::new(1.260000, 0.0, 0.960000)),
                (1.90, Vec3::new(-0.149500, 0.0, 0.985500), Vec3::new(1.485000, 0.0, 0.285000)),
                (2.20, Vec3::new(0.296000, 0.0, 0.944000), Vec3::new(1.440000, 0.0, -0.540000)),
                (2.50, Vec3::new(0.687500, 0.0, 0.687500), Vec3::new(1.125000, 0.0, -1.125000)),
                (2.80, Vec3::new(0.944000, 0.0, 0.296000), Vec3::new(0.540000, 0.0, -1.440000)),
                (3.10, Vec3::new(0.985500, 0.0, -0.149500), Vec3::new(-0.285000, 0.0, -1.485000)),
                (3.40, Vec3::new(0.792000, 0.0, -0.568000), Vec3::new(-0.960000, 0.0, -1.260000)),
                (3.70, Vec3::new(0.436500, 0.0, -0.878500), Vec3::new(-1.365000, 0.0, -0.765000)),
                (4.00, Vec3::new(0.000000, 0.0, -1.000000), Vec3::new(-1.500000, 0.0, -0.000000)),
                (4.30, Vec3::new(-0.436500, 0.0, -0.878500), Vec3::new(-1.365000, 0.0, 0.765000)),
                (4.60, Vec3::new(-0.792000, 0.0, -0.568000), Vec3::new(-0.960000, 0.0, 1.260000)),
                (4.90, Vec3::new(-0.985500, 0.0, -0.149500), Vec3::new(-0.285000, 0.0, 1.485000)),
            ];
            check_curve_samples(err, closed, &samples);
        }
    }

    {
        assert_close_real(err, periodic.basis.t_min, 0.0);
        assert_close_real(err, periodic.basis.t_max, 4.0);
        ufbxt_assert!(periodic.basis.knot_vector.len() == 11);
        ufbxt_assert!(periodic.basis.spans.len() == 5);
        ufbxt_assert!(periodic.control_points.len() == 4);

        {
            let samples: [CurveSample; 21] = [
                (0.00, Vec3::new(0.000000, 0.0, -1.000000), Vec3::new(-1.500000, 0.0, 0.000000)),
                (0.20, Vec3::new(-0.296000, 0.0, -0.944000), Vec3::new(-1.440000, 0.0, 0.540000)),
                (0.40, Vec3::new(-0.568000, 0.0, -0.792000), Vec3::new(-1.260000, 0.0, 0.960000)),
                (0.60, Vec3::new(-0.792000, 0.0, -0.568000), Vec3::new(-0.960000, 0.0, 1.260000)),
                (0.80, Vec3::new(-0.944000, 0.0, -0.296000), Vec3::new(-0.540000, 0.0, 1.440000)),
                (1.00, Vec3::new(-1.000000, 0.0, 0.000000), Vec3::new(0.000000, 0.0, 1.500000)),
                (1.20, Vec3::new(-0.944000, 0.0, 0.296000), Vec3::new(0.540000, 0.0, 1.440000)),
                (1.40, Vec3::new(-0.792000, 0.0, 0.568000), Vec3::new(0.960000, 0.0, 1.260000)),
                (1.60, Vec3::new(-0.568000, 0.0, 0.792000), Vec3::new(1.260000, 0.0, 0.960000)),
                (1.80, Vec3::new(-0.296000, 0.0, 0.944000), Vec3::new(1.440000, 0.0, 0.540000)),
                (2.00, Vec3::new(-0.000000, 0.0, 1.000000), Vec3::new(1.500000, 0.0, 0.000000)),
                (2.20, Vec3::new(0.296000, 0.0, 0.944000), Vec3::new(1.440000, 0.0, -0.540000)),
                (2.40, Vec3::new(0.568000, 0.0, 0.792000), Vec3::new(1.260000, 0.0, -0.960000)),
                (2.60, Vec3::new(0.792000, 0.0, 0.568000), Vec3::new(0.960000, 0.0, -1.260000)),
                (2.80, Vec3::new(0.944000, 0.0, 0.296000), Vec3::new(0.540000, 0.0, -1.440000)),
                (3.00, Vec3::new(1.000000, 0.0, -0.000000), Vec3::new(-0.000000, 0.0, -1.500000)),
                (3.20, Vec3::new(0.944000, 0.0, -0.296000), Vec3::new(-0.540000, 0.0, -1.440000)),
                (3.40, Vec3::new(0.792000, 0.0, -0.568000), Vec3::new(-0.960000, 0.0, -1.260000)),
                (3.60, Vec3::new(0.568000, 0.0, -0.792000), Vec3::new(-1.260000, 0.0, -0.960000)),
                (3.80, Vec3::new(0.296000, 0.0, -0.944000), Vec3::new(-1.440000, 0.0, -0.540000)),
                (4.00, Vec3::new(-0.000000, 0.0, -1.000000), Vec3::new(-1.500000, 0.0, 0.000000)),
            ];
            check_curve_samples(err, periodic, &samples);
        }
    }
});

file_test!(max_nurbs_curve_rational, |scene, err, load_error| {
    let node = find_node(scene, "Curve001").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::NurbsCurve));
    let curve = nurbs_curve(scene, node.unwrap());

    let samples: [CurveSample; 31] = [
        (0.000000, Vec3::new(0.000000, -40.000000, 0.000000), Vec3::new(-24.322954, 0.000000, 6.080737)),
        (0.974593, Vec3::new(-17.005555, -37.570554, 4.572092), Vec3::new(-12.337594, 4.358807, 3.678146)),
        (1.949186, Vec3::new(-26.036811, -32.223433, 7.596081), Vec3::new(-6.784099, 6.393973, 2.654863)),
        (2.923779, Vec3::new(-30.970646, -25.399582, 9.909292), Vec3::new(-3.585288, 7.504839, 2.146619)),
        (3.898372, Vec3::new(-33.353768, -17.752430, 11.854892), Vec3::new(-1.420225, 8.123470, 1.872876)),
        (4.872965, Vec3::new(-33.899033, -9.674799, 13.598350), Vec3::new(0.243486, 8.402346, 1.719326)),
        (5.847558, Vec3::new(-32.960866, -1.468619, 15.227145), Vec3::new(1.651919, 8.390724, 1.631066)),
        (6.822152, Vec3::new(-30.721680, 6.587631, 16.788850), Vec3::new(2.927439, 8.093263, 1.577779)),
        (7.796745, Vec3::new(-27.278008, 14.208835, 18.307632), Vec3::new(4.130179, 7.494472, 1.540372)),
        (8.771338, Vec3::new(-22.686868, 21.090577, 19.792049), Vec3::new(5.283867, 6.572031, 1.505301)),
        (9.745931, Vec3::new(-16.994772, 26.907367, 21.239065), Vec3::new(6.387689, 5.306512, 1.462049)),
        (10.720524, Vec3::new(-10.270443, 31.351748, 22.650823), Vec3::new(7.377118, 3.800029, 1.451715)),
        (11.695117, Vec3::new(-2.706724, 34.283196, 24.096090), Vec3::new(8.085573, 2.198608, 1.524685)),
        (12.669710, Vec3::new(5.351791, 35.610305, 25.636226), Vec3::new(8.370286, 0.517035, 1.639198)),
        (13.644303, Vec3::new(13.435696, 35.294565, 27.290675), Vec3::new(8.125935, -1.151753, 1.752252)),
        (14.618896, Vec3::new(21.010971, 33.416195, 29.038561), Vec3::new(7.331955, -2.663870, 1.825309)),
        (15.593489, Vec3::new(27.575750, 30.201053, 30.826822), Vec3::new(6.075349, -3.873362, 1.832138)),
        (16.568082, Vec3::new(32.759100, 25.997218, 32.585338), Vec3::new(4.530977, -4.682518, 1.764466)),
        (17.542675, Vec3::new(36.382081, 21.211799, 34.244771), Vec3::new(2.907510, -5.069834, 1.631536)),
        (18.517268, Vec3::new(38.464958, 16.297312, 35.744787), Vec3::new(1.405886, -4.869993, 1.433972)),
        (19.491861, Vec3::new(39.216555, 11.905556, 37.036473), Vec3::new(0.183372, -4.095993, 1.223988)),
        (20.466455, Vec3::new(38.892809, 8.347449, 38.154114), Vec3::new(-0.825634, -3.206871, 1.084857)),
        (21.441048, Vec3::new(37.616454, 5.638129, 39.185141), Vec3::new(-1.808639, -2.363861, 1.050754)),
        (22.415641, Vec3::new(35.306462, 3.716150, 40.244592), Vec3::new(-2.930163, -1.617486, 1.138404)),
        (23.390234, Vec3::new(31.980875, 2.380957, 41.416165), Vec3::new(-3.862865, -1.167548, 1.271465)),
        (24.364827, Vec3::new(27.819473, 1.365943, 42.736393), Vec3::new(-4.662313, -0.950026, 1.446458)),
        (25.339420, Vec3::new(22.913277, 0.459596, 44.257763), Vec3::new(-5.398160, -0.950224, 1.690832)),
        (26.314013, Vec3::new(17.312988, -0.588856, 46.074971), Vec3::new(-6.083534, -1.274792, 2.070482)),
        (27.288606, Vec3::new(11.096546, -2.257805, 48.396616), Vec3::new(-6.632300, -2.349350, 2.781984)),
        (28.263199, Vec3::new(4.579470, -5.973973, 51.849971), Vec3::new(-6.516068, -6.140789, 4.679450)),
        (29.237792, Vec3::new(-0.000000, -20.000002, 60.000000), Vec3::new(-0.000000, -32.801115, 16.400558)),
    ];
    check_curve_samples(err, curve, &samples);
});

file_test!(maya_nurbs_curve_multiplicity, |scene, err, load_error| {
    let node = find_node(scene, "curve1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::NurbsCurve));
    let curve = nurbs_curve(scene, node.unwrap());

    ufbxt_assert!(curve.basis.order == 6);

    assert_close_real(err, curve.basis.t_min, 0.0);
    assert_close_real(err, curve.basis.t_max, 43.257);

    for i in 1..curve.basis.spans.len() {
        ufbxt_assert!(curve.basis.spans[i - 1] < curve.basis.spans[i]);
    }

    let samples: [CurveSample; 41] = [
        (0.000000, Vec3::new(0.908068, 0.000000, -0.405819), Vec3::new(0.253319, 0.000000, -0.445464)),
        (1.081421, Vec3::new(1.119511, 0.000000, -0.868651), Vec3::new(0.205140, 0.000000, -0.285504)),
        (2.162843, Vec3::new(1.387340, 0.000000, -0.963056), Vec3::new(0.256681, 0.000000, 0.053404)),
        (3.244264, Vec3::new(1.633207, 0.000000, -0.849916), Vec3::new(0.200801, 0.000000, 0.139006)),
        (4.325685, Vec3::new(1.835297, 0.000000, -0.683140), Vec3::new(0.178149, 0.000000, 0.161468)),
        (5.407107, Vec3::new(2.026478, 0.000000, -0.516048), Vec3::new(0.177779, 0.000000, 0.141353)),
        (6.488528, Vec3::new(2.222653, 0.000000, -0.388757), Vec3::new(0.185504, 0.000000, 0.089817)),
        (7.569949, Vec3::new(2.427523, 0.000000, -0.328685), Vec3::new(0.192948, 0.000000, 0.019186)),
        (8.651371, Vec3::new(2.638875, 0.000000, -0.349293), Vec3::new(0.197550, 0.000000, -0.057045)),
        (9.732792, Vec3::new(2.854870, 0.000000, -0.448812), Vec3::new(0.202560, 0.000000, -0.124212)),
        (10.814214, Vec3::new(3.080323, 0.000000, -0.608983), Vec3::new(0.217038, 0.000000, -0.166482)),
        (11.895635, Vec3::new(3.332994, 0.000000, -0.793795), Vec3::new(0.255859, 0.000000, -0.166851)),
        (12.977056, Vec3::new(3.649869, 0.000000, -0.948214), Vec3::new(0.339710, 0.000000, -0.107146)),
        (14.058478, Vec3::new(3.985671, 0.000000, 0.194786), Vec3::new(-0.143623, 0.000000, 5.538350)),
        (15.139899, Vec3::new(3.504179, 0.000000, 3.786682), Vec3::new(-0.683192, 0.000000, 1.734052)),
        (16.221320, Vec3::new(2.613011, 0.000000, 4.871228), Vec3::new(-0.929318, 0.000000, 0.531104)),
        (17.302742, Vec3::new(1.550585, 0.000000, 5.237214), Vec3::new(-1.016831, 0.000000, 0.198109)),
        (18.384163, Vec3::new(0.445465, 0.000000, 5.328320), Vec3::new(-1.014626, 0.000000, -0.029385)),
        (19.465584, Vec3::new(-0.620174, 0.000000, 5.174213), Vec3::new(-0.946609, 0.000000, -0.255004)),
        (20.547006, Vec3::new(-1.584181, 0.000000, 4.779433), Vec3::new(-0.829428, 0.000000, -0.473124)),
        (21.628427, Vec3::new(-2.402332, 0.000000, 4.157381), Vec3::new(-0.679606, 0.000000, -0.673164)),
        (22.709848, Vec3::new(-3.048214, 0.000000, 3.335470), Vec3::new(-0.513558, 0.000000, -0.839798)),
        (23.791270, Vec3::new(-3.513108, 0.000000, 2.360250), Vec3::new(-0.347591, 0.000000, -0.952957)),
        (24.872691, Vec3::new(-3.805868, 0.000000, 1.302539), Vec3::new(-0.197901, 0.000000, -0.987827)),
        (25.954113, Vec3::new(-3.952805, 0.000000, 0.262553), Vec3::new(-0.080577, 0.000000, -0.914853)),
        (27.035534, Vec3::new(-3.997572, 0.000000, -0.624962), Vec3::new(-0.011599, 0.000000, -0.699733)),
        (28.116955, Vec3::new(-2.943720, 0.000000, -0.502852), Vec3::new(0.978119, 0.000000, 1.270640)),
        (29.198377, Vec3::new(-2.049079, 0.000000, -0.429651), Vec3::new(0.608714, 0.000000, -0.657963)),
        (30.279798, Vec3::new(-1.426889, 0.000000, -0.907600), Vec3::new(0.591440, 0.000000, -0.061570)),
        (31.361219, Vec3::new(-1.030212, 0.000000, -0.664031), Vec3::new(0.116667, 0.000000, 0.406368)),
        (32.442641, Vec3::new(-0.988133, 0.000000, -0.182528), Vec3::new(0.031241, 0.000000, 0.447589)),
        (33.524062, Vec3::new(-0.910710, 0.000000, 0.269968), Vec3::new(0.117183, 0.000000, 0.384560)),
        (34.605483, Vec3::new(-0.736010, 0.000000, 0.651700), Vec3::new(0.200556, 0.000000, 0.327364)),
        (35.686905, Vec3::new(-0.495401, 0.000000, 0.997855), Vec3::new(0.233755, 0.000000, 0.324195)),
        (36.768326, Vec3::new(-0.254763, 0.000000, 1.378865), Vec3::new(0.200505, 0.000000, 0.392064)),
        (37.849747, Vec3::new(-0.080667, 0.000000, 1.866711), Vec3::new(0.115792, 0.000000, 0.516827)),
        (38.931169, Vec3::new(-0.006578, 0.000000, 2.501231), Vec3::new(0.025863, 0.000000, 0.653179)),
        (40.012590, Vec3::new(1.291208, 0.000000, 3.143478), Vec3::new(1.958604, 0.000000, 0.677384)),
        (41.094012, Vec3::new(0.552930, 0.000000, 3.783813), Vec3::new(-1.959784, 0.000000, 0.211471)),
        (42.175433, Vec3::new(-1.250738, 0.000000, 3.642614), Vec3::new(-1.058494, 0.000000, -0.348306)),
        (43.256854, Vec3::new(-1.669232, 0.000000, 3.251459), Vec3::new(0.154248, 0.000000, -0.305748)),
    ];
    check_curve_samples(err, curve, &samples);
});

file_test!(maya_nurbs_curve_linear, |scene, err, load_error| {
    let node = find_node(scene, "curve1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::NurbsCurve));
    let curve = nurbs_curve(scene, node.unwrap());

    ufbxt_assert!(curve.basis.order == 2);

    let samples: [CurveSample; 10] = [
        (0.000000, Vec3::new(0.000000, 0.000000, -1.000000), Vec3::new(1.000000, 0.000000, 0.000000)),
        (1.400000, Vec3::new(1.000000, 0.000000, -1.400000), Vec3::new(0.000000, 0.000000, -1.000000)),
        (2.800000, Vec3::new(1.800000, 0.000000, -2.000000), Vec3::new(1.000000, 0.000000, 0.000000)),
        (4.200000, Vec3::new(1.800000, 0.000000, 1.000000), Vec3::new(-1.000000, 0.000000, 0.000000)),
        (5.600000, Vec3::new(1.000000, 0.000000, 1.600000), Vec3::new(0.000000, 0.000000, 1.000000)),
        (7.000000, Vec3::new(-1.000000, 0.000000, 2.000000), Vec3::new(0.000000, 0.000000, -1.000000)),
        (8.400000, Vec3::new(-1.400000, 0.000000, 1.000000), Vec3::new(-1.000000, 0.000000, 0.000000)),
        (9.800000, Vec3::new(-2.000000, 0.000000, -1.400000), Vec3::new(0.000000, 0.000000, -3.000000)),
        (11.200000, Vec3::new(-1.000000, 0.000000, -1.800000), Vec3::new(0.000000, 0.000000, 1.000000)),
        (12.600000, Vec3::new(-0.400000, 0.000000, -1.000000), Vec3::new(1.000000, 0.000000, 0.000000)),
    ];
    check_curve_samples(err, curve, &samples);
});

file_test!(maya_nurbs_surface_plane, |scene, err, load_error| {
    let node = find_node(scene, "nurbsPlane1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::NurbsSurface));
    let surface = nurbs_surface(scene, node.unwrap());

    ufbxt_assert!(surface.span_subdivision_u == 4);
    ufbxt_assert!(surface.span_subdivision_v == 4);

    ufbxt_assert!(surface.material.is_some());
    ufbxt_assert!(scene.materials[surface.material.unwrap()].element.name == "lambert1");

    ufbxt_assert!(surface.num_control_points_u == 5);
    ufbxt_assert!(surface.num_control_points_v == 6);
    ufbxt_assert!(!surface.flip_normals);

    let samples: [SurfaceSample; 36] = [
        (0.000000, 0.000000, Vec3::new(-0.500000, -0.550589, 0.500000), Vec3::new(1.000000, 0.000000, 0.000000), Vec3::new(0.000000, 4.955298, -1.000000)),
        (0.000000, 0.200000, Vec3::new(-0.500000, 0.113108, 0.300000), Vec3::new(1.000000, 0.000000, 0.000000), Vec3::new(0.000000, 1.958420, -1.000000)),
        (0.000000, 0.400000, Vec3::new(-0.500000, 0.342215, 0.100000), Vec3::new(1.000000, 0.000000, 0.000000), Vec3::new(0.000000, 0.565126, -1.000000)),
        (0.000000, 0.600000, Vec3::new(-0.500000, 0.391664, -0.100000), Vec3::new(1.000000, 0.000000, 0.000000), Vec3::new(0.000000, 0.035320, -1.000000)),
        (0.000000, 0.800000, Vec3::new(-0.500000, 0.392449, -0.300000), Vec3::new(1.000000, 0.000000, 0.000000), Vec3::new(0.000000, -0.000000, -1.000000)),
        (0.000000, 1.000000, Vec3::new(-0.500000, 0.392449, -0.500000), Vec3::new(1.000000, 0.000000, 0.000000), Vec3::new(0.000000, 0.000000, -1.000000)),
        (0.200000, 0.000000, Vec3::new(-0.300000, -0.550589, 0.500000), Vec3::new(1.000000, 0.000000, 0.000000), Vec3::new(0.000000, 4.955298, -1.000000)),
        (0.200000, 0.200000, Vec3::new(-0.300000, 0.084626, 0.300000), Vec3::new(1.000000, -0.249221, -0.000000), Vec3::new(-0.000000, 1.734630, -1.000000)),
        (0.200000, 0.400000, Vec3::new(-0.300007, 0.276510, 0.100005), Vec3::new(0.999902, -0.574922, 0.000072), Vec3::new(-0.000294, 0.456622, -0.999783)),
        (0.200000, 0.600000, Vec3::new(-0.300418, 0.316464, -0.099691), Vec3::new(0.993731, -0.657995, 0.004628), Vec3::new(-0.004701, 0.028539, -0.996529)),
        (0.200000, 0.800000, Vec3::new(-0.302031, 0.317098, -0.298501), Vec3::new(0.969539, -0.659314, 0.022489), Vec3::new(-0.009697, -0.000000, -0.992841)),
        (0.200000, 1.000000, Vec3::new(-0.303265, 0.317098, -0.497590), Vec3::new(0.951027, -0.659314, 0.036156), Vec3::new(0.000000, 0.000000, -1.000000)),
        (0.400000, 0.000000, Vec3::new(-0.100000, -0.550589, 0.500000), Vec3::new(1.000000, 0.000000, 0.000000), Vec3::new(0.000000, 4.955298, -1.000000)),
        (0.400000, 0.200000, Vec3::new(-0.100000, 0.027661, 0.300000), Vec3::new(1.000000, -0.284823, -0.000000), Vec3::new(-0.000000, 1.287050, -1.000000)),
        (0.400000, 0.400000, Vec3::new(-0.100052, 0.145099, 0.100039), Vec3::new(0.999608, -0.657053, 0.000289), Vec3::new(-0.002351, 0.239613, -0.998265)),
        (0.400000, 0.600000, Vec3::new(-0.103343, 0.166065, -0.097532), Vec3::new(0.974926, -0.751994, 0.018512), Vec3::new(-0.037611, 0.014976, -0.972232)),
        (0.400000, 0.800000, Vec3::new(-0.116246, 0.166398, -0.288006), Vec3::new(0.878155, -0.753501, 0.089956), Vec3::new(-0.077574, 0.000000, -0.942729)),
        (0.400000, 1.000000, Vec3::new(-0.126119, 0.166398, -0.480717), Vec3::new(0.804107, -0.753501, 0.144624), Vec3::new(0.000000, 0.000000, -1.000000)),
        (0.600000, 0.000000, Vec3::new(0.100000, -0.550589, 0.500000), Vec3::new(1.000000, 0.000000, 0.000000), Vec3::new(0.000000, 4.955298, -1.000000)),
        (0.600000, 0.200000, Vec3::new(0.100000, -0.016249, 0.300000), Vec3::new(1.000000, -0.142412, -0.000000), Vec3::new(0.000000, 0.942041, -1.000000)),
        (0.600000, 0.400000, Vec3::new(0.099827, 0.043804, 0.100128), Vec3::new(0.999216, -0.328527, 0.000578), Vec3::new(-0.007787, 0.072336, -0.994251)),
        (0.600000, 0.600000, Vec3::new(0.088926, 0.050133, -0.091824), Vec3::new(0.949851, -0.375997, 0.037024), Vec3::new(-0.124588, 0.004521, -0.908019)),
        (0.600000, 0.800000, Vec3::new(0.046185, 0.050233, -0.260269), Vec3::new(0.756309, -0.376751, 0.179913), Vec3::new(-0.256963, 0.000000, -0.810289)),
        (0.600000, 1.000000, Vec3::new(0.013481, 0.050233, -0.436124), Vec3::new(0.608214, -0.376751, 0.289249), Vec3::new(0.000000, 0.000000, -1.000000)),
        (0.800000, 0.000000, Vec3::new(0.300000, -0.550589, 0.500000), Vec3::new(1.000000, -0.000000, 0.000000), Vec3::new(0.000000, 4.955298, -1.000000)),
        (0.800000, 0.200000, Vec3::new(0.300000, -0.032864, 0.300000), Vec3::new(1.000000, -0.035603, 0.000000), Vec3::new(0.000000, 0.811497, -1.000000)),
        (0.800000, 0.400000, Vec3::new(0.299670, 0.005475, 0.100243), Vec3::new(0.999314, -0.082132, 0.000506), Vec3::new(-0.014839, 0.009042, -0.989045)),
        (0.800000, 0.600000, Vec3::new(0.278896, 0.006267, -0.084419), Vec3::new(0.956120, -0.093999, 0.032396), Vec3::new(-0.237422, 0.000565, -0.824715)),
        (0.800000, 0.800000, Vec3::new(0.197447, 0.006279, -0.224287), Vec3::new(0.786770, -0.094188, 0.157424), Vec3::new(-0.489684, 0.000000, -0.638475)),
        (0.800000, 1.000000, Vec3::new(0.135123, 0.006279, -0.378275), Vec3::new(0.657187, -0.094188, 0.253093), Vec3::new(0.000000, 0.000000, -1.000000)),
        (1.000000, 0.000000, Vec3::new(0.500000, -0.550589, 0.500000), Vec3::new(1.000000, 0.000000, 0.000000), Vec3::new(0.000000, 4.955298, -1.000000)),
        (1.000000, 0.200000, Vec3::new(0.500000, -0.035238, 0.300000), Vec3::new(1.000000, -0.000000, 0.000000), Vec3::new(0.000000, 0.792848, -1.000000)),
        (1.000000, 0.400000, Vec3::new(0.499592, -0.000000, 0.100301), Vec3::new(1.000000, -0.000000, 0.000000), Vec3::new(-0.018365, 0.000000, -0.986441)),
        (1.000000, 0.600000, Vec3::new(0.473881, 0.000000, -0.080717), Vec3::new(1.000000, -0.000000, 0.000000), Vec3::new(-0.293840, 0.000000, -0.783063)),
        (1.000000, 0.800000, Vec3::new(0.373078, 0.000000, -0.206295), Vec3::new(1.000000, 0.000000, 0.000000), Vec3::new(-0.606044, 0.000000, -0.552568)),
        (1.000000, 1.000000, Vec3::new(0.295945, 0.000000, -0.349350), Vec3::new(1.000000, 0.000000, 0.000000), Vec3::new(0.000000, 0.000000, -1.000000)),
    ];

    for (i, sample) in samples.iter().enumerate() {
        ufbxt_hintf!("i: {i}");
        let p = evaluate_nurbs_surface(surface, sample.0, sample.1);
        assert_close_vec3(err, p.position, sample.2);
        assert_close_vec3(err, p.derivative_u, sample.3);
        assert_close_vec3(err, p.derivative_v, sample.4);
    }
});

file_test!(maya_nurbs_surface_sphere, |scene, err, load_error| {
    let node = find_node(scene, "nurbsSphere1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::NurbsSurface));
    let surface = nurbs_surface(scene, node.unwrap());

    ufbxt_assert!(surface.material.is_some());
    ufbxt_assert!(scene.materials[surface.material.unwrap()].element.name == "lambert1");

    assert_close_real(err, surface.basis_u.t_min, 0.0);
    assert_close_real(err, surface.basis_u.t_max, 8.0);
    assert_close_real(err, surface.basis_v.t_min, 0.0);
    assert_close_real(err, surface.basis_v.t_max, 10.0);

    // ufbx steps by the float constants, which as doubles are not 0.1, 0.05 and 7.9
    let mut u = 0.1f32 as Real;
    while u <= 7.9f32 as Real {
        let mut v = 0.0;
        while v <= 10.0 {
            let point = evaluate_nurbs_surface(surface, u, v);
            let normal = normalize(cross3(point.derivative_u, point.derivative_v));
            assert_close_vec3(err, mul3(point.position, 0.1), mul3(normal, 0.1));
            v += 0.05f32 as Real;
        }
        u += 0.05f32 as Real;
    }
});

file_test!(maya_nurbs_low_sphere, |scene, err, load_error| {});

file_test!(nurbs_tessellate_mesh_parts, file = "maya_nurbs_surface_plane_7500_binary", |scene, err, load_error| {
    let node = find_node(scene, "nurbsPlane1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::NurbsSurface));
    let surface = nurbs_surface(scene, node.unwrap());

    for skip_parts in 0..=1 {
        let opts = TessellateSurfaceOpts { skip_mesh_parts: skip_parts != 0, ..Default::default() };

        let tess_mesh = tessellate_nurbs_surface(surface, &opts);
        ufbxt_assert!(tess_mesh.is_ok());
        let tess_mesh = tess_mesh.unwrap();

        ufbxt_assert!(tess_mesh.materials.len() == 1);
        ufbxt_assert!(scene.materials[tess_mesh.materials[0].unwrap()].element.name == "lambert1");

        ufbxt_assert!(tess_mesh.material_parts.len() == if skip_parts != 0 { 0 } else { 1 });
    }
});

file_test!(synthetic_nurbs_surface_no_material, |scene, err, load_error| {
    let node = find_node(scene, "nurbsPlane1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::NurbsSurface));
    let surface = nurbs_surface(scene, node.unwrap());

    for skip_parts in 0..=1 {
        let opts = TessellateSurfaceOpts { skip_mesh_parts: skip_parts != 0, ..Default::default() };

        let tess_mesh = tessellate_nurbs_surface(surface, &opts);
        ufbxt_assert!(tess_mesh.is_ok());
        let tess_mesh = tess_mesh.unwrap();

        ufbxt_assert!(tess_mesh.materials.is_empty());

        ufbxt_assert!(tess_mesh.material_parts.len() == if skip_parts != 0 { 0 } else { 1 });
    }
});

file_test!(nurbs_alloc_fail, file = "maya_nurbs_surface_plane", |scene, err, load_error| {
    let node = find_node(scene, "nurbsPlane1").map(|n| &scene.nodes[n]);
    ufbxt_assert!(node.is_some_and(|n| n.attrib_type == ElementType::NurbsSurface));
    let _surface = nurbs_surface(scene, node.unwrap());
});

file_test!(synthetic_nurbs_invalid, |scene, err, load_error| {
    {
        let node = find_node(scene, "curve1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let curve = nurbs_curve(scene, node.unwrap());
        ufbxt_assert!(!curve.basis.valid);

        let line = tessellate_nurbs_curve(curve, &TessellateCurveOpts::default());
        ufbxt_assert!(line.is_err());
        ufbxt_assert!(line.unwrap_err().kind == ErrorType::BadNurbs);
    }

    {
        let node = find_node(scene, "nurbsPlane1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let surface = nurbs_surface(scene, node.unwrap());
        ufbxt_assert!(!surface.basis_u.valid);
        ufbxt_assert!(!surface.basis_v.valid);

        let mesh = tessellate_nurbs_surface(surface, &TessellateSurfaceOpts::default());
        ufbxt_assert!(mesh.is_err());
        ufbxt_assert!(mesh.unwrap_err().kind == ErrorType::BadNurbs);
    }
});

file_test!(synthetic_nurbs_truncated, |scene, err, load_error| {
    {
        let node = find_node(scene, "curve1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let curve = nurbs_curve(scene, node.unwrap());
        ufbxt_assert!(curve.basis.valid);

        let line = tessellate_nurbs_curve(curve, &TessellateCurveOpts::default());
        ufbxt_assert!(line.is_ok());
    }

    {
        let node = find_node(scene, "nurbsPlane1").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let surface = nurbs_surface(scene, node.unwrap());
        ufbxt_assert!(surface.basis_u.valid);
        ufbxt_assert!(surface.basis_v.valid);

        let mesh = tessellate_nurbs_surface(surface, &TessellateSurfaceOpts::default());
        ufbxt_assert!(mesh.is_ok());
    }
});

file_test!(max_nurbs_to_line, |scene, err, load_error| {
    let line_node = find_node(scene, "Line").map(|n| &scene.nodes[n]);
    let nurbs_node = find_node(scene, "Nurbs").map(|n| &scene.nodes[n]);
    ufbxt_assert!(line_node.is_some());
    ufbxt_assert!(nurbs_node.is_some());
    let line = line_node.unwrap().attrib.and_then(|a| scene.as_line_curve(a));
    let nurbs = nurbs_node.unwrap().attrib.and_then(|a| scene.as_nurbs_curve(a));
    ufbxt_assert!(line.is_some());
    ufbxt_assert!(nurbs.is_some());
    let (line, nurbs) = (line.unwrap(), nurbs.unwrap());

    let opts = TessellateCurveOpts { span_subdivision: 5 };
    let tess_line = tessellate_nurbs_curve(nurbs, &opts);
    ufbxt_assert!(tess_line.is_ok());
    let tess_line = tess_line.unwrap();

    let num_indices = line.point_indices.len();
    ufbxt_assert!(line.segments.len() == 1);
    ufbxt_assert!(tess_line.segments.len() == 1);
    ufbxt_assert!(tess_line.point_indices.len() == num_indices);

    ufbxt_assert!(line.segments[0].index_begin == 0);
    ufbxt_assert!(tess_line.segments[0].index_begin == 0);
    ufbxt_assert!(line.segments[0].num_indices as usize == num_indices);
    ufbxt_assert!(tess_line.segments[0].num_indices as usize == num_indices);

    for i in 0..num_indices {
        let point = line.control_points[line.point_indices[i] as usize];
        let tess_point = tess_line.control_points[tess_line.point_indices[i] as usize];
        assert_close_vec3(err, point, tess_point);
    }
});

file_test!(tessellate_line_alloc_fail, file = "max_nurbs_to_line", |scene, err, load_error| {
    let node = find_node(scene, "Nurbs").map(|n| &scene.nodes[n]);
    let nurbs = node.unwrap().attrib.and_then(|a| scene.as_nurbs_curve(a));
    ufbxt_assert!(nurbs.is_some());
});

file_test_allow_error!(synthetic_nurbs_dimension_overflow_fail, |scene, err, load_error| {
    ufbxt_assert!(scene.is_none());
});

file_test_allow_error!(synthetic_nurbs_order_overflow_fail, |scene, err, load_error| {
    ufbxt_assert!(scene.is_none());
});
