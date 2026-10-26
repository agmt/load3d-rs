mod ufbxt;
use ufbxt::*;

file_test!(maya_constraint_zoo, |scene, err, load_error| {
    let mut found_parent = false;
    let mut found_aim = false;
    let mut found_rotation = false;
    let mut found_scale = false;
    let mut found_position = false;
    let mut found_ik = false;
    for constraint in &scene.constraints {
        let name = scene.nodes[constraint.node.unwrap()].element.name.as_str();
        let target_name = |i: usize| scene.nodes[constraint.targets[i].node].element.name.as_str();

        if name == "joint1" && constraint.constraint_type == ConstraintType::Parent {
            found_parent = true;
            ufbxt_assert!(constraint.targets.len() == 1);
            ufbxt_assert!(constraint.constrain_translation[0] && constraint.constrain_translation[1] && constraint.constrain_translation[2]);
            ufbxt_assert!(constraint.constrain_rotation[0] && constraint.constrain_rotation[1] && constraint.constrain_rotation[2]);
            ufbxt_assert!(!constraint.constrain_scale[0] && !constraint.constrain_scale[1] && !constraint.constrain_scale[2]);
            assert_close_real(err, constraint.targets[0].weight, 1.0);
            ufbxt_assert!(target_name(0) == "Parent");
        } else if name == "joint2" && constraint.constraint_type == ConstraintType::Aim {
            found_aim = true;
            ufbxt_assert!(constraint.targets.len() == 1);
            ufbxt_assert!(!constraint.constrain_translation[0] && !constraint.constrain_translation[1] && !constraint.constrain_translation[2]);
            ufbxt_assert!(constraint.constrain_rotation[0] && constraint.constrain_rotation[1] && constraint.constrain_rotation[2]);
            ufbxt_assert!(!constraint.constrain_scale[0] && !constraint.constrain_scale[1] && !constraint.constrain_scale[2]);
            ufbxt_assert!(constraint.aim_up_node.is_some());
            ufbxt_assert!(constraint.aim_up_type == ConstraintAimUpType::ToNode);
            ufbxt_assert!(scene.nodes[constraint.aim_up_node.unwrap()].element.name == "Up");
            assert_close_real(err, constraint.targets[0].weight, 1.0);
            ufbxt_assert!(target_name(0) == "Aim");
        } else if name == "joint3" && constraint.constraint_type == ConstraintType::Rotation {
            found_rotation = true;
            ufbxt_assert!(constraint.targets.len() == 1);
            ufbxt_assert!(!constraint.constrain_translation[0] && !constraint.constrain_translation[1] && !constraint.constrain_translation[2]);
            ufbxt_assert!(constraint.constrain_rotation[0] && constraint.constrain_rotation[1] && constraint.constrain_rotation[2]);
            ufbxt_assert!(!constraint.constrain_scale[0] && !constraint.constrain_scale[1] && !constraint.constrain_scale[2]);
            assert_close_real(err, constraint.targets[0].weight, 1.0);
            ufbxt_assert!(target_name(0) == "joint1");
        } else if name == "joint4" && constraint.constraint_type == ConstraintType::Scale {
            found_scale = true;
            ufbxt_assert!(constraint.targets.len() == 1);
            ufbxt_assert!(!constraint.constrain_translation[0] && !constraint.constrain_translation[1] && !constraint.constrain_translation[2]);
            ufbxt_assert!(!constraint.constrain_rotation[0] && !constraint.constrain_rotation[1] && !constraint.constrain_rotation[2]);
            ufbxt_assert!(constraint.constrain_scale[0] && constraint.constrain_scale[1] && constraint.constrain_scale[2]);
            assert_close_real(err, constraint.targets[0].weight, 1.0);
            ufbxt_assert!(target_name(0) == "Scale");
        } else if name == "joint5" && constraint.constraint_type == ConstraintType::Position {
            found_position = true;
            ufbxt_assert!(constraint.targets.len() == 1);
            ufbxt_assert!(constraint.constrain_translation[0] && constraint.constrain_translation[1] && constraint.constrain_translation[2]);
            ufbxt_assert!(!constraint.constrain_rotation[0] && !constraint.constrain_rotation[1] && !constraint.constrain_rotation[2]);
            ufbxt_assert!(!constraint.constrain_scale[0] && !constraint.constrain_scale[1] && !constraint.constrain_scale[2]);
            assert_close_real(err, constraint.targets[0].weight, 1.0);
            ufbxt_assert!(target_name(0) == "Position");
        } else if name == "joint7" && constraint.constraint_type == ConstraintType::SingleChainIk {
            found_ik = true;
            ufbxt_assert!(constraint.targets.len() == 1);
            ufbxt_assert!(!constraint.constrain_translation[0] && !constraint.constrain_translation[1] && !constraint.constrain_translation[2]);
            ufbxt_assert!(constraint.constrain_rotation[0] && constraint.constrain_rotation[1] && constraint.constrain_rotation[2]);
            ufbxt_assert!(!constraint.constrain_scale[0] && !constraint.constrain_scale[1] && !constraint.constrain_scale[2]);
            assert_close_real(err, constraint.targets[0].weight, 1.0);
            ufbxt_assert!(target_name(0) == "Pole");
            ufbxt_assert!(constraint.ik_effector.is_some_and(|n| scene.nodes[n].element.name == "IKHandle"));
            ufbxt_assert!(constraint.ik_end_node.is_some_and(|n| scene.nodes[n].element.name == "joint10"));
        }
    }

    ufbxt_assert!(found_parent);
    ufbxt_assert!(found_aim);
    ufbxt_assert!(found_rotation);
    ufbxt_assert!(found_scale);
    ufbxt_assert!(found_position);
    ufbxt_assert!(found_ik);
});

