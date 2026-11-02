use std::borrow::Cow;
use std::cmp::Ordering;
use std::sync::Arc;

use crate::error::{Error, Res, check, utf8_valid_length};
#[cfg(feature = "skinning-evaluation")]
use crate::math::transform_position;
use crate::math::{euler_to_quat, mul_quat, quat_slerp, quat_to_euler};
use crate::parse::f64_to_i64;
use crate::props::name_key;
use crate::search::{lower_bound_eq, unstable_sort, upper_bound_eq};
use crate::types::*;

#[cfg(feature = "skinning-evaluation")]
use crate::api::{add_blend_vertex_offsets, get_skin_vertex_matrix};
#[cfg(all(feature = "geometry-cache", feature = "skinning-evaluation"))]
use crate::cache::sample_geometry_cache_vec3;
#[cfg(feature = "skinning-evaluation")]
use crate::topology::{compute_normals, compute_topology, generate_normal_mapping};
use crate::update::{get_rotation, get_scale, get_transform, update_scene};

#[inline]
fn find_cubic_bezier_t(p1: f64, p2: f64, x0: f64) -> f64 {
    let p1_3 = p1 * 3.0;
    let p2_3 = p2 * 3.0;
    let a = p1_3 - p2_3 + 1.0;
    let b = p2_3 - p1_3 - p1_3;
    let c = p1_3;

    let a_3 = 3.0 * a;
    let b_2 = 2.0 * b;
    let mut t = x0;
    let mut x1;
    let mut t2;
    let mut t3;

    t2 = t * t;
    t3 = t2 * t;
    x1 = a * t3 + b * t2 + c * t - x0;
    t -= x1 / (a_3 * t2 + b_2 * t + c);

    t2 = t * t;
    t3 = t2 * t;
    x1 = a * t3 + b * t2 + c * t - x0;
    t -= x1 / (a_3 * t2 + b_2 * t + c);

    t2 = t * t;
    t3 = t2 * t;
    x1 = a * t3 + b * t2 + c * t - x0;
    t -= x1 / (a_3 * t2 + b_2 * t + c);

    // 4 ULP from 1.0
    const EPS: f64 = 8.881784197001252e-16;
    if x1.abs() <= EPS {
        return t;
    }

    for _ in 0..4 {
        t2 = t * t;
        t3 = t2 * t;
        x1 = a * t3 + b * t2 + c * t - x0;
        t -= x1 / (a_3 * t2 + b_2 * t + c);

        t2 = t * t;
        t3 = t2 * t;
        x1 = a * t3 + b * t2 + c * t - x0;
        t -= x1 / (a_3 * t2 + b_2 * t + c);

        if x1.abs() <= EPS {
            return t;
        }
    }

    t
}

fn extrapolate_curve(curve: &AnimCurve, real_time: f64, flags: u32) -> Real {
    let pre = real_time < curve.min_time;
    let (key, ext) = if pre { (&curve.keyframes[0], curve.pre_extrapolation) } else { (&curve.keyframes[curve.keyframes.len() - 1], curve.post_extrapolation) };

    if ext.mode == ExtrapolationMode::Constant {
        return key.value;
    } else if ext.mode == ExtrapolationMode::Slope {
        let tangent = if pre { key.right } else { key.left };
        return key.value + tangent.dy as f64 * ((real_time - key.time) / tangent.dx as f64);
    } else if ext.repeat_count == 0 {
        return key.value;
    }

    // Perform all operations in KTime ticks to be frame perfect
    let scale = curve.ktime_second as f64;
    let min_time = (curve.min_time * scale).round_ties_even();
    let max_time = (curve.max_time * scale).round_ties_even();
    let time = real_time * scale;

    let delta = if pre { min_time - time } else { time - max_time };
    let duration = max_time - min_time;

    #[allow(clippy::neg_cmp_op_on_partial_ord)] // NaN fails both, as in C
    if !(duration >= 1.0) {
        return key.value;
    }

    let rep = delta / duration;
    let mut rep_n = rep.floor();
    let mut rep_d = delta - rep_n * duration;

    if ext.repeat_count > 0 && rep_n >= ext.repeat_count as f64 {
        rep_n = (ext.repeat_count - 1) as f64;
        rep_d = duration;
    }

    if ext.mode == ExtrapolationMode::Mirror {
        let rep_parity = rep_n * 0.5 - (rep_n * 0.5).floor();
        if rep_parity <= 0.25 {
            rep_d = duration - rep_d;
        }
    }

    if pre {
        rep_d = duration - rep_d;
    }
    let new_time = (min_time + rep_d) / scale;

    let mut value = evaluate_curve_flags(curve, new_time, key.value, flags | EVALUATE_FLAG_NO_EXTRAPOLATION);

    if ext.mode == ExtrapolationMode::RepeatRelative {
        let keys = &curve.keyframes;
        let mut val_delta = keys[keys.len() - 1].value - keys[0].value;
        if pre {
            val_delta = -val_delta;
        }
        value += val_delta * (rep_n + 1.0);
    }

    value
}

pub fn evaluate_curve(curve: &AnimCurve, time: f64, default_value: Real) -> Real {
    evaluate_curve_flags(curve, time, default_value, 0)
}

pub fn evaluate_curve_flags(curve: &AnimCurve, time: f64, default_value: Real, flags: u32) -> Real {
    let keys = &curve.keyframes;
    if keys.len() <= 1 {
        return keys.first().map_or(default_value, |k| k.value);
    }

    if flags & EVALUATE_FLAG_NO_EXTRAPOLATION == 0 && (time < curve.min_time || time > curve.max_time) {
        return extrapolate_curve(curve, time, flags);
    }

    let mut begin = 0;
    let mut end = keys.len();
    while end - begin >= 8 {
        let mid = (begin + end) >> 1;
        if keys[mid].time <= time {
            begin = mid + 1;
        } else {
            end = mid;
        }
    }

    end = keys.len();
    while begin < end {
        let next = &keys[begin];
        if next.time <= time {
            begin += 1;
            continue;
        }

        if begin == 0 {
            return next.value;
        }

        let prev = &keys[begin - 1];

        if prev.time == time {
            return prev.value;
        }

        let rcp_delta = 1.0 / (next.time - prev.time);
        let t = (time - prev.time) * rcp_delta;

        return match prev.interpolation {
            Interpolation::ConstantPrev => prev.value,
            Interpolation::ConstantNext => next.value,
            Interpolation::Linear => prev.value * (1.0 - t) + next.value * t,
            Interpolation::Cubic => {
                let x1 = prev.right.dx as f64 * rcp_delta;
                let x2 = 1.0 - next.left.dx as f64 * rcp_delta;
                let t = find_cubic_bezier_t(x1, x2, t);

                let t2 = t * t;
                let t3 = t2 * t;
                let u = 1.0 - t;
                let u2 = u * u;
                let u3 = u2 * u;

                let y0 = prev.value;
                let y3 = next.value;
                let y1 = y0 + prev.right.dy as f64;
                let y2 = y3 - next.left.dy as f64;

                u3 * y0 + 3.0 * (u2 * t * y1 + u * t2 * y2) + t3 * y3
            }
        };
    }

    keys[keys.len() - 1].value
}

pub fn evaluate_anim_value_real(scene: &Scene, value: &AnimValue, time: f64) -> Real {
    evaluate_anim_value_real_flags(scene, value, time, 0)
}

pub fn evaluate_anim_value_vec3(scene: &Scene, value: &AnimValue, time: f64) -> Vec3 {
    evaluate_anim_value_vec3_flags(scene, value, time, 0)
}

pub fn evaluate_anim_value_real_flags(scene: &Scene, value: &AnimValue, time: f64, flags: u32) -> Real {
    let mut res = value.default_value.x;
    if let Some(c) = value.curves[0] {
        res = evaluate_curve_flags(&scene.anim_curves[c], time, res, flags);
    }
    res
}

