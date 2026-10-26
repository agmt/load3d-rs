use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::common::{Rng, canon};
use crate::runner::{Args, Child, Job, Section, Tally, corpus_jobs};

type Lines = Vec<String>;

fn bits(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

fn hash(vals: &[f64]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &v in vals {
        h ^= canon(v);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

struct Variant {
    mirror: i32,
    scale: Option<f64>,
    fps: f64,
}

const VARIANTS: [Variant; 2] = [Variant { mirror: 0, scale: None, fps: 0.0 }, Variant { mirror: 2, scale: Some(2.5), fps: 24.0 }];

/// (additive, weight, ignore_transform, the value the buffer is filled with before)
const READS: [(bool, Option<f64>, bool, f64); 4] =
    [(false, None, false, 0.0), (true, Some(0.3), false, 1.5), (false, None, true, 0.0), (false, Some(-2.0), false, 0.0)];

fn sample_times(times: &[f64]) -> Vec<f64> {
    let mut out = vec![f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
    if let (Some(&first), Some(&last)) = (times.first(), times.last()) {
        out.push(first - 1.0);
        out.push(last + 1.0);
    }
    for w in times.windows(2) {
        out.push(w[0]);
        out.push(w[0] + 1e-9);
        out.push(w[1] - 1e-9);
        out.push(w[0] + (w[1] - w[0]) * 0.25);
        out.push((w[0] + w[1]) * 0.5);
    }
    if let Some(&last) = times.last() {
        out.push(last);
    }
    out.truncate(400);
    out
}

/// The text of a cache through one library: `$k` is the crate, `$amp` how its data functions take their options,
/// `data_opts` and `load` the library's own way to build options and load
macro_rules! dumper {
    ($m:ident, $k:ident, [$($amp:tt)?], { $($items:item)* }) => {
        mod $m {
            use super::*;

            $($items)*

            fn frame(f: &$k::CacheFrame) -> String {
                format!(
                    "frame {:?} {} {:?} {} {} {} {} {} {} {} {} {}",
                    f.channel.as_bytes(),
                    bits(f.time),
                    f.filename.as_bytes(),
                    f.file_format as i32,
                    f.mirror_axis as i32,
                    bits(f.scale_factor),
                    f.data_format as i32,
                    f.data_encoding as i32,
                    f.data_offset,
                    f.data_count,
                    f.data_element_bytes,
                    f.data_total_bytes
                )
            }

            fn reads(f: &$k::CacheFrame, out: &mut Lines) {
                let n = (f.data_count as usize).min(100_000);
                for (additive, weight, ignore, prefill) in READS {
                    for size in [n * 3 + 5, 7, 1] {
                        let mut buf = vec![prefill; size];
                        let r = $k::read_geometry_cache_real(f, &mut buf, $($amp)? data_opts(additive, weight, ignore));
                        out.push(format!("read {r} {:x}", hash(&buf)));
                    }
                }
                let mut v = vec![$k::Vec3 { x: 0.5, y: 0.25, z: 0.125 }; n + 1];
                let r = $k::read_geometry_cache_vec3(f, &mut v, $($amp)? data_opts(true, None, false));
                let flat: Vec<f64> = v.iter().flat_map(|v| [v.x, v.y, v.z]).collect();
                out.push(format!("read3 {r} {:x}", hash(&flat)));
            }

            fn samples(ch: &$k::CacheChannel, out: &mut Lines) {
                let times: Vec<f64> = ch.frames.iter().map(|f| f.time).collect();
                let n = ch.frames.iter().map(|f| f.data_count as usize).max().unwrap_or(0).min(100_000);
                for t in sample_times(&times) {
                    for (size, additive, weight, prefill) in [(n * 3 + 3, false, None, 0.0), (4, true, Some(0.5), 0.25)] {
                        let mut buf = vec![prefill; size];
                        let r = $k::sample_geometry_cache_real(ch, t, &mut buf, $($amp)? data_opts(additive, weight, false));
                        out.push(format!("sample {} {r} {:x}", bits(t), hash(&buf)));
                    }
                    let mut v = vec![$k::Vec3 { x: 1.0, y: 2.0, z: 3.0 }; 3];
                    let r = $k::sample_geometry_cache_vec3(ch, t, &mut v, $($amp)? data_opts(false, None, false));
                    let flat: Vec<f64> = v.iter().flat_map(|v| [v.x, v.y, v.z]).collect();
                    out.push(format!("sample3 {r} {:x}", hash(&flat)));
                }
            }

            pub fn dump(path: &str, var: &Variant) -> Lines {
                let mut out = Lines::new();
                match load(path, var) {
                    Err(e) => out.push(e),
                    Ok(cache) => {
                        out.push(format!("root {:?}", cache.root_filename.as_bytes()));
                        for x in cache.extra_info.iter() {
                            out.push(format!("extra {:?}", x.as_bytes()));
                        }
                        for f in cache.frames.iter() {
                            out.push(frame(f));
                        }
                        for ch in cache.channels.iter() {
                            out.push(format!(
                                "chan {:?} {} {:?} {} {} {}",
                                ch.name.as_bytes(),
                                ch.interpretation as i32,
                                ch.interpretation_name.as_bytes(),
                                ch.mirror_axis as i32,
                                bits(ch.scale_factor),
                                ch.frames.len()
                            ));
                            for f in ch.frames.iter() {
                                out.push(frame(f));
                                reads(f, &mut out);
                            }
                            samples(ch, &mut out);
                        }
                    }
                }
                out
            }
        }
    };
}

dumper!(c, ufbx, [], {
    fn data_opts(additive: bool, weight: Option<f64>, ignore_transform: bool) -> ufbx::GeometryCacheDataOpts<'static> {
        ufbx::GeometryCacheDataOpts { additive, use_weight: weight.is_some(), weight: weight.unwrap_or(0.0), ignore_transform, ..Default::default() }
    }

    fn load(path: &str, var: &Variant) -> Result<ufbx::GeometryCacheRoot, String> {
        let mut opts = ufbx::GeometryCacheOpts { frames_per_second: var.fps, ..Default::default() };
        opts.mirror_axis = [ufbx::MirrorAxis::None, ufbx::MirrorAxis::X, ufbx::MirrorAxis::Y, ufbx::MirrorAxis::Z][var.mirror as usize];
        if let Some(s) = var.scale {
            opts.use_scale_factor = true;
            opts.scale_factor = s;
        }
        ufbx::load_geometry_cache(path, opts).map_err(|e| format!("err {:?} {:?} {:?}", e.type_, e.description.as_bytes(), e.info()))
    }
});

dumper!(r, load3d, [&], {
    fn data_opts(additive: bool, weight: Option<f64>, ignore_transform: bool) -> load3d::GeometryCacheDataOpts {
        load3d::GeometryCacheDataOpts {
            open_file_cb: Some(Arc::new(load3d::default_open_file)),
            additive,
            use_weight: weight.is_some(),
            weight: weight.unwrap_or(0.0),
            ignore_transform,
        }
    }

    fn load(path: &str, var: &Variant) -> Result<load3d::GeometryCache, String> {
        let opts = load3d::GeometryCacheOpts {
            open_file_cb: Some(Arc::new(load3d::default_open_file)),
            frames_per_second: var.fps,
            mirror_axis: [load3d::MirrorAxis::None, load3d::MirrorAxis::X, load3d::MirrorAxis::Y, load3d::MirrorAxis::Z][var.mirror as usize],
            use_scale_factor: var.scale.is_some(),
            scale_factor: var.scale.unwrap_or(0.0),
        };
        load3d::load_geometry_cache(path, &opts).map_err(|e| format!("err {:?} {:?} {:?}", e.kind, e.description.as_bytes(), e.info))
    }
});

/// Both dumps, line by line up to the first that differs: the lines after it are out of step
fn compare(t: &mut Tally, path: &str, var: &Variant) -> bool {
    let (a, b) = (c::dump(path, var), r::dump(path, var));
    if a.first().is_some_and(|l| l.starts_with("root")) {
        t.count("caches loaded", 1);
        t.count("frames", a.iter().filter(|l| l.starts_with("frame")).count() as u64);
    }
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i), b.get(i));
        if !t.check(x == y, || format!("line {i} of {} / {}: C {x:?}, port {y:?}", a.len(), b.len())) {
            return false;
        }
    }
    true
}

