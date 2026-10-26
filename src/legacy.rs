use crate::context::{Context, DomAlias, ElementInfo, TopChildren};
use crate::error::{Res, check, fail};
use crate::math::{max_real, min_real, transform_to_matrix};
use crate::parse::{PArr, PNode, f64_to_i64, parse_double};
use crate::props::{deduplicate_properties, name_key, sort_properties};
use crate::read::{KEY_CLAMP_PROGRESSIVE, KEY_TIME_INDEPENDENT, arr_f64, arr_i32, as_u32, find_f64, find_i32, patch_mesh_reals, read_extrapolation, vec3s};
use crate::types::*;

/// ufbx names a synthetic element by a C string, so the name ends at its first NUL
fn c_str(s: &str) -> &str {
    s.split('\0').next().unwrap_or("")
}

fn child_path(path: &[u32], index: usize) -> Vec<u32> {
    let mut p = Vec::with_capacity(path.len() + 1);
    p.extend_from_slice(path);
    p.push(index as u32);
    p
}

fn double_to_char(value: f64) -> u8 {
    if (0.0..=127.0).contains(&value) { value as i32 as u8 } else { 0 }
}

fn read_take_anim_channel(ctx: &mut Context, node: &PNode, path: &[u32], value_fbx_id: u64, name: &str) -> Res<Option<Real>> {
    let default = node.find_f64("Default");

    let Some(keys) = find_f64(node, "Key") else { return Ok(default) };

    let curve_name = c_str(name);
    let dom = ctx.get_dom_node(path);
    let (curve_ix, curve_fbx_id) = ctx.push_synthetic_element(dom, Some(curve_name), ElementType::AnimCurve);
    ctx.connect_op(curve_fbx_id, value_fbx_id, curve_name);

    let curve = &mut ctx.scene.anim_curves[curve_ix];
    curve.pre_extrapolation = read_extrapolation(node, "Pre-Extrapolation");
    curve.post_extrapolation = read_extrapolation(node, "Post-Extrapolation");

    if ctx.opts.ignore_animation {
        return Ok(default);
    }

    let mut key_ver = node.find_i32("KeyVer").unwrap_or(0);
    if key_ver <= 0 {
        key_ver = if ctx.version() < 5000 {
            4003
        } else if ctx.version() < 6000 {
            4004
        } else {
            4005
        };
    }

    let Some(num_keys) = node.find_child("KeyCount").and_then(|n| n.size_at(0)) else { return fail() };

    let mut slope_left: f32 = 0.0;
    let mut weight_left: f32 = 0.333333;

    let mut next_time = 0.0f64;
    let mut next_value = 0.0f64;
    let mut prev_time = 0.0f64;

    // The keys are one array of times, values, modes as bare characters and their parameters, all read as doubles
    let data = keys.as_slice();
    let mut p = 0usize;

    if num_keys > 0 {
        check!(data.len() >= 2);
        next_time = data[0] / ctx.ktime_sec_double;
        next_value = data[1];
    }

    let mut keyframes: Vec<Keyframe> = Vec::with_capacity(num_keys.min(data.len() / 3));
    let (mut min_value, mut max_value) = (0.0, 0.0);

    for i in 0..num_keys {
        if i == 0 {
            min_value = next_value;
            max_value = next_value;
        } else {
            min_value = min_real(min_value, next_value);
            max_value = max_real(max_value, next_value);
        }

        check!(data.len() - p >= 3);
        let mut key = Keyframe { time: next_time, value: next_value, ..Default::default() };
        let mode = double_to_char(data[p + 2]);
        p += 3;

        let mut slope_right: f32 = 0.0;
        let mut weight_right: f32 = 0.333333;
        let mut next_slope_left: f32 = 0.0;
        let mut next_weight_left: f32 = 0.333333;
        let mut auto_slope = false;

        match mode {
            b'U' => {
                key.interpolation = Interpolation::Cubic;

                check!(data.len() - p >= 1);
                let slope_mode = double_to_char(data[p]);
                p += 1;

                // Only 's' and 'b' are understood, ufbx takes automatic tangents for the rest
                let mut num_weights = 1;
                match slope_mode {
                    b's' | b'b' => {
                        check!(data.len() - p >= 2);
                        slope_right = data[p] as f32;
                        next_slope_left = data[p + 1] as f32;
                        p += 2;
                        // KeyVer 4003 goes on to the next key without a weight mode
                        if key_ver == 4003 {
                            num_weights = 0;
                        }
                    }
                    b'a' => {
                        auto_slope = true;
                        if key_ver <= 4004 {
                            num_weights = 0;
                        }
                    }
                    b'p' | b'q' => {
                        auto_slope = true;
                        check!(data.len() - p >= 2);
                        p += 2;
                        num_weights = if key_ver <= 4004 { 1 } else { 2 };
                    }
                    b't' => {
                        auto_slope = true;
                        check!(data.len() - p >= 3);
                        p += 3;
                        num_weights = 0;
                    }
                    b'd' => {
                        auto_slope = true;
                        check!(data.len() - p >= 1);
                        p += 1;
                    }
                    _ => return fail(),
                }

                while num_weights > 0 {
                    check!(data.len() - p >= 1);
                    let weight_mode = double_to_char(data[p]);
                    p += 1;

                    match weight_mode {
                        b'n' | b'c' => {}
                        b'a' => {
                            check!(data.len() - p >= 2);
                            weight_right = data[p] as f32;
                            next_weight_left = data[p + 1] as f32;
                            p += 2;
                        }
                        b'l' => {
                            check!(data.len() - p >= 1);
                            next_weight_left = data[p] as f32;
                            p += 1;
                        }
                        b'r' => {
                            check!(data.len() - p >= 1);
                            weight_right = data[p] as f32;
                            p += 1;
                        }
                        _ => return fail(),
                    }
                    num_weights -= 1;
                }
            }
            b'L' => key.interpolation = Interpolation::Linear,
            b'C' => {
                if key_ver >= 4004 {
                    check!(data.len() - p >= 1);
                    key.interpolation = if double_to_char(data[p]) == b'n' { Interpolation::ConstantNext } else { Interpolation::ConstantPrev };
                    p += 1;
                } else {
                    key.interpolation = Interpolation::ConstantPrev;
                }
            }
            _ => return fail(),
        }

        if i + 1 < num_keys {
            check!(data.len() - p >= 2);
            next_time = data[p] / ctx.ktime_sec_double;
            next_value = data[p + 1];
        }

        if auto_slope {
            if i > 0 {
                let slope = ctx.solve_auto_tangent(
                    prev_time,
                    key.time,
                    next_time,
                    keyframes[i - 1].value,
                    key.value,
                    next_value,
                    weight_left,
                    weight_right,
                    0.0,
                    KEY_CLAMP_PROGRESSIVE | KEY_TIME_INDEPENDENT,
                );
                slope_left = slope;
                slope_right = slope;
            } else {
                slope_left = 0.0;
                slope_right = 0.0;
            }
        }

        if key.interpolation == Interpolation::Linear {
            if next_time > key.time {
                let slope = (next_value - key.value) / (next_time - key.time);
                slope_right = slope as f32;
                next_slope_left = slope as f32;
            } else {
                slope_right = 0.0;
                next_slope_left = 0.0;
            }
        }

        if key.time > prev_time {
            let delta = key.time - prev_time;
            key.left.dx = (weight_left as f64 * delta) as f32;
            key.left.dy = key.left.dx * slope_left;
        }

        if next_time > key.time {
            let delta = next_time - key.time;
            key.right.dx = (weight_right as f64 * delta) as f32;
            key.right.dy = key.right.dx * slope_right;
        }

        slope_left = next_slope_left;
        weight_left = next_weight_left;
        prev_time = key.time;
        keyframes.push(key);
    }

    check!(p == data.len());

    let curve = &mut ctx.scene.anim_curves[curve_ix];
    curve.keyframes = keyframes;
    curve.min_value = min_value;
    curve.max_value = max_value;
    Ok(default)
}