pub fn evaluate_anim_value_vec3_flags(scene: &Scene, value: &AnimValue, time: f64, flags: u32) -> Vec3 {
    let mut res = value.default_value;
    if let Some(c) = value.curves[0] {
        res.x = evaluate_curve_flags(&scene.anim_curves[c], time, res.x, flags);
    }
    if let Some(c) = value.curves[1] {
        res.y = evaluate_curve_flags(&scene.anim_curves[c], time, res.y, flags);
    }
    if let Some(c) = value.curves[2] {
        res.z = evaluate_curve_flags(&scene.anim_curves[c], time, res.z, flags);
    }
    res
}

#[cfg(not(feature = "skinning-evaluation"))]
pub(crate) fn evaluate_skinning(_scene: &mut Scene, _time: f64, _load_caches: bool, _cache_opts: &GeometryCacheDataOpts) -> Res<()> {
    Err(crate::error::Error::new("Feature disabled").set_err_info("skinning-evaluation"))
}

#[cfg(all(not(feature = "geometry-cache"), feature = "skinning-evaluation"))]
fn sample_geometry_cache_vec3(_channel: &CacheChannel, _time: f64, _data: &mut [Vec3], _opts: &GeometryCacheDataOpts) -> usize {
    0
}

#[cfg(feature = "skinning-evaluation")]
pub(crate) fn evaluate_skinning(scene: &mut Scene, time: f64, load_caches: bool, cache_opts: &GeometryCacheDataOpts) -> Res<()> {
    let deformed = |mesh: &Mesh| !(mesh.blend_deformers.is_empty() && mesh.skin_deformers.is_empty() && (mesh.cache_deformers.is_empty() || !load_caches));

    let max_skinned_indices = scene.meshes.iter().filter(|m| deformed(m)).map(|m| m.num_indices).max().unwrap_or(0);
    let mut topo = vec![TopoEdge::default(); max_skinned_indices];

    for mesh_ix in 0..scene.meshes.len() {
        let mesh = &scene.meshes[mesh_ix];
        if !deformed(mesh) || mesh.num_vertices == 0 {
            continue;
        }

        let num_vertices = mesh.num_vertices;
        let mut result_pos = vec![Vec3::ZERO; num_vertices];
        let mut skinned_is_local = None;
        let mut cached_normal_values = None;

        let mut cached_position = false;
        let mut cached_normals = false;
        if load_caches && !mesh.cache_deformers.is_empty() {
            for &deformer_ix in &mesh.cache_deformers {
                let deformer = &scene.cache_deformers[deformer_ix];
                let Some(channel) = deformer.external_cache.as_deref().zip(deformer.external_channel).and_then(|(cache, ix)| cache.channels.get(ix)) else {
                    continue;
                };

                let interpretation = channel.interpretation;
                if (interpretation == CacheInterpretation::VertexPosition || interpretation == CacheInterpretation::Points) && !cached_position {
                    let num_read = sample_geometry_cache_vec3(channel, time, &mut result_pos, cache_opts);
                    if num_read == num_vertices {
                        skinned_is_local = Some(true);
                        cached_position = true;
                    }
                } else if interpretation == CacheInterpretation::VertexNormal && !cached_normals {
                    let num_normals = mesh.skinned_normal.values.len();
                    let mut normal_data = vec![Vec3::ZERO; num_normals];
                    let num_read = sample_geometry_cache_vec3(channel, time, &mut normal_data, cache_opts);
                    if num_read == num_normals {
                        cached_normals = true;
                        cached_normal_values = Some(normal_data);
                    }
                }
            }
        }

        if !cached_position {
            result_pos.copy_from_slice(&mesh.vertices[..num_vertices]);

            for &blend in &mesh.blend_deformers {
                add_blend_vertex_offsets(scene, &scene.blend_deformers[blend], &mut result_pos, 1.0);
            }

            if let Some(&skin_ix) = mesh.skin_deformers.first() {
                let fallback = mesh.element.instances.first().map(|&node| &scene.nodes[node].geometry_to_world);
                let skin = &scene.skin_deformers[skin_ix];
                for (i, pos) in result_pos.iter_mut().enumerate() {
                    let mat = get_skin_vertex_matrix(scene, skin, i, fallback);
                    *pos = transform_position(&mat, *pos);
                }

                skinned_is_local = Some(false);
            }
        }

        let mesh = &mut scene.meshes[mesh_ix];
        mesh.skinned_position.values = result_pos;
        if let Some(local) = skinned_is_local {
            mesh.skinned_is_local = local;
        }
        if let Some(values) = cached_normal_values {
            mesh.skinned_normal.values = values;
        }

        if !cached_normals {
            let mesh = &scene.meshes[mesh_ix];
            let num_indices = mesh.num_indices;
            let mut normal_indices = vec![0u32; num_indices];

            compute_topology(mesh, &mut topo[..num_indices]);
            let num_normals = generate_normal_mapping(mesh, &topo[..num_indices], &mut normal_indices, false);

            let mut normal_data = vec![Vec3::ZERO; num_normals];
            compute_normals(mesh, &mesh.skinned_position, &normal_indices, &mut normal_data);

            let mesh = &mut scene.meshes[mesh_ix];
            if num_normals == mesh.num_vertices {
                mesh.skinned_normal.unique_per_vertex = true;
            }
            mesh.generated_normals = true;
            mesh.skinned_normal.exists = true;
            mesh.skinned_normal.values = normal_data;
            mesh.skinned_normal.indices = normal_indices;
            mesh.skinned_normal.value_reals = 3;
        }
    }

    Ok(())
}

/// What evaluation reads of a `ufbx_anim`: ufbx narrows `prop_overrides` to one element's while evaluating a scene
#[derive(Clone, Copy)]
struct AnimView<'a> {
    layers: &'a [usize],
    override_layer_weights: &'a [Real],
    prop_overrides: &'a [PropOverride],
    ignore_connections: bool,
}

impl<'a> From<&'a Anim> for AnimView<'a> {
    fn from(anim: &'a Anim) -> AnimView<'a> {
        AnimView {
            layers: &anim.layers,
            override_layer_weights: &anim.override_layer_weights,
            prop_overrides: &anim.prop_overrides,
            ignore_connections: anim.ignore_connections,
        }
    }
}

use crate::finalize::find_prop_connection;

fn find_anim_prop_start(layer: &AnimLayer, element: ElementId) -> Option<usize> {
    let aprops = &layer.anim_props;
    lower_bound_eq(0, aprops.len(), 16, |i| aprops[i].element < element, |i| aprops[i].element == element)
}

fn override_less_than_prop(over: &PropOverride, element_id: u32, prop: &Prop) -> bool {
    if over.element_id != element_id {
        return over.element_id < element_id;
    }
    if over.internal_key != prop.internal_key {
        return over.internal_key < prop.internal_key;
    }
    // As ufbx: `strcmp()` taken as a bool, true for any differing name
    over.prop_name != prop.name
}

fn override_equals_to_prop(over: &PropOverride, element_id: u32, prop: &Prop) -> bool {
    over.element_id == element_id && over.internal_key == prop.internal_key && over.prop_name == prop.name
}

fn find_prop_override(overrides: &[PropOverride], element_id: u32, prop: &mut Prop) -> bool {
    let found = lower_bound_eq(
        0,
        overrides.len(),
        16,
        |i| override_less_than_prop(&overrides[i], element_id, prop),
        |i| override_equals_to_prop(&overrides[i], element_id, prop),
    );
    let Some(ix) = found else {
        return false;
    };

    let over = &overrides[ix];
    let clear_flags = prop_flags::NO_VALUE | prop_flags::NOT_FOUND;
    prop.flags = (prop.flags & !clear_flags) | prop_flags::OVERRIDDEN;
    prop.value_real_arr = [over.value.x, over.value.y, over.value.z, 0.0];
    prop.value_int = over.value_int;
    prop.value_str = over.value_str.clone();
    prop.value_blob = over.value_str.as_bytes().to_vec();
    true
}

