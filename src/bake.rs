use std::mem::take;

use crate::error::Res;
use crate::evaluate::{evaluate_prop_flags, evaluate_transform_flags};
use crate::math::{is_vec3_zero, lerp3, quat_fix_antipodal, quat_slerp};
use crate::search::{lower_bound_eq, stable_sort, unstable_sort};
use crate::types::*;

use baked_key::{KEYFRAME, REDUCED, STEP_KEY, STEP_LEFT, STEP_RIGHT};

#[derive(Debug, Clone, Copy, PartialEq)]
struct BakeTime {
    time: f64,
    flags: u32,
}

#[derive(Debug, Clone, Copy)]
struct BakeProp<'a> {
    sort_id: u32,
    element_id: u32,
    prop_name: &'a str,
    anim_value: usize,
}

struct BakeContext<'a> {
    scene: &'a Scene,
    anim: &'a Anim,
    opts: BakeOpts,
    tmp_times: Vec<BakeTime>,
    layer_weight_times: Vec<BakeTime>,
    /// Per node typed id, its index in `nodes` once baked
    baked_nodes: Vec<Option<usize>>,
    nodes_to_bake: Vec<bool>,
    bake_stack: Vec<u32>,
    nodes: Vec<BakedNode>,
    elements: Vec<BakedElement>,
    props: Vec<BakedProp>,
    ktime_offset: f64,
    time_begin: f64,
    time_end: f64,
    time_min: f64,
    time_max: f64,
}

const TRANSFORM_PROPS: [&str; 10] = [
    "Lcl Translation",
    "Lcl Rotation",
    "Lcl Scaling",
    "PreRotation",
    "PostRotation",
    "RotationOffset",
    "ScalingOffset",
    "RotationPivot",
    "ScalingPivot",
    "RotationOrder",
];

const COMPLEX_TRANSLATION_PROPS: [&str; 4] = ["ScalingPivot", "RotationPivot", "RotationOffset", "ScalingOffset"];

const COMPLEX_ROTATION_PROPS: [&str; 3] = ["PreRotation", "PostRotation", "RotationOrder"];

const COMPLEX_ROTATION_SOURCES: [&str; 4] = ["Lcl Rotation", "PreRotation", "PostRotation", "RotationOrder"];

fn c_str(s: &str) -> &str {
    s.find('\0').map_or(s, |n| &s[..n])
}

fn bake_prop_less(a: &BakeProp, b: &BakeProp) -> bool {
    if a.sort_id != b.sort_id {
        return a.sort_id < b.sort_id;
    }
    if a.element_id != b.element_id {
        return a.element_id < b.element_id;
    }
    if a.prop_name != b.prop_name {
        return c_str(a.prop_name).as_bytes() < c_str(b.prop_name).as_bytes();
    }
    false
}

fn cmp_bake_time(a: BakeTime, b: BakeTime) -> i32 {
    if a.time != b.time {
        return if a.time < b.time { -1 } else { 1 };
    }
    let (a_step, b_step) = (a.flags & 0x3, b.flags & 0x3);
    if a_step != b_step {
        // `STEP_LEFT < none < STEP_RIGHT`
        return if (a_step ^ 0x1) < (b_step ^ 0x1) { -1 } else { 1 };
    }
    0
}

fn add_epsilon(a: f64, epsilon: f64) -> f64 {
    if a > 0.0 { a * epsilon } else { a / epsilon }
}

fn sub_epsilon(a: f64, epsilon: f64) -> f64 {
    if a > 0.0 { a / epsilon } else { a * epsilon }
}

fn bake_time_sample_time(time: BakeTime) -> f64 {
    if time.flags & STEP_LEFT != 0 {
        time.time.next_down()
    } else if time.flags & STEP_RIGHT != 0 {
        time.time.next_up()
    } else {
        time.time
    }
}

fn push_resampled_times(times: &mut Vec<BakeTime>, keys: &[BakedVec3]) {
    for i in 0..keys.len() {
        let flags = keys[i].flags;
        let mut time = keys[i].time;
        if flags & STEP_LEFT != 0 && i + 1 < keys.len() && keys[i + 1].flags & STEP_KEY != 0 {
            time = keys[i + 1].time;
        } else if flags & STEP_RIGHT != 0 && i > 0 && keys[i - 1].flags & STEP_KEY != 0 {
            time = keys[i - 1].time;
        }
        times.push(BakeTime { time, flags: flags & 0x7 });
    }
}

impl<'a> BakeContext<'a> {
    fn bake_times(&mut self, anim_value: usize, resample_linear: bool, key_flag: u32) {
        let scene = self.scene;
        let sample_rate = self.opts.resample_rate;
        let min_duration = if self.opts.minimum_sample_rate > 0.0 { 1.0 / self.opts.minimum_sample_rate } else { 0.0 };
        let max_segments = self.opts.max_keyframe_segments;

        for curve in scene.anim_values[anim_value].curves {
            let Some(curve) = curve else { continue };
            let keys = &scene.anim_curves[curve].keyframes;
            let num_keys = keys.len();
            for key_ix in 0..num_keys {
                let a = keys[key_ix];
                let a_time = a.time;
                self.tmp_times.push(BakeTime { time: a_time, flags: key_flag });
                if key_ix + 1 >= num_keys {
                    break;
                }
                let b = keys[key_ix + 1];
                let b_time = b.time;

                if a.value == b.value && a.right.dy == 0.0 && b.left.dy == 0.0 {
                    continue;
                }

                if a.interpolation == Interpolation::ConstantPrev {
                    self.tmp_times.push(BakeTime { time: b_time, flags: STEP_LEFT });
                } else if a.interpolation == Interpolation::ConstantNext {
                    self.tmp_times.push(BakeTime { time: a_time, flags: STEP_RIGHT });
                } else if (resample_linear || a.interpolation == Interpolation::Cubic) && sample_rate > 0.0 {
                    let duration = b_time - a_time;
                    if duration <= min_duration {
                        continue;
                    }

                    let mut factor = 1.0;
                    while duration * sample_rate / factor >= max_segments as f64 {
                        factor *= 2.0;
                    }

                    let padding = 0.5 / sample_rate;
                    let start = ((a_time + padding) * sample_rate / factor).ceil() * factor;
                    let stop = b_time - padding;
                    for i in 0..max_segments {
                        let time = (start + i as f64 * factor) / sample_rate;
                        if time >= stop {
                            break;
                        }
                        self.tmp_times.push(BakeTime { time, flags: 0 });
                    }
                }
            }
        }
    }