const MUTANT_SEED: &str = "cache mutant";

fn targets(a: &Args) -> Vec<PathBuf> {
    corpus_jobs(a, true, &["xml"]).into_iter().filter(|j| j.label.contains('/') && !j.label.starts_with("cache_fuzz")).map(|j| PathBuf::from(j.item)).collect()
}

fn mutate(rng: &mut Rng, data: &mut Vec<u8>) {
    let mut below = |n: usize| rng.below(n as u64) as usize;
    let len = data.len();
    match below(6) {
        0 => data.truncate(below(len + 1)),
        1 => {
            for _ in 0..1 + below(8) {
                if !data.is_empty() {
                    let i = below(data.len());
                    data[i] = below(256) as u8;
                }
            }
        }
        2 => {
            if len >= 4 {
                let i = below(len / 4) * 4;
                let vals = [0u32, 1, 0xffff_ffff, 0x7fff_ffff, 0x8000_0000, below(64) as u32, len as u32, 0x4000_0001];
                let v = vals[below(vals.len())];
                let b = if below(2) == 0 { v.to_be_bytes() } else { v.to_le_bytes() };
                data[i..i + 4].copy_from_slice(&b);
            }
        }
        3 => {
            let i = below(len + 1);
            let n = 1 + below(16);
            if below(2) == 0 {
                let end = (i + n).min(len);
                data.drain(i..end);
            } else {
                let ins: Vec<u8> = (0..n).map(|_| below(256) as u8).collect();
                data.splice(i..i, ins);
            }
        }
        4 => {
            let chars = b"<>\"'&;#x/!?-[]=0123456789 \0";
            for _ in 0..1 + below(4) {
                if !data.is_empty() {
                    let i = below(data.len());
                    data[i] = chars[below(chars.len())];
                }
            }
        }
        _ => {
            if len > 0 {
                let i = below(len);
                let n = 1 + below(64.min(len - i));
                let j = below(len);
                let chunk: Vec<u8> = data[i..i + n].to_vec();
                data.splice(j..j, chunk);
            }
        }
    }
}