/// The recursion goes one level deep, from "Transform" to "T", "R" and "S"
fn read_take_prop_channel(ctx: &mut Context, node: &PNode, path: &[u32], target_fbx_id: u64, layer_fbx_id: u64, name: &str) -> Res<()> {
    if name == "Transform" {
        // Model: { Channel: "Transform" { Channel: "T" { Channel "X" ... } } } read as the 7000 property "Lcl Translation"
        for (ci, child) in node.children.iter().enumerate() {
            if child.name != "Channel" {
                continue;
            }
            let Some(old_name) = child.str_at(0) else { return fail() };
            let new_name = match old_name {
                "T" => "Lcl Translation",
                "R" => "Lcl Rotation",
                "S" => "Lcl Scaling",
                _ => continue,
            };
            read_take_prop_channel(ctx, child, &child_path(path, ci), target_fbx_id, layer_fbx_id, new_name)?;
        }
        return Ok(());
    }

    // Pre-6000 files name the channels of blend shape keys with a " (Shape)" suffix
    let mut name = name;
    if ctx.version() < 6000 && name.len() > " (Shape)".len() && name.ends_with(" (Shape)") {
        name = &name[..name.len() - " (Shape)".len()];
    }

    let mut channels: Vec<(&PNode, Vec<u32>, &str)> = Vec::with_capacity(3);
    if node.find_child("Key").is_some() || node.find_child("Default").is_some() {
        channels.push((node, path.to_vec(), name));
    } else {
        for (ci, child) in node.children.iter().enumerate() {
            if child.name != "Channel" {
                continue;
            }
            if child.find_child("Key").is_none() && child.find_child("Default").is_none() {
                continue;
            }
            let Some(channel_name) = child.str_at(0) else { continue };
            channels.push((child, child_path(path, ci), channel_name));
            if channels.len() == 3 {
                break;
            }
        }
    }

    if channels.is_empty() {
        return Ok(());
    }

    let dom = ctx.get_dom_node(path);
    let (value_ix, value_fbx_id) = ctx.push_synthetic_element(dom, Some(c_str(name)), ElementType::AnimValue);

    ctx.connect_oo(value_fbx_id, layer_fbx_id);
    ctx.connect_op(value_fbx_id, target_fbx_id, name);

    for (i, (channel, channel_path, channel_name)) in channels.iter().enumerate() {
        if let Some(default) = read_take_anim_channel(ctx, channel, channel_path, value_fbx_id, channel_name)? {
            ctx.scene.anim_values[value_ix].default_value[i] = default;
        }
    }
    Ok(())
}

/// Pre-7000 objects are named by their unique "Type::Name"
fn read_take_object(ctx: &mut Context, node: &PNode, path: &[u32], layer_fbx_id: u64) -> Res<()> {
    let Some(type_and_name) = node.raw_at(0) else { return fail() };
    let target_fbx_id = ctx.synthetic_id_from_string(type_and_name);

    for (ci, child) in node.children.iter().enumerate() {
        if child.name != "Channel" {
            continue;
        }
        let Some(name) = child.str_at(0) else { continue };
        read_take_prop_channel(ctx, child, &child_path(path, ci), target_fbx_id, layer_fbx_id, name)?;
    }
    Ok(())
}

fn read_take(ctx: &mut Context, node: &PNode, path: &[u32]) -> Res<()> {
    let mut tmp_props = Vec::with_capacity(4);
    if let Some(n) = node.find_child("LocalTime")
        && let (Some(start), Some(stop)) = (n.i64_at(0), n.i64_at(1))
    {
        tmp_props.push(Context::init_synthetic_int_prop("LocalStart", start, PropType::Integer));
        tmp_props.push(Context::init_synthetic_int_prop("LocalStop", stop, PropType::Integer));
    }
    if let Some(n) = node.find_child("ReferenceTime")
        && let (Some(start), Some(stop)) = (n.i64_at(0), n.i64_at(1))
    {
        tmp_props.push(Context::init_synthetic_int_prop("ReferenceStart", start, PropType::Integer));
        tmp_props.push(Context::init_synthetic_int_prop("ReferenceStop", stop, PropType::Integer));
    }

    let Some(name) = node.str_at(0) else { return fail() };

    // 7000 files keep only the take's times, for stacks that lack them
    if ctx.version() >= 7000 {
        if let Some(&ix) = ctx.anim_stack_map.get(name) {
            let stack = &mut ctx.scene.anim_stacks[ix];
            if stack.element.props.props.is_empty() {
                stack.element.props.props = tmp_props;
            }
        }
        return Ok(());
    }

    let dom = ctx.get_dom_node(path);
    let (stack_ix, stack_fbx_id) = ctx.push_synthetic_element(dom.clone(), Some(c_str(name)), ElementType::AnimStack);
    ctx.scene.anim_stacks[stack_ix].element.props.props = tmp_props;

    let (_, layer_fbx_id) = ctx.push_synthetic_element(dom, Some("BaseLayer"), ElementType::AnimLayer);
    ctx.connect_oo(layer_fbx_id, stack_fbx_id);

    for (ci, child) in node.children.iter().enumerate() {
        if child.name != "Model" {
            continue;
        }
        read_take_object(ctx, child, &child_path(path, ci), layer_fbx_id)?;
    }
    Ok(())
}

pub(crate) fn read_takes(ctx: &mut Context, children: &mut TopChildren) -> Res<()> {
    while let Some(node) = ctx.parse_toplevel_child(children)? {
        if node.name == "Take" {
            read_take(ctx, &node, &[children.ix, children.read - 1])?;
        }
    }
    Ok(())
}

/// `node` is "Settings", top-level before 6000 and inside "Version5" after
pub(crate) fn read_legacy_settings(ctx: &mut Context, node: &PNode) -> Res<()> {
    if ctx.read_legacy_settings {
        return Ok(());
    }
    ctx.read_legacy_settings = true;

    let mut tmp_props = Vec::with_capacity(2);

    if let Some(frame_rate) = node.find_child("FrameRate") {
        let mut fps = 0.0;
        match frame_rate.f64_at(0) {
            Some(v) => fps = v,
            None => {
                if let Some(s) = frame_rate.str_at(0) {
                    let (val, end) = parse_double(s.as_bytes(), false);
                    if end == Some(s.len()) {
                        fps = val;
                    }
                }
            }
        }
        if fps > 0.0 {
            tmp_props.push(Context::init_synthetic_real_prop("CustomFrameRate", fps, PropType::Number));
            tmp_props.push(Context::init_synthetic_real_prop("TimeMode", TimeMode::Custom as i32 as Real, PropType::Integer));
        }
    }

    if !tmp_props.is_empty() {
        let props = &mut ctx.scene.settings.props.props;
        tmp_props.append(props);
        sort_properties(&mut tmp_props);
        deduplicate_properties(&mut tmp_props);
        *props = tmp_props;
    }
    Ok(())
}