    fn finalize_bake_times(&mut self) -> Vec<BakeTime> {
        if !self.layer_weight_times.is_empty() {
            self.tmp_times.extend_from_slice(&self.layer_weight_times);
        }

        if self.tmp_times.is_empty() {
            self.tmp_times.push(BakeTime { time: self.time_begin, flags: 0 });
            self.tmp_times.push(BakeTime { time: self.time_end, flags: 0 });
        }

        let mut times = take(&mut self.tmp_times);
        let mut num_times = times.len();
        // `ufbxi_sort_bake_times` in ufbx.c
        stable_sort(&mut times, 32, |a, b| cmp_bake_time(*a, *b) < 0);

        if num_times > 0 {
            let mut dst = 0;
            let mut prev = times[0];
            for src in 1..num_times {
                let mut next = times[src];
                if next.time == prev.time {
                    if (next.flags ^ prev.flags) & 0x3 == 0 {
                        prev.flags |= next.flags;
                        continue;
                    } else if prev.flags & STEP_LEFT != 0 {
                        next.flags |= STEP_KEY;
                    } else if next.flags & STEP_RIGHT != 0 {
                        prev.flags |= STEP_KEY;
                    }
                }

                times[dst] = prev;
                dst += 1;
                prev = next;
            }
            times[dst] = prev;
            dst += 1;
            num_times = dst;
        }

        if num_times > 0 {
            let min_dist = 0.25 / self.opts.resample_rate;
            let keep_flags = STEP_LEFT | STEP_RIGHT | STEP_KEY | KEYFRAME;

            let mut dst = 0;
            for src in 0..num_times {
                let cur = times[src];
                let mut delta = f64::INFINITY;

                let mut keep = true;
                if cur.flags & keep_flags == 0 {
                    if dst > 0 {
                        delta = cur.time - times[dst - 1].time;
                    }
                    if src + 1 < num_times {
                        delta = delta.min(times[src + 1].time - cur.time);
                    }
                    if delta < min_dist {
                        keep = false;
                    }
                }
                if keep {
                    times[dst] = cur;
                    dst += 1;
                }
            }
            num_times = dst;
        }

        if self.opts.maximum_sample_rate > 0.0 {
            let epsilon = 0.0078125 / self.opts.maximum_sample_rate;
            let sample_rate = self.opts.maximum_sample_rate;
            let max_interval = 1.0 / self.opts.maximum_sample_rate;
            let min_interval = 1.0 / self.opts.maximum_sample_rate - epsilon;
            let (mut dst, mut src) = (0, 0);

            for i in 0..num_times {
                if times[i].flags & (STEP_LEFT | STEP_RIGHT) != 0 {
                    let sign = if times[i].flags & STEP_LEFT != 0 { -1.0 } else { 1.0 };
                    let mut time = times[i].time + sign * max_interval;
                    if i > 0 {
                        time = time.max(times[i - 1].time);
                    }
                    if i + 1 < num_times {
                        time = time.min(times[i + 1].time);
                    }
                    times[i].time = time;
                    times[i].flags = REDUCED;
                }
            }

            let mut prev_time = BakeTime { time: -f64::INFINITY, flags: 0 };
            while src < num_times {
                let src_time = times[src];
                src += 1;

                let start_src = src;
                let next_time = BakeTime { time: (src_time.time * sample_rate - epsilon).ceil() / sample_rate, flags: REDUCED };
                while src < num_times && times[src].time <= next_time.time + epsilon {
                    src += 1;
                }

                if src != start_src || src_time.time - prev_time.time <= min_interval {
                    prev_time = next_time;
                } else {
                    prev_time = src_time;
                }

                if dst == 0 || prev_time.time > times[dst - 1].time {
                    times[dst] = prev_time;
                    dst += 1;
                }
            }

            num_times = dst;
        }

        if num_times > 0 {
            if times[0].time < self.time_min {
                self.time_min = times[0].time;
            }
            if times[num_times - 1].time > self.time_max {
                self.time_max = times[num_times - 1].time;
            }
        }

        times.truncate(num_times);
        times
    }

    fn postprocess_step(&self, prev_time: f64, next_time: f64, p_time: &mut f64, flags: u32) -> bool {
        let left = flags & STEP_LEFT != 0;

        let mut step = 0.001;
        let mut epsilon = 1.0 + (f32::EPSILON * 4.0) as f64;

        let mut time = *p_time;
        match self.opts.step_handling {
            BakeStepHandling::Default => {}
            BakeStepHandling::CustomDuration => {
                step = self.opts.step_custom_duration;
                epsilon = 1.0 + self.opts.step_custom_epsilon;
            }
            BakeStepHandling::IdenticalTime => return true,
            BakeStepHandling::AdjacentDouble => {
                return if left {
                    time = time.next_down();
                    *p_time = time;
                    time > prev_time
                } else {
                    time = time.next_up();
                    *p_time = time;
                    time < next_time
                };
            }
            BakeStepHandling::Ignore => return false,
        }

        if left {
            let min_time = (prev_time + step).max(add_epsilon(prev_time, epsilon));
            time = (time - step).min(sub_epsilon(time, epsilon));
            *p_time = time;
            time > min_time
        } else {
            let max_time = (next_time - step).min(sub_epsilon(next_time, epsilon));
            time = (time + step).max(add_epsilon(time, epsilon));
            *p_time = time;
            time < max_time
        }
    }

    fn offset_time(&self, time: f64) -> f64 {
        let scale = self.scene.metadata.ktime_second as f64;
        (time * scale + self.ktime_offset).round_ties_even() / scale
    }

