use std::io::Cursor;
use std::sync::{Arc, Mutex};

use crate::runner::{Args, Child, Job, Section, Status, Tally};
use crate::scene::{compare_results, mutate, scene_jobs};

#[derive(Clone, Copy, PartialEq)]
enum From {
    Memory,
    File,
    Stream,
}

/// `progress` the callback's interval hint, `None` for no callback
struct Way {
    name: &'static str,
    from: From,
    read_buffer_size: usize,
    progress: Option<u64>,
}

const WAYS: &[Way] = &[
    Way { name: "file", from: From::File, read_buffer_size: 0, progress: None },
    Way { name: "file_buffer_64", from: From::File, read_buffer_size: 64, progress: None },
    Way { name: "stream", from: From::Stream, read_buffer_size: 0, progress: None },
    Way { name: "stream_buffer_100", from: From::Stream, read_buffer_size: 100, progress: None },
    Way { name: "progress_memory", from: From::Memory, read_buffer_size: 0, progress: Some(0) },
    Way { name: "progress_memory_1000", from: From::Memory, read_buffer_size: 0, progress: Some(1000) },
    Way { name: "progress_file", from: From::File, read_buffer_size: 0, progress: Some(0) },
    Way { name: "progress_stream_37", from: From::Stream, read_buffer_size: 64, progress: Some(37) },
];

const CANCELLED: &[&str] = &["progress_memory", "progress_stream_37"];

type Calls = Vec<(u64, u64)>;

/// `cancel_at` the call, from 1, that cancels
fn c_load(path: &str, bytes: &[u8], way: &Way, cancel_at: Option<usize>) -> (ufbx::Result<ufbx::SceneRoot>, Calls) {
    let mut calls = Vec::new();
    let mut cb = |p: &ufbx::Progress| {
        calls.push((p.bytes_read, p.bytes_total));
        if cancel_at == Some(calls.len()) { ufbx::ProgressResult::Cancel } else { ufbx::ProgressResult::Continue }
    };
    let opts = ufbx::LoadOpts {
        filename: ufbx::StringOpt::Owned(path.to_string()),
        read_buffer_size: way.read_buffer_size,
        progress_cb: if way.progress.is_some() { ufbx::ProgressCb::Mut(&mut cb) } else { ufbx::ProgressCb::Unset },
        progress_interval_hint: way.progress.unwrap_or(0),
        ..Default::default()
    };
    let r = match way.from {
        From::Memory => ufbx::load_memory(bytes, opts),
        From::File => ufbx::load_file(path, opts),
        From::Stream => ufbx::load_stream(ufbx::Stream::Read(Box::new(Cursor::new(bytes.to_vec()))), opts),
    };
    (r, calls)
}

fn p_load(path: &str, bytes: &[u8], way: &Way, cancel_at: Option<usize>) -> (Result<load3d::Scene, load3d::Error>, Calls) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let rec = calls.clone();
    let cb: Arc<load3d::ProgressFn> = Arc::new(move |p: &load3d::Progress| {
        let mut v = rec.lock().unwrap();
        v.push((p.bytes_read, p.bytes_total));
        if cancel_at == Some(v.len()) { load3d::ProgressResult::Cancel } else { load3d::ProgressResult::Continue }
    });
    let opts = load3d::LoadOpts {
        filename: path.to_string(),
        read_buffer_size: way.read_buffer_size,
        progress_cb: way.progress.map(|_| cb),
        progress_interval_hint: way.progress.unwrap_or(0),
        ..Default::default()
    };
    let r = match way.from {
        From::Memory => load3d::load_memory(bytes, &opts),
        From::File => load3d::load_file(path, &opts),
        From::Stream => load3d::load_stream(&mut Cursor::new(bytes), &opts),
    };
    let calls = calls.lock().unwrap().clone();
    (r, calls)
}

fn compare(t: &mut Tally, path: &str, bytes: &[u8], way: &Way, cancel_at: Option<usize>, keep: usize) -> usize {
    let (c, c_calls) = c_load(path, bytes, way, cancel_at);
    let (p, p_calls) = p_load(path, bytes, way, cancel_at);
    compare_results(t, c, p, keep);
    t.checks += c_calls.len() as u64 + 1;
    if c_calls != p_calls {
        let at = c_calls.iter().zip(&p_calls).position(|(a, b)| a != b).unwrap_or(c_calls.len().min(p_calls.len()));
        t.diffs += 1;
        *t.fields.entry("progress calls".to_string()).or_default() += 1;
        t.keep(|| format!("progress call {at}: C {:?} of {}, port {:?} of {}", c_calls.get(at), c_calls.len(), p_calls.get(at), p_calls.len()));
        t.status(Status::Differ, format!("progress calls differ from call {at}"));
    }
    c_calls.len()
}

fn run_ways(a: &Args, path: &str, bytes: &[u8], child: &mut Child, files: bool) {
    for way in a.wanted(WAYS, |w| w.name) {
        if way.from == From::File && !files {
            continue;
        }
        let mut calls = 0;
        child.step(way.name, |t| calls = compare(t, path, bytes, way, None, a.keep));
        if !CANCELLED.contains(&way.name) {
            continue;
        }
        let mut at: Vec<usize> = [1, 2, calls / 2, calls].into_iter().filter(|&k| k >= 1 && k <= calls).collect();
        at.dedup();
        for k in at {
            child.step(&format!("{} cancelled", way.name), |t| {
                compare(t, path, bytes, way, Some(k), a.keep);
            });
        }
    }
}

fn io_jobs(a: &Args) -> Vec<Job> {
    let files = scene_jobs(a);
    let small: Vec<&Job> = files.iter().filter(|j| std::fs::metadata(&j.item).is_ok_and(|m| m.len() <= 4 << 20)).collect();
    let mut v: Vec<Job> = Vec::new();
    if !small.is_empty() {
        v.extend((0..a.mutants).map(|k| {
            let (file, variant) = (small[k % small.len()], k / small.len());
            Job { item: format!("gen:io-mutant:{variant}:{}", file.item), label: format!("{}#{variant}", file.label), generated: true }
        }));
    }
    files.into_iter().chain(v).collect()
}

fn run(a: &Args, item: &str, child: &mut Child) {
    match item.strip_prefix("gen:io-mutant:").and_then(|s| s.split_once(':')) {
        Some((variant, path)) => {
            let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let bytes = mutate(&std::fs::read(path).unwrap_or_default(), &name, variant.parse().expect("a variant"));
            run_ways(a, path, &bytes, child, false);
        }
        None => run_ways(a, item, &std::fs::read(item).unwrap_or_default(), child, true),
    }
}

pub const IO: Section = Section { name: "io", jobs: io_jobs, run, header: None };