file_test!(maya_character, flags = HEAVY_TO_FUZZ, |scene, err, load_error| {
});

file_test!(maya_constraint_multi, |scene, err, load_error| {
    ufbxt_assert!(scene.constraints.len() == 1);
    let constraint = &scene.constraints[0];
    ufbxt_assert!(constraint.constraint_type == ConstraintType::Position);
    ufbxt_assert!(constraint.constrain_translation[0] && constraint.constrain_translation[1] && constraint.constrain_translation[2]);
    ufbxt_assert!(!constraint.constrain_rotation[0] && !constraint.constrain_rotation[1] && !constraint.constrain_rotation[2]);
    ufbxt_assert!(!constraint.constrain_scale[0] && !constraint.constrain_scale[1] && !constraint.constrain_scale[2]);
    ufbxt_assert!(constraint.targets.len() == 2);
    for target in &constraint.targets {
        let target_name = scene.nodes[target.node].element.name.as_str();
        if target_name == "TargetA" {
            assert_close_real(err, target.weight, 1.0);
        } else if target_name == "TargetB" {
            assert_close_real(err, target.weight, 2.0);
        } else {
            ufbxt_assert!(false);
        }
    }
});

file_test!(maya_human_ik, flags = HEAVY_TO_FUZZ, |scene, err, load_error| {
    let mut ik_count = 0u32;
    let mut fk_count = 0u32;
    let mut unknown_count = 0u32;
    for marker in &scene.markers {
        if marker.marker_type == MarkerType::IkEffector {
            ik_count += 1;
        } else if marker.marker_type == MarkerType::FkEffector {
            fk_count += 1;
        } else {
            unknown_count += 1;
        }
    }

    ufbxt_assert!(ik_count == 28);
    ufbxt_assert!(fk_count == 62);
    ufbxt_assert!(unknown_count == 0);

    {
        let node = find_node(scene, "Character1_Ctrl_HipsEffector").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.attrib_type == ElementType::Marker);
        let marker = &scene.markers[scene.elements[node.attrib.unwrap() as usize].index];
        ufbxt_assert!(marker.marker_type == MarkerType::IkEffector);
    }

    {
        let node = find_node(scene, "Character1_Ctrl_RightFoot").map(|n| &scene.nodes[n]);
        ufbxt_assert!(node.is_some());
        let node = node.unwrap();
        ufbxt_assert!(node.attrib_type == ElementType::Marker);
        let marker = &scene.markers[scene.elements[node.attrib.unwrap() as usize].index];
        ufbxt_assert!(marker.marker_type == MarkerType::FkEffector);
    }
});