    fn bake_postprocess_vec3(&self, mut src: Vec<BakedVec3>) -> (Vec<BakedVec3>, bool) {
        if src.is_empty() {
            return (src, false);
        }
        // `count` shrinks without dropping the entries past it: ufbx reads its first entry even at a count of zero
        let mut count = src.len();

        if self.ktime_offset != 0.0 {
            for key in &mut src {
                key.time = self.offset_time(key.time);
            }
        }

        {
            let mut dst = 0;
            let mut prev_time = src[0].time;
            for i in 0..count {
                let mut cur = src[i];
                let next_time = if i + 1 < count { src[i + 1].time } else { f64::INFINITY };
                let mut keep = true;
                if cur.flags & (STEP_LEFT | STEP_RIGHT) != 0 {
                    keep = self.postprocess_step(prev_time, next_time, &mut cur.time, cur.flags);
                }
                if keep {
                    src[dst] = cur;
                    dst += 1;
                    prev_time = cur.time;
                }
            }
            count = dst;
        }

        if self.opts.key_reduction_enabled {
            let threshold = self.opts.key_reduction_threshold * self.opts.key_reduction_threshold;
            for _ in 0..self.opts.key_reduction_passes {
                let mut dst = 1;
                let mut i = 1;
                while i < count {
                    let prev = src[i - 1];
                    let cur = src[i];
                    if i + 1 < count {
                        let next = src[i + 1];
                        let delta = (cur.time - prev.time) / (next.time - prev.time);
                        let tmp = lerp3(prev.value, next.value, delta);
                        let mut error = 0.0;
                        error += (tmp.x - cur.value.x) * (tmp.x - cur.value.x);
                        error += (tmp.y - cur.value.y) * (tmp.y - cur.value.y);
                        error += (tmp.z - cur.value.z) * (tmp.z - cur.value.z);
                        if error <= threshold {
                            src[dst] = src[i + 1];
                            i += 2;
                            dst += 1;
                            continue;
                        }
                    }

                    src[dst] = src[i];
                    dst += 1;
                    i += 1;
                }
                if dst == count {
                    break;
                }
                count = dst;
            }
        }

        let reference = src[0].value;
        let constant = src[1..count.max(1)].iter().all(|k| k.value.x == reference.x && k.value.y == reference.y && k.value.z == reference.z);

        src.truncate(count);
        (src, constant)
    }

    fn bake_postprocess_quat(&self, mut src: Vec<BakedQuat>) -> (Vec<BakedQuat>, bool) {
        if src.is_empty() {
            return (src, false);
        }
        let mut count = src.len();

        if self.ktime_offset != 0.0 {
            for key in &mut src {
                key.time = self.offset_time(key.time);
            }
        }

        {
            let mut dst = 0;
            let mut prev_time = src[0].time;
            for i in 0..count {
                let mut cur = src[i];
                let next_time = if i + 1 < count { src[i + 1].time } else { f64::INFINITY };
                let mut keep = true;
                if cur.flags & (STEP_LEFT | STEP_RIGHT) != 0 {
                    keep = self.postprocess_step(prev_time, next_time, &mut cur.time, cur.flags);
                }
                if keep {
                    prev_time = cur.time;
                    src[dst] = cur;
                    dst += 1;
                }
            }
            count = dst;
        }

        for i in 1..count {
            src[i].value = quat_fix_antipodal(src[i].value, src[i - 1].value);
        }

        if self.opts.key_reduction_enabled {
            let threshold = self.opts.key_reduction_threshold * self.opts.key_reduction_threshold;
            for _ in 0..self.opts.key_reduction_passes {
                let mut dst = 1;
                let mut i = 1;
                while i < count {
                    let prev = src[i - 1];
                    let cur = src[i];
                    if i + 1 < count {
                        let next = src[i + 1];
                        let delta = (cur.time - prev.time) / (next.time - prev.time);
                        let mut error = 0.0;

                        if self.opts.key_reduction_rotation {
                            let tmp = quat_slerp(prev.value, next.value, delta);
                            error += (tmp.x - cur.value.x) * (tmp.x - cur.value.x);
                            error += (tmp.y - cur.value.y) * (tmp.y - cur.value.y);
                            error += (tmp.z - cur.value.z) * (tmp.z - cur.value.z);
                            error += (tmp.w - cur.value.w) * (tmp.w - cur.value.w);
                        } else {
                            error += (prev.value.x - cur.value.x) * (prev.value.x - cur.value.x);
                            error += (prev.value.y - cur.value.y) * (prev.value.y - cur.value.y);
                            error += (prev.value.z - cur.value.z) * (prev.value.z - cur.value.z);
                            error += (prev.value.w - cur.value.w) * (prev.value.w - cur.value.w);
                            error += (next.value.x - cur.value.x) * (next.value.x - cur.value.x);
                            error += (next.value.y - cur.value.y) * (next.value.y - cur.value.y);
                            error += (next.value.z - cur.value.z) * (next.value.z - cur.value.z);
                            error += (next.value.w - cur.value.w) * (next.value.w - cur.value.w);
                            error *= 0.5;
                        }

                        if error <= threshold {
                            src[dst] = src[i + 1];
                            i += 2;
                            dst += 1;
                            continue;
                        }
                    }

                    src[dst] = src[i];
                    dst += 1;
                    i += 1;
                }
                if dst == count {
                    break;
                }
                count = dst;
            }
        }

        let r = src[0].value;
        let constant = src[1..count.max(1)].iter().all(|k| k.value.x == r.x && k.value.y == r.y && k.value.z == r.z && k.value.w == r.w);

        src.truncate(count);
        (src, constant)
    }