fn mutant(t: &mut Tally, a: &Args, k: usize, var: &Variant) {
    let targets = targets(a);
    let target = &targets[k % targets.len()];
    let dir = target.parent().unwrap();
    let mut rng = Rng::named(&[MUTANT_SEED, &k.to_string()]);
    let work = std::env::temp_dir().join(format!("fbx-oracle-cache-{}-{k}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.is_file()).collect();
    files.sort();
    for f in &files {
        std::fs::copy(f, work.join(f.file_name().unwrap())).unwrap();
    }
    for _ in 0..1 + rng.below(2) {
        let victim = if rng.below(2) == 0 { target.as_path() } else { files[rng.below(files.len() as u64) as usize].as_path() };
        let victim = work.join(victim.file_name().unwrap());
        let mut data = std::fs::read(&victim).unwrap();
        mutate(&mut rng, &mut data);
        std::fs::write(&victim, data).unwrap();
    }
    let copy = work.join(target.file_name().unwrap());
    if compare(t, copy.to_str().unwrap(), var) {
        let _ = std::fs::remove_dir_all(&work);
    } else {
        t.keep(|| format!("the mutated copy of {} is kept in {}", Path::new(dir).display(), work.display()));
    }
}

fn jobs(a: &Args) -> Vec<Job> {
    let mut v = corpus_jobs(a, true, &["xml", "mc", "mcx", "pc2"]);
    if !targets(a).is_empty() {
        v.extend((0..a.mutants).map(|k| Job::generated(format!("gen:mutant:{k}"))));
    }
    v
}

fn run(a: &Args, item: &str, ch: &mut Child) {
    match item.strip_prefix("gen:mutant:") {
        Some(k) => {
            let k: usize = k.parse().expect("a mutant's number");
            for (i, var) in VARIANTS.iter().enumerate() {
                ch.step(&format!("mutant variant {i}"), |t| mutant(t, a, k, var));
            }
        }
        None => {
            for (i, var) in VARIANTS.iter().enumerate() {
                ch.step(&format!("variant {i}"), |t| {
                    compare(t, item, var);
                });
            }
        }
    }
}

pub const CACHE: Section = Section { name: "cache", jobs, run, header: None };
