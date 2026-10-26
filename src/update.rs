use std::sync::Arc;

use crate::context::Context;
use crate::math::*;
use crate::types::*;

const LIGHT_TYPES: [LightType; 5] = [LightType::Point, LightType::Directional, LightType::Spot, LightType::Area, LightType::Volume];
const LIGHT_DECAYS: [LightDecay; 4] = [LightDecay::None, LightDecay::Linear, LightDecay::Quadratic, LightDecay::Cubic];
const LIGHT_AREA_SHAPES: [LightAreaShape; 2] = [LightAreaShape::Rectangle, LightAreaShape::Sphere];
const PROJECTION_MODES: [ProjectionMode; 2] = [ProjectionMode::Perspective, ProjectionMode::Orthographic];
const ASPECT_MODES: [AspectMode; 5] =
    [AspectMode::WindowSize, AspectMode::FixedRatio, AspectMode::FixedResolution, AspectMode::FixedWidth, AspectMode::FixedHeight];
const APERTURE_MODES: [ApertureMode; 4] = [ApertureMode::HorizontalAndVertical, ApertureMode::Horizontal, ApertureMode::Vertical, ApertureMode::FocalLength];
const GATE_FITS: [GateFit; 6] = [GateFit::None, GateFit::Vertical, GateFit::Horizontal, GateFit::Fill, GateFit::Overscan, GateFit::Stretch];
const APERTURE_FORMATS: [ApertureFormat; 12] = [
    ApertureFormat::Custom,
    ApertureFormat::Theatrical16mm,
    ApertureFormat::Super16mm,
    ApertureFormat::Academy35mm,
    ApertureFormat::TvProjection35mm,
    ApertureFormat::FullAperture35mm,
    ApertureFormat::Projection185_35mm,
    ApertureFormat::Anamorphic35mm,
    ApertureFormat::Projection70mm,
    ApertureFormat::Vistavision,
    ApertureFormat::Dynavision,
    ApertureFormat::Imax,
];
const WRAP_MODES: [WrapMode; 2] = [WrapMode::Repeat, WrapMode::Clamp];
const TIME_MODES: [TimeMode; 18] = [
    TimeMode::Default,
    TimeMode::Fps120,
    TimeMode::Fps100,
    TimeMode::Fps60,
    TimeMode::Fps50,
    TimeMode::Fps48,
    TimeMode::Fps30,
    TimeMode::Fps30Drop,
    TimeMode::NtscDropFrame,
    TimeMode::NtscFullFrame,
    TimeMode::Pal,
    TimeMode::Fps24,
    TimeMode::Fps1000,
    TimeMode::FilmFullFrame,
    TimeMode::Custom,
    TimeMode::Fps96,
    TimeMode::Fps72,
    TimeMode::Fps59_94,
];
const TIME_PROTOCOLS: [TimeProtocol; 3] = [TimeProtocol::Smpte, TimeProtocol::FrameCount, TimeProtocol::Default];
const SNAP_MODES: [SnapMode; 4] = [SnapMode::None, SnapMode::Snap, SnapMode::Play, SnapMode::SnapAndPlay];
const AIM_UP_TYPES: [ConstraintAimUpType; 4] =
    [ConstraintAimUpType::Scene, ConstraintAimUpType::ToNode, ConstraintAimUpType::AlignNode, ConstraintAimUpType::Vector];

/// `all` lists the variants, its last one being ufbx's maximum
fn find_enum<T: Copy>(props: &Props, name: &str, def: i64, all: &[T]) -> T {
    all[props.find_enum(name, def, all.len() as i64 - 1) as usize]
}

fn lower_bound_eq<T>(data: &[T], linear: usize, less: impl Fn(&T) -> bool, eq: impl Fn(&T) -> bool) -> Option<usize> {
    crate::search::lower_bound_eq(0, data.len(), linear, |i| less(&data[i]), |i| eq(&data[i]))
}

fn mirror_translation(v: &mut Vec3, axis: MirrorAxis) {
    let i = axis as usize - 1;
    v[i] = -v[i];
}

fn mirror_rotation(q: &mut Quat, axis: MirrorAxis) {
    let a = axis as usize;
    q[a % 3] = -q[a % 3];
    q[(a + 1) % 3] = -q[(a + 1) % 3];
}

pub(crate) fn get_geometry_transform(props: &Props, node: &Node) -> Transform {
    let translation = props.vec3("GeometricTranslation", Vec3::ZERO);
    let rotation = props.vec3("GeometricRotation", Vec3::ZERO);
    let scaling = props.vec3("GeometricScaling", Vec3::ONE);

    let mut t = Transform::IDENTITY;

    mul_scale(&mut t, scaling);
    mul_rotate(&mut t, rotation, RotationOrder::Xyz);
    add_translate(&mut t, translation);

    if node.has_adjust_transform {
        t.translation.x *= node.adjust_translation_scale;
        t.translation.y *= node.adjust_translation_scale;
        t.translation.z *= node.adjust_translation_scale;
    }

    if node.adjust_mirror_axis != MirrorAxis::None {
        mirror_translation(&mut t.translation, node.adjust_mirror_axis);
        mirror_rotation(&mut t.rotation, node.adjust_mirror_axis);
    }

    t
}

pub(crate) fn get_rotation(props: &Props, order: RotationOrder, node: &Node) -> Quat {
    let rotation = props.vec3("Lcl Rotation", Vec3::ZERO);
    let pre_rotation = props.vec3("PreRotation", Vec3::ZERO);
    let post_rotation = props.vec3("PostRotation", Vec3::ZERO);

    let mut t = Transform::IDENTITY;

    if node.has_adjust_transform {
        mul_rotate_quat(&mut t, node.adjust_post_rotation);
    }

    if node.use_rotation_space {
        mul_inv_rotate(&mut t, post_rotation, RotationOrder::Xyz);
        mul_rotate(&mut t, rotation, order);
        mul_rotate(&mut t, pre_rotation, RotationOrder::Xyz);
    } else {
        mul_rotate(&mut t, rotation, RotationOrder::Xyz);
    }

    if node.has_adjust_transform {
        mul_rotate_quat(&mut t, node.adjust_pre_rotation);
    }

    if node.adjust_mirror_axis != MirrorAxis::None {
        mirror_rotation(&mut t.rotation, node.adjust_mirror_axis);
    }

    t.rotation
}

pub(crate) fn get_scale(props: &Props, node: &Node) -> Vec3 {
    let scaling = props.vec3("Lcl Scaling", Vec3::ONE);

    let mut t = Transform::IDENTITY;

    if node.has_adjust_transform {
        mul_scale_real(&mut t, node.adjust_post_scale);
    }

    mul_scale(&mut t, scaling);

    if node.has_adjust_transform {
        mul_scale_real(&mut t, node.adjust_pre_scale);
    }

    t.scale
}

pub(crate) fn get_transform(props: &Props, order: RotationOrder, node: &Node, translation_scale: Option<&Vec3>) -> Transform {
    let scale_pivot = props.vec3("ScalingPivot", Vec3::ZERO);
    let rot_pivot = props.vec3("RotationPivot", Vec3::ZERO);
    let scale_offset = props.vec3("ScalingOffset", Vec3::ZERO);
    let rot_offset = props.vec3("RotationOffset", Vec3::ZERO);

    let mut translation = props.vec3("Lcl Translation", Vec3::ZERO);
    let rotation = props.vec3("Lcl Rotation", Vec3::ZERO);
    let scaling = props.vec3("Lcl Scaling", Vec3::ONE);

    let pre_rotation = props.vec3("PreRotation", Vec3::ZERO);
    let post_rotation = props.vec3("PostRotation", Vec3::ZERO);

    let mut t = Transform::IDENTITY;

    // T * Roff * Rp * Rpre * R * Rpost^-1 * Rp^-1 * Soff * Sp * S * Sp^-1, applied from the right

    if let Some(s) = translation_scale {
        translation.x *= s.x;
        translation.y *= s.y;
        translation.z *= s.z;
    }

    if node.has_adjust_transform {
        mul_rotate_quat(&mut t, node.adjust_post_rotation);
        mul_scale_real(&mut t, node.adjust_post_scale);
    }

    sub_translate(&mut t, scale_pivot);
    mul_scale(&mut t, scaling);
    add_translate(&mut t, scale_pivot);

    add_translate(&mut t, scale_offset);

    sub_translate(&mut t, rot_pivot);
    if node.use_rotation_space {
        mul_inv_rotate(&mut t, post_rotation, RotationOrder::Xyz);
        mul_rotate(&mut t, rotation, order);
        mul_rotate(&mut t, pre_rotation, RotationOrder::Xyz);
    } else {
        mul_rotate(&mut t, rotation, RotationOrder::Xyz);
    }
    add_translate(&mut t, rot_pivot);

    add_translate(&mut t, rot_offset);

    add_translate(&mut t, translation);

    if node.has_adjust_transform {
        add_translate(&mut t, node.adjust_pre_translation);
        mul_rotate_quat(&mut t, node.adjust_pre_rotation);
        mul_scale_real(&mut t, node.adjust_pre_scale);
        t.translation.x *= node.adjust_translation_scale;
        t.translation.y *= node.adjust_translation_scale;
        t.translation.z *= node.adjust_translation_scale;
    }

    if node.adjust_mirror_axis != MirrorAxis::None {
        mirror_translation(&mut t.translation, node.adjust_mirror_axis);
        mirror_rotation(&mut t.rotation, node.adjust_mirror_axis);
    }

    t
}

pub(crate) fn get_texture_transform(props: &Props) -> Transform {
    let scale_pivot = props.vec3("TextureScalingPivot", Vec3::ZERO);
    let rot_pivot = props.vec3("TextureRotationPivot", Vec3::ZERO);

    let translation = props.vec3("Translation", Vec3::ZERO);
    let rotation = props.vec3("Rotation", Vec3::ZERO);
    let scaling = props.vec3("Scaling", Vec3::ONE);

    let mut t = Transform::IDENTITY;

    sub_translate(&mut t, scale_pivot);
    mul_scale(&mut t, scaling);
    add_translate(&mut t, scale_pivot);

    sub_translate(&mut t, rot_pivot);
    mul_rotate(&mut t, rotation, RotationOrder::Xyz);
    add_translate(&mut t, rot_pivot);

    add_translate(&mut t, translation);

    if props.int("UVSwap", 0) != 0 {
        mul_scale(&mut t, Vec3::new(-1.0, 0.0, 0.0));
        mul_rotate(&mut t, Vec3::new(0.0, 0.0, -90.0), RotationOrder::Xyz);
    }

    t
}