    fn bake_node_imp(&mut self, element_id: u32, props: &[BakeProp]) {
        let scene = self.scene;
        let node_ix = scene.elements[element_id as usize].index;
        let node = &scene.nodes[node_ix];

        let mut complex_translation = false;
        let mut complex_rotation = false;

        for name in COMPLEX_TRANSLATION_PROPS {
            if let Some(prop) = node.element.props.find_prop(name)
                && !is_vec3_zero(prop.value_vec3())
            {
                complex_translation = true;
            }
            if props.iter().any(|p| p.prop_name == name) {
                complex_translation = true;
            }
        }

        for name in COMPLEX_ROTATION_PROPS {
            if props.iter().any(|p| p.prop_name == name) {
                complex_rotation = true;
            }
        }

        let mut resample_translation = false;

        // The parent's resampled scale helper keeps the translation scaled as the parent is
        let mut scale_helper_t = None;
        let mut constant_scale_t = Vec3::ONE;
        if !node.is_scale_helper
            && let Some(parent) = node.parent
            && let Some(helper) = scene.nodes[parent].scale_helper
        {
            scale_helper_t = self.baked_nodes[scene.nodes[helper].element.typed_id as usize];
            if let Some(h) = scale_helper_t {
                if !self.nodes[h].constant_scale {
                    resample_translation = true;
                }
                push_resampled_times(&mut self.tmp_times, &self.nodes[h].scale_keys);
            } else {
                constant_scale_t = scene.nodes[helper].inherit_scale;
            }
        }

        if complex_translation {
            for prop in props {
                // Any transform property can move a complex translation
                if TRANSFORM_PROPS.contains(&prop.prop_name) {
                    let resample_linear = resample_translation || prop.prop_name != "Lcl Translation";
                    let key_flag = if prop.prop_name == "Lcl Translation" { KEYFRAME } else { 0 };
                    self.bake_times(prop.anim_value, resample_linear, key_flag);
                }
            }
        } else {
            for prop in props {
                if prop.prop_name == "Lcl Translation" {
                    self.bake_times(prop.anim_value, resample_translation, KEYFRAME);
                }
            }
        }

        let times_t = self.finalize_bake_times();

        if complex_rotation {
            for prop in props {
                if COMPLEX_ROTATION_SOURCES.contains(&prop.prop_name) {
                    let resample_linear = !self.opts.no_resample_rotation || prop.prop_name != "Lcl Rotation";
                    let key_flag = if prop.prop_name == "Lcl Rotation" { KEYFRAME } else { 0 };
                    self.bake_times(prop.anim_value, resample_linear, key_flag);
                }
            }
        } else {
            for prop in props {
                if prop.prop_name == "Lcl Rotation" {
                    self.bake_times(prop.anim_value, !self.opts.no_resample_rotation, KEYFRAME);
                }
            }
        }
        let times_r = self.finalize_bake_times();

        let mut resample_scale = false;

        let mut scale_helper_s = None;
        let mut constant_scale_s = Vec3::ONE;
        if node.is_scale_helper
            && let Some(parent) = node.parent
            && let Some(inherit_node) = scene.nodes[parent].inherit_scale_node
            && let Some(inherit_helper) = scene.nodes[inherit_node].scale_helper
        {
            scale_helper_s = self.baked_nodes[scene.nodes[inherit_helper].element.typed_id as usize];
            if let Some(h) = scale_helper_s {
                if !self.nodes[h].constant_scale {
                    resample_scale = true;
                }
                push_resampled_times(&mut self.tmp_times, &self.nodes[h].scale_keys);
            } else {
                constant_scale_s = scene.nodes[inherit_helper].local_transform.scale;
            }
        }

        for prop in props {
            if prop.prop_name == "Lcl Scaling" {
                self.bake_times(prop.anim_value, resample_scale, KEYFRAME);
            }
        }
        let times_s = self.finalize_bake_times();

        let mut keys_t = vec![BakedVec3::default(); times_t.len()];
        let mut keys_r = vec![BakedQuat::default(); times_r.len()];
        let mut keys_s = vec![BakedVec3::default(); times_s.len()];

        let (mut ix_t, mut ix_r, mut ix_s) = (0, 0, 0);
        while ix_t < times_t.len() || ix_r < times_r.len() || ix_s < times_s.len() {
            let mut bake_time = BakeTime { time: f64::INFINITY, flags: 0 };
            let (mut flags_r, mut flags_t, mut flags_s) = (0, 0, 0);

            let mut flags = 0;
            if ix_r < times_r.len() {
                bake_time = times_r[ix_r];
                flags_r = bake_time.flags;
                bake_time.flags &= 0x7;
                flags |= TRANSFORM_FLAG_INCLUDE_ROTATION;
            }
            if ix_t < times_t.len() {
                let t = times_t[ix_t];
                let cmp = cmp_bake_time(t, bake_time);
                if cmp <= 0 {
                    if cmp < 0 {
                        bake_time = t;
                        flags = 0;
                    }
                    bake_time.flags |= t.flags & 0x7;
                    flags_t = t.flags;
                    flags |= TRANSFORM_FLAG_INCLUDE_TRANSLATION;
                }
            }
            if ix_s < times_s.len() {
                let t = times_s[ix_s];
                let cmp = cmp_bake_time(t, bake_time);
                if cmp <= 0 {
                    if cmp < 0 {
                        bake_time = t;
                        flags = 0;
                    }
                    bake_time.flags |= t.flags & 0x7;
                    flags_s = t.flags;
                    flags |= TRANSFORM_FLAG_INCLUDE_SCALE;
                }
            }

            flags |= TRANSFORM_FLAG_IGNORE_SCALE_HELPER | TRANSFORM_FLAG_IGNORE_COMPONENTWISE_SCALE | TRANSFORM_FLAG_EXPLICIT_INCLUDES;
            if self.opts.evaluate_flags & EVALUATE_FLAG_NO_EXTRAPOLATION != 0 {
                flags |= TRANSFORM_FLAG_NO_EXTRAPOLATION;
            }

            let eval_time = bake_time_sample_time(bake_time);
            let mut transform = evaluate_transform_flags(scene, self.anim, node_ix, eval_time, flags);

            if flags & TRANSFORM_FLAG_INCLUDE_TRANSLATION != 0 {
                if let Some(h) = scale_helper_t {
                    let scale = evaluate_baked_vec3(&self.nodes[h].scale_keys, eval_time);
                    transform.translation.x *= scale.x;
                    transform.translation.y *= scale.y;
                    transform.translation.z *= scale.z;
                }

                transform.translation.x *= constant_scale_t.x;
                transform.translation.y *= constant_scale_t.y;
                transform.translation.z *= constant_scale_t.z;

                keys_t[ix_t] = BakedVec3 { time: bake_time.time, value: transform.translation, flags: bake_time.flags | flags_t };
                ix_t += 1;
            }
            if flags & TRANSFORM_FLAG_INCLUDE_ROTATION != 0 {
                keys_r[ix_r] = BakedQuat { time: bake_time.time, value: transform.rotation, flags: bake_time.flags | flags_r };
                ix_r += 1;
            }
            if flags & TRANSFORM_FLAG_INCLUDE_SCALE != 0 {
                if let Some(h) = scale_helper_s {
                    let scale = evaluate_baked_vec3(&self.nodes[h].scale_keys, eval_time);
                    transform.scale.x *= scale.x;
                    transform.scale.y *= scale.y;
                    transform.scale.z *= scale.z;
                }

                transform.scale.x *= constant_scale_s.x;
                transform.scale.y *= constant_scale_s.y;
                transform.scale.z *= constant_scale_s.z;

                keys_s[ix_s] = BakedVec3 { time: bake_time.time, value: transform.scale, flags: bake_time.flags | flags_s };
                ix_s += 1;
            }
        }

        let (translation_keys, constant_translation) = self.bake_postprocess_vec3(keys_t);
        let (rotation_keys, constant_rotation) = self.bake_postprocess_quat(keys_r);
        let (scale_keys, constant_scale) = self.bake_postprocess_vec3(keys_s);
        self.nodes.push(BakedNode {
            typed_id: node.element.typed_id,
            element_id: node.element.element_id,
            constant_translation,
            constant_rotation,
            constant_scale,
            translation_keys,
            rotation_keys,
            scale_keys,
        });
        self.baked_nodes[node.element.typed_id as usize] = Some(self.nodes.len() - 1);

        // A scale helper's siblings, and their own scale helpers, are baked with it even when the animation leaves them
        if node.is_scale_helper
            && let Some(parent) = node.parent
        {
            for &child_ix in &scene.nodes[parent].children {
                if child_ix == node_ix {
                    continue;
                }
                let child = &scene.nodes[child_ix];
                if !self.nodes_to_bake[child.element.typed_id as usize] {
                    self.nodes_to_bake[child.element.typed_id as usize] = true;
                    self.bake_stack.push(child.element.element_id);
                }
                if let Some(inherit_node) = child.inherit_scale_node
                    && let Some(inherit_helper) = scene.nodes[inherit_node].scale_helper
                    && let Some(child_helper) = child.scale_helper
                    && self.nodes_to_bake[scene.nodes[inherit_helper].element.typed_id as usize]
                {
                    let helper_id = scene.nodes[child_helper].element.typed_id as usize;
                    if !self.nodes_to_bake[helper_id] {
                        self.nodes_to_bake[helper_id] = true;
                        self.bake_stack.push(scene.nodes[child_helper].element.element_id);
                    }
                }
            }
        }
    }

