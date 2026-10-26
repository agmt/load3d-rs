use load3d as rs;

use crate::common::{both, same, same_quat, same3};
use crate::runner::{Args, Child, Job, Section, Status, Tally, corpus_jobs};
use crate::scene::generated::eq_ErrorType;
use crate::scene::rt::flag_bits;

type Opts = (ufbx::BakeOpts, rs::BakeOpts);

fn option_sets() -> Vec<(&'static str, Opts)> {
    let base = || -> Opts { (ufbx::BakeOpts::default(), rs::BakeOpts::default()) };
    let mut sets = vec![("default", base())];
    let mut o = base();
    both!(o, resample_rate = 10.0);
    both!(o, max_keyframe_segments = 4);
    sets.push(("rate10_seg4", o));
    let mut o = base();
    both!(o, resample_rate = 120.0);
    both!(o, minimum_sample_rate = 1000.0);
    sets.push(("rate120_min1000", o));
    let mut o = base();
    both!(o, minimum_sample_rate = 1.0);
    both!(o, maximum_sample_rate = 15.0);
    sets.push(("min1_max15", o));
    let mut o = base();
    both!(o, trim_start_time = true);
    sets.push(("trim", o));
    let mut o = base();
    both!(o, step_handling = BakeStepHandling::CustomDuration);
    both!(o, step_custom_duration = 0.01);
    both!(o, step_custom_epsilon = 0.001);
    sets.push(("step_custom", o));
    let mut o = base();
    both!(o, step_handling = BakeStepHandling::IdenticalTime);
    sets.push(("step_identical", o));
    let mut o = base();
    both!(o, step_handling = BakeStepHandling::AdjacentDouble);
    sets.push(("step_adjacent", o));
    let mut o = base();
    both!(o, step_handling = BakeStepHandling::Ignore);
    sets.push(("step_ignore", o));
    let mut o = base();
    both!(o, key_reduction_enabled = true);
    sets.push(("reduce", o));
    let mut o = base();
    both!(o, key_reduction_enabled = true);
    both!(o, key_reduction_rotation = true);
    both!(o, key_reduction_threshold = 0.001);
    both!(o, key_reduction_passes = 8);
    sets.push(("reduce_rot", o));
    let mut o = base();
    both!(o, skip_node_transforms = true);
    both!(o, bake_transform_props = true);
    sets.push(("skip_nodes_props", o));
    let mut o = base();
    both!(o, bake_transform_props = true);
    both!(o, no_resample_rotation = true);
    sets.push(("props_no_resample_rot", o));
    let mut o = base();
    both!(o, ignore_layer_weight_animation = true);
    both!(o, evaluate_flags = 0x1);
    sets.push(("no_weights_no_extrap", o));
    let mut o = base();
    both!(o, trim_start_time = true);
    both!(o, maximum_sample_rate = 24.0);
    both!(o, key_reduction_enabled = true);
    both!(o, step_handling = BakeStepHandling::AdjacentDouble);
    both!(o, resample_rate = 48.0);
    sets.push(("combo", o));
    sets
}

const LOADS: [&str; 4] = ["load_default", "load_helpers", "load_compensate", "load_helpers_adjust_m"];

fn load_opts(i: usize) -> (ufbx::LoadOpts<'static>, rs::LoadOpts) {
    let mut o = (ufbx::LoadOpts::default(), rs::LoadOpts::default());
    match i {
        1 => both!(o, inherit_mode_handling = InheritModeHandling::HelperNodes),
        2 => both!(o, inherit_mode_handling = InheritModeHandling::Compensate),
        3 => {
            both!(o, inherit_mode_handling = InheritModeHandling::HelperNodes);
            both!(o, space_conversion = SpaceConversion::AdjustTransforms);
            both!(o, target_unit_meters = 1.0);
        }
        _ => {}
    }
    o
}

fn sample_times(times: &[f64]) -> Vec<f64> {
    let mut out = Vec::new();
    if let (Some(&first), Some(&last)) = (times.first(), times.last()) {
        out.push(first - 1.0);
        out.push(last + 1.0);
    }
    for (i, &t) in times.iter().enumerate() {
        out.push(t);
        out.push(t.next_down());
        out.push(t.next_up());
        if let Some(&n) = times.get(i + 1) {
            out.push(0.5 * (t + n));
            out.push(t + 0.25 * (n - t));
        }
    }
    out
}

