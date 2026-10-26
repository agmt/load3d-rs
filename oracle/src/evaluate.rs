use std::cell::OnceCell;

use load3d as rs;

use crate::common::{both, same, same_transform};
use crate::runner::{Args, Child, Job, Section, Tally, corpus_jobs};
use crate::scene::{RH_Z, axes, compare_scenes, record};

const MODES: [(ufbx::ExtrapolationMode, rs::ExtrapolationMode); 5] = [
    (ufbx::ExtrapolationMode::Constant, rs::ExtrapolationMode::Constant),
    (ufbx::ExtrapolationMode::Repeat, rs::ExtrapolationMode::Repeat),
    (ufbx::ExtrapolationMode::Mirror, rs::ExtrapolationMode::Mirror),
    (ufbx::ExtrapolationMode::Slope, rs::ExtrapolationMode::Slope),
    (ufbx::ExtrapolationMode::RepeatRelative, rs::ExtrapolationMode::RepeatRelative),
];

const TRANSFORM_FLAGS: [u32; 13] = [0, 0x1, 0x2, 0x3, 0x24, 0x44, 0x64, 0x14, 0x34, 0x54, 0x4, 0x80, 0x81];

fn same_prop(c: &ufbx::Prop, r: &rs::Prop) -> bool {
    let v = c.value_vec4;
    *c.name == r.name
        && crate::scene::rt::flag_bits(c.flags) == r.flags
        && c.type_ as usize == r.prop_type as usize
        && same(v.x, r.value_real_arr[0])
        && same(v.y, r.value_real_arr[1])
        && same(v.z, r.value_real_arr[2])
        && same(v.w, r.value_real_arr[3])
        && c.value_int == r.value_int
        && *c.value_str == r.value_str
        && *c.value_blob == r.value_blob[..]
}

fn show_prop(c: &ufbx::Prop, r: &rs::Prop) -> String {
    let v = c.value_vec4;
    let flags = crate::scene::rt::flag_bits(c.flags);
    format!(
        "C {} {flags:#x} [{} {} {} {}] {}, port {} {:#x} {:?} {}",
        &*c.name, v.x, v.y, v.z, v.w, c.value_int, r.name, r.flags, r.value_real_arr, r.value_int
    )
}

fn sample_times(begin: f64, end: f64) -> Vec<f64> {
    let d = if end > begin { end - begin } else { 1.0 };
    let mut t = vec![begin - 2.3 * d - 0.01, begin - 0.6 * d, begin, end, end + 0.45 * d, end + 3.7 * d + 0.02];
    for i in 1..8 {
        t.push(begin + d * i as f64 / 8.0 + 0.003);
    }
    t
}

fn curve_times(keys: &[rs::Keyframe], min: f64, max: f64) -> Vec<f64> {
    let d = if max > min { max - min } else { 1.0 };
    let mut t = Vec::new();
    for (i, k) in keys.iter().enumerate() {
        t.push(k.time);
        if let Some(n) = keys.get(i + 1) {
            for f in [0.1, 0.25, 0.5, 0.77, 0.999] {
                t.push(k.time + (n.time - k.time) * f);
            }
        }
    }
    for f in [0.0001, 0.3, 0.5, 0.999, 1.0, 1.5, 2.0, 2.25, 3.0, 3.75, 4.0, 7.3, 12.5, 100.25] {
        t.push(min - d * f);
        t.push(max + d * f);
    }
    t
}

