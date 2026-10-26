mod ufbxt;
use ufbxt::*;

file_test!(max_curve_line, |scene, err, load_error| {
    let polyline_node = find_node(scene, "Polyline").map(|n| &scene.nodes[n]);
    let bezier_node = find_node(scene, "Bezier").map(|n| &scene.nodes[n]);
    ufbxt_assert!(polyline_node.is_some());
    ufbxt_assert!(bezier_node.is_some());
    let (polyline_node, bezier_node) = (polyline_node.unwrap(), bezier_node.unwrap());
    ufbxt_assert!(polyline_node.attrib_type == ElementType::LineCurve);
    ufbxt_assert!(bezier_node.attrib_type == ElementType::LineCurve);
    ufbxt_assert!(polyline_node.attrib.is_some());
    ufbxt_assert!(bezier_node.attrib.is_some());

    let ref_points = [
        Vec3::new(-1.0, 0.0, -1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(0.0, -1.0, 2.0),
        Vec3::new(-2.0, 0.0, 2.0),
        Vec3::new(-1.0, 0.0, 1.0),
        Vec3::new(-1.0, 0.0, -1.0),
    ];

    let polyline = scene.as_line_curve(polyline_node.attrib.unwrap()).unwrap();
    ufbxt_assert!(polyline.point_indices.len() == 7);
    ufbxt_assert!(polyline.segments.len() == 1);
    ufbxt_assert!(polyline.segments[0].index_begin == 0);
    ufbxt_assert!(polyline.segments[0].num_indices == 7);
    for i in 0..polyline.point_indices.len() {
        let mut p = polyline.control_points[polyline.point_indices[i] as usize];
        p = transform_position(&polyline_node.geometry_to_world, p);
        p.x *= 0.1;
        p.y *= 0.1;
        p.z *= 0.1;
        assert_close_vec3(err, p, ref_points[i]);
    }

    let polyline_color = Vec3::new(0.223529411764706, 0.0313725490196078, 0.533333333333333);
    assert_close_vec3(err, polyline.color, polyline_color);

    let bezier = scene.as_line_curve(bezier_node.attrib.unwrap()).unwrap();
    ufbxt_assert!(bezier.point_indices.len() == 15);
    ufbxt_assert!(bezier.segments.len() == 1);
    ufbxt_assert!(bezier.segments[0].index_begin == 0);
    ufbxt_assert!(bezier.segments[0].num_indices == 15);
    for i in 0..bezier.point_indices.len() {
        let mut p = bezier.control_points[bezier.point_indices[i] as usize];
        p = transform_position(&bezier_node.geometry_to_world, p);
        p.x *= 0.1;
        p.y *= 0.1;
        p.z *= 0.1;
        ufbxt_assert!(p.y >= -0.0001);
        assert_close_real(err, p.z, 0.0);
    }

    let bezier_color = Vec3::new(0.603921568627451, 0.725490196078431, 0.898039215686275);
    assert_close_vec3(err, bezier.color, bezier_color);
});