fn get_constraint_transform(props: &Props) -> Transform {
    let translation = props.vec3("Translation", Vec3::ZERO);
    let rotation = props.vec3("Rotation", Vec3::ZERO);
    let rotation_offset = props.vec3("RotationOffset", Vec3::ZERO);
    let scaling = props.vec3("Scaling", Vec3::ONE);

    let mut t = Transform::IDENTITY;

    mul_scale(&mut t, scaling);
    mul_rotate(&mut t, rotation, RotationOrder::Xyz);
    mul_rotate(&mut t, rotation_offset, RotationOrder::Xyz);
    add_translate(&mut t, translation);

    t
}

pub(crate) fn update_node(scene: &mut Scene, ix: usize, overrides: &[TransformOverride]) {
    let node = &mut scene.nodes[ix];
    let props = &node.element.props;
    node.rotation_order = RotationOrder::from_index(props.find_enum("RotationOrder", RotationOrder::Xyz as i64, RotationOrder::Spheric as i64));
    node.euler_rotation = props.vec3("Lcl Rotation", Vec3::ZERO);

    let parent = node.parent;
    if !node.is_root {
        let rotation_active = node.element.props.int("RotationActive", 1) != 0;
        let rotation_limit_only = node.element.props.int("RotationSpaceForLimitOnly", 0) != 0;
        node.use_rotation_space = rotation_active && !rotation_limit_only;

        let transform_scale = parent.and_then(|p| scene.nodes[p].scale_helper).map(|h| scene.nodes[h].local_transform.scale);
        let node = &scene.nodes[ix];
        scene.nodes[ix].local_transform = get_transform(&node.element.props, node.rotation_order, node, transform_scale.as_ref());
        if scene.nodes[ix].is_scale_helper
            && let Some(p) = parent
            && let Some(scale_parent) = scene.nodes[p].inherit_scale_node
            && let Some(helper) = scene.nodes[scale_parent].scale_helper
        {
            let inherit_scale = scene.nodes[helper].local_transform.scale;
            let scale = &mut scene.nodes[ix].local_transform.scale;
            scale.x *= inherit_scale.x;
            scale.y *= inherit_scale.y;
            scale.z *= inherit_scale.z;
        }

        let node = &mut scene.nodes[ix];
        if !overrides.is_empty() {
            let typed_id = node.element.typed_id;
            if let Some(i) = lower_bound_eq(overrides, 16, |a| a.node_id < typed_id, |a| a.node_id == typed_id) {
                node.local_transform = overrides[i].transform;
            }
        }
        node.node_to_parent = transform_to_matrix(&node.local_transform);
        node.geometry_transform = get_geometry_transform(&node.element.props, node);
    } else {
        node.geometry_transform = Transform::IDENTITY;
    }

    let node = &mut scene.nodes[ix];
    let unscaled_node_to_parent = unscaled_transform_to_matrix(&node.local_transform);

    node.inherit_scale = node.local_transform.scale;

    if let Some(p) = parent {
        let parent = &scene.nodes[p];
        let (parent_to_world, unscaled_parent_to_world, parent_inherit_scale) = (parent.node_to_world, parent.unscaled_node_to_world, parent.inherit_scale);
        let node = &scene.nodes[ix];
        if node.inherit_mode == InheritMode::Normal {
            let node_to_world = matrix_mul(&parent_to_world, &node.node_to_parent);
            let unscaled_node_to_world = matrix_mul(&parent_to_world, &unscaled_node_to_parent);
            let node = &mut scene.nodes[ix];
            node.node_to_world = node_to_world;
            node.unscaled_node_to_world = unscaled_node_to_world;
        } else {
            let mut transform = node.local_transform;

            let parent_scale = node.inherit_scale_node.map_or(Vec3::ONE, |n| scene.nodes[n].inherit_scale);

            transform.scale.x *= parent_scale.x;
            transform.scale.y *= parent_scale.y;
            transform.scale.z *= parent_scale.z;
            transform.translation.x *= parent_inherit_scale.x;
            transform.translation.y *= parent_inherit_scale.y;
            transform.translation.z *= parent_inherit_scale.z;

            let node_to_unscaled_parent = transform_to_matrix(&transform);
            let unscaled_node_to_unscaled_parent = unscaled_transform_to_matrix(&transform);

            let node = &mut scene.nodes[ix];
            node.inherit_scale = transform.scale;
            node.node_to_world = matrix_mul(&unscaled_parent_to_world, &node_to_unscaled_parent);
            node.unscaled_node_to_world = matrix_mul(&unscaled_parent_to_world, &unscaled_node_to_unscaled_parent);
        }
    } else {
        node.node_to_world = node.node_to_parent;
        node.unscaled_node_to_world = unscaled_node_to_parent;
    }

    let node = &mut scene.nodes[ix];
    if !is_transform_identity(&node.geometry_transform) {
        node.geometry_to_node = transform_to_matrix(&node.geometry_transform);
        node.geometry_to_world = matrix_mul(&node.node_to_world, &node.geometry_to_node);
        node.has_geometry_transform = true;
    } else {
        node.geometry_to_node = Matrix::IDENTITY;
        node.geometry_to_world = node.node_to_world;
        node.has_geometry_transform = false;
    }

    node.visible = node.element.props.int("Visibility", 1) != 0;
}

fn update_light(light: &mut Light) {
    let props = &light.element.props;
    // ufbx's quirk: FBX stores 100 times the intensity Maya and Blender show
    light.intensity = props.real("Intensity", 100.0) / 100.0;

    light.color = props.vec3("Color", Vec3::ONE);
    light.light_type = find_enum(props, "LightType", 0, &LIGHT_TYPES);
    light.decay = find_enum(props, "DecayType", LightDecay::None as i64, &LIGHT_DECAYS);
    light.area_shape = find_enum(props, "AreaLightShape", 0, &LIGHT_AREA_SHAPES);
    light.inner_angle = props.real("HotSpot", 0.0);
    light.inner_angle = props.real("InnerAngle", light.inner_angle);
    light.outer_angle = props.real("Cone angle", 0.0);
    light.outer_angle = props.real("ConeAngle", light.outer_angle);
    light.outer_angle = props.real("OuterAngle", light.outer_angle);
    light.cast_light = props.int("CastLight", 1) != 0;
    light.cast_shadows = props.int("CastShadows", 0) != 0;
}

/// `ufbxi_aperture_formats` in ufbx.c: Film width and height in thousandths of an inch, by `ApertureFormat`
const APERTURE_FILM_SIZES: [(u16, u16); 12] =
    [(1000, 1000), (404, 295), (493, 292), (864, 630), (816, 612), (980, 735), (825, 446), (864, 732), (2066, 906), (1485, 991), (2080, 1480), (2772, 2072)];