fn find_element_prop_overrides(overrides: &[PropOverride], element_id: u32) -> &[PropOverride] {
    let count = overrides.len();
    let begin = lower_bound_eq(0, count, 32, |i| overrides[i].element_id < element_id, |i| overrides[i].element_id == element_id).unwrap_or(count);
    let end = upper_bound_eq(begin, count, 32, |i| overrides[i].element_id == element_id);
    &overrides[begin..end]
}

/// `ufbxi_anim_layer_combine_ctx` in ufbx.c
struct CombineCtx<'a> {
    scene: &'a Scene,
    anim: AnimView<'a>,
    element: ElementId,
    time: f64,
    rotation_order: RotationOrder,
    has_rotation_order: bool,
}

fn pow_abs(v: f64, e: f64) -> f64 {
    if e <= 0.0 {
        return 1.0;
    }
    if e >= 1.0 {
        return v;
    }
    let sign = if v < 0.0 { -1.0 } else { 1.0 };
    sign * (v * sign).powf(e)
}

fn combine_anim_layer(ctx: &mut CombineCtx, layer: &AnimLayer, weight: Real, prop_name: &str, result: &mut Vec3, value: &Vec3) {
    if layer.compose_rotation && layer.blended && prop_name == "Lcl Rotation" && !ctx.has_rotation_order {
        let rp = evaluate_prop_imp(ctx.scene, ctx.anim, ctx.element, "RotationOrder", ctx.time, 0);
        // 0 (XYZ) if the property is not found
        ctx.rotation_order =
            if (0..=RotationOrder::Spheric as i64).contains(&rp.value_int) { RotationOrder::from_index(rp.value_int) } else { RotationOrder::Xyz };
        ctx.has_rotation_order = true;
    }

    if layer.additive {
        if layer.compose_scale && prop_name == "Lcl Scaling" {
            result.x *= pow_abs(value.x, weight);
            result.y *= pow_abs(value.y, weight);
            result.z *= pow_abs(value.z, weight);
        } else if layer.compose_rotation && prop_name == "Lcl Rotation" {
            let a = euler_to_quat(*result, ctx.rotation_order);
            let mut b = euler_to_quat(*value, ctx.rotation_order);
            b = quat_slerp(Quat::IDENTITY, b, weight);
            let res = mul_quat(a, b);
            *result = quat_to_euler(res, ctx.rotation_order);
        } else {
            result.x += value.x * weight;
            result.y += value.y * weight;
            result.z += value.z * weight;
        }
    } else if layer.blended {
        let res_weight = 1.0 - weight;
        if layer.compose_scale && prop_name == "Lcl Scaling" {
            result.x = pow_abs(result.x, res_weight) * pow_abs(value.x, weight);
            result.y = pow_abs(result.y, res_weight) * pow_abs(value.y, weight);
            result.z = pow_abs(result.z, res_weight) * pow_abs(value.z, weight);
        } else if layer.compose_rotation && prop_name == "Lcl Rotation" {
            let a = euler_to_quat(*result, ctx.rotation_order);
            let b = euler_to_quat(*value, ctx.rotation_order);
            let res = quat_slerp(a, b, weight);
            *result = quat_to_euler(res, ctx.rotation_order);
        } else {
            result.x = result.x * res_weight + value.x * weight;
            result.y = result.y * res_weight + value.y * weight;
            result.z = result.z * res_weight + value.z * weight;
        }
    } else {
        *result = *value;
    }
}

fn anim_layer_might_contain_id(layer: &AnimLayer, id: u32) -> bool {
    let id_mask = layer.element_id_bitmask.len() as u32 - 1;
    let mut ok = id.wrapping_sub(layer.min_element_id) <= layer.max_element_id.wrapping_sub(layer.min_element_id);
    ok &= layer.element_id_bitmask[((id >> 5) & id_mask) as usize] & (1u32 << (id & 31)) != 0;
    ok
}

/// `ufbxi_evaluate_props` in ufbx.c
fn evaluate_props_imp(scene: &Scene, anim: AnimView, element: ElementId, time: f64, props: &mut [Prop], flags: u32) {
    let mut combine_ctx = CombineCtx { scene, anim, element, time, rotation_order: RotationOrder::Xyz, has_rotation_order: false };

    for (layer_ix, &layer_id) in anim.layers.iter().enumerate() {
        let layer = &scene.anim_layers[layer_id];
        if !anim_layer_might_contain_id(layer, element) {
            continue;
        }

        let mut weight = anim.override_layer_weights.get(layer_ix).copied().unwrap_or(layer.weight);
        if layer.weight_is_animated
            && layer.blended
            && let Some(weight_ix) = find_anim_prop_start(layer, layer.element.element_id)
        {
            let weight_value = &scene.anim_values[layer.anim_props[weight_ix].anim_value];
            weight = evaluate_anim_value_real_flags(scene, weight_value, time, flags) / 100.0;
            if weight < 0.0 {
                weight = 0.0;
            }
            if weight > 0.99999f32 as f64 {
                weight = 1.0;
            }
        }

        let Some(mut ap) = find_anim_prop_start(layer, element) else {
            continue;
        };
        let aprops = &layer.anim_props;

        for prop in props.iter_mut() {
            if prop.flags & prop_flags::OVERRIDDEN != 0 {
                continue;
            }

            // Connections override animation by default
            if prop.flags & prop_flags::CONNECTED != 0 && !anim.ignore_connections {
                continue;
            }

            while ap < aprops.len() && aprops[ap].element == element && aprops[ap].internal_key < prop.internal_key {
                ap += 1;
            }
            while ap < aprops.len() && aprops[ap].element == element && aprops[ap].prop_name.as_bytes() < prop.name.as_bytes() {
                ap += 1;
            }

            // As ufbx, which compares the interned name alone: past this element's props it may match the next one's
            if ap < aprops.len() && aprops[ap].prop_name == prop.name {
                let v = evaluate_anim_value_vec3_flags(scene, &scene.anim_values[aprops[ap].anim_value], time, flags);
                let mut value = prop.value_vec3();
                if layer_ix == 0 {
                    value = v;
                } else {
                    combine_anim_layer(&mut combine_ctx, layer, weight, &prop.name, &mut value, &v);
                }
                prop.value_real_arr[..3].copy_from_slice(&value.to_array());
            }
        }
    }

    for prop in props.iter_mut() {
        if prop.flags & prop_flags::OVERRIDDEN != 0 {
            continue;
        }
        prop.value_int = f64_to_i64(prop.value_real_arr[0]);
    }
}

fn evaluate_connected_prop(scene: &Scene, prop: &mut Prop, anim: AnimView, element: ElementId, name: &str, time: f64, flags: u32) {
    let mut conn = find_prop_connection(scene.element(element), name);

    let mut i = 0;
    while i < 1000
        && let Some(c) = conn
    {
        let Some(next) = find_prop_connection(scene.element(c.src), &c.src_prop) else {
            break;
        };
        conn = Some(next);
        i += 1;
    }

    if let Some(c) = conn
        && find_prop_connection(scene.element(c.src), &c.src_prop).is_none()
    {
        let ep = evaluate_prop_imp(scene, anim, c.src, &c.src_prop, time, flags);
        prop.value_real_arr = ep.value_real_arr;
        prop.value_int = ep.value_int;
        prop.value_str = ep.value_str;
        prop.value_blob = ep.value_blob;
    } else {
        // Connection not found, maybe it's animated?
        prop.flags &= !prop_flags::CONNECTED;
    }
}