    fn bake_node(&mut self, element_id: u32, props: &[BakeProp]) {
        self.bake_node_imp(element_id, props);
        while let Some(child_id) = self.bake_stack.pop() {
            self.bake_node_imp(child_id, &[]);
        }
    }

    fn bake_anim_prop(&mut self, element_id: u32, prop_name: &str, props: &[BakeProp]) {
        for prop in props {
            self.bake_times(prop.anim_value, false, KEYFRAME);
        }

        let times = self.finalize_bake_times();
        let name = c_str(prop_name);

        let keys = times
            .iter()
            .map(|&bake_time| {
                let eval_time = bake_time_sample_time(bake_time);
                let prop = evaluate_prop_flags(self.scene, self.anim, element_id, name, eval_time, self.opts.evaluate_flags);
                BakedVec3 { time: bake_time.time, value: prop.value_vec3(), flags: bake_time.flags }
            })
            .collect();

        let (keys, constant_value) = self.bake_postprocess_vec3(keys);
        self.props.push(BakedProp { name: name.to_string(), constant_value, keys });
    }

    fn bake_element(&mut self, element_id: u32, props: &[BakeProp]) {
        let is_node = self.scene.elements[element_id as usize].element_type == ElementType::Node;
        if is_node && !self.opts.skip_node_transforms {
            self.bake_node(element_id, props);
        }

        let mut begin = 0;
        while begin < props.len() {
            let prop_name = props[begin].prop_name;
            let mut end = begin + 1;
            while end < props.len() && props[end].prop_name == prop_name {
                end += 1;
            }

            if is_node && !self.opts.bake_transform_props && TRANSFORM_PROPS.contains(&prop_name) {
                begin = end;
                continue;
            }

            self.bake_anim_prop(element_id, prop_name, &props[begin..end]);
            begin = end;
        }

        if !self.props.is_empty() {
            let props = take(&mut self.props);
            self.elements.push(BakedElement { element_id, props });
        }
    }