fn update_camera(metadata: &Metadata, camera: &mut Camera) {
    let props = &camera.element.props;
    camera.projection_mode = find_enum(props, "CameraProjectionType", 0, &PROJECTION_MODES);
    camera.aspect_mode = find_enum(props, "AspectRatioMode", 0, &ASPECT_MODES);
    camera.aperture_mode = find_enum(props, "ApertureMode", ApertureMode::Vertical as i64, &APERTURE_MODES);
    camera.aperture_format = find_enum(props, "ApertureFormat", ApertureFormat::Custom as i64, &APERTURE_FORMATS);
    camera.gate_fit = find_enum(props, "GateFit", 0, &GATE_FITS);

    camera.near_plane = props.real("NearPlane", 0.0);
    camera.far_plane = props.real("FarPlane", 0.0);

    let mut aspect_x = props.real("AspectW", 0.0);
    let mut aspect_y = props.real("AspectH", 0.0);
    aspect_x = props.real("AspectWidth", aspect_x);
    aspect_y = props.real("AspectHeight", aspect_y);

    let fov = props.real("FieldOfView", 0.0);
    let fov_x = props.real("FieldOfViewX", 0.0);
    let fov_y = props.real("FieldOfViewY", 0.0);

    let focal_length = props.real("FocalLength", 0.0);
    let mut ortho_extent = metadata.ortho_size_unit * props.real("OrthoZoom", 1.0);

    let (film_x, film_y) = APERTURE_FILM_SIZES[camera.aperture_format as usize];
    let mut film_size = Vec2::new(film_x as Real * 0.001, film_y as Real * 0.001);
    let mut squeeze_ratio = if camera.aperture_format == ApertureFormat::Anamorphic35mm { 2.0 } else { 1.0 };

    film_size.x = props.real("FilmWidth", film_size.x);
    film_size.y = props.real("FilmHeight", film_size.y);
    squeeze_ratio = props.real("FilmSqueezeRatio", squeeze_ratio);

    if aspect_x <= 0.0 && aspect_y <= 0.0 {
        aspect_x = if film_size.x > 0.0 { film_size.x } else { 1.0 };
        aspect_y = if film_size.y > 0.0 { film_size.y } else { 1.0 };
    } else if aspect_x <= 0.0 {
        if film_size.x > 0.0 && film_size.y > 0.0 {
            aspect_x = aspect_y / film_size.y * film_size.x;
        } else {
            aspect_x = aspect_y;
        }
    } else if aspect_y <= 0.0 {
        if film_size.x > 0.0 && film_size.y > 0.0 {
            aspect_y = aspect_x / film_size.x * film_size.y;
        } else {
            aspect_y = aspect_x;
        }
    }

    film_size.y *= squeeze_ratio;

    ortho_extent *= metadata.geometry_scale;
    camera.near_plane *= metadata.geometry_scale;
    camera.far_plane *= metadata.geometry_scale;

    camera.focal_length_mm = focal_length;
    camera.film_size_inch = film_size;
    camera.squeeze_ratio = squeeze_ratio;
    camera.orthographic_extent = ortho_extent;

    match camera.aspect_mode {
        AspectMode::WindowSize | AspectMode::FixedRatio => {
            camera.resolution_is_pixels = false;
            camera.resolution = Vec2::new(aspect_x, aspect_y);
        }
        AspectMode::FixedResolution => {
            camera.resolution_is_pixels = true;
            camera.resolution = Vec2::new(aspect_x, aspect_y);
        }
        AspectMode::FixedWidth => {
            camera.resolution_is_pixels = true;
            camera.resolution = Vec2::new(aspect_x, aspect_x * aspect_y);
        }
        AspectMode::FixedHeight => {
            camera.resolution_is_pixels = true;
            camera.resolution = Vec2::new(aspect_y * aspect_x, aspect_y);
        }
    }

    let aspect_ratio = camera.resolution.x / camera.resolution.y;
    let film_ratio = film_size.x / film_size.y;

    camera.aspect_ratio = aspect_ratio;

    let mut effective_fit = camera.gate_fit;
    if effective_fit == GateFit::Fill {
        effective_fit = if aspect_ratio > film_ratio { GateFit::Horizontal } else { GateFit::Vertical };
    } else if effective_fit == GateFit::Overscan {
        effective_fit = if aspect_ratio < film_ratio { GateFit::Horizontal } else { GateFit::Vertical };
    }

    match effective_fit {
        GateFit::Vertical => {
            camera.aperture_size_inch = Vec2::new(camera.film_size_inch.y * aspect_ratio, camera.film_size_inch.y);
            camera.orthographic_size = Vec2::new(ortho_extent * aspect_ratio, ortho_extent);
        }
        GateFit::Horizontal => {
            camera.aperture_size_inch = Vec2::new(camera.film_size_inch.x, camera.film_size_inch.x / aspect_ratio);
            camera.orthographic_size = Vec2::new(ortho_extent, ortho_extent / aspect_ratio);
        }
        GateFit::None | GateFit::Fill | GateFit::Overscan | GateFit::Stretch => {
            camera.aperture_size_inch = camera.film_size_inch;
            camera.orthographic_size = Vec2::new(ortho_extent, ortho_extent);
        }
    }

    let fov_deg = &mut camera.field_of_view_deg;
    let fov_tan = &mut camera.field_of_view_tan;
    match camera.aperture_mode {
        ApertureMode::HorizontalAndVertical => {
            fov_deg.x = fov_x;
            fov_deg.y = fov_y;
            fov_tan.x = (fov_x * (DEG_TO_RAD * 0.5)).tan();
            fov_tan.y = (fov_y * (DEG_TO_RAD * 0.5)).tan();
        }
        ApertureMode::Horizontal => {
            fov_deg.x = fov;
            fov_tan.x = (fov * (DEG_TO_RAD * 0.5)).tan();
            fov_tan.y = fov_tan.x / aspect_ratio;
            fov_deg.y = fov_tan.y.atan() * RAD_TO_DEG * 2.0;
        }
        ApertureMode::Vertical => {
            fov_deg.y = fov;
            fov_tan.y = (fov * (DEG_TO_RAD * 0.5)).tan();
            fov_tan.x = fov_tan.y * aspect_ratio;
            fov_deg.x = fov_tan.x.atan() * RAD_TO_DEG * 2.0;
        }
        ApertureMode::FocalLength => {
            fov_tan.x = camera.aperture_size_inch.x / (camera.focal_length_mm * MM_TO_INCH) * 0.5;
            fov_tan.y = camera.aperture_size_inch.y / (camera.focal_length_mm * MM_TO_INCH) * 0.5;
            fov_deg.x = fov_tan.x.atan() * RAD_TO_DEG * 2.0;
            fov_deg.y = fov_tan.y.atan() * RAD_TO_DEG * 2.0;
        }
    }

    camera.projection_plane = if camera.projection_mode == ProjectionMode::Perspective { camera.field_of_view_tan } else { camera.orthographic_size };
}

fn update_bone(metadata: &Metadata, bone: &mut Bone) {
    let unit = metadata.bone_prop_size_unit;

    bone.radius = bone.element.props.real("Size", unit) / unit;
    if metadata.bone_prop_limb_length_relative {
        bone.relative_length = bone.element.props.real("LimbLength", 1.0);
    } else {
        bone.relative_length = 1.0;
    }
}

fn update_line_curve(line: &mut LineCurve) {
    line.color = line.element.props.vec3("Color", Vec3::ONE);
}

fn update_pose(nodes: &[Node], pose: &mut Pose) {
    for i in 0..pose.bone_poses.len() {
        let node = &nodes[pose.bone_poses[i].bone_node];

        let parent_to_world = match node.parent.and_then(|p| crate::api::get_bone_pose(pose, p)) {
            Some(bone_pose) => bone_pose.bone_to_world,
            None => node.parent.map_or(Matrix::IDENTITY, |p| nodes[p].node_to_world),
        };

        let world_to_parent = matrix_invert(&parent_to_world);
        let bone = &mut pose.bone_poses[i];
        bone.bone_to_parent = matrix_mul(&world_to_parent, &bone.bone_to_world);
    }
}

fn update_skin_cluster(nodes: &[Node], cluster: &mut SkinCluster) {
    cluster.geometry_to_world = match cluster.bone_node {
        Some(bone) => matrix_mul(&nodes[bone].node_to_world, &cluster.geometry_to_bone),
        None => matrix_mul(&cluster.bind_to_world, &cluster.geometry_to_bone),
    };
    cluster.geometry_to_world_transform = matrix_to_transform(&cluster.geometry_to_world);
}

fn update_blend_channel(channel: &mut BlendChannel) {
    let weight = channel.element.props.real("DeformPercent", 0.0) * 0.01;
    channel.weight = weight;

    let keys = &mut channel.keyframes;
    let num_keys = keys.len() as isize;
    if num_keys > 0 {
        let mut last_negative: isize = -1;
        for i in 0..num_keys {
            keys[i as usize].effective_weight = 0.0;
            if keys[i as usize].target_weight < 0.0 {
                last_negative = i;
            }
        }

        // `None` is ufbx's zero keyframe, a weight of 0 that nothing keeps
        let (mut prev, mut next): (Option<usize>, Option<usize>) = (None, None);
        if weight > 0.0 {
            if last_negative >= 0 {
                prev = Some(last_negative as usize);
            }
            for i in (last_negative + 1) as usize..num_keys as usize {
                prev = next;
                next = Some(i);
                if keys[i].target_weight > weight {
                    break;
                }
            }
        } else {
            if last_negative + 1 < num_keys {
                prev = Some((last_negative + 1) as usize);
            }
            for i in (0..=last_negative).rev() {
                prev = next;
                next = Some(i as usize);
                if keys[i as usize].target_weight < weight {
                    break;
                }
            }
        }

        let target = |k: Option<usize>| k.map_or(0.0, |i| keys[i].target_weight);
        let delta = target(next) - target(prev);
        if delta != 0.0 {
            let t = (weight - target(prev)) / delta;
            if let Some(p) = prev {
                keys[p].effective_weight = 1.0 - t;
            }
            if let Some(n) = next {
                keys[n].effective_weight = t;
            }
        }
    }
}

fn update_material(scene: &mut Scene, material: usize) {
    if scene.materials[material].element.props.num_animated > 0 {
        crate::materials::fetch_maps(scene, material);
    }
}

fn update_texture(scene: &mut Scene, ix: usize) {
    let texture = &mut scene.textures[ix];
    texture.uv_transform = get_texture_transform(&texture.element.props);
    if !is_transform_identity(&texture.uv_transform) {
        texture.has_uv_transform = true;
        texture.texture_to_uv = transform_to_matrix(&texture.uv_transform);
        texture.uv_to_texture = matrix_invert(&texture.texture_to_uv);
    } else {
        texture.has_uv_transform = false;
        texture.texture_to_uv = Matrix::IDENTITY;
        texture.uv_to_texture = Matrix::IDENTITY;
    }
    texture.wrap_u = find_enum(&texture.element.props, "WrapModeU", 0, &WRAP_MODES);
    texture.wrap_v = find_enum(&texture.element.props, "WrapModeV", 0, &WRAP_MODES);

    if let Some(mut shader) = texture.shader.take() {
        crate::textures::update_shader_texture(scene, ix, &mut shader);
        scene.textures[ix].shader = Some(shader);
    }
}

fn update_anim_stack(metadata: &Metadata, stack: &mut AnimStack) {
    let props = &stack.element.props;
    let (mut begin, mut end) = (props.find_prop("LocalStart"), props.find_prop("LocalStop"));
    if begin.is_none() || end.is_none() {
        begin = props.find_prop("ReferenceStart");
        end = props.find_prop("ReferenceStop");
    }

    if let (Some(begin), Some(end)) = (begin, end) {
        stack.time_begin = begin.value_int as f64 / metadata.ktime_second as f64;
        stack.time_end = end.value_int as f64 / metadata.ktime_second as f64;
    }

    let anim = Arc::make_mut(&mut stack.anim);
    anim.time_begin = stack.time_begin;
    anim.time_end = stack.time_end;
}

fn update_display_layer(layer: &mut DisplayLayer) {
    let props = &layer.element.props;
    layer.visible = props.int("Show", 1) != 0;
    layer.frozen = props.int("Freeze", 1) != 0;
    let gray = 0.8f32 as Real;
    layer.ui_color = props.vec3("Color", Vec3::new(gray, gray, gray));
}

fn find_bool3(props: &Props, name: &str, default_value: bool) -> [bool; 3] {
    let def = default_value as i64;
    ["X", "Y", "Z"].map(|axis| props.find_int(&format!("{name}{axis}"), def) != 0)
}

