// the comparator's names follow ufbx's types; its raw reads take pointers ufbx keeps valid while the scene lives
#![allow(non_snake_case, clippy::not_unsafe_ptr_arg_deref)]

pub mod rt;
pub mod special;

#[allow(unused_variables, unused_mut, unused_braces, unused_parens, dead_code, clippy::all)]
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated_cmp.rs"));
}

use std::collections::{BTreeMap, HashMap};

use crate::common::{Rng, both};
use crate::runner::{Args, Child, Job, Section, Status, Tally, corpus_jobs};

#[derive(Debug, Default)]
pub struct Outcome {
    pub checks: u64,
    pub count: usize,
    pub kept: Vec<rt::Diff>,
    /// Field path with the indices dropped (`meshes[].vertex_normal.values[].y`) -> differences there
    pub by_field: HashMap<String, usize>,
}

pub fn compare_scenes(c: &ufbx::Scene, p: &load3d::Scene, keep: usize) -> Outcome {
    let mut cx = rt::Cx::new(c, keep);
    generated::cmp_Scene(&mut cx, c, p);
    Outcome { checks: cx.checks, count: cx.count, kept: cx.kept, by_field: cx.by_field }
}

/// The C scene as the port's types, for checking the comparator against ufbx itself
pub fn convert_scene(c: &ufbx::Scene) -> load3d::Scene {
    let k = rt::Kx { dom: rt::DomMap::new(c) };
    generated::conv_Scene(&k, c)
}

pub fn record(t: &mut Tally, o: Outcome) {
    t.checks += o.checks;
    t.diffs += o.count as u64;
    for d in o.kept {
        t.keep(|| format!("{}: C {}, port {}", d.path, d.c, d.p));
    }
    for (k, n) in o.by_field {
        *t.fields.entry(k).or_default() += n as u64;
    }
}