    fn bake_anim(&mut self) -> BakedAnim {
        let scene = self.scene;
        let anim = self.anim;

        if !self.opts.skip_node_transforms {
            self.baked_nodes = vec![None; scene.nodes.len()];
            self.nodes_to_bake = vec![false; scene.nodes.len()];
        }

        let mut props: Vec<BakeProp> = Vec::new();
        for &layer in &anim.layers {
            for anim_prop in &scene.anim_layers[layer].anim_props {
                let element = scene.element(anim_prop.element);

                // Nodes go first, by `typed_id`, so that they are baked in order
                let sort_id = if element.element_type == ElementType::Node {
                    if let Some(to_bake) = self.nodes_to_bake.get_mut(element.typed_id as usize) {
                        *to_bake = true;
                    }
                    element.typed_id
                } else {
                    u32::MAX
                };

                props.push(BakeProp { sort_id, element_id: element.element_id, prop_name: &anim_prop.prop_name, anim_value: anim_prop.anim_value });
            }
        }

        unstable_sort(&mut props, bake_prop_less);

        if !self.opts.ignore_layer_weight_animation {
            let mut has_weight_times = false;
            for prop in &props {
                if prop.prop_name != "Weight" {
                    continue;
                }
                if scene.elements[prop.element_id as usize].element_type == ElementType::AnimLayer {
                    self.bake_times(prop.anim_value, true, 0);
                    has_weight_times = true;
                }
            }

            if has_weight_times {
                self.layer_weight_times = self.finalize_bake_times();
            }
        }

        let mut begin = 0;
        while begin < props.len() {
            let element_id = props[begin].element_id;
            let mut end = begin + 1;
            while end < props.len() && props[end].element_id == element_id {
                end += 1;
            }
            self.bake_element(element_id, &props[begin..end]);
            begin = end;
        }

        let mut bake = BakedAnim { nodes: take(&mut self.nodes), elements: take(&mut self.elements), ..Default::default() };

        // `ufbxi_baked_node_less` in ufbx.c
        unstable_sort(&mut bake.nodes, |a, b| a.typed_id < b.typed_id);
        // `ufbxi_baked_element_less` in ufbx.c
        unstable_sort(&mut bake.elements, |a, b| a.element_id < b.element_id);

        if self.time_min < self.time_max {
            bake.key_time_min = self.time_min;
            bake.key_time_max = self.time_max;
        }

        if self.time_begin < self.time_end {
            bake.playback_time_begin = self.time_begin;
            bake.playback_time_end = self.time_end;
            bake.playback_duration = self.time_end - self.time_begin;
        }

        bake
    }
}

/// With `ufbxi_bake_anim_imp` in ufbx.c: the memory statistics in `metadata` are left at zero
pub fn bake_anim(scene: &Scene, anim: &Anim, opts: &BakeOpts) -> Res<BakedAnim> {
    let mut opts = opts.clone();
    if opts.resample_rate <= 0.0 {
        opts.resample_rate = 30.0;
    }
    if opts.minimum_sample_rate <= 0.0 {
        opts.minimum_sample_rate = 19.5;
    }
    if opts.max_keyframe_segments == 0 {
        opts.max_keyframe_segments = 32;
    }
    if opts.key_reduction_threshold == 0.0 {
        opts.key_reduction_threshold = 0.000001;
    }
    if opts.key_reduction_passes == 0 {
        opts.key_reduction_passes = 4;
    }

    let mut ktime_offset = 0.0;
    if opts.trim_start_time && anim.time_begin > 0.0 {
        ktime_offset = -anim.time_begin * scene.metadata.ktime_second as f64;
    }

    let (mut time_begin, mut time_end) = (0.0, 0.0);
    if anim.time_begin < anim.time_end {
        time_begin = anim.time_begin;
        time_end = anim.time_end;
    }

    let mut bc = BakeContext {
        scene,
        anim,
        opts,
        tmp_times: Vec::new(),
        layer_weight_times: Vec::new(),
        baked_nodes: Vec::new(),
        nodes_to_bake: Vec::new(),
        bake_stack: Vec::new(),
        nodes: Vec::new(),
        elements: Vec::new(),
        props: Vec::new(),
        ktime_offset,
        time_begin,
        time_end,
        time_min: f64::INFINITY,
        time_max: -f64::INFINITY,
    };
    Ok(bc.bake_anim())
}

pub fn find_baked_node_by_typed_id(bake: &BakedAnim, typed_id: u32) -> Option<&BakedNode> {
    let nodes = &bake.nodes;
    lower_bound_eq(0, nodes.len(), 8, |i| nodes[i].typed_id < typed_id, |i| nodes[i].typed_id == typed_id).map(|i| &nodes[i])
}

/// `node` is an index of `Scene::nodes`
pub fn find_baked_node(bake: &BakedAnim, node: usize) -> Option<&BakedNode> {
    find_baked_node_by_typed_id(bake, u32::try_from(node).ok()?)
}

pub fn find_baked_element_by_element_id(bake: &BakedAnim, element_id: u32) -> Option<&BakedElement> {
    let elements = &bake.elements;
    lower_bound_eq(0, elements.len(), 8, |i| elements[i].element_id < element_id, |i| elements[i].element_id == element_id).map(|i| &elements[i])
}

pub fn find_baked_element(bake: &BakedAnim, element: ElementId) -> Option<&BakedElement> {
    find_baked_element_by_element_id(bake, element)
}

/// Zero with no keys, where ufbx reads before the list
pub fn evaluate_baked_vec3(keyframes: &[BakedVec3], time: f64) -> Vec3 {
    let keys = keyframes;
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
        let next = keys[begin];
        if next.time <= time {
            begin += 1;
            continue;
        }
        if begin == 0 {
            return next.value;
        }

        let mut prev = begin - 1;
        if prev > 0 && keys[prev].flags & STEP_RIGHT != 0 && keys[prev - 1].time == time {
            prev -= 1;
        }
        let prev = keys[prev];
        if time == prev.time {
            return prev.value;
        }
        let mut t = (time - prev.time) / (next.time - prev.time);
        if prev.flags & STEP_LEFT != 0 {
            t = 0.0;
        }
        if next.flags & STEP_RIGHT != 0 {
            t = 1.0;
        }
        return lerp3(prev.value, next.value, t);
    }

    keys.last().map_or(Vec3::ZERO, |k| k.value)
}