fn compare_curves(t: &mut Tally, scene: &ufbx::Scene, rscene: &rs::Scene) {
    for (curve, base) in scene.anim_curves.iter().zip(&rscene.anim_curves) {
        t.count("curves", 1);
        let times = curve_times(&base.keyframes, base.min_time, base.max_time);
        let mut variants = vec![(None, None)];
        for (cm, rm) in MODES {
            for count in [-1, 0, 1, 2, 3] {
                variants.push((Some((cm, rm, count)), Some((MODES[((count + 3) as usize) % 5], count + 1))));
            }
        }
        for (pre, post) in variants {
            // a bitwise copy of ufbx's curve, still pointing at its scene, with other extrapolations
            let mut c = std::mem::ManuallyDrop::new(unsafe { std::ptr::read(&**curve) });
            let mut r = base.clone();
            if let Some((cm, rm, count)) = pre {
                c.pre_extrapolation = ufbx::Extrapolation { mode: cm, repeat_count: count };
                r.pre_extrapolation = rs::Extrapolation { mode: rm, repeat_count: count };
            }
            if let Some(((cm, rm), count)) = post {
                c.post_extrapolation = ufbx::Extrapolation { mode: cm, repeat_count: count };
                r.post_extrapolation = rs::Extrapolation { mode: rm, repeat_count: count };
            }
            for &time in &times {
                for flags in [0, rs::EVALUATE_FLAG_NO_EXTRAPOLATION] {
                    let a = ufbx::evaluate_curve_flags(&c, time, -7.25, flags);
                    let b = rs::evaluate_curve_flags(&r, time, -7.25, flags);
                    t.check(same(a, b), || {
                        let i = r.keyframes.partition_point(|k| k.time <= time);
                        let near = &r.keyframes[i.saturating_sub(1)..(i + 1).min(r.keyframes.len())];
                        format!(
                            "curve {} t={time:?} flags={flags} {:?}/{:?}: C {a:?}, port {b:?}, keys {near:?}",
                            &*curve.element.name, r.pre_extrapolation, r.post_extrapolation
                        )
                    });
                }
            }
        }
    }
}

fn names_of(e: &ufbx::Element) -> Vec<String> {
    let mut names: Vec<String> = e.props.props.iter().map(|p| p.name.to_string()).collect();
    if let Some(d) = &e.props.defaults {
        names.extend(d.props.iter().map(|p| p.name.to_string()));
    }
    for extra in ["Lcl Rotation", "Lcl Scaling", "Lcl Translation", "RotationOrder", "DeformPercent", "Weight", "NoSuchProp", "A"] {
        names.push(extra.to_string());
    }
    names.sort();
    names.dedup();
    names
}

#[allow(clippy::too_many_arguments)]
fn compare_element(t: &mut Tally, rscene: &rs::Scene, canim: &ufbx::Anim, ranim: &rs::Anim, e: &ufbx::Element, times: &[f64], all_names: bool) {
    let id = e.element_id;
    let flags_of = |p: &ufbx::Prop| crate::scene::rt::flag_bits(p.flags);
    let animated = e.props.props.iter().any(|p| flags_of(p) & (rs::prop_flags::ANIMATED | rs::prop_flags::CONNECTED) != 0);
    let names = if all_names || animated || !ranim.prop_overrides.is_empty() { names_of(e) } else { vec!["Lcl Translation".to_string()] };
    for &time in times {
        for flags in [0, rs::EVALUATE_FLAG_NO_EXTRAPOLATION] {
            let size = e.props.props.len() + canim.prop_overrides.len() + 4;
            let mut buf: Vec<ufbx::Prop> = (0..size).map(|_| unsafe { std::mem::zeroed() }).collect();
            let cp = ufbx::evaluate_props_flags(canim, e, time, &mut buf[0], size, flags);
            let rp = rs::evaluate_props_flags(rscene, ranim, id, time, flags);
            t.check(cp.props.len() == rp.props.len() && cp.num_animated == rp.num_animated, || {
                format!("props of {} ({id}) t={time}: C {}, port {}", &*e.name, cp.props.len(), rp.props.len())
            });
            for (c, r) in cp.props.iter().zip(&rp.props) {
                t.check(same_prop(c, r), || format!("props of {} ({id}) t={time}: {}", &*e.name, show_prop(c, r)));
            }
            for name in &names {
                // ufbx `strcmp`s the caller's name for a property the element lacks, so it gets a terminated one
                let terminated = format!("{name}\0");
                let c = ufbx::evaluate_prop_flags(canim, e, &terminated[..name.len()], time, flags);
                let r = rs::evaluate_prop_flags(rscene, ranim, id, name, time, flags);
                t.check(same_prop(&c, &r), || format!("prop {name} of {} ({id}) t={time} flags={flags}: {}", &*e.name, show_prop(&c, &r)));
            }
        }
    }
}