/// `root` indexes `ctx.scene.nodes`
pub(crate) fn setup_root_node(ctx: &mut Context, root: usize) {
    let (local_transform, node_to_parent) = if ctx.opts.use_root_transform {
        (ctx.opts.root_transform, transform_to_matrix(&ctx.opts.root_transform))
    } else {
        (Transform::IDENTITY, Matrix::IDENTITY)
    };
    let node = &mut ctx.scene.nodes[root];
    node.local_transform = local_transform;
    node.node_to_parent = node_to_parent;
    node.is_root = true;
}

pub(crate) fn supports_version(version: u32) -> bool {
    (3000..=7700).contains(&version)
}

/// A pre-6000 value, kept in a node of its own, read as a 7000 property
struct LegacyProp {
    prop_name: &'static str,
    prop_type: PropType,
    node_name: &'static str,
    /// One code per value of the node: 'L' integer, 'R' real, 'S' string, '_' skipped
    node_fmt: &'static [u8],
}

const fn legacy(prop_name: &'static str, prop_type: PropType, node_name: &'static str, node_fmt: &'static [u8]) -> LegacyProp {
    LegacyProp { prop_name, prop_type, node_name, node_fmt }
}

// The tables are sorted by `prop_name`, which keeps the properties read from them sorted

const LEGACY_LIGHT_PROPS: &[LegacyProp] = &[
    legacy("CastLight", PropType::Boolean, "CastLight", b"L"),
    legacy("CastShadows", PropType::Boolean, "CastShadows", b"L"),
    legacy("Color", PropType::Color, "Color", b"RRR"),
    legacy("ConeAngle", PropType::Number, "ConeAngle", b"R"),
    legacy("HotSpot", PropType::Number, "HotSpot", b"R"),
    legacy("Intensity", PropType::Number, "Intensity", b"R"),
    legacy("LightType", PropType::Integer, "LightType", b"L"),
];

const LEGACY_CAMERA_PROPS: &[LegacyProp] = &[
    legacy("ApertureMode", PropType::Integer, "ApertureMode", b"L"),
    legacy("AspectH", PropType::Number, "AspectH", b"R"),
    legacy("AspectRatioMode", PropType::Integer, "AspectType", b"L"),
    legacy("AspectW", PropType::Number, "AspectW", b"R"),
    legacy("FieldOfView", PropType::Number, "Aperture", b"R"),
    legacy("FieldOfViewX", PropType::Number, "FieldOfViewXProperty", b"R"),
    legacy("FieldOfViewY", PropType::Number, "FieldOfViewYProperty", b"R"),
    legacy("FilmHeight", PropType::Number, "CameraAperture", b"_R"),
    legacy("FilmSqueezeRatio", PropType::Number, "SqueezeRatio", b"R"),
    legacy("FilmWidth", PropType::Number, "CameraAperture", b"R_"),
    legacy("FocalLength", PropType::Number, "FocalLength", b"R"),
];

const LEGACY_BONE_PROPS: &[LegacyProp] = &[legacy("Size", PropType::Number, "Size", b"R")];

const LEGACY_MATERIAL_PROPS: &[LegacyProp] = &[
    legacy("AmbientColor", PropType::Color, "Ambient", b"RRR"),
    legacy("DiffuseColor", PropType::Color, "Diffuse", b"RRR"),
    legacy("EmissiveColor", PropType::Color, "Emissive", b"RRR"),
    legacy("ShadingModel", PropType::Color, "ShadingModel", b"S"),
    legacy("Shininess", PropType::Number, "Shininess", b"R"),
    legacy("SpecularColor", PropType::Color, "Specular", b"RRR"),
];

fn read_legacy_prop(node: &PNode, prop: &mut Prop, legacy_prop: &LegacyProp) -> bool {
    let mut value_ix = 0;
    let mut flags = 0u32;

    for (fmt_ix, &c) in legacy_prop.node_fmt.iter().enumerate() {
        match c {
            b'L' => {
                let Some(v) = node.i64_at(fmt_ix) else { return false };
                prop.value_int = v;
                prop.value_real_arr = [v as Real, 0.0, 0.0, 0.0];
                prop.value_str = String::new();
                prop.value_blob = Vec::new();
                flags |= prop_flags::VALUE_INT;
                value_ix += 1;
            }
            b'R' => {
                let Some(v) = node.f64_at(fmt_ix) else { return false };
                prop.value_real_arr[value_ix] = v;
                if value_ix == 0 {
                    prop.value_int = f64_to_i64(v);
                    prop.value_real_arr[1] = 0.0;
                    prop.value_real_arr[2] = 0.0;
                    prop.value_real_arr[3] = 0.0;
                    prop.value_str = String::new();
                    prop.value_blob = Vec::new();
                }
                flags &= !(prop_flags::VALUE_REAL | prop_flags::VALUE_VEC2 | prop_flags::VALUE_VEC3 | prop_flags::VALUE_VEC4);
                flags |= prop_flags::VALUE_REAL << value_ix;
                value_ix += 1;
            }
            b'S' => {
                let Some(s) = node.str_at(fmt_ix) else { return false };
                prop.value_str = s.to_string();
                prop.value_blob = node.raw_at(fmt_ix).unwrap_or(&[]).to_vec();
                prop.value_real_arr = [0.0; 4];
                prop.value_int = 0;
                flags |= prop_flags::VALUE_STR;
                value_ix += 1;
            }
            _ => {}
        }
    }

    prop.flags = flags;
    true
}

fn read_legacy_props(node: &PNode, legacy_props: &[LegacyProp]) -> Vec<Prop> {
    let mut props = Vec::with_capacity(legacy_props.len());
    for legacy_prop in legacy_props {
        let Some(n) = node.find_child(legacy_prop.node_name) else { continue };
        let mut prop = Prop::default();
        if !read_legacy_prop(n, &mut prop, legacy_prop) {
            continue;
        }
        prop.name = legacy_prop.prop_name.to_string();
        prop.internal_key = name_key(prop.name.as_bytes());
        // ufbx drops the value flags read_legacy_prop worked out
        prop.flags = 0;
        prop.prop_type = legacy_prop.prop_type;
        props.push(prop);
    }
    props
}

fn read_legacy_material(ctx: &mut Context, node: &PNode, dom: Option<DomRef>, name: &str) -> u64 {
    let (ix, fbx_id) = ctx.push_synthetic_element(dom, Some(c_str(name)), ElementType::Material);
    let props = read_legacy_props(node, LEGACY_MATERIAL_PROPS);
    let material = &mut ctx.scene.materials[ix];
    material.shading_model_name = String::new();
    material.element.props.props = props;
    material.shader_prop_prefix = String::new();
    fbx_id
}

fn read_legacy_link(ctx: &mut Context, node: &PNode, dom: Option<DomRef>, name: &str) -> Res<u64> {
    let info = ElementInfo { fbx_id: ctx.push_synthetic_id(), name: c_str(name).to_string(), props: Props::default(), dom_node: dom };
    ctx.read_skin_cluster(node, &info)?;
    Ok(info.fbx_id)
}