pub struct Pass {
    pub name: &'static str,
    pub opts: fn() -> (ufbx::LoadOpts<'static>, load3d::LoadOpts),
}

fn base() -> (ufbx::LoadOpts<'static>, load3d::LoadOpts) {
    (ufbx::LoadOpts::default(), load3d::LoadOpts::default())
}

const AXES: [ufbx::CoordinateAxis; 7] = [
    ufbx::CoordinateAxis::PositiveX,
    ufbx::CoordinateAxis::NegativeX,
    ufbx::CoordinateAxis::PositiveY,
    ufbx::CoordinateAxis::NegativeY,
    ufbx::CoordinateAxis::PositiveZ,
    ufbx::CoordinateAxis::NegativeZ,
    ufbx::CoordinateAxis::Unknown,
];

/// Right, up and front by ufbx's numbering of `ufbx_coordinate_axis` (+X, -X, +Y, -Y, +Z, -Z, unknown)
pub type Axes = (usize, usize, usize);

pub fn axes((right, up, front): Axes) -> (ufbx::CoordinateAxes, load3d::CoordinateAxes) {
    let p = |i: usize| load3d::CoordinateAxis::from_index(i as i64);
    (ufbx::CoordinateAxes { right: AXES[right], up: AXES[up], front: AXES[front] }, load3d::CoordinateAxes { right: p(right), up: p(up), front: p(front) })
}

macro_rules! axes_both {
    ($o:ident, $f:ident = $a:expr) => {{
        let (c, p) = axes($a);
        $o.0.$f = c;
        $o.1.$f = p;
    }};
}

pub const RH_Y: Axes = (0, 2, 4);
pub const RH_Z: Axes = (0, 4, 3);
pub const LH_Y: Axes = (0, 2, 5);
pub const LH_Z: Axes = (0, 4, 2);
pub const NONE: Axes = (6, 6, 6);

macro_rules! pass {
    ($name:literal, |$o:ident| $body:expr) => {
        Pass {
            name: $name,
            opts: || {
                #[allow(unused_mut)]
                let mut $o = base();
                $body;
                $o
            },
        }
    };
}

pub const SCENE_PASSES: &[Pass] = &[
    pass!("default", |o| {}),
    pass!("z_up_m", |o| {
        axes_both!(o, target_axes = RH_Z);
        both!(o, target_unit_meters = 1.0);
    }),
    pass!("normals", |o| both!(o, generate_missing_normals = true)),
    pass!("dom", |o| both!(o, retain_dom = true)),
    pass!("skinning", |o| both!(o, evaluate_skinning = true)),
    pass!("no_geometry", |o| both!(o, ignore_geometry = true)),
    pass!("modify", |o| modify(&mut o, false)),
    pass!("dom_modify", |o| modify(&mut o, true)),
    pass!("helpers", |o| {
        both!(o, geometry_transform_handling = GeometryTransformHandling::HelperNodes);
        both!(o, inherit_mode_handling = InheritModeHandling::HelperNodes);
        both!(o, pivot_handling = PivotHandling::AdjustToPivot);
        both!(o, reverse_winding = true);
    }),
];

fn modify(o: &mut (ufbx::LoadOpts<'static>, load3d::LoadOpts), dom: bool) {
    axes_both!(o, target_axes = LH_Y);
    both!(o, target_unit_meters = 1.0);
    both!(o, space_conversion = SpaceConversion::ModifyGeometry);
    both!(o, geometry_transform_handling = GeometryTransformHandling::ModifyGeometry);
    both!(o, handedness_conversion_axis = MirrorAxis::X);
    both!(o, retain_dom = dom);
    both!(o, normalize_normals = true);
}

pub const OBJ_PASSES: &[Pass] = &[
    pass!("default", |o| {}),
    pass!("ext", |o| external(&mut o, true)),
    pass!("strict_missing", |o| external(&mut o, false)),
    pass!("search", |o| {
        external(&mut o, true);
        both!(o, obj_search_mtl_by_filename = true);
    }),
    pass!("split", |o| {
        external(&mut o, true);
        both!(o, obj_split_groups = true);
    }),
    pass!("merge", |o| {
        external(&mut o, true);
        both!(o, obj_merge_objects = true);
        both!(o, obj_merge_groups = true);
    }),
    pass!("merge_objects", |o| {
        external(&mut o, true);
        both!(o, obj_merge_objects = true);
    }),
    pass!("noindex", |o| {
        external(&mut o, true);
        both!(o, index_error_handling = IndexErrorHandling::NoIndex);
    }),
    pass!("noparts", |o| {
        external(&mut o, true);
        both!(o, skip_mesh_parts = true);
    }),
    pass!("empty_faces", |o| {
        external(&mut o, true);
        both!(o, allow_empty_faces = true);
    }),
    pass!("noquirks", |o| {
        external(&mut o, true);
        both!(o, disable_quirks = true);
    }),
    pass!("nogeom", |o| {
        external(&mut o, true);
        both!(o, ignore_geometry = true);
    }),
];

fn external(o: &mut (ufbx::LoadOpts<'static>, load3d::LoadOpts), ignore_missing: bool) {
    both!(o, load_external_files = true);
    both!(o, ignore_missing_external_files = ignore_missing);
}

pub const UPDATE_PASSES: &[Pass] = &[
    pass!("transform_root", |o| update(&mut o, 0)),
    pass!("adjust_rh_z", |o| update(&mut o, 1)),
    pass!("modify_lh_y", |o| update(&mut o, 2)),
    pass!("compensate_pivot", |o| update(&mut o, 3)),
    pass!("helpers_lh_z", |o| update(&mut o, 4)),
];

fn update(o: &mut (ufbx::LoadOpts<'static>, load3d::LoadOpts), i: usize) {
    let (target, unit, cam, light, mirror) =
        [(NONE, 0.0, NONE, NONE, 0), (RH_Z, 1.0, RH_Y, RH_Z, 0), (LH_Y, 0.01, LH_Z, RH_Y, 1), (RH_Y, 1.0, NONE, LH_Y, 3), (LH_Z, 1.0, RH_Z, NONE, 2)][i];
    match i {
        0 | 4 => both!(o, space_conversion = SpaceConversion::TransformRoot),
        1 | 3 => both!(o, space_conversion = SpaceConversion::AdjustTransforms),
        _ => both!(o, space_conversion = SpaceConversion::ModifyGeometry),
    }
    axes_both!(o, target_axes = target);
    both!(o, target_unit_meters = unit);
    axes_both!(o, target_camera_axes = cam);
    axes_both!(o, target_light_axes = light);
    match i {
        2 => both!(o, geometry_transform_handling = GeometryTransformHandling::ModifyGeometry),
        4 => both!(o, geometry_transform_handling = GeometryTransformHandling::HelperNodes),
        _ => {}
    }
    match i {
        3 => {
            both!(o, inherit_mode_handling = InheritModeHandling::Compensate);
            both!(o, pivot_handling = PivotHandling::AdjustToPivot);
        }
        4 => both!(o, inherit_mode_handling = InheritModeHandling::HelperNodes),
        _ => {}
    }
    match mirror {
        1 => both!(o, handedness_conversion_axis = MirrorAxis::X),
        2 => both!(o, handedness_conversion_axis = MirrorAxis::Y),
        3 => both!(o, handedness_conversion_axis = MirrorAxis::Z),
        _ => {}
    }
    both!(o, obj_unit_meters = 0.5);
    axes_both!(o, obj_axes = if i.is_multiple_of(2) { RH_Y } else { NONE });
}

pub const LEGACY_PASSES: &[Pass] = &[
    pass!("default", |o| {}),
    pass!("ignore_geometry", |o| both!(o, ignore_geometry = true)),
    pass!("ignore_animation", |o| both!(o, ignore_animation = true)),
    pass!("retain_dom", |o| both!(o, retain_dom = true)),
    pass!("strict", |o| both!(o, strict = true)),
];

pub fn run_pass(t: &mut Tally, path: &str, pass: &Pass, keep: usize, self_test: bool) {
    run_pass_on(t, path, &std::fs::read(path).unwrap_or_default(), pass, keep, self_test);
}

fn run_pass_on(t: &mut Tally, path: &str, bytes: &[u8], pass: &Pass, keep: usize, self_test: bool) {
    let c_load = || ufbx::load_memory(bytes, ufbx::LoadOpts { filename: ufbx::StringOpt::Owned(path.to_string()), ..(pass.opts)().0 });
    let c = c_load();
    let p = if self_test {
        c_load().map(|s| convert_scene(&s)).map_err(|e| load3d::Error {
            kind: generated::conv_enum_ErrorType(e.type_),
            description: e.description.to_string(),
            info: e.info().to_string(),
        })
    } else {
        load3d::load_memory(bytes, &load3d::LoadOpts { filename: path.to_string(), ..(pass.opts)().1 })
    };
    compare_results(t, c, p, keep);
}

pub fn compare_results(t: &mut Tally, c: Result<ufbx::SceneRoot, ufbx::Error>, p: Result<load3d::Scene, load3d::Error>, keep: usize) {
    match (c, p) {
        (Ok(c), Ok(p)) => {
            let o = compare_scenes(&c, &p, keep);
            let n = o.count;
            record(t, o);
            if n > 0 {
                t.status(Status::Differ, format!("{n} differences"));
            }
        }
        (Err(c), Err(p)) => {
            let same = generated::eq_ErrorType(c.type_, p.kind) && *c.description == *p.description;
            t.checks += 1;
            let detail = if same && c.info() != p.info {
                format!("{:?} {:?}; info C {:?} / port {:?}", c.type_, &*c.description, c.info(), p.info)
            } else if same {
                format!("{:?} {:?}", c.type_, &*c.description)
            } else {
                format!("C {:?} {:?} / port {:?} {:?}", c.type_, &*c.description, p.kind, p.description)
            };
            t.status(if same { Status::ErrorEqual } else { Status::ErrorMismatch }, detail);
        }
        (Ok(_), Err(p)) => t.status(Status::ErrorMismatch, format!("C loads / port {:?} {:?} {:?}", p.kind, p.description, p.info)),
        (Err(c), Ok(_)) => t.status(Status::ErrorMismatch, format!("C {:?} {:?} {:?} / port loads", c.type_, &*c.description, c.info())),
    }
}

fn run_passes(a: &Args, path: &str, child: &mut Child, passes: &[Pass]) {
    for pass in a.wanted(passes, |p| p.name) {
        child.step(pass.name, |t| run_pass(t, path, pass, a.keep, a.self_test));
    }
}

fn header(a: &Args) {
    if a.self_test {
        println!("self-test: the port replaced by ufbx's own scene, converted");
    }
    let mut by_type: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for (t, f, ty) in generated::MISSING {
        by_type.entry(t).or_default().push(format!("{f} ({ty})"));
    }
    let missing: Vec<String> = by_type.iter().map(|(t, fs)| format!("{t}: {}", fs.join(", "))).collect();
    let mismatch: Vec<String> = generated::MISMATCH.iter().map(|(t, f, c, p)| format!("{t}.{f}: ufbx {c}, port {p}")).collect();
    let none = |v: &[String]| if v.is_empty() { "none".to_string() } else { v.join("; ") };
    println!(
        "comparator: {} ufbx structs from ufbx-rust generated.rs and load3d types.rs; ufbx fields missing in the port: {}; type mismatches: {}",
        generated::COMPARED_STRUCTS,
        none(&missing),
        none(&mismatch)
    );
    if !generated::MISSING_STRUCTS.is_empty() {
        println!("ufbx structs without a port struct: {}", generated::MISSING_STRUCTS.join(", "));
    }
    if !generated::PORT_ONLY.is_empty() {
        let v: Vec<String> = generated::PORT_ONLY.iter().map(|(t, f)| format!("{t}.{f}")).collect();
        println!("port fields beyond ufbx (not compared): {}", v.join(", "));
    }
    let notes: Vec<&&str> = generated::ENUM_NOTES.iter().filter(|n| !n.contains("by spelling")).collect();
    if !notes.is_empty() {
        println!("enum differences: {}", notes.iter().map(|n| n.to_string()).collect::<Vec<_>>().join("; "));
    }
}

/// The version an FBX file's header names: after `Kaydara FBX Binary` a little-endian u32 at byte 23, in a text
/// file `FBXVersion: N`
fn fbx_version(path: &str) -> Option<u32> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.starts_with(b"Kaydara FBX Binary") {
        return bytes.get(23..27).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    }
    let head = &bytes[..bytes.len().min(1 << 16)];
    let at = head.windows(11).position(|w| w == b"FBXVersion:")? + 11;
    let digits: String = head[at..].iter().skip_while(|c| c.is_ascii_whitespace()).take_while(|c| c.is_ascii_digit()).map(|&c| c as char).collect();
    digits.parse().ok()
}

pub fn mutate(bytes: &[u8], name: &str, variant: usize) -> Vec<u8> {
    const HUGE: [u32; 7] = [0xffff_ffff, 0xffff_fff0, 0x7fff_ffff, 0x8000_0000, 0x4000_0000, 0x1000_0000, 0x0100_0000];
    let mut rng = Rng::named(&["scene mutant", name, &variant.to_string()]);
    let mut d = bytes.to_vec();
    if d.is_empty() {
        return d;
    }
    let huge = |rng: &mut Rng| HUGE[rng.below(HUGE.len() as u64) as usize];
    match rng.below(4) {
        0 => {
            for _ in 0..1 + rng.below(8) {
                let at = rng.below(d.len() as u64) as usize;
                d[at] = rng.below(256) as u8;
            }
        }
        1 => d.truncate(rng.below(d.len() as u64) as usize),
        2 if d.len() > 4 => {
            let at = rng.below(d.len() as u64 - 4) as usize;
            d[at..at + 4].copy_from_slice(&huge(&mut rng).to_le_bytes());
        }
        _ => {
            let mut runs = Vec::new();
            let mut i = 0;
            while i < d.len() {
                let start = i;
                while i < d.len() && d[i].is_ascii_digit() {
                    i += 1;
                }
                if i > start {
                    runs.push((start, i));
                }
                i += 1;
            }
            if !runs.is_empty() {
                let (start, end) = runs[rng.below(runs.len() as u64) as usize];
                d.splice(start..end, huge(&mut rng).to_string().into_bytes());
            }
        }
    }
    d
}

fn mutant_jobs(a: &Args) -> Vec<Job> {
    let files: Vec<Job> = scene_jobs(a).into_iter().filter(|j| std::fs::metadata(&j.item).is_ok_and(|m| m.len() <= 4 << 20)).collect();
    if files.is_empty() {
        return Vec::new();
    }
    (0..a.mutants)
        .map(|k| {
            let (file, variant) = (&files[k % files.len()], k / files.len());
            Job { item: format!("gen:mutant:{variant}:{}", file.item), label: format!("{}#{variant}", file.label), generated: true }
        })
        .collect()
}

fn run_mutant(a: &Args, item: &str, child: &mut Child) {
    let (variant, path) = item.strip_prefix("gen:mutant:").and_then(|s| s.split_once(':')).expect("a mutant's variant and path");
    let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let bytes = mutate(&std::fs::read(path).unwrap_or_default(), &name, variant.parse().expect("a variant"));
    for pass in a.wanted(SCENE_PASSES, |p| p.name) {
        child.step(pass.name, |t| run_pass_on(t, path, &bytes, pass, a.keep, a.self_test));
    }
}

pub fn scene_jobs(a: &Args) -> Vec<Job> {
    corpus_jobs(a, true, &["fbx", "obj", "mtl"])
}

fn obj_jobs(a: &Args) -> Vec<Job> {
    corpus_jobs(a, true, &["obj", "mtl"])
}

fn update_jobs(a: &Args) -> Vec<Job> {
    corpus_jobs(a, false, &["fbx", "obj"])
}

fn legacy_jobs(a: &Args) -> Vec<Job> {
    let mut v = corpus_jobs(a, true, &["fbx"]);
    v.retain(|j| fbx_version(&j.item).is_some_and(|n| n < 7000));
    v
}

pub const SCENE: Section = Section { name: "scene", jobs: scene_jobs, run: |a, p, ch| run_passes(a, p, ch, SCENE_PASSES), header: Some(header) };
pub const MUTANTS: Section = Section { name: "mutants", jobs: mutant_jobs, run: run_mutant, header: None };
pub const OBJ: Section = Section { name: "obj", jobs: obj_jobs, run: |a, p, ch| run_passes(a, p, ch, OBJ_PASSES), header: None };
pub const UPDATE: Section = Section { name: "update", jobs: update_jobs, run: |a, p, ch| run_passes(a, p, ch, UPDATE_PASSES), header: None };
pub const LEGACY: Section = Section { name: "legacy", jobs: legacy_jobs, run: |a, p, ch| run_passes(a, p, ch, LEGACY_PASSES), header: None };