struct PropIter<'a> {
    props: &'a [Prop],
    prop: usize,
    overs: &'a [PropOverride],
    over: usize,
}

impl<'a> PropIter<'a> {
    /// With `ufbxi_init_prop_iter_slow` in ufbx.c
    fn init_prop_iter(anim: AnimView<'a>, element: &'a Element) -> PropIter<'a> {
        let overs = if anim.prop_overrides.is_empty() { &[] } else { find_element_prop_overrides(anim.prop_overrides, element.element_id) };
        PropIter { props: &element.props.props, prop: 0, overs, over: 0 }
    }

    /// With `ufbxi_next_prop_slow` in ufbx.c
    fn next_prop(&mut self) -> Option<Cow<'a, Prop>> {
        let Some(over) = self.overs.get(self.over) else {
            let prop = self.props.get(self.prop)?;
            self.prop += 1;
            return Some(Cow::Borrowed(prop));
        };

        let prop = self.props.get(self.prop);
        let prop_key = prop.map_or(u32::MAX, |p| p.internal_key);
        let over_key = over.internal_key;
        let cmp = match prop {
            Some(p) if prop_key == over_key => p.name.as_bytes().cmp(over.prop_name.as_bytes()),
            _ => prop_key.cmp(&over_key),
        };

        if cmp != Ordering::Less {
            let v = over.value;
            let dst = Prop {
                name: over.prop_name.clone(),
                internal_key: over.internal_key,
                prop_type: PropType::Unknown,
                flags: prop_flags::OVERRIDDEN,
                value_str: over.value_str.clone(),
                value_blob: over.value_str.as_bytes().to_vec(),
                value_int: over.value_int,
                value_real_arr: [v.x, v.y, v.z, v.w],
            };
            self.over += 1;
            if cmp == Ordering::Equal {
                self.prop += 1;
            }
            Some(Cow::Owned(dst))
        } else {
            self.prop += 1;
            prop.map(Cow::Borrowed)
        }
    }
}

fn evaluated_props(props: Vec<Prop>, defaults: &Props) -> Props {
    Props { num_animated: props.len(), props, defaults: Some(Arc::new(defaults.clone())) }
}

/// `prop_names` must be sorted as properties are
fn evaluate_selected_props(scene: &Scene, anim: AnimView, element: ElementId, time: f64, prop_names: &[&str], flags: u32) -> Props {
    let elem = scene.element(element);
    let mut name = prop_names[0];
    let mut key = name_key(name.as_bytes());
    let mut props = Vec::new();

    let mut name_ix = 0;
    let mut iter = PropIter::init_prop_iter(anim, elem);
    while let Some(prop) = iter.next_prop() {
        while name_ix < prop_names.len() {
            if key > prop.internal_key {
                break;
            }
            if name == prop.name {
                if prop.flags & prop_flags::CONNECTED != 0 && !anim.ignore_connections {
                    let mut dst = prop.into_owned();
                    evaluate_connected_prop(scene, &mut dst, anim, element, name, time, flags);
                    props.push(dst);
                } else if prop.flags & (prop_flags::ANIMATED | prop_flags::OVERRIDDEN) != 0 {
                    props.push(prop.into_owned());
                }
                break;
            } else if name.as_bytes() < prop.name.as_bytes() {
                name_ix += 1;
                if name_ix < prop_names.len() {
                    name = prop_names[name_ix];
                    key = name_key(name.as_bytes());
                }
            } else {
                break;
            }
        }
    }

    evaluate_props_imp(scene, anim, element, time, &mut props, flags);
    evaluated_props(props, &elem.props)
}

/// `ufbx_evaluate_prop_flags_len` in ufbx.c
fn evaluate_prop_imp(scene: &Scene, anim: AnimView, element: ElementId, name: &str, time: f64, flags: u32) -> Prop {
    let elem = scene.element(element);
    let mut result = match elem.props.find(name) {
        Some(prop) => prop.clone(),
        None => Prop { name: name.to_string(), internal_key: name_key(name.as_bytes()), flags: prop_flags::NOT_FOUND, ..Default::default() },
    };

    if !anim.prop_overrides.is_empty() {
        find_prop_override(anim.prop_overrides, element, &mut result);
        return result;
    }

    if result.flags & (prop_flags::ANIMATED | prop_flags::CONNECTED) == 0 {
        return result;
    }

    if result.flags & prop_flags::CONNECTED != 0 && !anim.ignore_connections {
        evaluate_connected_prop(scene, &mut result, anim, element, name, time, flags);
    }

    evaluate_props_imp(scene, anim, element, time, std::slice::from_mut(&mut result), flags);
    result
}

/// Part of `ufbx_evaluate_props_flags` in ufbx.c
fn evaluate_element_props(scene: &Scene, anim: AnimView, element: ElementId, time: f64, max_props: usize, flags: u32) -> Vec<Prop> {
    let elem = scene.element(element);
    let mut props: Vec<Prop> = Vec::new();

    let mut iter = PropIter::init_prop_iter(anim, elem);
    while let Some(prop) = iter.next_prop() {
        if prop.flags & (prop_flags::ANIMATED | prop_flags::OVERRIDDEN | prop_flags::CONNECTED) == 0 {
            continue;
        }
        if props.len() >= max_props {
            break;
        }

        let mut dst = prop.into_owned();
        if dst.flags & prop_flags::CONNECTED != 0 && !anim.ignore_connections {
            let name = dst.name.clone();
            evaluate_connected_prop(scene, &mut dst, anim, element, &name, time, flags);
        }
        props.push(dst);
    }

    evaluate_props_imp(scene, anim, element, time, &mut props, flags);
    props
}

pub fn evaluate_prop(scene: &Scene, anim: &Anim, element: ElementId, name: &str, time: f64) -> Prop {
    evaluate_prop_flags(scene, anim, element, name, time, 0)
}

pub fn evaluate_prop_flags(scene: &Scene, anim: &Anim, element: ElementId, name: &str, time: f64, flags: u32) -> Prop {
    evaluate_prop_imp(scene, anim.into(), element, name, time, flags)
}

/// The animated, connected and overridden properties, the element's own as their defaults
pub fn evaluate_props(scene: &Scene, anim: &Anim, element: ElementId, time: f64) -> Props {
    evaluate_props_flags(scene, anim, element, time, 0)
}

pub fn evaluate_props_flags(scene: &Scene, anim: &Anim, element: ElementId, time: f64, flags: u32) -> Props {
    let props = evaluate_element_props(scene, anim.into(), element, time, usize::MAX, flags);
    evaluated_props(props, &scene.element(element).props)
}

const TRANSFORM_PROPS_ALL: [&str; 10] = [
    "Lcl Rotation",
    "Lcl Scaling",
    "Lcl Translation",
    "PostRotation",
    "PreRotation",
    "RotationOffset",
    "RotationOrder",
    "RotationPivot",
    "ScalingOffset",
    "ScalingPivot",
];

const TRANSFORM_PROPS_ROTATION: [&str; 4] = ["Lcl Rotation", "PostRotation", "PreRotation", "RotationOrder"];

const TRANSFORM_PROPS_SCALE: [&str; 1] = ["Lcl Scaling"];

const TRANSFORM_PROPS_ROTATION_SCALE: [&str; 5] = ["Lcl Rotation", "Lcl Scaling", "PostRotation", "PreRotation", "RotationOrder"];

pub fn evaluate_transform(scene: &Scene, anim: &Anim, node: usize, time: f64) -> Transform {
    evaluate_transform_flags(scene, anim, node, time, 0)
}