fn anims<'a>(scene: &'a ufbx::Scene, rscene: &'a rs::Scene) -> Vec<(&'a ufbx::Anim, &'a rs::Anim, f64, f64)> {
    let mut v = vec![(&*scene.anim, &*rscene.anim, scene.anim.time_begin, scene.anim.time_end)];
    for (s, r) in scene.anim_stacks.iter().zip(&rscene.anim_stacks) {
        v.push((&*s.anim, &*r.anim, s.time_begin, s.time_end));
    }
    for (l, r) in scene.anim_layers.iter().zip(&rscene.anim_layers).take(4) {
        v.push((&*l.anim, &*r.anim, scene.anim.time_begin, scene.anim.time_end));
    }
    v
}

fn compare_anims(t: &mut Tally, scene: &ufbx::Scene, rscene: &rs::Scene) {
    for (canim, ranim, begin, end) in anims(scene, rscene) {
        let times = sample_times(begin, end);
        for e in scene.elements.iter() {
            compare_element(t, rscene, canim, ranim, e, &times, false);
        }
        for n in scene.nodes.iter() {
            for &time in &times {
                for flags in TRANSFORM_FLAGS {
                    let a = ufbx::evaluate_transform_flags(canim, n, time, flags);
                    let b = rs::evaluate_transform_flags(rscene, ranim, n.element.typed_id as usize, time, flags);
                    t.check(same_transform(&a, &b), || format!("transform {} t={time} flags={flags:#x}: C {a:?}, port {b:?}", &*n.element.name));
                }
            }
        }
        for ch in scene.blend_channels.iter() {
            for &time in &times {
                let a = ufbx::evaluate_blend_weight(canim, ch, time);
                let b = rs::evaluate_blend_weight(rscene, ranim, ch.element.typed_id as usize, time);
                t.check(same(a, b), || format!("blend weight {} t={time}: C {a}, port {b}", &*ch.element.name));
            }
        }
    }
}

fn desc<'a>(element_id: u32, name: &str, x: f64, int: i64, s: &str) -> (ufbx::PropOverrideDesc<'a>, rs::PropOverrideDesc) {
    (
        ufbx::PropOverrideDesc {
            element_id,
            prop_name: ufbx::StringOpt::Owned(name.to_string()),
            value: ufbx::Vec4 { x, y: -2.5, z: 0.125, w: 9.0 },
            value_str: ufbx::StringOpt::Owned(s.to_string()),
            value_int: int,
        },
        rs::PropOverrideDesc { element_id, prop_name: name.to_string(), value: rs::Vec4::new(x, -2.5, 0.125, 9.0), value_str: s.to_string(), value_int: int },
    )
}

type Override = (u32, u32, String, [u64; 4], String, i64);

fn c_overrides(a: &ufbx::Anim) -> (Vec<Override>, Vec<usize>) {
    let o = a
        .prop_overrides
        .iter()
        .map(|o| {
            (
                o.element_id,
                rs::props::name_key(o.prop_name.as_bytes()),
                o.prop_name.to_string(),
                [o.value.x, o.value.y, o.value.z, o.value.w].map(f64::to_bits),
                o.value_str.to_string(),
                o.value_int,
            )
        })
        .collect();
    (o, a.layers.iter().map(|l| l.element.typed_id as usize).collect())
}

fn p_overrides(a: &rs::Anim) -> (Vec<Override>, Vec<usize>) {
    let o = a
        .prop_overrides
        .iter()
        .map(|o| {
            (
                o.element_id,
                o.internal_key,
                o.prop_name.clone(),
                [o.value.x, o.value.y, o.value.z, o.value.w].map(f64::to_bits),
                o.value_str.clone(),
                o.value_int,
            )
        })
        .collect();
    (o, a.layers.clone())
}