fn compare_vec3_keys(t: &mut Tally, ctx: &str, c: &[ufbx::BakedVec3], r: &[rs::BakedVec3]) {
    if !t.check(c.len() == r.len(), || {
        let first = c.iter().zip(r).position(|(c, r)| !same(c.time, r.time) || flag_bits(c.flags) != r.flags);
        format!("{ctx}: key count C {}, port {}; first differing key {first:?}", c.len(), r.len())
    }) {
        return;
    }
    for (i, (ck, rk)) in c.iter().zip(r).enumerate() {
        t.count("keys", 1);
        for (b, name) in ["STEP_LEFT keys", "STEP_RIGHT keys", "STEP_KEY keys", "KEYFRAME keys", "REDUCED keys"].into_iter().enumerate() {
            if flag_bits(ck.flags) & (1 << b) != 0 {
                t.count(name, 1);
            }
        }
        t.check(same(ck.time, rk.time), || format!("{ctx}: key {i} time C {:?}, port {:?}", ck.time, rk.time));
        t.check(flag_bits(ck.flags) == rk.flags, || format!("{ctx}: key {i} flags C {:#x}, port {:#x}", flag_bits(ck.flags), rk.flags));
        t.check(same3(&ck.value, &rk.value), || format!("{ctx}: key {i} (t={:?}) value C {:?}, port {:?}", ck.time, ck.value, rk.value));
    }
    let keys: Vec<rs::BakedVec3> = c.iter().map(|k| rs::BakedVec3 { time: k.time, value: crate::common::v3(&k.value), flags: flag_bits(k.flags) }).collect();
    for time in sample_times(&keys.iter().map(|k| k.time).collect::<Vec<_>>()) {
        let cv = ufbx::evaluate_baked_vec3(c, time);
        let rv = rs::evaluate_baked_vec3(&keys, time);
        t.count("evaluations", 1);
        t.check(same3(&cv, &rv), || format!("{ctx}: evaluate_baked_vec3 at {time:?} C {cv:?}, port {rv:?}"));
    }
}

fn compare_quat_keys(t: &mut Tally, ctx: &str, c: &[ufbx::BakedQuat], r: &[rs::BakedQuat]) {
    if !t.check(c.len() == r.len(), || format!("{ctx}: key count C {}, port {}", c.len(), r.len())) {
        return;
    }
    for (i, (ck, rk)) in c.iter().zip(r).enumerate() {
        t.count("keys", 1);
        t.check(same(ck.time, rk.time), || format!("{ctx}: key {i} time C {:?}, port {:?}", ck.time, rk.time));
        t.check(flag_bits(ck.flags) == rk.flags, || format!("{ctx}: key {i} flags C {:#x}, port {:#x}", flag_bits(ck.flags), rk.flags));
        t.check(same_quat(&ck.value, &rk.value), || format!("{ctx}: key {i} (t={:?}) value C {:?}, port {:?}", ck.time, ck.value, rk.value));
    }
    let keys: Vec<rs::BakedQuat> = c.iter().map(|k| rs::BakedQuat { time: k.time, value: crate::common::quat(&k.value), flags: flag_bits(k.flags) }).collect();
    for time in sample_times(&keys.iter().map(|k| k.time).collect::<Vec<_>>()) {
        let cv = ufbx::evaluate_baked_quat(c, time);
        let rv = rs::evaluate_baked_quat(&keys, time);
        t.count("evaluations", 1);
        t.check(same_quat(&cv, &rv), || format!("{ctx}: evaluate_baked_quat at {time:?} C {cv:?}, port {rv:?}"));
    }
}