pub fn evaluate_transform_flags(scene: &Scene, anim: &Anim, node: usize, time: f64, flags: u32) -> Transform {
    let view = AnimView::from(anim);
    let node_ix = node;
    let node = &scene.nodes[node_ix];
    if node.is_root {
        return node.local_transform;
    }

    let mut flags = flags;
    if flags & TRANSFORM_FLAG_EXPLICIT_INCLUDES == 0 {
        flags |= TRANSFORM_FLAG_INCLUDE_ROTATION | TRANSFORM_FLAG_INCLUDE_SCALE | TRANSFORM_FLAG_INCLUDE_TRANSLATION;
    }

    let components = flags & (TRANSFORM_FLAG_INCLUDE_ROTATION | TRANSFORM_FLAG_INCLUDE_SCALE | TRANSFORM_FLAG_INCLUDE_TRANSLATION);
    let prop_names: &[&str] = if components == TRANSFORM_FLAG_INCLUDE_ROTATION | TRANSFORM_FLAG_INCLUDE_SCALE {
        &TRANSFORM_PROPS_ROTATION_SCALE
    } else if components == TRANSFORM_FLAG_INCLUDE_ROTATION {
        &TRANSFORM_PROPS_ROTATION
    } else if components == TRANSFORM_FLAG_INCLUDE_SCALE {
        &TRANSFORM_PROPS_SCALE
    } else if components == 0 {
        return Transform::IDENTITY;
    } else {
        &TRANSFORM_PROPS_ALL
    };

    let mut translation_scale = None;
    let mut scale_factor = Vec3::ONE;
    let mut use_scale_factor = false;

    if let Some(parent_ix) = node.parent
        && flags & (TRANSFORM_FLAG_INCLUDE_SCALE | TRANSFORM_FLAG_INCLUDE_TRANSLATION) != 0
    {
        let parent = &scene.nodes[parent_ix];

        if flags & TRANSFORM_FLAG_IGNORE_COMPONENTWISE_SCALE == 0
            && let Some(inherit_ix) = parent.inherit_scale_node
        {
            if node.is_scale_helper {
                use_scale_factor = true;
            }

            let mut p = Some(inherit_ix);
            while let Some(pi) = p
                && let Some(helper) = scene.nodes[pi].scale_helper
            {
                let scale = evaluate_prop_imp(scene, view, scene.nodes[helper].element.element_id, "Lcl Scaling", time, 0);
                scale_factor.x *= scale.value_real_arr[0];
                scale_factor.y *= scale.value_real_arr[1];
                scale_factor.z *= scale.value_real_arr[2];
                p = scene.nodes[pi].inherit_scale_node;
            }
        }

        if let Some(helper) = parent.scale_helper
            && flags & TRANSFORM_FLAG_IGNORE_SCALE_HELPER == 0
        {
            let helper_scale = evaluate_prop_imp(scene, view, scene.nodes[helper].element.element_id, "Lcl Scaling", time, 0);
            let mut value = helper_scale.value_vec3();
            if helper_scale.flags & prop_flags::NOT_FOUND != 0 {
                value = Vec3::ONE;
            }
            value.x *= scale_factor.x;
            value.y *= scale_factor.y;
            value.z *= scale_factor.z;
            translation_scale = Some(value);
        }
    }

    let mut eval_flags = 0;
    if flags & TRANSFORM_FLAG_NO_EXTRAPOLATION != 0 {
        eval_flags |= EVALUATE_FLAG_NO_EXTRAPOLATION;
    }

    let props = evaluate_selected_props(scene, view, node.element.element_id, time, prop_names, eval_flags);
    let order = RotationOrder::from_index(props.find_enum("RotationOrder", RotationOrder::Xyz as i64, RotationOrder::Spheric as i64));

    let mut transform = if components & TRANSFORM_FLAG_INCLUDE_TRANSLATION != 0 {
        get_transform(&props, order, node, translation_scale.as_ref())
    } else {
        Transform {
            translation: Vec3::ZERO,
            rotation: if components & TRANSFORM_FLAG_INCLUDE_ROTATION != 0 { get_rotation(&props, order, node) } else { Quat::IDENTITY },
            scale: if components & TRANSFORM_FLAG_INCLUDE_SCALE != 0 { get_scale(&props, node) } else { Vec3::ONE },
        }
    };

    if use_scale_factor {
        transform.scale.x *= scale_factor.x;
        transform.scale.y *= scale_factor.y;
        transform.scale.z *= scale_factor.z;
    }
    transform
}

pub fn evaluate_blend_weight(scene: &Scene, anim: &Anim, channel: usize, time: f64) -> Real {
    evaluate_blend_weight_flags(scene, anim, channel, time, 0)
}

pub fn evaluate_blend_weight_flags(scene: &Scene, anim: &Anim, channel: usize, time: f64, flags: u32) -> Real {
    let channel = &scene.blend_channels[channel];
    let props = evaluate_selected_props(scene, anim.into(), channel.element.element_id, time, &["DeformPercent"], flags);
    props.real("DeformPercent", channel.weight * 100.0) * 0.01
}

pub fn evaluate_scene(scene: &Scene, anim: &Anim, time: f64, opts: &EvaluateOpts) -> Res<Scene> {
    evaluate_imp(scene, anim, time, opts).map_err(|e| e.fix_error_type("Failed to evaluate"))
}

fn evaluate_imp(src: &Scene, anim: &Anim, time: f64, opts: &EvaluateOpts) -> Res<Scene> {
    let mut scene = src.clone();

    let overrides = &anim.prop_overrides;
    let mut over = 0;

    // In element order on the copy, as ufbx, so connected properties read the elements already evaluated
    for id in 0..scene.elements.len() as u32 {
        let elem = scene.element(id);
        let mut num_animated = elem.props.num_animated;
        let mut num_override = 0;

        while over < overrides.len() && overrides[over].element_id == elem.element_id {
            num_override += 1;
            over += 1;
        }

        num_animated += num_override;
        if num_animated == 0 {
            continue;
        }

        let view = AnimView { prop_overrides: &overrides[over - num_override..over], ..AnimView::from(anim) };
        let props = evaluate_element_props(&scene, view, elem.element_id, time, num_animated, opts.evaluate_flags);
        scene.element_mut(id).props = evaluated_props(props, &src.element(id).props);
    }

    update_scene(&mut scene, false, &anim.transform_overrides);

    if opts.evaluate_skinning {
        let cache_opts = GeometryCacheDataOpts { open_file_cb: opts.open_file_cb.clone(), ..Default::default() };
        evaluate_skinning(&mut scene, time, opts.load_external_files && opts.evaluate_caches, &cache_opts)?;
    }

    Ok(scene)
}

fn check_string(s: &str) -> Res<()> {
    check!(utf8_valid_length(s.as_bytes()) == s.len(), "Invalid UTF-8");
    Ok(())
}

fn prop_override_less(a: &PropOverride, b: &PropOverride) -> bool {
    if a.element_id != b.element_id {
        return a.element_id < b.element_id;
    }
    if a.internal_key != b.internal_key {
        return a.internal_key < b.internal_key;
    }
    a.prop_name.as_bytes() < b.prop_name.as_bytes()
}

/// C's `(int64_t)` cast as x86-64 executes it: NaN and values out of range give `i64::MIN`
fn trunc_i64(v: f64) -> i64 {
    if (-9223372036854775808.0..9223372036854775808.0).contains(&v) { v as i64 } else { i64::MIN }
}

pub fn create_anim(scene: &Scene, opts: &AnimOpts) -> Res<Anim> {
    create_anim_imp(scene, opts).map_err(|e| e.fix_error_type("Failed to create anim"))
}