/// The identity with no keys, where ufbx reads before the list
pub fn evaluate_baked_quat(keyframes: &[BakedQuat], time: f64) -> Quat {
    let keys = keyframes;
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
        let next = keys[begin];
        if next.time <= time {
            begin += 1;
            continue;
        }
        if begin == 0 {
            return next.value;
        }

        // ufbx steps back here without looking at the flags, unlike in `evaluate_baked_vec3`
        let mut prev = begin - 1;
        if prev > 0 && keys[prev - 1].time == time {
            prev -= 1;
        }
        if time == keys[prev].time {
            return keys[prev].value;
        }
        let mut t = (time - keys[prev].time) / (next.time - keys[prev].time);
        if prev > 0 && keys[prev].flags & STEP_RIGHT != 0 && keys[prev - 1].time == time {
            prev -= 1;
        }
        if keys[prev].flags & STEP_LEFT != 0 {
            t = 0.0;
        }
        if next.flags & STEP_RIGHT != 0 {
            t = 1.0;
        }
        return quat_slerp(keys[prev].value, next.value, t);
    }

    keys.last().map_or(Quat::IDENTITY, |k| k.value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::load_memory;

    const SCENE: &str = r#"; FBX 7.4.0 project file
FBXHeaderExtension:  {
	FBXHeaderVersion: 1003
	FBXVersion: 7400
}
GlobalSettings:  {
	Version: 1000
	Properties70:  {
		P: "TimeMode", "enum", "", "",6
	}
}
Objects:  {
	Model: 1, "Model::Stepped", "Null" {
		Version: 232
	}
	Model: 7, "Model::Ramp", "Null" {
		Version: 232
	}
	AnimationStack: 2, "AnimStack::Take", "" {
		Properties70:  {
			P: "LocalStart", "KTime", "Time", "",0
			P: "LocalStop", "KTime", "Time", "",92372316000
		}
	}
	AnimationLayer: 3, "AnimLayer::Base", "" {
	}
	AnimationCurveNode: 4, "AnimCurveNode::T", "" {
		Properties70:  {
			P: "d|X", "Number", "", "A",0
			P: "d|Y", "Number", "", "A",0
			P: "d|Z", "Number", "", "A",0
		}
	}
	AnimationCurve: 5, "AnimCurve::", "" {
		Default: 0
		KeyVer: 4009
		KeyTime: *3 {
			a: 0,46186158000,92372316000
		}
		KeyValueFloat: *3 {
			a: 0,1,2
		}
		KeyAttrFlags: *1 {
			a: 2
		}
		KeyAttrDataFloat: *4 {
			a: 0,0,218434821,0
		}
		KeyAttrRefCount: *1 {
			a: 3
		}
	}
	AnimationCurve: 6, "AnimCurve::", "" {
		Default: 0
		KeyVer: 4009
		KeyTime: *2 {
			a: 0,92372316000
		}
		KeyValueFloat: *2 {
			a: 0,4
		}
		KeyAttrFlags: *1 {
			a: 4
		}
		KeyAttrDataFloat: *4 {
			a: 0,0,218434821,0
		}
		KeyAttrRefCount: *1 {
			a: 2
		}
	}
	AnimationCurveNode: 8, "AnimCurveNode::T", "" {
		Properties70:  {
			P: "d|X", "Number", "", "A",0
			P: "d|Y", "Number", "", "A",0
			P: "d|Z", "Number", "", "A",0
		}
	}
	AnimationCurve: 9, "AnimCurve::", "" {
		Default: 0
		KeyVer: 4009
		KeyTime: *2 {
			a: 0,92372316000
		}
		KeyValueFloat: *2 {
			a: 0,4
		}
		KeyAttrFlags: *1 {
			a: 8
		}
		KeyAttrDataFloat: *4 {
			a: 2,2,218434821,0
		}
		KeyAttrRefCount: *1 {
			a: 2
		}
	}
}
Connections:  {
	C: "OO",1,0
	C: "OO",7,0
	C: "OO",3,2
	C: "OO",4,3
	C: "OO",8,3
	C: "OP",4,1, "Lcl Translation"
	C: "OP",5,4, "d|X"
	C: "OP",6,4, "d|Y"
	C: "OP",8,7, "Lcl Translation"
	C: "OP",9,8, "d|X"
}
"#;

    fn scene() -> Scene {
        load_memory(SCENE.as_bytes(), &LoadOpts::default()).unwrap()
    }

    fn node_index(scene: &Scene, name: &str) -> usize {
        scene.nodes.iter().position(|n| n.element.name == name).unwrap()
    }

    fn bake(scene: &Scene, opts: &BakeOpts) -> BakedAnim {
        bake_anim(scene, &scene.anim_stacks[0].anim, opts).unwrap()
    }

    fn times_and_flags(keys: &[BakedVec3]) -> Vec<(f64, u32)> {
        keys.iter().map(|k| (k.time, k.flags)).collect()
    }

    #[cfg(feature = "format-fbx")]
    #[test]
    fn a_stepped_curve_bakes_to_steps_a_millisecond_wide() {
        let scene = scene();
        let bake = bake(&scene, &BakeOpts::default());
        let node = find_baked_node(&bake, node_index(&scene, "Stepped")).unwrap();

        let step_key = KEYFRAME | STEP_KEY;
        assert_eq!(
            times_and_flags(&node.translation_keys),
            [(0.0, KEYFRAME), (1.0 - 0.001, STEP_LEFT), (1.0, step_key), (2.0 - 0.001, STEP_LEFT), (2.0, step_key)]
        );
        let x: Vec<Real> = node.translation_keys.iter().map(|k| k.value.x).collect();
        assert_eq!(x, [0.0, 0.0, 1.0, 1.0, 2.0]);
        assert!((node.translation_keys[1].value.y - 2.0).abs() < 1e-9);
        assert!(!node.constant_translation && node.constant_rotation && node.constant_scale);
        assert_eq!(node.rotation_keys.len(), 2);

        assert_eq!((bake.playback_time_begin, bake.playback_time_end, bake.playback_duration), (0.0, 2.0, 2.0));
        assert_eq!((bake.key_time_min, bake.key_time_max), (0.0, 2.0));
        assert!(bake.elements.is_empty(), "transform properties are baked only when asked");
        assert!(find_baked_node(&bake, scene.root_node).is_none());
    }

    #[cfg(feature = "format-fbx")]
    #[test]
    fn step_handling_places_or_drops_the_step_keys() {
        let scene = scene();
        let stepped = node_index(&scene, "Stepped");
        let times = |step_handling| {
            let bake = bake(&scene, &BakeOpts { step_handling, ..Default::default() });
            find_baked_node(&bake, stepped).unwrap().translation_keys.iter().map(|k| k.time).collect::<Vec<_>>()
        };
        assert_eq!(times(BakeStepHandling::IdenticalTime), [0.0, 1.0, 1.0, 2.0, 2.0]);
        assert_eq!(times(BakeStepHandling::AdjacentDouble), [0.0, 1.0f64.next_down(), 1.0, 2.0f64.next_down(), 2.0]);
        assert_eq!(times(BakeStepHandling::Ignore), [0.0, 1.0, 2.0]);
        let custom = BakeStepHandling::CustomDuration;
        let bake = bake(&scene, &BakeOpts { step_handling: custom, step_custom_duration: 0.25, ..Default::default() });
        assert_eq!(find_baked_node(&bake, stepped).unwrap().translation_keys[1].time, 0.75);
    }

    #[cfg(feature = "format-fbx")]
    #[test]
    fn a_cubic_curve_is_resampled_and_its_keys_on_a_line_reduced() {
        let scene = scene();
        let ramp = node_index(&scene, "Ramp");

        let bake_ramp = |opts: &BakeOpts| find_baked_node(&bake(&scene, opts), ramp).unwrap().translation_keys.clone();
        let keys = bake_ramp(&BakeOpts::default());
        assert_eq!(keys.len(), 31, "at most 32 segments between two keys: every other of 30 samples a second");
        assert_eq!((keys[1].time, keys[1].flags), (2.0 / 30.0, 0));
        assert_eq!(bake_ramp(&BakeOpts { max_keyframe_segments: 64, ..Default::default() }).len(), 61);
        let keys = bake_ramp(&BakeOpts { resample_rate: 10.0, ..Default::default() });
        assert_eq!(keys.len(), 21);

        let reduced = bake_ramp(&BakeOpts { key_reduction_enabled: true, ..Default::default() });
        assert!(reduced.len() < 8, "every pass drops every other key on the line: {}", reduced.len());
        assert_eq!((reduced[0].time, reduced.last().unwrap().time), (0.0, 2.0));

        let thinned = bake_ramp(&BakeOpts { maximum_sample_rate: 5.0, ..Default::default() });
        assert!(thinned.windows(2).all(|w| w[1].time - w[0].time > 0.19), "{:?}", times_and_flags(&thinned));
    }

    #[cfg(feature = "format-fbx")]
    #[test]
    fn trimming_moves_the_keys_to_start_from_zero() {
        let mut scene = scene();
        let stepped = node_index(&scene, "Stepped");
        let mut anim = (*scene.anim_stacks[0].anim).clone();
        anim.time_begin = 1.0;
        scene.anim_stacks[0].anim = anim.into();
        let bake = bake(&scene, &BakeOpts { trim_start_time: true, ..Default::default() });
        let keys = &find_baked_node(&bake, stepped).unwrap().translation_keys;
        assert_eq!((keys[0].time, keys[2].time, keys[4].time), (-1.0, 0.0, 1.0));
        assert_eq!(bake.playback_time_begin, 1.0, "the playback range stays the animation's");
    }

    fn v(time: f64, x: Real, flags: u32) -> BakedVec3 {
        BakedVec3 { time, value: Vec3::new(x, 0.0, 0.0), flags }
    }

    #[test]
    fn baked_vectors_interpolate_linearly_and_hold_their_steps() {
        let keys = [v(0.0, 0.0, KEYFRAME), v(1.0, 10.0, KEYFRAME), v(2.0, 10.0, STEP_LEFT), v(2.0, 20.0, STEP_KEY), v(3.0, 30.0, 0)];
        assert_eq!(evaluate_baked_vec3(&keys, -1.0).x, 0.0);
        assert_eq!(evaluate_baked_vec3(&keys, 0.25).x, 2.5);
        assert_eq!(evaluate_baked_vec3(&keys, 1.5).x, 10.0);
        assert_eq!(evaluate_baked_vec3(&keys, 2.0).x, 20.0);
        assert_eq!(evaluate_baked_vec3(&keys, 2.5).x, 25.0);
        assert_eq!(evaluate_baked_vec3(&keys, 9.0).x, 30.0);
        assert_eq!(evaluate_baked_vec3(&[], 1.0), Vec3::ZERO);

        let many: Vec<BakedVec3> = (0..40).map(|i| v(i as f64, 2.0 * i as f64, 0)).collect();
        assert_eq!(evaluate_baked_vec3(&many, 30.5).x, 61.0);
    }

    #[test]
    fn baked_rotations_slerp_and_step_back_over_keys_at_the_time() {
        let q = |time: f64, z: Real, flags: u32| {
            let half = z * 0.5;
            BakedQuat { time, value: Quat { x: 0.0, y: 0.0, z: half.sin(), w: half.cos() }, flags }
        };
        let quarter = std::f64::consts::FRAC_PI_2;
        let keys = [q(0.0, 0.0, 0), q(1.0, quarter, 0), q(1.0, 2.0 * quarter, 0), q(2.0, 2.0 * quarter, 0)];
        let mid = evaluate_baked_quat(&keys, 0.5);
        assert!((mid.z - (quarter * 0.25).sin()).abs() < 1e-12);
        // ufbx's quirk: at a time held by two keys the earlier one wins, whatever their flags
        assert_eq!(evaluate_baked_quat(&keys, 1.0), keys[1].value);
        assert_eq!(evaluate_baked_quat(&keys, 5.0), keys[3].value);
        assert_eq!(evaluate_baked_quat(&[], 1.0), Quat::IDENTITY);
    }

    #[test]
    fn baked_nodes_and_elements_are_found_by_their_ids() {
        let bake = BakedAnim {
            nodes: (0..20).map(|i| BakedNode { typed_id: i * 2, element_id: i * 3, ..Default::default() }).collect(),
            elements: (0..20).map(|i| BakedElement { element_id: i * 3, ..Default::default() }).collect(),
            ..Default::default()
        };
        assert_eq!(find_baked_node_by_typed_id(&bake, 14).map(|n| n.element_id), Some(21));
        assert!(find_baked_node_by_typed_id(&bake, 15).is_none());
        assert_eq!(find_baked_node(&bake, 38).map(|n| n.typed_id), Some(38));
        assert_eq!(find_baked_element_by_element_id(&bake, 57).map(|e| e.element_id), Some(57));
        assert!(find_baked_element(&bake, 58).is_none());
    }
}