fn compare_bake(t: &mut Tally, ctx: &str, c: &ufbx::BakedAnim, r: &rs::BakedAnim, num_nodes: usize, num_elements: usize) {
    t.count("bakes", 1);
    for (what, a, b) in [
        ("playback_time_begin", c.playback_time_begin, r.playback_time_begin),
        ("playback_time_end", c.playback_time_end, r.playback_time_end),
        ("playback_duration", c.playback_duration, r.playback_duration),
        ("key_time_min", c.key_time_min, r.key_time_min),
        ("key_time_max", c.key_time_max, r.key_time_max),
    ] {
        t.check(same(a, b), || format!("{ctx}: {what} C {a:?}, port {b:?}"));
    }

    if t.check(c.nodes.len() == r.nodes.len(), || format!("{ctx}: node count C {}, port {}", c.nodes.len(), r.nodes.len())) {
        for (cn, rn) in c.nodes.iter().zip(&r.nodes) {
            t.count("nodes", 1);
            if !(cn.constant_translation && cn.constant_rotation && cn.constant_scale) {
                t.count("nodes not constant", 1);
            }
            let nctx = format!("{ctx} node {}", cn.typed_id);
            t.check(cn.typed_id == rn.typed_id && cn.element_id == rn.element_id, || {
                format!("{nctx}: ids C ({}, {}), port ({}, {})", cn.typed_id, cn.element_id, rn.typed_id, rn.element_id)
            });
            let constant = (cn.constant_translation, cn.constant_rotation, cn.constant_scale);
            let rconstant = (rn.constant_translation, rn.constant_rotation, rn.constant_scale);
            t.check(constant == rconstant, || format!("{nctx}: constant C {constant:?}, port {rconstant:?}"));
            compare_vec3_keys(t, &format!("{nctx} T"), &cn.translation_keys, &rn.translation_keys);
            compare_quat_keys(t, &format!("{nctx} R"), &cn.rotation_keys, &rn.rotation_keys);
            compare_vec3_keys(t, &format!("{nctx} S"), &cn.scale_keys, &rn.scale_keys);
        }
    }

    if t.check(c.elements.len() == r.elements.len(), || format!("{ctx}: element count C {}, port {}", c.elements.len(), r.elements.len())) {
        for (ce, re) in c.elements.iter().zip(&r.elements) {
            let ectx = format!("{ctx} element {}", ce.element_id);
            t.check(ce.element_id == re.element_id, || format!("{ectx}: id port {}", re.element_id));
            if !t.check(ce.props.len() == re.props.len(), || format!("{ectx}: prop count C {}, port {}", ce.props.len(), re.props.len())) {
                continue;
            }
            for (cp, rp) in ce.props.iter().zip(&re.props) {
                t.count("props", 1);
                let pctx = format!("{ectx} prop {:?}", &*cp.name);
                t.check(*cp.name == *rp.name, || format!("{pctx}: name port {:?}", rp.name));
                t.check(cp.constant_value == rp.constant_value, || format!("{pctx}: constant C {}, port {}", cp.constant_value, rp.constant_value));
                compare_vec3_keys(t, &pctx, &cp.keys, &rp.keys);
            }
        }
    }

    let cb = c as *const ufbx::BakedAnim as *mut ufbx::BakedAnim;
    for id in 0..=num_nodes as u32 {
        let cf = unsafe { ufbx::ufbx_find_baked_node_by_typed_id(cb, id) };
        let cf = if cf.is_null() { None } else { Some(unsafe { (*cf).typed_id }) };
        let rf = rs::find_baked_node_by_typed_id(r, id).map(|n| n.typed_id);
        let rf2 = rs::find_baked_node(r, id as usize).map(|n| n.typed_id);
        t.count("lookups", 1);
        t.check(cf == rf && rf == rf2, || format!("{ctx}: find_baked_node_by_typed_id({id}) C {cf:?}, port {rf:?} {rf2:?}"));
    }
    for id in 0..=num_elements as u32 {
        let cf = unsafe { ufbx::ufbx_find_baked_element_by_element_id(cb, id) };
        let cf = if cf.is_null() { None } else { Some(unsafe { (*cf).element_id }) };
        let rf = rs::find_baked_element_by_element_id(r, id).map(|e| e.element_id);
        let rf2 = rs::find_baked_element(r, id).map(|e| e.element_id);
        t.count("lookups", 1);
        t.check(cf == rf && rf == rf2, || format!("{ctx}: find_baked_element_by_element_id({id}) C {cf:?}, port {rf:?} {rf2:?}"));
    }
}