fn create_anim_imp(scene: &Scene, opts: &AnimOpts) -> Res<Anim> {
    let mut anim = Anim { ignore_connections: opts.ignore_connections, custom: true, ..Default::default() };

    let num_layers = opts.layer_ids.len();
    if !opts.override_layer_weights.is_empty() {
        check!(opts.override_layer_weights.len() == num_layers, "override_layer_weights[] count must match layer_ids[] count");
        anim.override_layer_weights = opts.override_layer_weights.clone();
    }

    for &index in &opts.layer_ids {
        check!((index as usize) < scene.anim_layers.len(), "layer_ids out of bounds");
        anim.layers.push(index as usize);
    }

    if !opts.prop_overrides.is_empty() {
        let mut overrides = Vec::with_capacity(opts.prop_overrides.len());
        for src in &opts.prop_overrides {
            let mut dst = PropOverride { element_id: src.element_id, value: src.value, value_int: src.value_int, ..Default::default() };

            if dst.value.x != 0.0 && dst.value_int == 0 {
                dst.value_int = trunc_i64(dst.value.x);
            } else if dst.value_int != 0 && dst.value.x == 0.0 {
                dst.value.x = dst.value_int as Real;
            }

            check_string(&src.prop_name)?;
            check_string(&src.value_str)?;
            dst.prop_name = src.prop_name.clone();
            dst.value_str = src.value_str.clone();

            dst.internal_key = name_key(dst.prop_name.as_bytes());
            overrides.push(dst);
        }

        // ufbx sorts by name first only to share equal name strings, which leaves no trace in the result here
        unstable_sort(&mut overrides, prop_override_less);

        for pair in overrides.windows(2) {
            let (prev, next) = (&pair[0], &pair[1]);
            if prev.element_id == next.element_id && prev.prop_name == next.prop_name {
                return Err(Error::new("Duplicate override").set_err_info(format!("element {} prop \"{}\"", prev.element_id, prev.prop_name)));
            }
        }
        anim.prop_overrides = overrides;
    }

    if !opts.transform_overrides.is_empty() {
        anim.transform_overrides = opts.transform_overrides.clone();
        // `ufbxi_transform_override_less` in ufbx.c
        unstable_sort(&mut anim.transform_overrides, |a, b| a.node_id < b.node_id);
    }

    Ok(anim)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorType;

    const KTIME: i64 = 46186158000;

    fn key(time: f64, value: Real, interpolation: Interpolation) -> Keyframe {
        Keyframe { time, value, interpolation, ..Default::default() }
    }

    fn curve(keyframes: Vec<Keyframe>) -> AnimCurve {
        let min_time = keyframes.first().map_or(0.0, |k| k.time);
        let max_time = keyframes.last().map_or(0.0, |k| k.time);
        AnimCurve { keyframes, min_time, max_time, ktime_second: KTIME, ..Default::default() }
    }

    fn ramp(pre: Extrapolation, post: Extrapolation) -> AnimCurve {
        let mut c = curve(vec![key(0.0, 0.0, Interpolation::Linear), key(1.0, 10.0, Interpolation::Linear)]);
        c.pre_extrapolation = pre;
        c.post_extrapolation = post;
        c.keyframes[0].right = Tangent { dx: 0.5, dy: 5.0 };
        c.keyframes[1].left = Tangent { dx: 0.5, dy: 5.0 };
        c
    }

    fn ext(mode: ExtrapolationMode, repeat_count: i32) -> Extrapolation {
        Extrapolation { mode, repeat_count }
    }

    #[test]
    fn curves_interpolate_by_the_earlier_key() {
        let mut c = curve(vec![
            key(0.0, 1.0, Interpolation::ConstantPrev),
            key(1.0, 2.0, Interpolation::ConstantNext),
            key(2.0, 4.0, Interpolation::Linear),
            key(3.0, 8.0, Interpolation::Cubic),
            key(4.0, 9.0, Interpolation::Linear),
        ]);
        c.keyframes[3].right = Tangent { dx: 1.0 / 3.0, dy: 0.0 };
        c.keyframes[4].left = Tangent { dx: 1.0 / 3.0, dy: 0.0 };

        assert_eq!(evaluate_curve(&c, 0.5, -1.0), 1.0);
        assert_eq!(evaluate_curve(&c, 1.5, -1.0), 4.0);
        assert_eq!(evaluate_curve(&c, 2.25, -1.0), 5.0);
        assert_eq!(evaluate_curve(&c, 3.0, -1.0), 8.0);
        assert!((evaluate_curve(&c, 3.5, -1.0) - 8.5).abs() < 1e-12);
        let early = evaluate_curve(&c, 3.1, -1.0);
        assert!(early > 8.0 && early < 8.1, "flat tangents ease in: {early}");
        assert_eq!(evaluate_curve(&c, 4.0, -1.0), 9.0);
    }

    #[test]
    fn curves_with_one_key_or_none() {
        assert_eq!(evaluate_curve(&curve(vec![]), 1.0, 7.0), 7.0);
        assert_eq!(evaluate_curve(&curve(vec![key(5.0, 3.0, Interpolation::Linear)]), 1.0, 7.0), 3.0);
    }

    #[test]
    fn curves_extrapolate_constant_and_by_slope() {
        let c = ramp(ext(ExtrapolationMode::Constant, -1), ext(ExtrapolationMode::Constant, -1));
        assert_eq!(evaluate_curve(&c, -1.0, 0.0), 0.0);
        assert_eq!(evaluate_curve(&c, 2.0, 0.0), 10.0);

        let c = ramp(ext(ExtrapolationMode::Slope, -1), ext(ExtrapolationMode::Slope, -1));
        assert_eq!(evaluate_curve(&c, -1.0, 0.0), -10.0);
        assert_eq!(evaluate_curve(&c, 2.0, 0.0), 20.0);
        assert_eq!(evaluate_curve_flags(&c, 2.0, 0.0, EVALUATE_FLAG_NO_EXTRAPOLATION), 10.0);
        assert_eq!(evaluate_curve_flags(&c, -1.0, 0.0, EVALUATE_FLAG_NO_EXTRAPOLATION), 0.0);
    }

    #[test]
    fn curves_extrapolate_by_repeating() {
        let c = ramp(ext(ExtrapolationMode::Repeat, -1), ext(ExtrapolationMode::Repeat, -1));
        assert_eq!(evaluate_curve(&c, 1.25, 0.0), 2.5);
        assert_eq!(evaluate_curve(&c, 3.25, 0.0), 2.5);
        assert_eq!(evaluate_curve(&c, -0.25, 0.0), 7.5);

        let c = ramp(ext(ExtrapolationMode::Mirror, -1), ext(ExtrapolationMode::Mirror, -1));
        assert_eq!(evaluate_curve(&c, 1.25, 0.0), 7.5);
        assert_eq!(evaluate_curve(&c, 2.25, 0.0), 2.5);
        assert_eq!(evaluate_curve(&c, -0.25, 0.0), 2.5);

        let c = ramp(ext(ExtrapolationMode::RepeatRelative, -1), ext(ExtrapolationMode::RepeatRelative, -1));
        assert_eq!(evaluate_curve(&c, 1.25, 0.0), 12.5);
        assert_eq!(evaluate_curve(&c, 2.25, 0.0), 22.5);
        assert_eq!(evaluate_curve(&c, -0.25, 0.0), -2.5);
    }

    #[test]
    fn curves_repeat_only_so_many_times() {
        let c = ramp(ext(ExtrapolationMode::Repeat, 0), ext(ExtrapolationMode::Repeat, 0));
        assert_eq!(evaluate_curve(&c, 1.25, 0.0), 10.0);
        assert_eq!(evaluate_curve(&c, -0.25, 0.0), 0.0);

        let c = ramp(ext(ExtrapolationMode::Repeat, 2), ext(ExtrapolationMode::Repeat, 2));
        assert_eq!(evaluate_curve(&c, 1.25, 0.0), 2.5);
        assert_eq!(evaluate_curve(&c, 5.5, 0.0), 10.0);
        assert_eq!(evaluate_curve(&c, -5.5, 0.0), 0.0);

        let c = ramp(ext(ExtrapolationMode::RepeatRelative, 2), ext(ExtrapolationMode::RepeatRelative, 2));
        assert_eq!(evaluate_curve(&c, 5.5, 0.0), 30.0);
    }

    struct Built {
        scene: Scene,
        anim: Anim,
    }

    fn prop(name: &str, flags: u32, v: [Real; 3]) -> Prop {
        Prop { name: name.to_string(), internal_key: name_key(name.as_bytes()), flags, value_real_arr: [v[0], v[1], v[2], 0.0], ..Default::default() }
    }

    fn element(id: ElementId, element_type: ElementType, typed_id: u32, mut props: Vec<Prop>) -> Element {
        props.sort_by(crate::props::prop_less);
        let num_animated = props.iter().filter(|p| p.flags & (prop_flags::ANIMATED | prop_flags::CONNECTED) != 0).count();
        Element { name: format!("e{id}"), props: Props { props, num_animated, defaults: None }, element_id: id, typed_id, element_type, ..Default::default() }
    }

    fn layer(id: ElementId, typed_id: u32, weight: Real, additive: bool, aprops: Vec<AnimProp>) -> AnimLayer {
        let mut layer = AnimLayer {
            element: element(id, ElementType::AnimLayer, typed_id, vec![]),
            weight,
            blended: additive,
            additive,
            min_element_id: u32::MAX,
            ..Default::default()
        };
        for ap in &aprops {
            layer.min_element_id = layer.min_element_id.min(ap.element);
            layer.max_element_id = layer.max_element_id.max(ap.element);
            layer.element_id_bitmask[((ap.element >> 5) & 3) as usize] |= 1 << (ap.element & 31);
        }
        layer.anim_props = aprops;
        layer
    }

    fn aprop(element: ElementId, name: &str, anim_value: usize) -> AnimProp {
        AnimProp { element, internal_key: name_key(name.as_bytes()), prop_name: name.to_string(), anim_value }
    }

    fn value(id: ElementId, typed_id: u32, default_value: Vec3, curve: Option<usize>) -> AnimValue {
        AnimValue { element: element(id, ElementType::AnimValue, typed_id, vec![]), default_value, curves: [curve, None, None] }
    }

    fn built() -> Built {
        let mut scene = Scene::default();
        let anim_flag = prop_flags::ANIMATED | prop_flags::ANIMATABLE;

        scene.nodes.push(Node {
            element: element(
                0,
                ElementType::Node,
                0,
                vec![prop("Lcl Translation", anim_flag, [1.0, 2.0, 3.0]), prop("Visibility", prop_flags::ANIMATABLE, [1.0, 0.0, 0.0])],
            ),
            ..Default::default()
        });
        scene.anim_layers.push(layer(1, 0, 1.0, false, vec![aprop(0, "Lcl Translation", 0), aprop(8, "DeformPercent", 2)]));
        scene.anim_values.push(value(2, 0, Vec3::new(1.0, 2.0, 3.0), Some(0)));
        let mut ramp = curve(vec![key(0.0, 0.0, Interpolation::Linear), key(1.0, 10.0, Interpolation::Linear)]);
        ramp.element = element(3, ElementType::AnimCurve, 0, vec![]);
        scene.anim_curves.push(ramp);

        let mut connected =
            Node { element: element(4, ElementType::Node, 1, vec![prop("Lcl Translation", prop_flags::CONNECTED, [0.0, 0.0, 0.0])]), ..Default::default() };
        connected.element.connections_dst.push(Connection { src: 0, dst: 4, src_prop: "Lcl Translation".to_string(), dst_prop: "Lcl Translation".to_string() });
        scene.nodes.push(connected);

        scene.anim_layers.push(layer(5, 1, 0.5, true, vec![aprop(0, "Lcl Translation", 1)]));
        scene.anim_values.push(value(6, 1, Vec3::new(2.0, 0.0, 0.0), None));
        scene.anim_values.push(value(7, 2, Vec3::new(50.0, 0.0, 0.0), Some(1)));
        let mut weights = curve(vec![key(0.0, 0.0, Interpolation::Linear), key(1.0, 100.0, Interpolation::Linear)]);
        weights.element = element(9, ElementType::AnimCurve, 1, vec![]);
        scene.anim_curves.push(weights);
        scene.blend_channels.push(BlendChannel {
            element: element(8, ElementType::BlendChannel, 0, vec![prop("DeformPercent", anim_flag, [0.0, 0.0, 0.0])]),
            weight: 0.0,
            ..Default::default()
        });

        let refs = [
            (ElementType::Node, 0),
            (ElementType::AnimLayer, 0),
            (ElementType::AnimValue, 0),
            (ElementType::AnimCurve, 0),
            (ElementType::Node, 1),
            (ElementType::AnimLayer, 1),
            (ElementType::AnimValue, 1),
            (ElementType::AnimValue, 2),
            (ElementType::BlendChannel, 0),
            (ElementType::AnimCurve, 1),
        ];
        scene.elements = refs.iter().map(|&(element_type, index)| ElementRef { element_type, index }).collect();

        let anim = Anim { layers: vec![0], ..Default::default() };
        Built { scene, anim }
    }

    #[test]
    fn props_take_their_animation() {
        let Built { scene, anim } = built();

        let p = evaluate_prop(&scene, &anim, 0, "Lcl Translation", 0.5);
        assert_eq!(p.value_vec3(), Vec3::new(5.0, 2.0, 3.0));
        assert_eq!(p.value_int, 5);

        let p = evaluate_prop(&scene, &anim, 0, "Visibility", 0.5);
        assert_eq!(p.value_real(), 1.0);
        assert_eq!(p.flags & prop_flags::ANIMATED, 0);

        let p = evaluate_prop(&scene, &anim, 0, "Missing", 0.5);
        assert_eq!((p.name.as_str(), p.flags), ("Missing", prop_flags::NOT_FOUND));

        let props = evaluate_props(&scene, &anim, 0, 0.25);
        assert_eq!(props.props.len(), 1);
        assert_eq!(props.num_animated, 1);
        assert_eq!(props.props[0].value_vec3(), Vec3::new(2.5, 2.0, 3.0));
        assert_eq!(props.find("Visibility").map(|p| p.value_real()), Some(1.0));
    }

    #[test]
    fn layers_combine_and_connections_follow_their_source() {
        let Built { scene, mut anim } = built();
        anim.layers = vec![0, 1];

        let p = evaluate_prop(&scene, &anim, 0, "Lcl Translation", 0.5);
        assert_eq!(p.value_vec3(), Vec3::new(6.0, 2.0, 3.0));

        anim.override_layer_weights = vec![1.0, 1.0];
        let p = evaluate_prop(&scene, &anim, 0, "Lcl Translation", 0.5);
        assert_eq!(p.value_vec3(), Vec3::new(7.0, 2.0, 3.0));

        let p = evaluate_prop(&scene, &anim, 4, "Lcl Translation", 0.5);
        assert_eq!(p.value_vec3(), Vec3::new(7.0, 2.0, 3.0));
        assert_ne!(p.flags & prop_flags::CONNECTED, 0);

        anim.ignore_connections = true;
        let p = evaluate_prop(&scene, &anim, 4, "Lcl Translation", 0.5);
        assert_eq!(p.value_vec3(), Vec3::ZERO);
    }

    #[test]
    fn a_layer_lends_the_next_elements_prop_as_ufbx_does() {
        // The C library gives node A (4, 202, 303) on the same layers read from an FBX file
        let anim_flag = prop_flags::ANIMATED | prop_flags::ANIMATABLE;
        let node = |id, typed_id, props| Node { element: element(id, ElementType::Node, typed_id, props), ..Default::default() };
        let mut scene = Scene::default();
        scene.nodes.push(node(0, 0, vec![prop("Lcl Rotation", anim_flag, [0.0; 3]), prop("Lcl Translation", anim_flag, [0.0; 3])]));
        scene.nodes.push(node(1, 1, vec![prop("Lcl Translation", anim_flag, [0.0; 3])]));
        scene.anim_layers.push(layer(2, 0, 1.0, true, vec![aprop(0, "Lcl Rotation", 0), aprop(0, "Lcl Translation", 1)]));
        scene.anim_layers.push(layer(3, 1, 1.0, true, vec![aprop(0, "Lcl Rotation", 2), aprop(1, "Lcl Translation", 3)]));
        let defaults = [Vec3::new(10.0, 20.0, 30.0), Vec3::new(1.0, 2.0, 3.0), Vec3::new(5.0, 0.0, 0.0), Vec3::new(3.0, 200.0, 300.0)];
        for (i, d) in defaults.into_iter().enumerate() {
            scene.anim_values.push(value(4 + i as u32, i as u32, d, None));
        }
        let mut refs = vec![(ElementType::Node, 0), (ElementType::Node, 1), (ElementType::AnimLayer, 0), (ElementType::AnimLayer, 1)];
        refs.extend((0..4).map(|i| (ElementType::AnimValue, i)));
        scene.elements = refs.into_iter().map(|(element_type, index)| ElementRef { element_type, index }).collect();
        let anim = Anim { layers: vec![0, 1], ..Default::default() };

        assert_eq!(evaluate_prop(&scene, &anim, 0, "Lcl Translation", 0.5).value_vec3(), Vec3::new(4.0, 202.0, 303.0));
        assert_eq!(evaluate_prop(&scene, &anim, 1, "Lcl Translation", 0.5).value_vec3(), Vec3::new(3.0, 200.0, 300.0));
    }

    #[test]
    fn blend_weights_are_the_deform_percent() {
        let Built { scene, anim } = built();
        assert_eq!(evaluate_blend_weight(&scene, &anim, 0, 0.25), 0.25);
        assert_eq!(evaluate_blend_weight(&scene, &Anim::default(), 0, 0.25), 0.0);
    }

    #[test]
    fn overrides_replace_values_and_stop_animation() {
        let Built { scene, .. } = built();
        let desc = |element_id, name: &str, x: Real, int: i64| PropOverrideDesc {
            element_id,
            prop_name: name.to_string(),
            value: Vec4::new(x, 0.0, 0.0, 9.0),
            value_int: int,
            ..Default::default()
        };
        let opts = AnimOpts {
            layer_ids: vec![0],
            prop_overrides: vec![desc(4, "Lcl Translation", 3.0, 0), desc(0, "Visibility", 0.0, 2), desc(0, "Custom", 2.5, 0)],
            ..Default::default()
        };
        let anim = create_anim(&scene, &opts).unwrap();
        assert!(anim.custom);
        let order: Vec<(u32, &str, Real, i64)> = anim.prop_overrides.iter().map(|o| (o.element_id, o.prop_name.as_str(), o.value.x, o.value_int)).collect();
        assert_eq!(order, [(0, "Custom", 2.5, 2), (0, "Visibility", 2.0, 2), (4, "Lcl Translation", 3.0, 3)]);

        let p = evaluate_prop(&scene, &anim, 0, "Visibility", 0.5);
        assert_eq!((p.value_real_arr, p.value_int), ([2.0, 0.0, 0.0, 0.0], 2));
        assert_ne!(p.flags & prop_flags::OVERRIDDEN, 0);

        // As ufbx: with any override in the anim, evaluate_prop returns the static value
        let p = evaluate_prop(&scene, &anim, 0, "Lcl Translation", 0.5);
        assert_eq!(p.value_vec3(), Vec3::new(1.0, 2.0, 3.0));

        let props = evaluate_props(&scene, &anim, 0, 0.5);
        let names: Vec<&str> = props.props.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Custom", "Lcl Translation", "Visibility"]);
        assert_eq!(props.props[0].value_real_arr, [2.5, 0.0, 0.0, 9.0]);
        assert_eq!(props.props[1].value_vec3(), Vec3::new(5.0, 2.0, 3.0));
    }

    #[test]
    fn create_anim_checks_its_options() {
        let Built { scene, .. } = built();
        let err = |opts: AnimOpts| create_anim(&scene, &opts).unwrap_err();

        let e = err(AnimOpts { layer_ids: vec![2], ..Default::default() });
        assert_eq!(e.description, "layer_ids out of bounds");

        let e = err(AnimOpts { layer_ids: vec![0], override_layer_weights: vec![1.0, 1.0], ..Default::default() });
        assert_eq!(e.description, "override_layer_weights[] count must match layer_ids[] count");

        let over = |name: &str| PropOverrideDesc { element_id: 3, prop_name: name.to_string(), ..Default::default() };
        let e = err(AnimOpts { prop_overrides: vec![over("A"), over("B"), over("A")], ..Default::default() });
        assert_eq!((e.kind, e.description.as_str(), e.info.as_str()), (ErrorType::DuplicateOverride, "Duplicate override", "element 3 prop \"A\""));

        let e = err(AnimOpts { prop_overrides: vec![over("A\0")], ..Default::default() });
        assert_eq!(e.kind, ErrorType::InvalidUtf8);

        let t = |node_id, x| TransformOverride { node_id, transform: Transform { translation: Vec3::new(x, 0.0, 0.0), ..Transform::IDENTITY } };
        let anim = create_anim(&scene, &AnimOpts { transform_overrides: vec![t(5, 1.0), t(1, 2.0), t(3, 3.0)], ..Default::default() }).unwrap();
        let ids: Vec<u32> = anim.transform_overrides.iter().map(|t| t.node_id).collect();
        assert_eq!(ids, [1, 3, 5]);
    }

    #[test]
    fn heap_sort_leaves_equal_entries_as_ufbx_does() {
        let mut v = [(1, 'a'), (0, 'b'), (1, 'c'), (0, 'd'), (1, 'e')];
        unstable_sort(&mut v, |a, b| a.0 < b.0);
        assert_eq!(v, [(0, 'b'), (0, 'd'), (1, 'c'), (1, 'a'), (1, 'e')]);
    }

    #[test]
    fn transforms_of_the_root_and_of_nothing() {
        let Built { mut scene, anim } = built();
        scene.nodes[0].is_root = true;
        scene.nodes[0].local_transform.translation = Vec3::new(4.0, 0.0, 0.0);
        assert_eq!(evaluate_transform(&scene, &anim, 0, 0.5), scene.nodes[0].local_transform);
        assert_eq!(evaluate_transform_flags(&scene, &anim, 1, 0.5, TRANSFORM_FLAG_EXPLICIT_INCLUDES), Transform::IDENTITY);
    }

    #[test]
    fn transforms_take_the_animated_translation() {
        let Built { scene, anim } = built();
        let t = evaluate_transform(&scene, &anim, 0, 0.5);
        assert_eq!(t.translation, Vec3::new(5.0, 2.0, 3.0));
        assert_eq!(t.rotation, Quat::IDENTITY);
        assert_eq!(t.scale, Vec3::ONE);
    }

    #[test]
    fn scenes_evaluate_to_a_copy() {
        let Built { scene, anim } = built();
        let out = evaluate_scene(&scene, &anim, 0.5, &EvaluateOpts::default()).unwrap();
        let props = &out.nodes[0].element.props;
        assert_eq!(props.props.len(), 1);
        assert_eq!(props.props[0].value_vec3(), Vec3::new(5.0, 2.0, 3.0));
        assert_eq!(props.defaults.as_deref(), Some(&scene.nodes[0].element.props));
        assert_eq!(scene.nodes[0].element.props.props[0].value_vec3(), Vec3::new(1.0, 2.0, 3.0));
    }
}