fn read_legacy_light(ctx: &mut Context, node: &PNode, info: &ElementInfo) {
    let ix = ctx.push_element(info, ElementType::Light);
    ctx.scene.lights[ix].element.props.props = read_legacy_props(node, LEGACY_LIGHT_PROPS);
}

fn read_legacy_camera(ctx: &mut Context, node: &PNode, info: &ElementInfo) {
    let ix = ctx.push_element(info, ElementType::Camera);
    ctx.scene.cameras[ix].element.props.props = read_legacy_props(node, LEGACY_CAMERA_PROPS);
}

fn read_legacy_limb_node(ctx: &mut Context, node: &PNode, info: &ElementInfo) {
    let ix = ctx.push_element(info, ElementType::Bone);
    let props = match node.find_child("Properties") {
        Some(prop_node) => read_legacy_props(prop_node, LEGACY_BONE_PROPS),
        None => Vec::new(),
    };
    ctx.scene.bones[ix].element.props.props = props;
}

/// Polygon meshes only: NURBS and the like are passed over
fn read_legacy_mesh(ctx: &mut Context, node: &PNode, path: &[u32], info: &ElementInfo) -> Res<()> {
    let (Some(node_vertices), Some(node_indices)) = (node.find_child("Vertices"), node.find_child("PolygonVertexIndex")) else { return Ok(()) };

    let ix = ctx.push_element(info, ElementType::Mesh);
    ctx.read_synthetic_blend_shapes(node, path, info)?;

    let mut mesh = std::mem::take(&mut ctx.scene.meshes[ix]);
    let r = read_legacy_mesh_data(ctx, node, node_vertices, node_indices, path, info, &mut mesh);
    patch_mesh_reals(&mut mesh);
    ctx.scene.meshes[ix] = mesh;
    r
}

/// Part of `ufbxi_read_legacy_mesh` in ufbx.c
fn read_legacy_mesh_data(
    ctx: &mut Context,
    node: &PNode,
    node_vertices: &PNode,
    node_indices: &PNode,
    path: &[u32],
    info: &ElementInfo,
    mesh: &mut Mesh,
) -> Res<()> {
    if ctx.opts.ignore_geometry {
        return Ok(());
    }

    let (Some(vertices), Some(indices)) = (arr_f64(node_vertices), arr_i32(node_indices)) else { return fail() };
    check!(vertices.len().is_multiple_of(3));

    mesh.num_vertices = vertices.len() / 3;
    mesh.num_indices = indices.len();

    let mut index_data = as_u32(indices);

    let m = mesh.element.typed_id as usize;
    let base = info.dom_node.clone();
    ctx.dom_alias(&base, node, "Vertices", DomAlias::MeshVertices(m));
    mesh.vertices = vec3s(vertices);
    mesh.vertex_position.exists = true;
    mesh.vertex_position.values = mesh.vertices.clone();
    mesh.vertex_position.unique_per_vertex = true;

    if mesh.num_indices > 0 {
        let last = mesh.num_indices - 1;
        if (index_data[last] as i32) >= 0 {
            ctx.fail_strict("Non-negated last index")?;
            index_data[last] = !index_data[last];
        }
    }

    ctx.process_indices(mesh, &mut index_data)?;
    mesh.vertex_indices = index_data.clone();
    mesh.vertex_position.indices = index_data;

    // Normals are per vertex or per index; where both fit, 5000 files mean per vertex
    if let Some(normals) = find_f64(node, "Normals") {
        let num_normals = normals.len() / 3;
        let per_vertex = num_normals == mesh.num_vertices;
        let per_index = num_normals == mesh.num_indices;
        if per_vertex || per_index {
            ctx.dom_alias(&base, node, "Normals", DomAlias::NormalValues(m));
        }
        if per_vertex && (!per_index || ctx.version() == 5000) {
            mesh.vertex_normal =
                VertexAttrib { exists: true, values: vec3s(normals), indices: mesh.vertex_indices.clone(), unique_per_vertex: true, ..Default::default() };
        } else if per_index {
            mesh.vertex_normal = VertexAttrib {
                exists: true,
                values: vec3s(normals),
                indices: (0..mesh.num_indices as u32).collect(),
                unique_per_vertex: false,
                ..Default::default()
            };
        }
    }

    if let Some(uv_info) = node.find_child("GeometryUVInfo") {
        let (vertex_uv, vs, is) = ctx.read_vertex_element(mesh, uv_info, "TextureUV", "TextureUVVerticeIndex", None, 2, |c| Vec2::new(c[0], c[1]))?;
        let uv_base = crate::read::child_path(&base, node, "GeometryUVInfo");
        if vs {
            ctx.dom_alias(&uv_base, uv_info, "TextureUV", DomAlias::UvValues(m, 0));
        }
        if is {
            ctx.dom_alias(&uv_base, uv_info, "TextureUVVerticeIndex", DomAlias::UvIndices(m, 0));
        }
        let set = UvSet { name: String::new(), index: 0, vertex_uv, ..Default::default() };
        mesh.vertex_uv = set.vertex_uv.clone();
        mesh.uv_sets = vec![set];
    }

    let Some(mapping) = node.find_str("MaterialAssignation") else { return fail() };
    if mapping == "ByPolygon" {
        if let Some(v) = ctx.read_truncated_array(find_i32(node, "Materials"), "Materials", mesh.num_faces) {
            mesh.face_material = as_u32(&v);
            if find_i32(node, "Materials").is_some_and(|a| a.len() >= mesh.num_faces) {
                ctx.dom_alias(&base, node, "Materials", DomAlias::FaceMaterial(m));
            }
        }
    } else if mapping == "AllSame" {
        let material = match find_i32(node, "Materials") {
            Some(arr) if !arr.is_empty() => arr[0] as u32,
            _ => 0,
        };
        mesh.face_material = vec![material; mesh.num_faces];
    }

    let mut skin_fbx_id: Option<u64> = None;
    for (ci, child) in node.children.iter().enumerate() {
        if child.name == "Material" {
            let Some(type_and_name) = child.raw_at(0) else { return fail() };
            let (_, name) = ctx.split_type_and_name(type_and_name)?;
            let dom = ctx.get_dom_node(&child_path(path, ci));
            let fbx_id = read_legacy_material(ctx, child, dom, &name);
            ctx.connect_oo(fbx_id, info.fbx_id);
        } else if child.name == "Link" {
            let Some(type_and_name) = child.raw_at(0) else { return fail() };
            let (_, name) = ctx.split_type_and_name(type_and_name)?;
            let dom = ctx.get_dom_node(&child_path(path, ci));
            let fbx_id = read_legacy_link(ctx, child, dom, &name)?;

            let node_fbx_id = ctx.synthetic_id_from_string(type_and_name);
            ctx.connect_oo(node_fbx_id, fbx_id);
            let skin = match skin_fbx_id {
                Some(id) => id,
                None => {
                    let (_, id) = ctx.push_synthetic_element(None, Some(c_str(&info.name)), ElementType::SkinDeformer);
                    ctx.connect_oo(id, info.fbx_id);
                    skin_fbx_id = Some(id);
                    id
                }
            };
            ctx.connect_oo(fbx_id, skin);
        }
    }

    mesh.skinned_is_local = true;
    mesh.skinned_position = mesh.vertex_position.clone();
    mesh.skinned_normal = mesh.vertex_normal.clone();
    Ok(())
}