fn compare_overrides(t: &mut Tally, scene: &ufbx::Scene, rscene: &rs::Scene) {
    let layer_ids: Vec<u32> = scene.anim.layers.iter().map(|l| l.element.typed_id).collect();
    let weights: Vec<f64> = (0..layer_ids.len()).map(|i| 0.35 + 0.2 * i as f64).collect();
    let n = scene.elements.len() as u32;
    let times = sample_times(scene.anim.time_begin, scene.anim.time_end);

    for (k, e) in scene.elements.iter().enumerate().filter(|(k, _)| k % 5 == 0) {
        let id = e.element_id;
        let own: Vec<String> = e.props.props.iter().map(|p| p.name.to_string()).collect();
        let mut descs = vec![desc(id, "ZzzOverride", 1.5, 0, "str"), desc(id, "A", 0.0, 7, ""), desc((id + 1) % n, "Lcl Translation", 2.75, 0, "")];
        if let Some(first) = own.first() {
            descs.push(desc(id, first, 0.0, 0, "x"));
        }
        if own.len() > 1 {
            descs.push(desc(id, &own[own.len() - 1], -3.5, 0, ""));
        }
        if let Some(d) = e.props.defaults.as_ref().and_then(|d| d.props.first()) {
            descs.push(desc(id, &d.name, 1e300, 0, ""));
        }
        if k % 10 == 0 {
            descs.push(desc(id, "ZzzOverride", 4.0, 0, ""));
        }
        let with_weights = k % 3 == 0;
        let (cdescs, rdescs): (Vec<_>, Vec<_>) = descs.into_iter().unzip();
        let copts = ufbx::AnimOpts {
            layer_ids: ufbx::ListOpt::Owned(layer_ids.clone()),
            override_layer_weights: if with_weights { ufbx::ListOpt::Owned(weights.clone()) } else { ufbx::ListOpt::Unset },
            // `ListOpt::Owned` drops each desc while ufbx still points at its strings (ufbx crate 0.11.5)
            prop_overrides: ufbx::ListOpt::Ref(&cdescs),
            ignore_connections: k % 4 == 0,
            ..Default::default()
        };
        let ropts = rs::AnimOpts {
            layer_ids: layer_ids.clone(),
            override_layer_weights: if with_weights { weights.clone() } else { vec![] },
            prop_overrides: rdescs,
            ignore_connections: k % 4 == 0,
            ..Default::default()
        };
        let (canim, ranim) = match (ufbx::create_anim(scene, copts), rs::create_anim(rscene, &ropts)) {
            (Ok(c), Ok(r)) => (c, r),
            (Err(ce), Err(re)) => {
                t.check(*ce.description == re.description && ce.info() == re.info, || {
                    format!("create_anim error: C {} {}, port {} {}", &*ce.description, ce.info(), re.description, re.info)
                });
                continue;
            }
            (c, r) => {
                t.check(false, || format!("create_anim on element {id}: C ok {}, port ok {}", c.is_ok(), r.is_ok()));
                continue;
            }
        };
        let (co, ro) = (c_overrides(&canim), p_overrides(&ranim));
        t.check(co == ro, || format!("create_anim overrides on element {id}: C {co:?}, port {ro:?}"));
        compare_element(t, rscene, &canim, &ranim, e, &times, true);
        let next = &scene.elements[((id + 1) % n) as usize];
        compare_element(t, rscene, &canim, &ranim, next, &times, true);
    }

    let tid = [5u32, 1, 5, 3, 1, 7, 3, 5, 0, 1];
    let ct: Vec<ufbx::TransformOverride> = tid
        .iter()
        .enumerate()
        .map(|(i, &node_id)| ufbx::TransformOverride {
            node_id,
            transform: ufbx::Transform {
                translation: ufbx::Vec3 { x: i as f64, y: 0.0, z: 0.0 },
                rotation: ufbx::Quat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 },
                scale: ufbx::Vec3 { x: 1.0, y: 1.0, z: 1.0 },
            },
        })
        .collect();
    let rt: Vec<rs::TransformOverride> = tid
        .iter()
        .enumerate()
        .map(|(i, &node_id)| rs::TransformOverride {
            node_id,
            transform: rs::Transform { translation: rs::Vec3::new(i as f64, 0.0, 0.0), ..rs::Transform::IDENTITY },
        })
        .collect();
    let c = ufbx::create_anim(scene, ufbx::AnimOpts { transform_overrides: ufbx::ListOpt::Ref(&ct), ..Default::default() }).unwrap();
    let r = rs::create_anim(rscene, &rs::AnimOpts { transform_overrides: rt, ..Default::default() }).unwrap();
    let corder: Vec<(u32, f64)> = c.transform_overrides.iter().map(|t| (t.node_id, t.transform.translation.x)).collect();
    let rorder: Vec<(u32, f64)> = r.transform_overrides.iter().map(|t| (t.node_id, t.transform.translation.x)).collect();
    t.check(corder == rorder, || format!("transform override order: C {corder:?}, port {rorder:?}"));

    let bad = rs::AnimOpts { layer_ids: vec![scene.anim_layers.len() as u32], ..Default::default() };
    let cbad = ufbx::AnimOpts { layer_ids: ufbx::ListOpt::Owned(vec![scene.anim_layers.len() as u32]), ..Default::default() };
    let (ce, re) = (ufbx::create_anim(scene, cbad).err(), rs::create_anim(rscene, &bad).err());
    t.check(ce.as_ref().map(|e| e.description.to_string()) == re.as_ref().map(|e| e.description.clone()), || "layer_ids out of bounds".to_string());
}

