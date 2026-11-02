//! The time of a load, this crate against ufbx's C, file by file from memory, the scene freed included:
//! `cargo bench --bench load -- [DIR|FILE]... [--rounds N] [--top N]`, ufbx's `data` folder when no path is given.
//! Each file's time is its fastest of the rounds, the two libraries taking turns on it.

use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

struct File {
    name: String,
    bytes: Vec<u8>,
    kind: &'static str,
    c: Duration,
    port: Duration,
}

fn walk(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(path).unwrap().map(|e| e.unwrap().path()).collect();
        entries.sort();
        for e in entries {
            walk(&e, out);
        }
    } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("fbx") || e.eq_ignore_ascii_case("obj")) {
        out.push(path.to_path_buf());
    }
}

fn kind(path: &Path, bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"Kaydara FBX Binary") {
        "FBX binary"
    } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("fbx")) {
        "FBX text"
    } else {
        "OBJ"
    }
}

fn c_load(bytes: &[u8]) -> bool {
    black_box(ufbx::load_memory(bytes, ufbx::LoadOpts::default())).is_ok()
}

fn port_load(bytes: &[u8]) -> bool {
    black_box(load3d::load_memory(bytes, &load3d::LoadOpts::default())).is_ok()
}

fn time(f: fn(&[u8]) -> bool, bytes: &[u8]) -> Duration {
    let t = Instant::now();
    f(bytes);
    t.elapsed()
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

fn row(label: &str, files: &[&File]) {
    let c: Duration = files.iter().map(|f| f.c).sum();
    let port: Duration = files.iter().map(|f| f.port).sum();
    let mb = files.iter().map(|f| f.bytes.len()).sum::<usize>() as f64 / 1e6;
    let ratio = port.as_secs_f64() / c.as_secs_f64();
    println!("{label:<12} {:>5} {mb:>8.1} {:>10.1} {:>10.1} {ratio:>7.2}", files.len(), ms(c), ms(port));
}

fn main() {
    let mut paths = Vec::new();
    let (mut rounds, mut top) = (5, 10);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--rounds" => rounds = args.next().and_then(|n| n.parse().ok()).expect("--rounds N"),
            "--top" => top = args.next().and_then(|n| n.parse().ok()).expect("--top N"),
            // what `cargo bench` passes to a target without the test harness
            "--bench" => {}
            _ => paths.push(PathBuf::from(a)),
        }
    }
    if paths.is_empty() {
        paths.push(std::env::var_os("UFBX_DATA").map_or_else(|| PathBuf::from("../../ufbx/data"), PathBuf::from));
    }
    let mut found = Vec::new();
    for p in &paths {
        assert!(p.exists(), "{} not found", p.display());
        walk(p, &mut found);
    }

    let (mut c_only, mut port_only, mut neither) = (0, 0, 0);
    let mut files = Vec::new();
    for path in found {
        let bytes = std::fs::read(&path).unwrap();
        let counter = match (c_load(&bytes), port_load(&bytes)) {
            (true, true) => None,
            (true, false) => Some(&mut c_only),
            (false, true) => Some(&mut port_only),
            (false, false) => Some(&mut neither),
        };
        if let Some(n) = counter {
            *n += 1;
        } else {
            let name = path.strip_prefix(&paths[0]).unwrap_or(&path).display().to_string();
            files.push(File { name, kind: kind(&path, &bytes), bytes, c: Duration::MAX, port: Duration::MAX });
        }
    }
    println!("{} files loaded by both; left out: {neither} that neither loads, {c_only} only ufbx, {port_only} only load3d", files.len());

    for _ in 0..rounds {
        for f in &mut files {
            f.c = f.c.min(time(c_load, &f.bytes));
            f.port = f.port.min(time(port_load, &f.bytes));
        }
    }

    println!("\n{:<12} {:>5} {:>8} {:>10} {:>10} {:>7}", "", "files", "MB", "ufbx ms", "load3d ms", "ratio");
    for k in ["FBX binary", "FBX text", "OBJ"] {
        let of: Vec<&File> = files.iter().filter(|f| f.kind == k).collect();
        if !of.is_empty() {
            row(k, &of);
        }
    }
    row("all", &files.iter().collect::<Vec<_>>());

    files.sort_by_key(|f| std::cmp::Reverse(f.port.saturating_sub(f.c)));
    println!("\nmost time lost against ufbx:\n{:>10} {:>10} {:>7}  file", "ufbx ms", "load3d ms", "ratio");
    for f in files.iter().take(top) {
        println!("{:>10.3} {:>10.3} {:>7.2}  {}", ms(f.c), ms(f.port), f.port.as_secs_f64() / f.c.as_secs_f64(), f.name);
    }
}