fn bake_file(t: &mut Tally, path: &str, load: usize) {
    let (c_load, r_load) = load_opts(load);
    let (c, r) = match (ufbx::load_file(path, c_load), load3d::load_file(path, &r_load)) {
        (Ok(c), Ok(r)) => (c, r),
        (Err(c), Err(r)) => {
            t.count("loads failed in both", 1);
            let same = eq_ErrorType(c.type_, r.kind) && *c.description == r.description;
            if t.check(same, || format!("load errors: C {:?} {:?}, port {:?} {:?}", c.type_, &*c.description, r.kind, r.description)) {
                t.status(Status::ErrorEqual, format!("{:?} {:?}", c.type_, &*c.description));
            }
            return;
        }
        (c, r) => {
            t.check(false, || format!("only one loads: C {:?}, port {:?}", c.err().map(|e| e.description.to_string()), r.err().map(|e| e.description)));
            return;
        }
    };
    t.count("loads", 1);
    if c.anim_layers.is_empty() && c.anim_stacks.is_empty() {
        return;
    }
    let cn = [c.nodes.len(), c.elements.len(), c.anim_stacks.len(), c.anim_layers.len()];
    let rn = [r.nodes.len(), r.elements.len(), r.anim_stacks.len(), r.anim_layers.len()];
    if !t.check(cn == rn, || format!("counts of nodes, elements, stacks and layers: C {cn:?}, port {rn:?}")) {
        return;
    }
    t.count("animated loads", 1);

    let mut anims: Vec<(String, &ufbx::Anim, &rs::Anim)> = vec![("scene".into(), &c.anim, &r.anim)];
    for (i, (cs, rst)) in c.anim_stacks.iter().zip(&r.anim_stacks).enumerate() {
        anims.push((format!("stack{i}"), &cs.anim, &rst.anim));
    }
    for (i, (cl, rl)) in c.anim_layers.iter().zip(&r.anim_layers).enumerate() {
        anims.push((format!("layer{i}"), &cl.anim, &rl.anim));
    }

    let layer_ids: Vec<u32> = (0..c.anim_layers.len() as u32).rev().collect();
    let weights = vec![0.5; layer_ids.len()];
    let c_rev = ufbx::create_anim(
        &c,
        ufbx::AnimOpts { layer_ids: ufbx::ListOpt::Ref(&layer_ids), override_layer_weights: ufbx::ListOpt::Ref(&weights), ..Default::default() },
    );
    let r_rev = rs::create_anim(&r, &rs::AnimOpts { layer_ids: layer_ids.clone(), override_layer_weights: weights.clone(), ..Default::default() });
    let target = c.anim_layers.iter().flat_map(|l| l.anim_props.iter()).find(|p| p.element.type_ == ufbx::ElementType::Node).map(|p| p.element.element_id);
    let c_over = target.map(|id| {
        ufbx::create_anim(
            &c,
            ufbx::AnimOpts {
                prop_overrides: ufbx::ListOpt::Owned(vec![ufbx::PropOverrideDesc {
                    element_id: id,
                    prop_name: ufbx::StringOpt::Ref("Lcl Translation"),
                    value: ufbx::Vec4 { x: 1.0, y: 2.0, z: 3.0, w: 0.0 },
                    value_str: ufbx::StringOpt::Unset,
                    value_int: 0,
                }]),
                ignore_connections: true,
                ..Default::default()
            },
        )
    });
    let r_over = target.map(|id| {
        rs::create_anim(
            &r,
            &rs::AnimOpts {
                prop_overrides: vec![rs::PropOverrideDesc {
                    element_id: id,
                    prop_name: "Lcl Translation".into(),
                    value: rs::Vec4::new(1.0, 2.0, 3.0, 0.0),
                    value_str: String::new(),
                    value_int: 0,
                }],
                ignore_connections: true,
                ..Default::default()
            },
        )
    });
    match (&c_rev, &r_rev) {
        (Ok(ca), Ok(ra)) => anims.push(("custom_rev_half".into(), ca, ra)),
        (ca, ra) => {
            t.check(false, || format!("create_anim C ok {}, port ok {}", ca.is_ok(), ra.is_ok()));
        }
    }
    if let (Some(ca), Some(ra)) = (&c_over, &r_over) {
        match (ca, ra) {
            (Ok(ca), Ok(ra)) => anims.push(("custom_override".into(), ca, ra)),
            (ca, ra) => {
                t.check(false, || format!("create_anim C ok {}, port ok {}", ca.is_ok(), ra.is_ok()));
            }
        }
    }

    t.count("scale helper nodes", c.nodes.iter().filter(|n| n.is_scale_helper).count() as u64);
    for (label, ca, ra) in &anims {
        for (set, (co, ro)) in option_sets() {
            let ctx = format!("{label} {set}");
            match (ufbx::bake_anim(&c, ca, co), rs::bake_anim(&r, ra, &ro)) {
                (Ok(cb), Ok(rb)) => compare_bake(t, &ctx, &cb, &rb, c.nodes.len(), c.elements.len()),
                (cb, rb) => {
                    t.check(false, || format!("{ctx}: bake C ok {}, port ok {}", cb.is_ok(), rb.is_ok()));
                }
            }
        }
    }
}

fn jobs(a: &Args) -> Vec<Job> {
    corpus_jobs(a, true, &["fbx"])
}

fn run(_: &Args, path: &str, ch: &mut Child) {
    for (i, kind) in LOADS.iter().enumerate() {
        ch.step(kind, |t| bake_file(t, path, i));
    }
}

pub const BAKE: Section = Section { name: "bake", jobs, run, header: None };