/// The videos take the "Media" node as their DOM node
fn read_legacy_media(ctx: &mut Context, node: &PNode, top_index: u32) -> Res<()> {
    if let Some(videos) = node.find_child("Video") {
        for child in &videos.children {
            let Some(name) = child.str_at(0) else { return fail() };
            let fbx_id = ctx.push_synthetic_id();
            let video_info = ElementInfo { fbx_id, name: name.to_string(), props: Props::default(), dom_node: ctx.get_dom_node(&[top_index]) };
            ctx.read_video(child, &video_info);
        }
    }
    Ok(())
}

fn read_legacy_model(ctx: &mut Context, node: &PNode, top_index: u32) -> Res<()> {
    let path = [top_index];
    let Some(type_and_name) = node.raw_at(0) else { return fail() };
    let (_, name) = ctx.split_type_and_name(type_and_name)?;

    let info =
        ElementInfo { fbx_id: ctx.synthetic_id_from_string(type_and_name), name: name.clone(), props: Props::default(), dom_node: ctx.get_dom_node(&path) };

    let node_ix = ctx.push_element(&info, ElementType::Node);
    let node_id = ctx.scene.nodes[node_ix].element.element_id;
    ctx.tmp_node_ids.push(node_id);

    let attrib_info = ElementInfo { fbx_id: ctx.push_synthetic_id(), name, props: Props::default(), dom_node: info.dom_node.clone() };

    // An attribute that is never read leaves an unused connection
    ctx.connect_oo(attrib_info.fbx_id, info.fbx_id);

    let attrib_type = node.find_str("Type").unwrap_or("");

    let has_attrib = match attrib_type {
        "Light" => {
            read_legacy_light(ctx, node, &attrib_info);
            true
        }
        "Camera" => {
            read_legacy_camera(ctx, node, &attrib_info);
            true
        }
        "LimbNode" => {
            read_legacy_limb_node(ctx, node, &attrib_info);
            true
        }
        _ if node.find_child("Vertices").is_some() => {
            read_legacy_mesh(ctx, node, &path, &attrib_info)?;
            true
        }
        _ => false,
    };

    if has_attrib {
        ctx.insert_fbx_attr(info.fbx_id, attrib_info.fbx_id);
    }

    if let Some(PArr::Str(names)) = node.find_array("Children") {
        for child_name in names {
            let child_fbx_id = ctx.synthetic_id_from_string(&child_name.raw);
            ctx.connect_oo(child_fbx_id, info.fbx_id);
        }
    }

    for (ci, child) in node.children.iter().enumerate() {
        if child.name != "Channel" {
            continue;
        }
        let Some(channel_name) = child.str_at(0) else { continue };
        // The layer is made at the end, so that it is not the first stack
        if ctx.legacy_implicit_anim_layer_id == 0 {
            ctx.legacy_implicit_anim_layer_id = ctx.push_synthetic_id();
        }
        let layer_fbx_id = ctx.legacy_implicit_anim_layer_id;
        read_take_prop_channel(ctx, child, &child_path(&path, ci), info.fbx_id, layer_fbx_id, channel_name)?;
    }
    Ok(())
}

pub(crate) fn read_legacy_root(ctx: &mut Context) -> Res<()> {
    // Some files have an `Fbx_Root` node, but no other format has a root with a transform: an empty one is made
    let (root_ix, root_fbx_id) = ctx.push_synthetic_element(None, Some(""), ElementType::Node);
    ctx.root_id = root_fbx_id;
    setup_root_node(ctx, root_ix);
    let root_id = ctx.scene.nodes[root_ix].element.element_id;
    ctx.tmp_node_ids.push(root_id);

    // The header extension is optional, so the KTime unit starts as the default one
    ctx.ktime_sec = 46186158000;
    ctx.ktime_sec_double = ctx.ktime_sec as f64;

    while let Some(node) = ctx.p.parse_legacy_toplevel()? {
        let top_index = (ctx.p.top_nodes.len() - 1) as u32;
        match node.name.as_str() {
            "FBXHeaderExtension" => ctx.read_header_extension(&mut TopChildren::parsed(top_index, node.children))?,
            "Media" => read_legacy_media(ctx, &node, top_index)?,
            "Takes" => read_takes(ctx, &mut TopChildren::parsed(top_index, node.children))?,
            "Model" => read_legacy_model(ctx, &node, top_index)?,
            "Settings" => read_legacy_settings(ctx, &node)?,
            _ => {}
        }
    }

    if ctx.legacy_implicit_anim_layer_id != 0 {
        let layer_info = ElementInfo { fbx_id: ctx.legacy_implicit_anim_layer_id, name: "(internal)".to_string(), ..Default::default() };
        ctx.push_element(&layer_info, ElementType::AnimLayer);

        let stack_info = ElementInfo { fbx_id: ctx.push_synthetic_id(), ..layer_info.clone() };
        ctx.push_element(&stack_info, ElementType::AnimStack);

        ctx.connect_oo(layer_info.fbx_id, stack_info.fbx_id);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::unscaled_transform_to_matrix;

    fn context(data: &[u8], opts: LoadOpts) -> Context<'_> {
        Context::new(data, opts)
    }

    fn top(ctx: &mut Context, name: &str) -> Res<Option<(PNode, u32)>> {
        let Some(i) = ctx.p.parse_toplevel(name)? else { return Ok(None) };
        let mut node = ctx.p.top_nodes[i].clone();
        if ctx.p.is_lazy(i) {
            while let Some(child) = ctx.p.next_lazy_child()? {
                node.children.push(child);
            }
        }
        Ok(Some((node, i as u32)))
    }

    fn read(data: &[u8]) -> Res<Context<'_>> {
        read_with(data, LoadOpts::default())
    }

    fn read_with(data: &[u8], opts: LoadOpts) -> Res<Context<'_>> {
        let mut ctx = context(data, opts);
        ctx.p.begin_parse()?;
        if ctx.version() < 6000 {
            read_legacy_root(&mut ctx)?;
            return Ok(ctx);
        }
        let mut header = ctx.toplevel_children("FBXHeaderExtension")?;
        ctx.read_header_extension(&mut header)?;
        ctx.match_exporter();
        if ctx.version() >= 7000 {
            let mut documents = ctx.toplevel_children("Documents")?;
            ctx.read_document(&mut documents)?;
        } else {
            let root_name: &[u8] = if ctx.from_ascii() { b"Model::Scene" } else { b"Scene\x00\x01Model" };
            ctx.root_id = ctx.synthetic_id_from_string(root_name);
        }
        let root_info = ElementInfo { fbx_id: ctx.root_id, ..Default::default() };
        let root = ctx.push_element(&root_info, ElementType::Node);
        setup_root_node(&mut ctx, root);
        let root_id = ctx.scene.nodes[root].element.element_id;
        ctx.tmp_node_ids.push(root_id);
        let mut definitions = ctx.toplevel_children("Definitions")?;
        ctx.read_definitions(&mut definitions)?;
        if let Some(i) = ctx.p.parse_toplevel("Objects")? {
            ctx.read_objects(i)?;
        }
        let mut connections = ctx.toplevel_children("Connections")?;
        ctx.read_connections(&mut connections)?;
        let mut takes = ctx.toplevel_children("Takes")?;
        read_takes(&mut ctx, &mut takes)?;
        if let Some((n, _)) = top(&mut ctx, "Version5")?
            && let Some(settings) = n.find_child("Settings")
        {
            read_legacy_settings(&mut ctx, settings)?;
        }
        Ok(ctx)
    }

    // The expected values below are what ufbx 0.23.1 (the `ufbx` crate 0.11.5) loads from the same text

    const TAKE_6100: &str = r#"; FBX 6.1.0 project file
