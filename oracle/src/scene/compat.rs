use super::{record, Outcome, rt};
use crate::runner::{Args, Child, Section, Status, Tally};
use load3d::compat as x;
use rt::Cx;
use std::fmt::Debug;
use ufbx as u;

trait CRef<T> {
    fn get(&self) -> Option<&T>;
}

impl<T> CRef<T> for x::Ref<T> {
    fn get(&self) -> Option<&T> {
        if self.is_null() { None } else { Some(&**self) }
    }
}

impl<T> CRef<T> for Option<x::Ref<T>> {
    fn get(&self) -> Option<&T> {
        self.as_ref().and_then(|r| r.get())
    }
}

fn same_enum<A: Debug, B: Debug>(cx: &mut Cx, a: &A, b: &B) {
    let (a, b) = (format!("{a:?}"), format!("{b:?}"));
    if !cx.same(a == b) {
        cx.diff(a, b);
    }
}

pub fn chk_DomValue(cx: &mut Cx, c: &u::DomValue, b: &x::DomValue) {
    same_enum(cx, &c.type_, &b.type_);
    cx.str(rt::cstr(&c.value_str), b.value_str.as_bytes());
    cx.int(c.value_int as i128, b.value_int as i128);
    cx.f64(c.value_float, b.value_float);
}

fn dom_arrays(cx: &mut Cx, c: &u::DomNode, b: &x::DomNode) {
    cx.bool(c.is_array(), b.is_array());
    cx.int(c.array_size() as i128, b.array_size() as i128);
    cx.same(c.as_int32_list() == b.as_int32_list());
    cx.same(c.as_int64_list() == b.as_int64_list());
    let (cf, bf) = (c.as_double_list(), b.as_double_list());
    cx.len(cf.len(), bf.len());
    for (a, d) in cf.iter().zip(bf) {
        cx.f64(*a, *d);
    }
    let (cb, bb) = (c.as_blob_list(), b.as_blob_list());
    cx.len(cb.len(), bb.len());
    for (a, d) in cb.iter().zip(bb) {
        cx.bytes(a, d);
    }
}

include!(concat!(env!("OUT_DIR"), "/generated_compat_cmp.rs"));

fn compare_scenes(t: &mut Tally, c: &u::Scene, b: &x::Scene, keep: usize) {
    let mut cx = Cx::new(c, keep);
    chk_Scene(&mut cx, c, b);
    let o = Outcome { checks: cx.checks, count: cx.count, kept: cx.kept, by_field: cx.by_field };
    let n = o.count;
    record(t, o);
    if n > 0 {
        t.status(Status::Differ, format!("{n} differences"));
    }
}

fn compare(t: &mut Tally, c: Result<u::SceneRoot, u::Error>, b: Result<x::SceneRoot, x::Error>, keep: usize) {
    match (&c, &b) {
        (Ok(c), Ok(b)) => compare_scenes(t, c, b, keep),
        (Err(c), Err(b)) => {
            let same = format!("{:?}", c.type_) == format!("{:?}", b.type_) && *c.description == *b.description && c.info() == b.info();
            t.checks += 1;
            t.status(if same { Status::ErrorEqual } else { Status::ErrorMismatch }, format!("C {c:?} / layer {b:?}"));
        }
        (Ok(_), Err(b)) => t.status(Status::ErrorMismatch, format!("C loads / layer {b:?}")),
        (Err(c), Ok(_)) => t.status(Status::ErrorMismatch, format!("C {c:?} / layer loads")),
    }
}

fn bake(t: &mut Tally, c: &u::SceneRoot, b: &x::SceneRoot, keep: usize) {
    match (u::bake_anim(c, &c.anim, Default::default()), x::bake_anim(b, &b.anim, Default::default())) {
        (Ok(cb), Ok(bb)) => {
            let mut cx = Cx::detached(keep);
            chk_BakedAnim(&mut cx, &cb, &bb);
            let o = Outcome { checks: cx.checks, count: cx.count, kept: cx.kept, by_field: cx.by_field };
            let n = o.count;
            record(t, o);
            if n > 0 {
                t.status(Status::Differ, format!("{n} differences"));
            }
        }
        (Err(_), Err(_)) => t.status(Status::ErrorEqual, String::new()),
        _ => t.status(Status::ErrorMismatch, "only one baked".to_string()),
    }
}

fn run(a: &Args, path: &str, ch: &mut Child) {
    let bytes = std::fs::read(path).unwrap_or_default();
    let keep = a.keep;
    let name = || path.to_string();
    ch.step("default", |t| {
        let c = u::load_memory(&bytes, u::LoadOpts { filename: name().into(), ..Default::default() });
        let b = x::load_memory(&bytes, x::LoadOpts { filename: name().into(), ..Default::default() });
        compare(t, c, b, keep);
    });
    ch.step("dom", |t| {
        let c = u::load_memory(&bytes, u::LoadOpts { filename: name().into(), retain_dom: true, ..Default::default() });
        let b = x::load_memory(&bytes, x::LoadOpts { filename: name().into(), retain_dom: true, ..Default::default() });
        compare(t, c, b, keep);
    });
    ch.step("z_up_m", |t| {
        let c = u::load_memory(&bytes, u::LoadOpts { filename: name().into(), target_axes: u::axes_right_handed_z_up(), target_unit_meters: 1.0, ..Default::default() });
        let b = x::load_memory(&bytes, x::LoadOpts { filename: name().into(), target_axes: x::axes_right_handed_z_up(), target_unit_meters: 1.0, ..Default::default() });
        compare(t, c, b, keep);
    });
    ch.step("evaluate_bake", |t| {
        let c = u::load_memory(&bytes, u::LoadOpts { filename: name().into(), ..Default::default() });
        let b = x::load_memory(&bytes, x::LoadOpts { filename: name().into(), ..Default::default() });
        if let (Ok(c), Ok(b)) = (c, b) {
            let es = u::evaluate_scene(&c, &c.anim, 0.25, Default::default());
            let eb = x::evaluate_scene(&b, &b.anim, 0.25, Default::default());
            compare(t, es, eb, keep);
            bake(t, &c, &b, keep);
        }
    });
}

pub const COMPAT: Section = Section { name: "compat", jobs: super::scene_jobs, run, header: None };