fn update_constraint(nodes: &[Node], constraint: &mut Constraint) {
    let constraint_type = constraint.constraint_type;

    constraint.transform_offset = get_constraint_transform(&constraint.element.props);

    let props = &constraint.element.props;
    constraint.weight = props.real("Weight", 100.0) / 100.0;

    for target in &mut constraint.targets {
        let name = nodes[target.node].element.name.as_str();

        // ufbx: IK weights do not seem to be scaled by 100
        let weight_scale = if constraint_type == ConstraintType::SingleChainIk { 1.0 } else { 100.0 };

        let prop = props.find_concat(&[name, ".Weight"]);
        target.weight = prop.map_or(weight_scale, |p| p.value_real()) / weight_scale;

        if constraint_type == ConstraintType::Parent {
            let t = props.find_concat(&[name, ".Offset T"]).map_or(Vec3::ZERO, |p| p.value_vec3());
            let r = props.find_concat(&[name, ".Offset R"]).map_or(Vec3::ZERO, |p| p.value_vec3());
            let s = props.find_concat(&[name, ".Offset S"]).map_or(Vec3::ONE, |p| p.value_vec3());

            target.transform.translation = t;
            target.transform.rotation = euler_to_quat(r, RotationOrder::Xyz);
            target.transform.scale = s;
        }
    }

    constraint.active = props.find_int("Active", 1) != 0;
    match constraint_type {
        ConstraintType::Aim => {
            constraint.constrain_rotation = find_bool3(props, "Affect", true);

            let up_type = props.find_int("WorldUpType", 0);
            if up_type >= 0 && up_type < ConstraintAimUpType::None as i64 {
                constraint.aim_up_type = AIM_UP_TYPES[up_type as usize];
            }
            constraint.aim_vector = props.find_vec3("AimVector", Vec3::new(1.0, 0.0, 0.0));
            constraint.aim_up_vector = props.find_vec3("UpVector", Vec3::new(0.0, 1.0, 0.0));
        }
        ConstraintType::Parent => {
            constraint.constrain_translation = find_bool3(props, "AffectTranslation", true);
            constraint.constrain_rotation = find_bool3(props, "AffectRotation", true);
            constraint.constrain_scale = find_bool3(props, "AffectScale", false);
        }
        ConstraintType::Position => constraint.constrain_translation = find_bool3(props, "Affect", true),
        ConstraintType::Rotation => constraint.constrain_rotation = find_bool3(props, "Affect", true),
        ConstraintType::Scale => constraint.constrain_scale = find_bool3(props, "Affect", true),
        ConstraintType::SingleChainIk => {
            constraint.constrain_rotation = [true; 3];
            constraint.ik_pole_vector = props.find_vec3("PoleVectorType", Vec3::ZERO);
        }
        ConstraintType::Unknown => {}
    }
}

pub(crate) fn update_anim(scene: &mut Scene) {
    if let Some(stack) = scene.anim_stacks.first() {
        scene.anim = stack.anim.clone();
    }
}

pub(crate) fn mirror_matrix_dst(m: &mut Matrix, axis: MirrorAxis) {
    if axis == MirrorAxis::None {
        return;
    }
    let ax = axis as usize - 1;
    for i in 0..4 {
        let mut c = m.col(i);
        c[ax] = -c[ax];
        m.set_col(i, c);
    }
}

pub(crate) fn mirror_matrix_src(m: &mut Matrix, axis: MirrorAxis) {
    if axis == MirrorAxis::None {
        return;
    }
    let ax = axis as usize - 1;
    m.set_col(ax, neg3(m.col(ax)));
}

pub(crate) fn mirror_matrix(m: &mut Matrix, axis: MirrorAxis) {
    if axis == MirrorAxis::None {
        return;
    }
    mirror_matrix_src(m, axis);
    mirror_matrix_dst(m, axis);
}

fn scale_translation(m: &mut Matrix, s: Real) {
    m.m03 *= s;
    m.m13 *= s;
    m.m23 *= s;
}

fn fetch_src(scene: &Scene, element: ElementId, dst_type: ElementType) -> Option<usize> {
    crate::finalize::fetch_src_element(scene, element, false, "", dst_type).map(|id| scene.elements[id as usize].index)
}

pub(crate) fn update_initial_clusters(scene: &mut Scene) {
    for cluster in &mut scene.skin_clusters {
        cluster.geometry_to_bone = cluster.mesh_node_to_bone;
    }

    let mirror_axis = scene.metadata.mirror_axis;
    let geometry_scale = scene.metadata.geometry_scale;

    {
        let world_to_units;
        let mut translation_scale = 1.0;

        if scene.metadata.space_conversion == SpaceConversion::TransformRoot && scene.metadata.mirror_axis == MirrorAxis::None {
            world_to_units = scene.nodes[scene.root_node].node_to_parent;
        } else {
            let root_scale = scene.metadata.root_scale;
            let root_transform =
                Transform { translation: Vec3::ZERO, rotation: scene.metadata.root_rotation, scale: Vec3::new(root_scale, root_scale, root_scale) };
            world_to_units = transform_to_matrix(&root_transform);
            translation_scale = scene.metadata.geometry_scale;
        }

        for cluster in &mut scene.skin_clusters {
            cluster.bind_to_world = matrix_mul(&world_to_units, &cluster.bind_to_world);
            scale_translation(&mut cluster.bind_to_world, translation_scale);
            mirror_matrix(&mut cluster.bind_to_world, mirror_axis);
        }

        for pose in &mut scene.poses {
            for bone in &mut pose.bone_poses {
                bone.bone_to_world = matrix_mul(&world_to_units, &bone.bone_to_world);
                scale_translation(&mut bone.bone_to_world, translation_scale);
                mirror_matrix(&mut bone.bone_to_world, mirror_axis);
            }
        }
    }

    for ci in 0..scene.skin_clusters.len() {
        let cluster_id = scene.skin_clusters[ci].element.element_id;
        let Some(skin) = fetch_src(scene, cluster_id, ElementType::SkinDeformer) else {
            continue;
        };

        let skin_id = scene.skin_deformers[skin].element.element_id;
        let mut node = fetch_src(scene, skin_id, ElementType::Node);
        if node.is_none()
            && let Some(mesh) = fetch_src(scene, skin_id, ElementType::Mesh)
        {
            node = scene.meshes[mesh].element.instances.first().copied();
        }
        let Some(mut node) = node else {
            continue;
        };

        if scene.nodes[node].is_geometry_transform_helper {
            let Some(parent) = scene.nodes[node].parent else {
                continue;
            };
            node = parent;
        }

        let node = &scene.nodes[node];
        let geometry_node_to_parent = node.geometry_transform_helper.map(|g| scene.nodes[g].node_to_parent);
        let cluster = &mut scene.skin_clusters[ci];
        if matrix_all_zero(&cluster.mesh_node_to_bone) {
            let world_to_bind = matrix_invert(&cluster.bind_to_world);
            cluster.mesh_node_to_bone = matrix_mul(&world_to_bind, &node.node_to_world);
        } else {
            mirror_matrix(&mut cluster.mesh_node_to_bone, mirror_axis);
            if geometry_scale != 1.0 {
                scale_translation(&mut cluster.mesh_node_to_bone, geometry_scale);
            }
        }

        // ufbx's own HACK: geometry transforms through the helper node's transform, if there is one
        if let Some(geo_node_to_parent) = geometry_node_to_parent {
            cluster.geometry_to_bone = matrix_mul(&cluster.mesh_node_to_bone, &geo_node_to_parent);
        } else if node.has_geometry_transform {
            cluster.geometry_to_bone = matrix_mul(&cluster.mesh_node_to_bone, &node.geometry_to_node);
        } else {
            cluster.geometry_to_bone = cluster.mesh_node_to_bone;
        }
    }
}

fn find_axis(props: &Props, axis_name: &str, sign_name: &str) -> CoordinateAxis {
    let axis = props.int(axis_name, 3);
    let sign = props.int(sign_name, 2);

    match axis {
        0 => {
            if sign > 0 {
                CoordinateAxis::PositiveX
            } else {
                CoordinateAxis::NegativeX
            }
        }
        1 => {
            if sign > 0 {
                CoordinateAxis::PositiveY
            } else {
                CoordinateAxis::NegativeY
            }
        }
        2 => {
            if sign > 0 {
                CoordinateAxis::PositiveZ
            } else {
                CoordinateAxis::NegativeZ
            }
        }
        _ => CoordinateAxis::Unknown,
    }
}

/// By `TimeMode`; ufbx writes them as floats
const TIME_MODE_FPS: [Real; 18] = [
    30.0,
    120.0,
    100.0,
    60.0,
    50.0,
    48.0,
    30.0,
    30.0,
    29.97f32 as Real,
    29.97f32 as Real,
    25.0,
    24.0,
    1000.0,
    23.976f32 as Real,
    24.0,
    96.0,
    72.0,
    59.94f32 as Real,
];

/// `None` where the axes are the same; both must be valid
pub(crate) fn axis_matrix(src: CoordinateAxes, dst: CoordinateAxes) -> Option<Matrix> {
    let (src_x, dst_x) = (src.right as u32, dst.right as u32);
    let (src_y, dst_y) = (src.up as u32, dst.up as u32);
    let (src_z, dst_z) = (src.front as u32, dst.front as u32);

    if src_x == dst_x && src_y == dst_y && src_z == dst_z {
        return None;
    }

    // the axis is the enum halved, its sign the parity
    let mut cols = [Vec3::ZERO; 4];
    let sign = |a: u32, b: u32| if (a ^ b) & 1 == 0 { 1.0 } else { -1.0 };
    cols[(src_x >> 1) as usize][(dst_x >> 1) as usize] = sign(src_x, dst_x);
    cols[(src_y >> 1) as usize][(dst_y >> 1) as usize] = sign(src_y, dst_y);
    cols[(src_z >> 1) as usize][(dst_z >> 1) as usize] = sign(src_z, dst_z);

    Some(Matrix::from_cols(cols))
}

pub(crate) fn update_adjust_transforms(ctx: &mut Context) {
    adjust_transforms(&mut ctx.scene, &ctx.opts, &ctx.axis_matrix, ctx.unit_scale);
}