FBXHeaderExtension:  {
	FBXHeaderVersion: 1003
	FBXVersion: 6100
	Creator: "test"
}
Objects:  {
	Model: "Model::Box", "Null" {
		Version: 232
	}
}
Connections:  {
	Connect: "OO", "Model::Box", "Model::Scene"
}
Takes:  {
	Current: "Take 001"
	Take: "Take 001" {
		FileName: "Take_001.tak"
		LocalTime: 0,323303106000
		ReferenceTime: 46186158000,277116948000
		Model: "Model::Box" {
			Version: 1.1
			Channel: "Transform" {
				Channel: "T" {
					Channel: "X" {
						Default: 1.5
						KeyVer: 4005
						KeyCount: 8
						Key: 0,0,U,s,0,2,a,0.25,0.5,46186158000,1,L,92372316000,3,C,n,138558474000,2,U,p,1,2,n,n
							,184744632000,5,U,t,0.1,0.2,0.3,230930790000,4,U,d,0,r,0.4,277116948000,6,U,q,1,2,l,0.2,c
							,323303106000,7,U,b,1,1,n
						Pre-Extrapolation:  {
							Type: 77
							Repetition: 3
						}
						Color: 1,0,0
					}
					Channel: "Y" {
						Default: 2
					}
				}
				Channel: "S" {
					Channel: "X" {
						Default: 3
						KeyVer: 4005
						KeyCount: 2
						Key: 0,1,C,p,46186158000,2,L
					}
				}
			}
			Channel: "Visibility" {
				Default: 1
				KeyVer: 4005
				KeyCount: 1
				Key: 0,1,L
			}
		}
	}
}
Version5:  {
	Settings:  {
		FrameRate: "24"
	}
}
"#;

    const LEGACY_5800: &str = r#"; FBX 5.8.0 project file