fn compare_evaluated(t: &mut Tally, scene: &ufbx::Scene, rscene: &rs::Scene, keep: usize) {
    for (canim, ranim, begin, end) in anims(scene, rscene).into_iter().take(1 + scene.anim_stacks.len()) {
        for (time, skinning) in [(begin, false), (0.5 * (begin + end), true), (end, false)] {
            t.count("evaluated scenes", 1);
            let c = ufbx::evaluate_scene(scene, canim, time, ufbx::EvaluateOpts { evaluate_skinning: skinning, ..Default::default() });
            let r = rs::evaluate_scene(rscene, ranim, time, &rs::EvaluateOpts { evaluate_skinning: skinning, ..Default::default() });
            match (c, r) {
                (Ok(c), Ok(r)) => record(t, compare_scenes(&c, &r, keep)),
                (c, r) => {
                    t.check(false, || {
                        format!("evaluate_scene t={time}: C {:?}, port {:?}", c.err().map(|e| e.description.to_string()), r.err().map(|e| e.description))
                    });
                }
            }
        }
    }
}

const LOADS: [&str; 2] = ["default", "helpers"];

fn load_opts(helpers: bool) -> (ufbx::LoadOpts<'static>, rs::LoadOpts) {
    let mut o = (ufbx::LoadOpts::default(), rs::LoadOpts::default());
    if helpers {
        both!(o, inherit_mode_handling = InheritModeHandling::HelperNodes);
        both!(o, geometry_transform_handling = GeometryTransformHandling::HelperNodes);
        both!(o, space_conversion = SpaceConversion::AdjustTransforms);
        (o.0.target_axes, o.1.target_axes) = axes(RH_Z);
        both!(o, target_unit_meters = 0.01);
    }
    o
}

fn jobs(a: &Args) -> Vec<Job> {
    let mut v = corpus_jobs(a, false, &["fbx", "obj", "mtl"]);
    v.retain(|j| std::fs::read(&j.item).is_ok_and(|b| b.windows(14).any(|w| w == b"AnimationCurve")));
    v
}

fn port_scene<'a>(cell: &'a OnceCell<Result<rs::Scene, String>>, bytes: &[u8], opts: &rs::LoadOpts, t: &mut Tally) -> Option<&'a rs::Scene> {
    match cell.get_or_init(|| rs::load_memory(bytes, opts).map_err(|e| e.description)) {
        Ok(r) => Some(r),
        Err(e) => {
            t.check(false, || format!("C loads, the port fails: {e}"));
            None
        }
    }
}

fn run(a: &Args, path: &str, ch: &mut Child) {
    let Ok(bytes) = std::fs::read(path) else { return };
    for (i, load) in LOADS.iter().enumerate() {
        let (c_opts, p_opts) = load_opts(i == 1);
        let Ok(scene) = ufbx::load_memory(&bytes, c_opts) else { continue };
        let loaded = OnceCell::new();
        ch.step(&format!("{load}: curves"), |t| {
            if let Some(r) = port_scene(&loaded, &bytes, &p_opts, t) {
                compare_curves(t, &scene, r);
            }
        });
        ch.step(&format!("{load}: props, transforms, blend weights"), |t| {
            if let Some(r) = port_scene(&loaded, &bytes, &p_opts, t) {
                t.count("elements", scene.elements.len() as u64);
                compare_anims(t, &scene, r);
            }
        });
        ch.step(&format!("{load}: create_anim"), |t| {
            if let Some(r) = port_scene(&loaded, &bytes, &p_opts, t) {
                compare_overrides(t, &scene, r);
            }
        });
        ch.step(&format!("{load}: evaluate_scene"), |t| {
            if let Some(r) = port_scene(&loaded, &bytes, &p_opts, t) {
                compare_evaluated(t, &scene, r, a.keep);
            }
        });
    }
}

pub const EVALUATE: Section = Section { name: "evaluate", jobs, run, header: None };