/// `ufbxi_update_adjust_transforms` in ufbx.c
fn adjust_transforms(scene: &mut Scene, opts: &LoadOpts, axis_mat: &Matrix, unit_scale: Real) {
    let mut root_transform = Transform::IDENTITY;
    if !matrix_all_zero(axis_mat) {
        root_transform = matrix_to_transform(axis_mat);
    }
    root_transform.scale.x *= unit_scale;
    root_transform.scale.y *= unit_scale;
    root_transform.scale.z *= unit_scale;

    let conversion = opts.space_conversion;

    let mut light_post_rotation = Quat::IDENTITY;
    let mut camera_post_rotation = Quat::IDENTITY;
    let mut light_direction = Vec3::new(0.0, -1.0, 0.0);
    let mut has_light_transform = false;
    let mut has_camera_transform = false;

    if coordinate_axes_valid(opts.target_light_axes) {
        let light_axes = CoordinateAxes { right: CoordinateAxis::PositiveX, up: CoordinateAxis::NegativeZ, front: CoordinateAxis::PositiveY };
        if let Some(mat) = axis_matrix(opts.target_light_axes, light_axes) {
            light_post_rotation = matrix_to_transform(&mat).rotation;

            let inv = matrix_invert(&mat);
            light_direction = transform_direction(&inv, light_direction);
            has_light_transform = true;
        }
    }

    if coordinate_axes_valid(opts.target_camera_axes) {
        let camera_axes = CoordinateAxes { right: CoordinateAxis::PositiveZ, up: CoordinateAxis::PositiveY, front: CoordinateAxis::NegativeX };
        if let Some(mat) = axis_matrix(opts.target_camera_axes, camera_axes) {
            camera_post_rotation = matrix_to_transform(&mat).rotation;
            has_camera_transform = true;
        }
    }

    for light in &mut scene.lights {
        light.local_direction = Vec3::new(0.0, -1.0, 0.0);
    }

    let metadata = &mut scene.metadata;
    metadata.space_conversion = conversion;
    metadata.geometry_transform_handling = opts.geometry_transform_handling;
    metadata.inherit_mode_handling = opts.inherit_mode_handling;
    metadata.pivot_handling = opts.pivot_handling;
    metadata.handedness_conversion_axis = opts.handedness_conversion_axis;

    let root_scale = min3(root_transform.scale);
    if conversion == SpaceConversion::ModifyGeometry {
        metadata.geometry_scale = root_scale;
        metadata.root_scale = 1.0;
    } else {
        metadata.geometry_scale = 1.0;
        metadata.root_scale = root_scale;
    }
    metadata.root_rotation = root_transform.rotation;

    for ix in 0..scene.nodes.len() {
        let node = &mut scene.nodes[ix];

        node.adjust_post_rotation = Quat::IDENTITY;
        node.adjust_pre_rotation = Quat::IDENTITY;
        node.adjust_pre_scale = 1.0;
        node.adjust_post_scale = 1.0;
        node.adjust_translation_scale = 1.0;

        if conversion == SpaceConversion::AdjustTransforms {
            if node.node_depth <= 1 && !node.is_root {
                node.adjust_pre_rotation = root_transform.rotation;
                node.adjust_pre_scale = root_scale;
                node.has_adjust_transform = true;
                node.has_root_adjust_transform = true;
            }
        } else if conversion == SpaceConversion::ModifyGeometry && !node.is_root {
            if node.node_depth <= 1 {
                node.adjust_pre_rotation = root_transform.rotation;
            }
            node.adjust_translation_scale = root_scale;
            node.has_adjust_transform = true;
        }

        if let Some(p) = node.parent {
            // the root scale is not inherited through local scale, so it is propagated here, with scale compensation
            let parent = &scene.nodes[p];
            let parent_root_adjust = parent.has_root_adjust_transform;
            let compensate_scale = parent.is_scale_compensate_parent.then(|| parent.element.props.vec3("Lcl Scaling", Vec3::ONE));
            let node = &mut scene.nodes[ix];
            if parent_root_adjust && node.inherit_mode == InheritMode::IgnoreParentScale {
                node.adjust_post_scale *= root_scale;
                node.has_adjust_transform = true;
                node.has_root_adjust_transform = true;
            }
            if let Some(scale) = compensate_scale
                && node.original_inherit_mode == InheritMode::IgnoreParentScale
            {
                let mut size = scale.x;
                if (scale.y - 1.0).abs() < (size - 1.0).abs() {
                    size = scale.y;
                }
                if (scale.z - 1.0).abs() < (size - 1.0).abs() {
                    size = scale.z;
                }
                node.adjust_post_scale *= 1.0 / size;
                node.has_adjust_transform = true;
            }
        }

        let node = &mut scene.nodes[ix];
        if node.all_attribs.len() == 1 {
            if has_light_transform && let Some(light) = node.light {
                node.adjust_post_rotation = light_post_rotation;
                scene.lights[light].local_direction = light_direction;
                node.has_adjust_transform = true;
            }
            if has_camera_transform && let Some(camera) = node.camera {
                node.adjust_post_rotation = camera_post_rotation;
                scene.cameras[camera].projection_axes = opts.target_camera_axes;
                node.has_adjust_transform = true;
            }
        }
    }
}

pub(crate) fn update_scene(scene: &mut Scene, initial: bool, overrides: &[TransformOverride]) {
    for ix in 0..scene.nodes.len() {
        update_node(scene, ix, overrides);
    }

    for light in &mut scene.lights {
        update_light(light);
    }

    for camera in &mut scene.cameras {
        update_camera(&scene.metadata, camera);
    }

    for bone in &mut scene.bones {
        update_bone(&scene.metadata, bone);
    }

    for line in &mut scene.line_curves {
        update_line_curve(line);
    }

    if initial {
        update_initial_clusters(scene);

        for pose in &mut scene.poses {
            update_pose(&scene.nodes, pose);
        }
    }

    for cluster in &mut scene.skin_clusters {
        update_skin_cluster(&scene.nodes, cluster);
    }

    for channel in &mut scene.blend_channels {
        update_blend_channel(channel);
    }

    for ix in 0..scene.textures.len() {
        update_texture(scene, ix);
    }

    crate::textures::propagate_main_textures(scene);

    for ix in 0..scene.materials.len() {
        update_material(scene, ix);
    }

    for stack in &mut scene.anim_stacks {
        update_anim_stack(&scene.metadata, stack);
    }

    for layer in &mut scene.display_layers {
        update_display_layer(layer);
    }

    for constraint in &mut scene.constraints {
        update_constraint(&scene.nodes, constraint);
    }

    update_anim(scene);
}

pub(crate) fn update_scene_metadata(metadata: &mut Metadata) {
    let props = &metadata.scene_props;
    let find = |name: &str| props.find_string(name, "").to_string();
    metadata.original_application =
        Application { vendor: find("Original|ApplicationVendor"), name: find("Original|ApplicationName"), version: find("Original|ApplicationVersion") };
    metadata.latest_application =
        Application { vendor: find("LastSaved|ApplicationVendor"), name: find("LastSaved|ApplicationName"), version: find("LastSaved|ApplicationVersion") };
}

pub(crate) const POW10_TARGETS: [Real; 19] = [0.0, 1e-8, 1e-7, 1e-6, 1e-5, 1e-4, 1e-3, 1e-2, 1e-1, 1e+0, 1e+1, 1e+2, 1e+3, 1e+4, 1e+5, 1e+6, 1e+7, 1e+8, 1e+9];

/// The first target within 2^-20 of the value relative (7.5e-37 at least), else the value
pub(crate) fn round_if_near(targets: &[Real], value: Real) -> Real {
    for &target in targets {
        let mut error = target * 9.5367431640625e-7;
        if error < 0.0 {
            error = -error;
        }
        if error < 7.52316384526264005e-37 {
            error = 7.52316384526264005e-37;
        }
        if value >= target - error && value <= target + error {
            return target;
        }
    }
    value
}

pub(crate) fn update_scene_settings(settings: &mut SceneSettings) {
    let props = &settings.props;
    let unit_scale_factor = props.real("UnitScaleFactor", 1.0);
    let original_unit_scale_factor = props.real("OriginalUnitScaleFactor", unit_scale_factor);

    settings.axes.up = find_axis(props, "UpAxis", "UpAxisSign");
    settings.axes.front = find_axis(props, "FrontAxis", "FrontAxisSign");
    settings.axes.right = find_axis(props, "CoordAxis", "CoordAxisSign");
    settings.unit_meters = round_if_near(&POW10_TARGETS, unit_scale_factor * 0.01);
    settings.original_unit_meters = round_if_near(&POW10_TARGETS, original_unit_scale_factor * 0.01);
    settings.frames_per_second = props.real("CustomFrameRate", 24.0);
    settings.ambient_color = props.vec3("AmbientColor", Vec3::ZERO);
    settings.original_axis_up = find_axis(props, "OriginalUpAxis", "OriginalUpAxisSign");

    settings.default_camera = props.find_prop("DefaultCamera").map_or_else(String::new, |p| p.value_str.clone());

    settings.time_mode = find_enum(props, "TimeMode", TimeMode::Fps24 as i64, &TIME_MODES);
    settings.time_protocol = find_enum(props, "TimeProtocol", TimeProtocol::Default as i64, &TIME_PROTOCOLS);
    settings.snap_mode = find_enum(props, "SnapOnFrameMode", SnapMode::None as i64, &SNAP_MODES);

    if settings.time_mode != TimeMode::Custom {
        settings.frames_per_second = TIME_MODE_FPS[settings.time_mode as usize];
    }
}

pub(crate) fn update_scene_settings_obj(ctx: &mut Context) {
    scene_settings_obj(&mut ctx.scene.settings, &ctx.opts);
}