FBXHeaderExtension:  {
	FBXHeaderVersion: 1003
	FBXVersion: 5800
	Creator: "test"
}
Media:  {
	Video:  {
		Video: "Video::Wood" {
			Type: "Clip"
			Filename: "C:/wood.png"
			RelativeFilename: "wood.png"
		}
	}
}
Model: "Model::Lamp" {
	Type: "Light"
	LightType: 2
	CastLight: 1
	Color: 1,0.5,0.25
	Intensity: 80
	ConeAngle: 45
	HotSpot: "x"
}
Model: "Model::Cam" {
	Type: "Camera"
	FocalLength: 35
	CameraAperture: 1.4,0.9
	AspectType: 2
	Aperture: 40
}
Model: "Model::Bone" {
	Type: "LimbNode"
	Properties:  {
		Size: 3
	}
}
Model: "Model::Quad" {
	Vertices: 0,0,0,1,0,0,1,1,0,0,1,0
	PolygonVertexIndex: 0,1,2,-4,0,2,3
	Normals: 0,0,1,0,0,1,0,0,1,0,0,1,0,0,1,0,0,1,0,0,1
	MaterialAssignation: "ByPolygon"
	Materials: 1
	GeometryUVInfo:  {
		MappingInformationType: "ByPolygonVertex"
		TextureUV: 0,0,1,0,1,1,0,1
		TextureUVVerticeIndex: 0,1,2,3,0,2,3
	}
	Material: "Material::Red" {
		Diffuse: 1,0,0
		Shininess: 20
		ShadingModel: "phong"
	}
	Material: "Material::Blue" {
		Diffuse: 0,0,1
	}
	Link: "Model::Bone" {
		Indexes: 0,1
		Weights: 1,0.5
		Transform: 1,0,0,0,0,1,0,0,0,0,1,0,1,2,3,1
		TransformLink: 2,0,0,0,0,2,0,0,0,0,2,0,4,5,6,1
	}
	Children: "Model::Bone"
	Channel: "Visibility (Shape)" {
		Default: 0.5
		KeyVer: 4004
		KeyCount: 2
		Key: 0,1,C,n,46186158000,0,U,a
	}
}
Settings:  {
	FrameRate: "24"
}
"#;

    type Key = (f64, Real, Interpolation, f32, f32, f32, f32);

    fn keys(curve: &AnimCurve) -> Vec<Key> {
        curve.keyframes.iter().map(|k| (k.time, k.value, k.interpolation, k.left.dx, k.left.dy, k.right.dx, k.right.dy)).collect()
    }

    fn fbx_id(ctx: &Context, element_id: ElementId) -> u64 {
        ctx.element_fbx_ids[element_id as usize]
    }

    fn connected(ctx: &Context, src: u64, dst: u64, dst_prop: &str) -> bool {
        ctx.tmp_connections.iter().any(|c| c.src == src && c.dst == dst && c.src_prop.is_empty() && c.dst_prop == dst_prop)
    }

    fn prop_values(props: &Props) -> Vec<(&str, PropType, i64, [Real; 4], &str)> {
        props.props.iter().map(|p| (p.name.as_str(), p.prop_type, p.value_int, p.value_real_arr, p.value_str.as_str())).collect()
    }

    #[test]
    fn take_keys_as_ufbx_reads_them() {
        use Interpolation::*;
        let ctx = read(TAKE_6100.as_bytes()).unwrap();
        let s = &ctx.scene;
        let w = 0.333333f32;

        assert_eq!(s.anim_curves.len(), 3);
        let x = &s.anim_curves[0];
        assert_eq!(x.element.name, "X");
        assert_eq!(x.pre_extrapolation, Extrapolation { mode: ExtrapolationMode::Mirror, repeat_count: 3 });
        assert_eq!(x.post_extrapolation, Extrapolation { mode: ExtrapolationMode::Constant, repeat_count: -1 });
        assert_eq!((x.min_value, x.max_value), (0.0, 7.0));
        let expected: Vec<Key> = vec![
            (0.0, 0.0, Cubic, 0.0, 0.0, 0.25, 0.0),
            (1.0, 1.0, Linear, 0.5, 1.0, w, 0.666666),
            (2.0, 3.0, ConstantNext, w, 0.666666, w, 0.0),
            (3.0, 2.0, Cubic, w, 0.0, w, 0.0),
            (4.0, 5.0, Cubic, w, 0.0, w, 0.0),
            (5.0, 4.0, Cubic, w, 0.0, 0.4, 0.0),
            (6.0, 6.0, Cubic, w, 0.49999946, w, 0.49999946),
            (7.0, 7.0, Cubic, 0.2, 0.0, 0.0, 0.0),
        ];
        assert_eq!(keys(x), expected);
        assert_eq!(keys(&s.anim_curves[1]), vec![(0.0, 1.0, ConstantPrev, 0.0, 0.0, w, 0.0), (1.0, 2.0, Linear, w, 0.0, 0.0, 0.0)]);
        assert_eq!(s.anim_curves[2].element.name, "Visibility");
        assert_eq!(keys(&s.anim_curves[2]), vec![(0.0, 1.0, Linear, 0.0, 0.0, 0.0, 0.0)]);

        let values: Vec<(&str, Vec3)> = s.anim_values.iter().map(|v| (v.element.name.as_str(), v.default_value)).collect();
        assert_eq!(
            values,
            vec![("Lcl Translation", Vec3::new(1.5, 2.0, 0.0)), ("Lcl Scaling", Vec3::new(3.0, 0.0, 0.0)), ("Visibility", Vec3::new(1.0, 0.0, 0.0))]
        );

        assert_eq!(s.anim_stacks.len(), 1);
        let stack = &s.anim_stacks[0];
        assert_eq!(stack.element.name, "Take 001");
        let times: Vec<(&str, i64, u32)> = stack.element.props.props.iter().map(|p| (p.name.as_str(), p.value_int, p.flags)).collect();
        let flags = prop_flags::SYNTHETIC | prop_flags::VALUE_REAL | prop_flags::VALUE_INT;
        assert_eq!(
            times,
            vec![("LocalStart", 0, flags), ("LocalStop", 323303106000, flags), ("ReferenceStart", 46186158000, flags), ("ReferenceStop", 277116948000, flags)]
        );
        assert_eq!(s.anim_layers.len(), 1);
        assert_eq!(s.anim_layers[0].element.name, "BaseLayer");

        let layer = fbx_id(&ctx, s.anim_layers[0].element.element_id);
        let box_id = ctx.string_ids[b"Model::Box".as_slice()];
        let translation = fbx_id(&ctx, s.anim_values[0].element.element_id);
        assert!(connected(&ctx, layer, fbx_id(&ctx, stack.element.element_id), ""));
        assert!(connected(&ctx, translation, layer, ""));
        assert!(connected(&ctx, translation, box_id, "Lcl Translation"));
        assert!(connected(&ctx, fbx_id(&ctx, x.element.element_id), translation, "X"));

        let settings = prop_values(&s.settings.props);
        assert_eq!(
            settings,
            vec![("CustomFrameRate", PropType::Number, 24, [24.0, 0.0, 0.0, 0.0], ""), ("TimeMode", PropType::Integer, 14, [14.0, 0.0, 0.0, 0.0], "")]
        );
    }

    #[test]
    fn legacy_file_as_ufbx_reads_it() {
        let ctx = read(LEGACY_5800.as_bytes()).unwrap();
        let s = &ctx.scene;

        let nodes: Vec<&str> = s.nodes.iter().map(|n| n.element.name.as_str()).collect();
        assert_eq!(nodes, vec!["", "Lamp", "Cam", "Bone", "Quad"]);
        assert!(s.nodes[0].is_root && !s.nodes[1].is_root);
        assert_eq!(ctx.root_id, fbx_id(&ctx, s.nodes[0].element.element_id));
        assert_eq!(ctx.tmp_node_ids.len(), 5);

        use PropType::*;
        assert_eq!(
            prop_values(&s.lights[0].element.props),
            vec![
                ("CastLight", Boolean, 1, [1.0, 0.0, 0.0, 0.0], ""),
                ("Color", Color, 1, [1.0, 0.5, 0.25, 0.0], ""),
                ("ConeAngle", Number, 45, [45.0, 0.0, 0.0, 0.0], ""),
                ("Intensity", Number, 80, [80.0, 0.0, 0.0, 0.0], ""),
                ("LightType", Integer, 2, [2.0, 0.0, 0.0, 0.0], ""),
            ]
        );
        assert!(s.lights[0].element.props.props.iter().all(|p| p.flags == 0));
        assert_eq!(
            prop_values(&s.cameras[0].element.props),
            vec![
                ("AspectRatioMode", Integer, 2, [2.0, 0.0, 0.0, 0.0], ""),
                ("FieldOfView", Number, 40, [40.0, 0.0, 0.0, 0.0], ""),
                ("FilmHeight", Number, 0, [0.9, 0.0, 0.0, 0.0], ""),
                ("FilmWidth", Number, 1, [1.4, 0.0, 0.0, 0.0], ""),
                ("FocalLength", Number, 35, [35.0, 0.0, 0.0, 0.0], ""),
            ]
        );
        assert_eq!(prop_values(&s.bones[0].element.props), vec![("Size", Number, 3, [3.0, 0.0, 0.0, 0.0], "")]);

        let mesh = &s.meshes[0];
        assert_eq!(mesh.element.name, "Quad");
        assert_eq!((mesh.num_vertices, mesh.num_indices, mesh.num_faces, mesh.num_triangles), (4, 7, 2, 3));
        assert_eq!(mesh.vertex_indices, vec![0, 1, 2, 3, 0, 2, 3]);
        assert_eq!(mesh.vertex_position.indices, mesh.vertex_indices);
        assert_eq!(mesh.faces, vec![Face { index_begin: 0, num_indices: 4 }, Face { index_begin: 4, num_indices: 3 }]);
        assert!(mesh.vertex_normal.exists && !mesh.vertex_normal.unique_per_vertex);
        assert_eq!(mesh.vertex_normal.values, vec![Vec3::new(0.0, 0.0, 1.0); 7]);
        assert_eq!(mesh.vertex_normal.indices, (0..7).collect::<Vec<u32>>());
        assert!(mesh.vertex_uv.exists && !mesh.vertex_uv.unique_per_vertex);
        assert_eq!(mesh.vertex_uv.values, vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0), Vec2::new(0.0, 1.0)]);
        assert_eq!(mesh.vertex_uv.indices, vec![0, 1, 2, 3, 0, 2, 3]);
        assert_eq!(mesh.uv_sets.len(), 1);
        assert_eq!(mesh.face_material, vec![1, 1]);
        assert!(mesh.skinned_is_local);
        assert_eq!(mesh.skinned_normal, mesh.vertex_normal);
        assert_eq!(mesh.vertex_position.value_reals, 3);

        let materials: Vec<&str> = s.materials.iter().map(|m| m.element.name.as_str()).collect();
        assert_eq!(materials, vec!["Red", "Blue"]);
        assert_eq!(
            prop_values(&s.materials[0].element.props),
            vec![
                ("DiffuseColor", Color, 1, [1.0, 0.0, 0.0, 0.0], ""),
                ("ShadingModel", Color, 0, [0.0, 0.0, 0.0, 0.0], "phong"),
                ("Shininess", Number, 20, [20.0, 0.0, 0.0, 0.0], ""),
            ]
        );

        let cluster = &s.skin_clusters[0];
        assert_eq!(cluster.element.name, "Bone");
        assert_eq!((cluster.num_weights, cluster.vertices.clone(), cluster.weights.clone()), (2, vec![0, 1], vec![1.0, 0.5]));
        assert_eq!((cluster.mesh_node_to_bone.m03, cluster.mesh_node_to_bone.m13, cluster.mesh_node_to_bone.m23), (1.0, 2.0, 3.0));
        assert_eq!((cluster.bind_to_world.m00, cluster.bind_to_world.m03, cluster.bind_to_world.m23), (2.0, 4.0, 6.0));
        assert_eq!(s.skin_deformers[0].element.name, "Quad");

        let video = &s.videos[0];
        assert_eq!(
            (video.element.name.as_str(), video.absolute_filename.as_str(), video.relative_filename.as_str()),
            ("Video::Wood", "C:/wood.png", "wood.png")
        );

        assert_eq!(s.anim_curves.len(), 1);
        let w = 0.333333f32;
        assert_eq!(s.anim_curves[0].element.name, "Visibility");
        assert_eq!(
            keys(&s.anim_curves[0]),
            vec![(0.0, 1.0, Interpolation::ConstantNext, 0.0, 0.0, w, 0.0), (1.0, 0.0, Interpolation::Cubic, w, -0.0, 0.0, 0.0)]
        );
        assert_eq!((s.anim_values[0].element.name.as_str(), s.anim_values[0].default_value.x), ("Visibility", 0.5));
        assert_eq!((s.anim_stacks[0].element.name.as_str(), s.anim_layers[0].element.name.as_str()), ("(internal)", "(internal)"));
        assert_eq!(ctx.legacy_implicit_anim_layer_id, fbx_id(&ctx, s.anim_layers[0].element.element_id));

        let quad = ctx.string_ids[b"Model::Quad".as_slice()];
        let bone = ctx.string_ids[b"Model::Bone".as_slice()];
        let mesh_id = fbx_id(&ctx, mesh.element.element_id);
        let cluster_id = fbx_id(&ctx, cluster.element.element_id);
        let skin_id = fbx_id(&ctx, s.skin_deformers[0].element.element_id);
        assert_eq!(ctx.fbx_attr_map[&quad], mesh_id);
        assert_eq!(ctx.fbx_attr_map.len(), 4);
        assert!(connected(&ctx, mesh_id, quad, ""));
        assert!(connected(&ctx, bone, quad, ""));
        assert!(connected(&ctx, fbx_id(&ctx, s.materials[1].element.element_id), mesh_id, ""));
        assert!(connected(&ctx, bone, cluster_id, ""));
        assert!(connected(&ctx, cluster_id, skin_id, ""));
        assert!(connected(&ctx, skin_id, mesh_id, ""));
        assert!(connected(&ctx, fbx_id(&ctx, s.anim_values[0].element.element_id), quad, "Visibility"));

        assert_eq!(prop_values(&s.settings.props)[0], ("CustomFrameRate", Number, 24, [24.0, 0.0, 0.0, 0.0], ""));
    }

    fn take_with_key(key_ver: u32, key_count: usize, key: &str) -> String {
        format!(
            "; FBX 6.1.0 project file\nTakes:  {{\n\tTake: \"T\" {{\n\t\tModel: \"Model::A\" {{\n\t\t\tChannel: \"V\" {{\n\t\t\t\tKeyVer: {key_ver}\n\t\t\t\tKeyCount: {key_count}\n\t\t\t\tKey: {key}\n\t\t\t}}\n\t\t}}\n\t}}\n}}\n"
        )
    }

    fn take_error(key_ver: u32, key_count: usize, key: &str) -> String {
        let text = take_with_key(key_ver, key_count, key);
        match read(text.as_bytes()) {
            Ok(_) => "ok".to_string(),
            Err(e) => e.description,
        }
    }

    #[test]
    fn bad_keys_fail_as_ufbx_fails() {
        assert_eq!(take_error(4005, 1, "0,1,L"), "ok");
        // ufbx fails these with `ufbxi_fail`, whose text reaches no description
        assert_eq!(take_error(4005, 1, "0,1,X"), "");
        assert_eq!(take_error(4005, 1, "0,1,U,z"), "");
        assert_eq!(take_error(4005, 1, "0,1,U,s,0,0,z"), "");
        assert_eq!(take_error(4003, 1, "0,1,U,s,0,0"), "ok");
        assert_eq!(take_error(4004, 1, "0,1,U,s,0,0"), "");
        assert_eq!(take_error(4005, 1, "0,1,L,5"), "");
        assert_eq!(take_error(4005, 2, "0,1,L"), "");
        assert_eq!(take_error(4005, 0, "0,1,L"), "");
        let no_count = "; FBX 6.1.0 project file\nTakes:  {\n\tTake: \"T\" {\n\t\tModel: \"Model::A\" {\n\t\t\tChannel: \"V\" {\n\t\t\t\tKey: 0,1,L\n\t\t\t}\n\t\t}\n\t}\n}\n";
        assert!(read(no_count.as_bytes()).is_err());
    }

    #[test]
    fn ignored_animation_keeps_no_curves() {
        let mut ctx = context(TAKE_6100.as_bytes(), LoadOpts { ignore_animation: true, ..Default::default() });
        ctx.p.begin_parse().unwrap();
        let mut takes = ctx.toplevel_children("Takes").unwrap();
        read_takes(&mut ctx, &mut takes).unwrap();
        assert!(ctx.scene.anim_curves.is_empty());
        assert_eq!(ctx.scene.anim_values.len(), 3);
        assert_eq!(ctx.scene.anim_values[0].default_value, Vec3::new(1.5, 2.0, 0.0));
    }

    #[test]
    fn legacy_settings_are_read_once() {
        let node = |fps: &str| PNode {
            name: "Settings".to_string(),
            raw_name: None,
            dom_name: None,
            data: crate::parse::PData::Vals(Vec::new()),
            children: vec![PNode {
                name: "FrameRate".to_string(),
                raw_name: None,
                dom_name: None,
                data: crate::parse::PData::Vals(vec![crate::parse::PVal::Str(crate::parse::SStr {
                    raw: fps.as_bytes().to_vec(),
                    utf8: Some(fps.to_string()),
                })]),
                children: Vec::new(),
            }],
        };
        let mut ctx = context(b"", LoadOpts::default());
        ctx.scene.settings.props.props = vec![Context::init_synthetic_real_prop("TimeMode", 6.0, PropType::Integer)];
        read_legacy_settings(&mut ctx, &node("30x")).unwrap();
        assert_eq!(ctx.scene.settings.props.props.len(), 1);
        read_legacy_settings(&mut ctx, &node("25")).unwrap();
        assert_eq!(ctx.scene.settings.props.props.len(), 1);

        let mut ctx = context(b"", LoadOpts::default());
        ctx.scene.settings.props.props = vec![Context::init_synthetic_real_prop("TimeMode", 6.0, PropType::Integer)];
        read_legacy_settings(&mut ctx, &node("25")).unwrap();
        let props: Vec<(&str, Real)> = ctx.scene.settings.props.props.iter().map(|p| (p.name.as_str(), p.value_real())).collect();
        assert_eq!(props, vec![("CustomFrameRate", 25.0), ("TimeMode", 6.0)]);
    }

    #[test]
    fn root_node_and_versions() {
        let t = Transform { translation: Vec3::new(1.0, 2.0, 3.0), rotation: Quat { x: 0.0, y: 0.6, z: 0.0, w: 0.8 }, scale: Vec3::new(2.0, 3.0, 4.0) };
        assert_eq!(unscaled_transform_to_matrix(&t), transform_to_matrix(&Transform { scale: Vec3::ONE, ..t }));

        let mut ctx = context(b"", LoadOpts { use_root_transform: true, root_transform: t, ..Default::default() });
        let (root, _) = ctx.push_synthetic_element(None, Some(""), ElementType::Node);
        setup_root_node(&mut ctx, root);
        let node = &ctx.scene.nodes[root];
        assert!(node.is_root);
        assert_eq!((node.local_transform, node.node_to_parent), (t, transform_to_matrix(&t)));

        assert!(supports_version(3000) && supports_version(7700));
        assert!(!supports_version(2999) && !supports_version(7701));
        assert_eq!((double_to_char(85.0), double_to_char(-1.0), double_to_char(128.0), double_to_char(97.9)), (b'U', 0, 0, b'a'));
    }
}
