mod ufbxt;
use ufbxt::*;

file_test!(motionbuilder_lights, |scene, err, load_error| {
    let white_color = Vec3::new(1.0, 1.0, 1.0);
    let ref_color = Vec3::new(0.2, 0.4, 0.6);

    {
        let node = find_node(scene, "PointConstant").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.light.is_some()));
        let node = node.unwrap();
        let light = &scene.lights[node.light.unwrap()];
        ufbxt_assert!(light.light_type == LightType::Point);
        ufbxt_assert!(light.decay == LightDecay::None);
        assert_close_vec3(err, light.color, white_color);
        assert_close_real(err, light.intensity, 1.0);
        ufbxt_assert!(light.cast_shadows);
    }

    {
        let node = find_node(scene, "PointLinear").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.light.is_some()));
        let node = node.unwrap();
        let light = &scene.lights[node.light.unwrap()];
        ufbxt_assert!(light.light_type == LightType::Point);
        ufbxt_assert!(light.decay == LightDecay::Linear);
        assert_close_vec3(err, light.color, ref_color);
        assert_close_real(err, light.intensity, 0.5);
        ufbxt_assert!(light.cast_shadows);
    }

    {
        let node = find_node(scene, "PointQuadratic").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.light.is_some()));
        let node = node.unwrap();
        let light = &scene.lights[node.light.unwrap()];
        ufbxt_assert!(light.light_type == LightType::Point);
        ufbxt_assert!(light.decay == LightDecay::Quadratic);
        assert_close_vec3(err, light.color, ref_color);
        assert_close_real(err, light.intensity, 1.5);
        ufbxt_assert!(light.cast_shadows);
    }

    {
        let node = find_node(scene, "PointCubic").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.light.is_some()));
        let node = node.unwrap();
        let light = &scene.lights[node.light.unwrap()];
        ufbxt_assert!(light.light_type == LightType::Point);
        ufbxt_assert!(light.decay == LightDecay::Cubic);
        assert_close_vec3(err, light.color, ref_color);
        assert_close_real(err, light.intensity, 2.0);
        ufbxt_assert!(!light.cast_shadows);
    }

    {
        let node = find_node(scene, "SpotConstant").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.light.is_some()));
        let node = node.unwrap();
        let light = &scene.lights[node.light.unwrap()];
        ufbxt_assert!(light.light_type == LightType::Spot);
        ufbxt_assert!(light.decay == LightDecay::None);
        assert_close_vec3(err, light.color, white_color);
        assert_close_real(err, light.intensity, 1.0);
        ufbxt_assert!(light.cast_shadows);
    }

    {
        let node = find_node(scene, "SpotLinear").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.light.is_some()));
        let node = node.unwrap();
        let light = &scene.lights[node.light.unwrap()];
        ufbxt_assert!(light.light_type == LightType::Spot);
        ufbxt_assert!(light.decay == LightDecay::Linear);
        assert_close_vec3(err, light.color, ref_color);
        assert_close_real(err, light.intensity, 1.1);
        ufbxt_assert!(light.cast_shadows);
    }

    {
        let node = find_node(scene, "SpotQuadratic").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.light.is_some()));
        let node = node.unwrap();
        let light = &scene.lights[node.light.unwrap()];
        ufbxt_assert!(light.light_type == LightType::Spot);
        ufbxt_assert!(light.decay == LightDecay::Quadratic);
        assert_close_vec3(err, light.color, ref_color);
        assert_close_real(err, light.intensity, 1.2);
        ufbxt_assert!(light.cast_shadows);
    }

    {
        let node = find_node(scene, "SpotCubic").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.light.is_some()));
        let node = node.unwrap();
        let light = &scene.lights[node.light.unwrap()];
        ufbxt_assert!(light.light_type == LightType::Spot);
        ufbxt_assert!(light.decay == LightDecay::Cubic);
        assert_close_vec3(err, light.color, ref_color);
        assert_close_real(err, light.intensity, 1.3);
        ufbxt_assert!(!light.cast_shadows);
    }

    {
        let node = find_node(scene, "SpotLook").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some_and(|n| n.light.is_some()));
        let node = node.unwrap();
        let light = &scene.lights[node.light.unwrap()];
        ufbxt_assert!(light.light_type == LightType::Spot);
        ufbxt_assert!(light.decay == LightDecay::Quadratic);
        assert_close_vec3(err, light.color, ref_color);
        assert_close_real(err, light.intensity, 1.0);
        ufbxt_assert!(light.cast_shadows);

        let look_at = find_prop_element(scene, &node.element, "LookAtProperty", ElementType::Node);
        ufbxt_assert!(look_at.is_some());
        ufbxt_assert!(scene.element(look_at.unwrap()).name == "Cube");
    }
});