/// `ufbxi_update_scene_settings_obj` in ufbx.c
fn scene_settings_obj(settings: &mut SceneSettings, opts: &LoadOpts) {
    settings.unit_meters = opts.obj_unit_meters;
    settings.original_unit_meters = opts.obj_unit_meters;
    if coordinate_axes_valid(opts.obj_axes) {
        settings.axes = opts.obj_axes;
    } else {
        settings.axes = CoordinateAxes::UNKNOWN;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::props::{name_key, sort_properties};

    enum V<'a> {
        N(&'a [Real]),
        Str(&'a str),
    }
    use V::{N, Str};

    fn props(list: &[(&str, V)]) -> Props {
        let mut props: Vec<Prop> = list
            .iter()
            .map(|(name, v)| {
                let mut p = Prop { name: name.to_string(), internal_key: name_key(name.as_bytes()), ..Default::default() };
                match v {
                    N(xs) => {
                        p.value_real_arr[..xs.len()].copy_from_slice(xs);
                        p.value_int = xs[0] as i64;
                    }
                    Str(s) => p.value_str = s.to_string(),
                }
                p
            })
            .collect();
        sort_properties(&mut props);
        Props { props, ..Default::default() }
    }

    fn bits(xs: &[Real]) -> Vec<u64> {
        xs.iter().map(|x| x.to_bits()).collect()
    }

    fn transform_bits(t: &Transform) -> Vec<u64> {
        let (p, q, s) = (t.translation, t.rotation, t.scale);
        bits(&[p.x, p.y, p.z, q.x, q.y, q.z, q.w, s.x, s.y, s.z])
    }

    /// (name, parent, inherit mode, inherit-scale node, props, local_transform, node_to_world, geometry_to_world,
    /// use_rotation_space, visible)
    type NodeCase = (&'static str, &'static str, usize, &'static str, &'static [(&'static str, V<'static>)], [u64; 10], [u64; 12], [u64; 12], bool, bool);
    /// (name, props, [resolution, fov deg, fov tan, ortho extent, ortho size, projection plane, aspect, near, far, focal
    /// length, film size, aperture size, squeeze], resolution_is_pixels)
    type CameraCase = (&'static str, &'static [(&'static str, V<'static>)], [u64; 20], bool);
    /// (props, [right, up, front, original up], [unit meters, original unit meters, fps, ambient], default camera,
    /// [time mode, time protocol, snap mode])
    type SettingsCase = (&'static [(&'static str, V<'static>)], [i64; 4], [u64; 6], &'static str, [i64; 3]);

    // ufbx 0.23.1's results, its C built with `-ffp-contract=off`, for these properties loaded from ASCII FBX files;
    // the nodes in ufbx's order, parents first
    const NODES: &[NodeCase] = &[
        (
            "Pivots",
            "",
            0,
            "",
            &[
                ("RotationOrder", N(&[4.0])),
                ("RotationActive", N(&[1.0])),
                ("InheritType", N(&[1.0])),
                ("Lcl Translation", N(&[1.0, -2.0, 3.5])),
                ("Lcl Rotation", N(&[30.0, 45.0, -60.0])),
                ("Lcl Scaling", N(&[2.0, 0.5, 1.5])),
                ("PreRotation", N(&[10.0, 20.0, 30.0])),
                ("PostRotation", N(&[-15.0, 5.0, 25.0])),
                ("RotationPivot", N(&[0.25, 0.5, -0.75])),
                ("ScalingPivot", N(&[-1.0, 0.5, 2.0])),
                ("RotationOffset", N(&[0.125, -0.5, 1.0])),
                ("ScalingOffset", N(&[0.5, 0.25, -0.125])),
            ],
            [
                0x3ff01772fbb0d2dc,
                0xc00836490829dc6c,
                0x4008451346b7bf69,
                0xbfc78f5eeac66c28,
                0x3fde3c50871c9e60,
                0xbfe1868a0426c593,
                0x3fe54c59acbfb2d9,
                0x4000000000000000,
                0x3fe0000000000000,
                0x3ff8000000000000,
            ],
            [
                0xbfb7b14eb4bf1ec0,
                0xbffce4eab49cf3ed,
                0xbfeb58393e41969d,
                0x3fd1c38228a03c7f,
                0x3fc5453822c8d17e,
                0xbfd8665029bc356a,
                0x3ff3ee7fc4fed7c8,
                0xbfda280a08e5f9da,
                0x3fe7523c78e010ea,
                0x3ff01772fbb0d2dc,
                0xc00836490829dc6c,
                0x4008451346b7bf69,
            ],
            [
                0xbfb7b14eb4bf1ec0,
                0xbffce4eab49cf3ed,
                0xbfeb58393e41969d,
                0x3fd1c38228a03c7f,
                0x3fc5453822c8d17e,
                0xbfd8665029bc356a,
                0x3ff3ee7fc4fed7c8,
                0xbfda280a08e5f9da,
                0x3fe7523c78e010ea,
                0x3ff01772fbb0d2dc,
                0xc00836490829dc6c,
                0x4008451346b7bf69,
            ],
            true,
            true,
        ),
        (
            "LimitOnly",
            "",
            0,
            "",
            &[
                ("RotationOrder", N(&[3.0])),
                ("RotationActive", N(&[1.0])),
                ("RotationSpaceForLimitOnly", N(&[1.0])),
                ("Lcl Rotation", N(&[5.0, 10.0, 15.0])),
                ("PreRotation", N(&[0.0, 0.0, 45.0])),
                ("Visibility", N(&[0.0])),
            ],
            [
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
                0x3fa03d20e042f041,
                0x3fb78d4a74a8aed5,
                0x3fc0253e5e143da6,
                0x3fef975fc76ce6a2,
                0x3ff0000000000000,
                0x3ff0000000000000,
                0x3ff0000000000000,
            ],
            [
                0x3fee70a67222f685,
                0x3fd0501192b41c49,
                0xbfc63a1a7e0b7389,
                0xbfcf21aecd14c931,
                0x3feeead79ddb9d51,
                0x3fb5f91026c4cc9f,
                0x3fc846782f20070f,
                0xbfa42e034eb6bf7e,
                0x3fef64d87db06a5b,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                0x3fee70a67222f685,
                0x3fd0501192b41c49,
                0xbfc63a1a7e0b7389,
                0xbfcf21aecd14c931,
                0x3feeead79ddb9d51,
                0x3fb5f91026c4cc9f,
                0x3fc846782f20070f,
                0xbfa42e034eb6bf7e,
                0x3fef64d87db06a5b,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            false,
            false,
        ),
        (
            "Child",
            "Pivots",
            2,
            "Pivots",
            &[
                ("RotationOrder", N(&[2.0])),
                ("RotationActive", N(&[1.0])),
                ("InheritType", N(&[0.0])),
                ("Lcl Translation", N(&[0.5, 1.0, -1.0])),
                ("Lcl Rotation", N(&[-20.0, 70.0, 15.0])),
                ("Lcl Scaling", N(&[1.0, 3.0, 0.25])),
                ("PreRotation", N(&[0.0, 90.0, 0.0])),
                ("PostRotation", N(&[45.0, 0.0, 0.0])),
                ("RotationPivot", N(&[1.0, 0.0, 0.0])),
                ("ScalingPivot", N(&[0.0, 1.0, 0.0])),
                ("GeometricTranslation", N(&[0.0, 0.0, 1.0])),
                ("GeometricRotation", N(&[0.0, 0.0, 90.0])),
                ("GeometricScaling", N(&[2.0, 2.0, 2.0])),
            ],
            [
                0x4009bd35c0f4175c,
                0x3fdc5d815175b742,
                0xc0028deee4456bdf,
                0xbfc89f3e5e4f01b2,
                0x3feaa09f43cfaf93,
                0x3fe07317b0ed8396,
                0x3fb465c72a9f2748,
                0x3ff0000000000000,
                0x4008000000000000,
                0x3fd0000000000000,
            ],
            [
                0xbfe75148aae92f06,
                0x3ffabc6ae115dce0,
                0x3fea535ad24585e4,
                0x3ff62ec1b6736eba,
                0x3fd9f54698037cda,
                0x3fd9ddc165c8d767,
                0x3fa5de0d77cfb15c,
                0x3fc6f60b63e5cc76,
                0xbfd4e5e17e86104b,
                0xc00077499027fe7d,
                0xc01f42c73e678cf8,
                0xbff9329019eb72d2,
            ],
            [
                0x40062ec1b6736eb9,
                0x3fe9f54698037ce1,
                0x3fe9ddc165c8d76a,
                0x3ff75148aae92f09,
                0xc00abc6ae115dce0,
                0xbffa535ad24585e3,
                0x3fb5de0d77cfb15c,
                0x3fd6f60b63e5cc76,
                0xbfe4e5e17e86104b,
                0xc0001fd15a48bfb8,
                0xc01e8b16e3485e94,
                0xbffe6c08798cf6e5,
            ],
            true,
            true,
        ),
        (
            "Grand",
            "Child",
            1,
            "Pivots",
            &[
                ("RotationOrder", N(&[5.0])),
                ("RotationActive", N(&[0.0])),
                ("InheritType", N(&[2.0])),
                ("Lcl Translation", N(&[1.0, 1.0, 1.0])),
                ("Lcl Rotation", N(&[10.0, 20.0, 30.0])),
                ("Lcl Scaling", N(&[1.0, 2.0, 1.0])),
                ("PreRotation", N(&[30.0, 0.0, 0.0])),
            ],
            [
                0x3ff0000000000000,
                0x3ff0000000000000,
                0x3ff0000000000000,
                0x3fa386600d9cd25a,
                0x3fc83b3d683aed72,
                0x3fcea153f34ce654,
                0x3fee7315e43cde2e,
                0x3ff0000000000000,
                0x4000000000000000,
                0x3ff0000000000000,
            ],
            [
                0x3fc94c2bc0de3610,
                0x3ff4967365f6eb59,
                0x3ff84afe7e466a9e,
                0x3fefd6dec188bf36,
                0xbfaa7c85a22c4afc,
                0xbfb5eec53c32e812,
                0xbf9857b522155d30,
                0x3ff254d81e0f44e2,
                0xbfeef8ccdb38b2c6,
                0xbff5b9855392a838,
                0xc0163ca7c182af8f,
                0xbfe595d56deffc32,
            ],
            [
                0x3fc94c2bc0de3610,
                0x3ff4967365f6eb59,
                0x3ff84afe7e466a9e,
                0x3fefd6dec188bf36,
                0xbfaa7c85a22c4afc,
                0xbfb5eec53c32e812,
                0xbf9857b522155d30,
                0x3ff254d81e0f44e2,
                0xbfeef8ccdb38b2c6,
                0xbff5b9855392a838,
                0xc0163ca7c182af8f,
                0xbfe595d56deffc32,
            ],
            false,
            true,
        ),
    ];

    const ORDERS: &[(i64, [u64; 4])] = &[
        (0, [0x3fdbe7e6307f17e0, 0x3fc31239482441df, 0x3fea42da52803779, 0x3fd5a09e13189310]),
        (1, [0xbfce4492f512b839, 0x3f912d27a46d2320, 0x3fec0d97822ce64d, 0x3fdacb362c497a63]),
        (2, [0x3fe0a643a599f87f, 0xbfd1b3cf0518c831, 0x3fe59ae7e480a389, 0x3fdc66f6872a0ff0]),
        (3, [0x3fd882284ce14684, 0x3fc71e9c4a6108a4, 0x3fe6cd0abd396bc0, 0x3fe1e75ba5bd5b32]),
        (4, [0xbfb95fa9f0dac688, 0xbfdc304ab0027a52, 0x3feadb74a9741e15, 0x3fd3637567f8d3f4]),
        (5, [0xbfc37b50bfa905fd, 0xbfda2a192ee416f0, 0x3fe765a5142d525c, 0x3fe0c8c7502d7ba2]),
    ];

    const CAMERAS: &[CameraCase] = &[
        (
            "VerticalFill",
            &[
                ("FieldOfView", N(&[40.0])),
                ("AspectRatioMode", N(&[2.0])),
                ("AspectWidth", N(&[1920.0])),
                ("AspectHeight", N(&[1080.0])),
                ("GateFit", N(&[3.0])),
                ("FilmWidth", N(&[1.417])),
                ("FilmHeight", N(&[0.945])),
                ("NearPlane", N(&[0.5])),
                ("FarPlane", N(&[2500.0])),
            ],
            [
                0x409e000000000000,
                0x4090e00000000000,
                0x405073de2ad59d2a,
                0x4044000000000000,
                0x3fe4b4b362dd57d9,
                0x3fd74b49cf3902d4,
                0x403e000000000000,
                0x403e000000000000,
                0x4030e00000000000,
                0x3fe4b4b362dd57d9,
                0x3fd74b49cf3902d4,
                0x3ffc71c71c71c71c,
                0x3fe0000000000000,
                0x40a3880000000000,
                0x0000000000000000,
                0x3ff6ac083126e979,
                0x3fee3d70a3d70a3d,
                0x3ff6ac083126e979,
                0x3fe98189374bc6a9,
                0x3ff0000000000000,
            ],
            true,
        ),
        (
            "HorizontalOverscan",
            &[
                ("ApertureMode", N(&[1.0])),
                ("FieldOfView", N(&[75.0])),
                ("AspectRatioMode", N(&[1.0])),
                ("AspectW", N(&[2.0])),
                ("AspectH", N(&[1.0])),
                ("GateFit", N(&[4.0])),
            ],
            [
                0x4000000000000000,
                0x3ff0000000000000,
                0x4052c00000000000,
                0x4044fd6fa5414241,
                0x3fe88df153d6a676,
                0x3fd88df153d6a676,
                0x403e000000000000,
                0x404e000000000000,
                0x403e000000000000,
                0x3fe88df153d6a676,
                0x3fd88df153d6a676,
                0x4000000000000000,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
                0x3ff0000000000000,
                0x3ff0000000000000,
                0x4000000000000000,
                0x3ff0000000000000,
                0x3ff0000000000000,
            ],
            false,
        ),
        (
            "BothStretch",
            &[
                ("ApertureMode", N(&[0.0])),
                ("FieldOfViewX", N(&[60.0])),
                ("FieldOfViewY", N(&[45.0])),
                ("AspectRatioMode", N(&[0.0])),
                ("AspectWidth", N(&[640.0])),
                ("AspectHeight", N(&[480.0])),
                ("GateFit", N(&[5.0])),
            ],
            [
                0x4084000000000000,
                0x407e000000000000,
                0x404e000000000000,
                0x4046800000000000,
                0x3fe279a74590331c,
                0x3fda827999fcef32,
                0x403e000000000000,
                0x403e000000000000,
                0x403e000000000000,
                0x3fe279a74590331c,
                0x3fda827999fcef32,
                0x3ff5555555555555,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
                0x3ff0000000000000,
                0x3ff0000000000000,
                0x3ff0000000000000,
                0x3ff0000000000000,
                0x3ff0000000000000,
            ],
            false,
        ),
        (
            "FocalAnamorphicOrtho",
            &[
                ("ApertureMode", N(&[3.0])),
                ("FocalLength", N(&[35.0])),
                ("ApertureFormat", N(&[7.0])),
                ("AspectRatioMode", N(&[3.0])),
                ("AspectW", N(&[1.5])),
                ("AspectH", N(&[0.5])),
                ("GateFit", N(&[2.0])),
                ("CameraProjectionType", N(&[1.0])),
                ("OrthoZoom", N(&[2.0])),
            ],
            [
                0x3ff8000000000000,
                0x3fe8000000000000,
                0x4041681a8ea1d94f,
                0x4031d15656ad399f,
                0x3fd4108641ab3079,
                0x3fc4108641ab3079,
                0x404e000000000000,
                0x404e000000000000,
                0x403e000000000000,
                0x404e000000000000,
                0x403e000000000000,
                0x4000000000000000,
                0x0000000000000000,
                0x0000000000000000,
                0x4041800000000000,
                0x3feba5e353f7ced9,
                0x3ff76c8b43958106,
                0x3feba5e353f7ced9,
                0x3fdba5e353f7ced9,
                0x4000000000000000,
            ],
            true,
        ),
        (
            "FixedHeightOnly",
            &[
                ("FieldOfView", N(&[30.0])),
                ("AspectRatioMode", N(&[4.0])),
                ("AspectHeight", N(&[720.0])),
                ("FilmWidth", N(&[0.98])),
                ("FilmHeight", N(&[0.735])),
                ("FilmSqueezeRatio", N(&[1.25])),
                ("GateFit", N(&[0.0])),
            ],
            [
                0x4125180000000000,
                0x4086800000000000,
                0x406671bea3dc1309,
                0x403e000000000000,
                0x407013b318b4e081,
                0x3fd126145e9ecd56,
                0x403e000000000000,
                0x403e000000000000,
                0x403e000000000000,
                0x407013b318b4e081,
                0x3fd126145e9ecd56,
                0x408e000000000000,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
                0x3fef5c28f5c28f5c,
                0x3fed666666666666,
                0x3fef5c28f5c28f5c,
                0x3fed666666666666,
                0x3ff4000000000000,
            ],
            true,
        ),
        (
            "FromFilmImaxVertical",
            &[("ApertureMode", N(&[3.0])), ("FocalLength", N(&[50.0])), ("ApertureFormat", N(&[11.0])), ("GateFit", N(&[1.0]))],
            [
                0x40062d0e56041894,
                0x40009374bc6a7efa,
                0x4051930fb2ec2166,
                0x404bc1de013dc729,
                0x3fe687e38f12e5e6,
                0x3fe0d759eed2a6ac,
                0x403e000000000000,
                0x4044114c1bacf915,
                0x403e000000000000,
                0x3fe687e38f12e5e6,
                0x3fe0d759eed2a6ac,
                0x3ff567c8a60dd67d,
                0x0000000000000000,
                0x0000000000000000,
                0x4049000000000000,
                0x40062d0e56041894,
                0x40009374bc6a7efa,
                0x40062d0e56041894,
                0x40009374bc6a7efa,
                0x3ff0000000000000,
            ],
            false,
        ),
    ];

    const SETTINGS: &[SettingsCase] = &[
        (
            &[
                ("UpAxis", N(&[2.0])),
                ("UpAxisSign", N(&[-1.0])),
                ("FrontAxis", N(&[1.0])),
                ("FrontAxisSign", N(&[1.0])),
                ("CoordAxis", N(&[0.0])),
                ("CoordAxisSign", N(&[1.0])),
                ("OriginalUpAxis", N(&[2.0])),
                ("OriginalUpAxisSign", N(&[1.0])),
                ("UnitScaleFactor", N(&[2.54])),
                ("OriginalUnitScaleFactor", N(&[100.00001])),
                ("AmbientColor", N(&[0.1, 0.2, 0.3])),
                ("DefaultCamera", Str("Producer Top")),
                ("TimeMode", N(&[9.0])),
                ("TimeProtocol", N(&[1.0])),
                ("SnapOnFrameMode", N(&[2.0])),
                ("CustomFrameRate", N(&[12.0])),
            ],
            [0, 5, 2, 4],
            [0x3f9a027525460aa7, 0x3ff0000000000000, 0x403df851e0000000, 0x3fb999999999999a, 0x3fc999999999999a, 0x3fd3333333333333],
            "Producer Top",
            [9, 1, 2],
        ),
        (
            &[
                ("UpAxis", N(&[5.0])),
                ("FrontAxis", N(&[2.0])),
                ("FrontAxisSign", N(&[-1.0])),
                ("UnitScaleFactor", N(&[0.0999999])),
                ("TimeMode", N(&[14.0])),
                ("CustomFrameRate", N(&[12.5])),
                ("SnapOnFrameMode", N(&[7.0])),
            ],
            [6, 6, 5, 6],
            [0x3f50624cc010eb7a, 0x3f50624cc010eb7a, 0x4029000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000],
            "",
            [14, 2, 0],
        ),
        (
            &[("TimeMode", N(&[99.0])), ("UnitScaleFactor", N(&[10000000.0]))],
            [6, 6, 6, 6],
            [0x40f86a0000000000, 0x40f86a0000000000, 0x4038000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000],
            "",
            [11, 2, 0],
        ),
        (
            &[("TimeMode", N(&[17.0])), ("UnitScaleFactor", N(&[100.0001]))],
            [6, 6, 6, 6],
            [0x3ff000010c6f7a0c, 0x3ff000010c6f7a0c, 0x404df851e0000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000],
            "",
            [17, 2, 0],
        ),
    ];

    #[test]
    fn node_transforms_match_ufbx_through_pivots_offsets_pre_and_post_rotation_and_inherit_modes() {
        let mut scene = Scene::default();
        scene.nodes.push(Node { is_root: true, local_transform: Transform::IDENTITY, node_to_parent: Matrix::IDENTITY, ..Default::default() });
        for (name, _, inherit, _, p, ..) in NODES {
            let mut node = Node::default();
            node.element.name = name.to_string();
            node.element.typed_id = scene.nodes.len() as u32;
            node.element.props = props(p);
            node.inherit_mode = [InheritMode::Normal, InheritMode::IgnoreParentScale, InheritMode::ComponentwiseScale][*inherit];
            scene.nodes.push(node);
        }
        let find = |name: &str| NODES.iter().position(|n| n.0 == name).map(|i| i + 1);
        for (i, (_, parent, _, scale_node, ..)) in NODES.iter().enumerate() {
            scene.nodes[i + 1].parent = Some(find(parent).unwrap_or(0));
            scene.nodes[i + 1].inherit_scale_node = find(scale_node);
        }

        for ix in 0..scene.nodes.len() {
            update_node(&mut scene, ix, &[]);
        }

        for (i, (name, _, _, _, p, local, world, geometry_world, rotation_space, visible)) in NODES.iter().enumerate() {
            let node = &scene.nodes[i + 1];
            assert_eq!(transform_bits(&node.local_transform), local, "{name} local_transform");
            assert_eq!(bits(&node.node_to_world.v()), world, "{name} node_to_world");
            assert_eq!(bits(&node.geometry_to_world.v()), geometry_world, "{name} geometry_to_world");
            assert_eq!(node.use_rotation_space, *rotation_space, "{name}");
            assert_eq!(node.visible, *visible, "{name}");
            assert_eq!(node.node_to_parent, transform_to_matrix(&node.local_transform), "{name}");
            let order = node.rotation_order;
            assert_eq!(get_rotation(&props(p), order, node), node.local_transform.rotation, "{name} get_rotation");
            assert_eq!(get_scale(&props(p), node), node.local_transform.scale, "{name} get_scale");
        }
        assert!(scene.nodes[NODES.iter().position(|n| n.0 == "Child").unwrap() + 1].has_geometry_transform);
    }

    #[test]
    fn every_rotation_order_matches_ufbx_with_pre_and_post_rotation() {
        let node = Node { use_rotation_space: true, ..Default::default() };
        for (order, rotation) in ORDERS {
            let p = props(&[
                ("RotationOrder", N(&[*order as Real])),
                ("Lcl Translation", N(&[-3.0, 0.5, 2.0])),
                ("Lcl Rotation", N(&[35.0, -50.0, 110.0])),
                ("PreRotation", N(&[-8.0, 14.0, 22.0])),
                ("PostRotation", N(&[12.0, -7.0, 3.0])),
            ]);
            let t = get_transform(&p, RotationOrder::from_index(*order), &node, None);
            let q = t.rotation;
            assert_eq!(bits(&[q.x, q.y, q.z, q.w]), rotation, "order {order}");
            assert_eq!(t.translation, Vec3::new(-3.0, 0.5, 2.0));
            assert_eq!(t.scale, Vec3::ONE);

            let scaled = get_transform(&p, RotationOrder::from_index(*order), &node, Some(&Vec3::new(2.0, 4.0, -1.0)));
            assert_eq!(scaled.translation, Vec3::new(-6.0, 2.0, -2.0));
        }
    }

    #[test]
    fn camera_field_of_view_and_aperture_match_ufbx_in_every_aperture_mode_and_gate_fit() {
        let metadata = Metadata { ortho_size_unit: 30.0, geometry_scale: 1.0, ..Default::default() };
        for (name, p, expected, pixels) in CAMERAS {
            let mut c = Camera { element: Element { props: props(p), ..Default::default() }, ..Default::default() };
            update_camera(&metadata, &mut c);
            let got = bits(&[
                c.resolution.x,
                c.resolution.y,
                c.field_of_view_deg.x,
                c.field_of_view_deg.y,
                c.field_of_view_tan.x,
                c.field_of_view_tan.y,
                c.orthographic_extent,
                c.orthographic_size.x,
                c.orthographic_size.y,
                c.projection_plane.x,
                c.projection_plane.y,
                c.aspect_ratio,
                c.near_plane,
                c.far_plane,
                c.focal_length_mm,
                c.film_size_inch.x,
                c.film_size_inch.y,
                c.aperture_size_inch.x,
                c.aperture_size_inch.y,
                c.squeeze_ratio,
            ]);
            assert_eq!(got, expected, "{name}");
            assert_eq!(c.resolution_is_pixels, *pixels, "{name}");
        }
    }

    #[test]
    fn scene_settings_match_ufbx() {
        for (p, axes, reals, camera, modes) in SETTINGS {
            let mut s = SceneSettings { props: props(p), ..Default::default() };
            update_scene_settings(&mut s);
            assert_eq!([s.axes.right as i64, s.axes.up as i64, s.axes.front as i64, s.original_axis_up as i64], *axes);
            let a = s.ambient_color;
            assert_eq!(bits(&[s.unit_meters, s.original_unit_meters, s.frames_per_second, a.x, a.y, a.z]), reals);
            assert_eq!(s.default_camera, *camera);
            assert_eq!([s.time_mode as i64, s.time_protocol as i64, s.snap_mode as i64], *modes);
        }

        let mut opts = LoadOpts { obj_unit_meters: 0.25, obj_axes: CoordinateAxes::LEFT_HANDED_Z_UP, ..Default::default() };
        let mut s = SceneSettings::default();
        scene_settings_obj(&mut s, &opts);
        assert_eq!((s.unit_meters, s.original_unit_meters, s.axes), (0.25, 0.25, CoordinateAxes::LEFT_HANDED_Z_UP));
        opts.obj_axes.up = CoordinateAxis::PositiveX;
        scene_settings_obj(&mut s, &opts);
        assert_eq!(s.axes, CoordinateAxes::UNKNOWN);
    }

    #[test]
    fn round_if_near_snaps_within_two_to_the_minus_twenty_of_a_power_of_ten() {
        let r = |v: Real| round_if_near(&POW10_TARGETS, v);
        assert_eq!(r(1.0 + 5e-7), 1.0);
        assert_eq!(r(1.0 - 9e-7), 1.0);
        assert_eq!(r(1.0 + 2e-6), 1.0 + 2e-6);
        assert_eq!(r(0.01 * (1.0 + 9e-7)), 0.01);
        assert_eq!(r(1e9 * (1.0 - 5e-7)), 1e9);
        assert_eq!(r(1e-40), 0.0);
        assert_eq!(r(1e-35), 1e-35);
        assert_eq!(r(0.0254), 0.0254);
        assert_eq!(r(-1.0), -1.0);
        assert_eq!(r(2e9), 2e9);
    }

    #[test]
    fn axis_matrix_matches_ufbx_and_is_none_for_equal_axes() {
        let y_up = CoordinateAxes::RIGHT_HANDED_Y_UP;
        let z_up = CoordinateAxes::RIGHT_HANDED_Z_UP;
        assert_eq!(axis_matrix(y_up, y_up), None);

        let cols = |c: [[Real; 3]; 3]| Matrix::from_cols([Vec3::from_array(c[0]), Vec3::from_array(c[1]), Vec3::from_array(c[2]), Vec3::ZERO]);
        let m = axis_matrix(y_up, z_up).unwrap();
        assert_eq!(bits(&m.v()), bits(&cols([[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]]).v()));
        let m = axis_matrix(z_up, CoordinateAxes::LEFT_HANDED_Y_UP).unwrap();
        assert_eq!(bits(&m.v()), bits(&cols([[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]]).v()));
        assert_eq!(matrix_determinant(&m), -1.0);
    }

    #[test]
    fn a_transform_override_replaces_its_nodes_local_transform_only() {
        let mut scene = Scene::default();
        scene.nodes.push(Node { is_root: true, local_transform: Transform::IDENTITY, node_to_parent: Matrix::IDENTITY, ..Default::default() });
        for i in 1..40 {
            let mut node = Node { parent: Some(0), ..Default::default() };
            node.element.typed_id = i;
            node.element.props = props(&[("Lcl Translation", N(&[i as Real, 0.0, 0.0]))]);
            scene.nodes.push(node);
        }
        let t = Transform { translation: Vec3::new(0.0, 7.0, 0.0), rotation: Quat::new(0.0, 0.0, 1.0, 0.0), scale: Vec3::ONE };
        let overrides: Vec<TransformOverride> = (0..40).step_by(2).map(|id| TransformOverride { node_id: id, transform: t }).collect();
        for ix in 0..scene.nodes.len() {
            update_node(&mut scene, ix, &overrides);
        }
        assert_eq!(scene.nodes[22].local_transform, t);
        assert_eq!(scene.nodes[22].node_to_world, transform_to_matrix(&t));
        assert_eq!(scene.nodes[23].local_transform.translation, Vec3::new(23.0, 0.0, 0.0));
        assert_eq!(scene.nodes[0].local_transform, Transform::IDENTITY);
    }

    #[test]
    fn blend_weights_split_between_the_keyframes_around_the_channel_weight() {
        let channel = |percent: Real, targets: &[Real]| {
            let mut c = BlendChannel { element: Element { props: props(&[("DeformPercent", N(&[percent]))]), ..Default::default() }, ..Default::default() };
            c.keyframes = targets.iter().map(|&target_weight| BlendKeyframe { target_weight, effective_weight: 9.0, ..Default::default() }).collect();
            update_blend_channel(&mut c);
            c.keyframes.iter().map(|k| k.effective_weight).collect::<Vec<_>>()
        };
        assert_eq!(channel(75.0, &[0.5, 1.0]), [0.5, 0.5]);
        assert_eq!(channel(25.0, &[0.5, 1.0]), [0.5, 0.0]);
        assert_eq!(channel(150.0, &[0.5, 1.0]), [-1.0, 2.0]);
        assert_eq!(channel(-50.0, &[-1.0, 1.0]), [0.5, 0.0]);
        assert_eq!(channel(0.0, &[]), [0.0; 0]);
    }
}
