//! `UFBXT_FUZZ=1` is ufbx's `--fuzz`, `UFBXT_FUZZ_QUALITY` `--fuzz-quality`, `UFBXT_HEAVY_FUZZ_QUALITY`
//! `--heavy-fuzz-quality`, `UFBXT_ALL_BYTE_VALUES=1` `--all-byte-values`.

use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use load3d::*;

use super::check_scene::check_scene;
use super::fuzz_checks::FUZZ_CHECKS;
use super::utils::{DiffError, OBJ_DIFF_FLAG_IGNORE_NORMALS, ObjFile, diff_to_obj, load_obj, read_file};

thread_local! {
    static HINT: RefCell<String> = const { RefCell::new(String::new()) };
    static EXPECT_FAIL: Cell<bool> = const { Cell::new(false) };
    static EXPECT_FAIL_COUNT: Cell<usize> = const { Cell::new(0) };
}

/// Part of `ufbxt_hintf` in test/runner.c
pub fn set_hint(hint: String) {
    HINT.with(|h| *h.borrow_mut() = hint);
}

pub fn assert_fail_imp(file: &str, line: u32, expr: &str, fatal: bool) {
    if !fatal && EXPECT_FAIL.get() {
        EXPECT_FAIL_COUNT.set(EXPECT_FAIL_COUNT.get() + 1);
        return;
    }
    let hint = HINT.with(|h| h.borrow().clone());
    if hint.is_empty() {
        panic!("{file}:{line}: assertion failed: {expr}");
    }
    panic!("{file}:{line}: assertion failed: {expr}\nhint: {hint}");
}

fn begin_expect_fail() {
    assert!(!EXPECT_FAIL.get());
    EXPECT_FAIL.set(true);
    EXPECT_FAIL_COUNT.set(0);
}

fn end_expect_fail() -> usize {
    assert!(EXPECT_FAIL.get());
    EXPECT_FAIL.set(false);
    let count = EXPECT_FAIL_COUNT.get();
    assert!(count > 0, "a diff expected to fail passed");
    count
}

fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v == "1")
}

fn env_int(name: &str) -> Option<i32> {
    std::env::var(name).ok().and_then(|v| v.parse().ok())
}

pub fn begin_fuzz() -> bool {
    env_flag("UFBXT_FUZZ")
}

pub fn fuzz_quality() -> i32 {
    env_int("UFBXT_FUZZ_QUALITY").unwrap_or(16).clamp(1, 31)
}

pub fn heavy_fuzz_quality() -> i32 {
    env_int("UFBXT_HEAVY_FUZZ_QUALITY").map(|q| q.clamp(1, 31)).unwrap_or((fuzz_quality() - 4).max(1))
}

pub fn data_root() -> PathBuf {
    let root = std::env::var_os("UFBX_DATA").expect("UFBX_DATA names ufbx's data/ (git clone --branch v0.23.1 https://github.com/ufbx/ufbx)");
    PathBuf::from(root)
}

pub fn data_path(name: &str) -> String {
    data_root().join(name).to_string_lossy().into_owned()
}

pub const FILE_VERSIONS: [u32; 16] = [0, 1, 2, 3, 2000, 3000, 5000, 5800, 6100, 7100, 7200, 7300, 7400, 7500, 7700, 8000];

const FILE_FORMATS: [&str; 4] = ["binary", "ascii", "obj", "mtl"];

/// A name ending `_<version>_<format>` split into the name, the format and the version
fn parse_format(name: &str) -> Option<(String, String, u32)> {
    let b = name.as_bytes();
    let mut p = b.len();
    for _ in 0..2 {
        while p > 0 && b[p - 1] != b'_' {
            p -= 1;
        }
        if p > 0 {
            p -= 1;
        }
    }
    if p == 0 {
        return None;
    }
    let name_end = p;
    p += 1;
    let mut version = 0u32;
    let mut digits = 0;
    while p < b.len() && b[p].is_ascii_digit() {
        version = version.wrapping_mul(10).wrapping_add((b[p] - b'0') as u32);
        digits += 1;
        p += 1;
    }
    if digits == 0 || p >= b.len() || b[p] != b'_' {
        return None;
    }
    p += 1;
    let format = &name[p..];
    if !FILE_FORMATS.contains(&format) {
        return None;
    }
    Some((name[..name_end].to_string(), format.to_string(), version))
}

pub struct FileIterator {
    pub path: String,
    pub root: Option<String>,
    pub allow_not_found: bool,
    version_ix: usize,
    format_ix: usize,
    num_found: usize,
}

impl FileIterator {
    pub fn new(path: &str) -> FileIterator {
        FileIterator { path: path.to_string(), root: None, allow_not_found: false, version_ix: 0, format_ix: 0, num_found: 0 }
    }
}

impl Iterator for FileIterator {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        loop {
            if self.version_ix >= FILE_VERSIONS.len() {
                assert!(self.num_found > 0 || self.allow_not_found, "no file of {}", self.path);
                return None;
            }
            let version = FILE_VERSIONS[self.version_ix];
            let (format, ext) = match self.format_ix {
                0 => ("binary", "fbx"),
                1 => ("ascii", "fbx"),
                2 => ("mtl", "mtl"),
                _ => ("obj", "obj"),
            };
            let name = format!("{}_{version}_{format}.{ext}", self.path);
            let path = match &self.root {
                Some(root) => format!("{root}{name}"),
                None => data_path(&name),
            };
            self.format_ix += 1;
            if self.format_ix >= 4 {
                self.format_ix = 0;
                self.version_ix += 1;
            }
            if std::fs::metadata(&path).is_ok_and(|m| m.is_file()) {
                eprintln!("{path}");
                self.num_found += 1;
                return Some(path);
            }
        }
    }
}

/// `ufbxt_file_test_flags` in test/runner.c
pub const ALTERNATIVE: u32 = 0x1;
pub const ALLOW_ERROR: u32 = 0x2;
pub const ALLOW_INVALID_UNICODE: u32 = 0x4;
pub const HEAVY_TO_FUZZ: u32 = 0x8;
pub const ALLOW_STRICT_ERROR: u32 = 0x10;
pub const SKIP_LOAD_OPTS_CHECKS: u32 = 0x20;
pub const FUZZ_ALWAYS: u32 = 0x40;
pub const FUZZ_OPTS: u32 = 0x80;
pub const DIFF_ALWAYS: u32 = 0x100;
pub const DIFF_EXPECT_FAIL: u32 = 0x200;
pub const DIFF_EXPECT_FAIL_POST_7000: u32 = 0x400;
pub const OPT_HANDLING_IGNORE_NORMALS_IN_DIFF: u32 = 0x800;
pub const ALLOW_FEWER_PROGRESS_CALLS: u32 = 0x1000;
pub const DIFF_SCALE_100: u32 = 0x2000;
pub const ALLOW_THREAD_ERROR: u32 = 0x4000;
pub const ALLOW_WARNINGS: u32 = 0x8000;
pub const NO_FUZZ: u32 = 0x10000;

pub struct FileTest {
    name: String,
    suffix: Option<String>,
    opts: LoadOpts,
    flags: u32,
}

impl FileTest {
    pub fn new(name: &str) -> FileTest {
        FileTest { name: name.to_string(), suffix: None, opts: LoadOpts::default(), flags: 0 }
    }

    pub fn file(&mut self, file: &str) {
        self.name = file.to_string();
        self.flags |= ALTERNATIVE;
    }

    pub fn path(&mut self, path: &str) {
        self.name = path.to_string();
    }

    pub fn suffix(&mut self, suffix: &str) {
        self.suffix = Some(suffix.to_string());
        self.flags |= ALTERNATIVE;
    }

    pub fn opts(&mut self, opts: LoadOpts) {
        self.opts = opts;
    }

    pub fn flags(&mut self, flags: u32) {
        self.flags |= flags;
    }

    pub fn run(&self, test_fn: &dyn Fn(Option<&Scene>, &mut DiffError, &Error)) {
        do_file_test(&self.name, test_fn, self.suffix.as_deref(), self.opts.clone(), self.flags);
    }
}

/// `ufbxt_progress_ctx` and `ufbxt_measure_progress` in test/runner.c
fn counting_progress() -> (Arc<ProgressFn>, Arc<Mutex<u64>>) {
    let calls = Arc::new(Mutex::new(0u64));
    let counter = calls.clone();
    let cb: Arc<ProgressFn> = Arc::new(move |_p: &Progress| {
        *counter.lock().unwrap() += 1;
        ProgressResult::Continue
    });
    (cb, calls)
}

/// Continues `calls` - 1 times, then cancels
fn cancel_progress(calls: usize) -> Arc<ProgressFn> {
    let left = Arc::new(Mutex::new(calls));
    Arc::new(move |_p: &Progress| {
        let mut left = left.lock().unwrap();
        *left -= 1;
        if *left > 0 { ProgressResult::Continue } else { ProgressResult::Cancel }
    })
}

fn no_error() -> Error {
    Error { kind: ErrorType::None, description: String::new(), info: String::new() }
}

pub fn log_error(error: &Error) {
    eprintln!("error: {error}");
}

fn load_checked(data: &[u8], opts: &LoadOpts) -> Result<Scene, Error> {
    load_memory(data, opts)
}

pub fn fuzz_should_skip(iter: usize, quality: i32) -> bool {
    let iter = iter as u64;
    let q = quality as u32;
    if (iter >> q >> q) != 0 { (iter & iter.wrapping_sub(1)) != 0 } else { (iter & ((1u64 << q) - 1)) % ((iter >> q) + 1) != 0 }
}

/// Part of `ufbxt_do_fuzz` and `ufbxt_do_file_test` in test/runner.c: their `omp parallel for` loops
fn fuzz_steps(range: std::ops::Range<usize>, quality: i32, data: &[u8], step: impl Fn(&mut Vec<u8>, usize) -> bool + Sync) -> Option<usize> {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let next = std::sync::atomic::AtomicUsize::new(range.start);
    let failed = Mutex::new(None);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                let mut copy = data.to_vec();
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= range.end {
                        break;
                    }
                    if !fuzz_should_skip(i, quality) && !step(&mut copy, i) {
                        *failed.lock().unwrap() = Some(i);
                    }
                }
            });
        }
    });
    failed.into_inner().unwrap()
}

fn test_fuzz(filename: &str, data: &[u8], default_opts: Option<&LoadOpts>, truncate_length: usize, cancel_step: usize) -> bool {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut opts = default_opts.cloned().unwrap_or_default();
        opts.load_external_files = true;
        opts.filename = filename.to_string();
        if cancel_step > 0 {
            opts.progress_cb = Some(cancel_progress(cancel_step));
            opts.progress_interval_hint = 1;
        }
        let data = if truncate_length > 0 { &data[..truncate_length] } else { data };
        if let Ok(scene) = load_memory(data, &opts) {
            check_scene(&scene);
        }
    }));
    result.is_ok()
}

/// The loops that fail every allocation in turn are not run: the port has no allocation limit.
pub fn do_fuzz(base_name: &str, data: &[u8], filename: &str, allow_error: bool, file_format: FileFormat, default_opts: Option<&LoadOpts>, quality: i32) {
    let mut progress_calls = 100;
    {
        let mut prog_opts = default_opts.cloned().unwrap_or_default();
        prog_opts.load_external_files = true;
        if file_format == FileFormat::Unknown || file_format == FileFormat::Obj {
            prog_opts.ignore_missing_external_files = true;
        }
        prog_opts.filename = filename.to_string();
        prog_opts.file_format = file_format;
        prog_opts.read_buffer_size = 1;
        prog_opts.temp_allocator.huge_threshold = 1;
        prog_opts.result_allocator.huge_threshold = 1;
        let (cb, calls) = counting_progress();
        prog_opts.progress_cb = Some(cb);
        prog_opts.progress_interval_hint = 1;
        let prog_scene = load_checked(data, &prog_opts);
        if !allow_error {
            if let Err(e) = &prog_scene {
                log_error(e);
            }
            ufbxt_assert!(prog_scene.is_ok());
        }
        if prog_scene.is_ok() {
            progress_calls = *calls.lock().unwrap() as usize;
        }
    }

    if begin_fuzz() {
        let mut fail_step = 0usize;
        let size = data.len();
        if let Some(i) = fuzz_steps(1..size, quality, data, |_, i| test_fuzz(filename, data, default_opts, i, 0)) {
            fail_step = 30000000 + i;
        }
        if let Some(i) = fuzz_steps(0..progress_calls, quality, data, |_, i| test_fuzz(filename, data, default_opts, 0, i + 1)) {
            fail_step = 40000000 + i;
        }
        let all_values = env_flag("UFBXT_ALL_BYTE_VALUES");
        let patched = fuzz_steps(0..size, quality, data, |copy, i| {
            let original = copy[i];
            let values: Vec<u8> = if all_values {
                (0..=255u8).collect()
            } else {
                let mut v = vec![original.wrapping_add(1), original.wrapping_sub(1)];
                if original != 0 {
                    v.push(0);
                }
                if original != 0xff {
                    v.push(0xff);
                }
                v
            };
            let mut ok = true;
            for value in values {
                copy[i] = value;
                ok &= test_fuzz(filename, copy, default_opts, 0, 0);
            }
            copy[i] = original;
            ok
        });
        if let Some(i) = patched {
            fail_step = i * 10;
        }
        ufbxt_hintf!("Fuzz failed on step: {fail_step}");
        ufbxt_assert!(fail_step == 0);
    } else {
        for check in FUZZ_CHECKS.iter().filter(|c| c.name == base_name) {
            let mut copy = data.to_vec();
            if check.patch_offset >= 0 {
                let at = check.patch_offset as usize;
                ufbxt_assert!(at < copy.len());
                copy[at] = check.patch_value;
            }
            let mut opts = default_opts.cloned().unwrap_or_default();
            opts.load_external_files = true;
            opts.filename = filename.to_string();
            if check.cancel_step > 0 {
                opts.progress_cb = Some(cancel_progress(check.cancel_step));
                opts.progress_interval_hint = 1;
            }
            let size = if check.truncate_length > 0 { check.truncate_length } else { copy.len() };
            if let Ok(scene) = load_memory(&copy[..size], &opts) {
                check_scene(&scene);
            }
        }
    }
}

const BUFFER_CHECKS: [(&str, usize); 9] = [
    ("blender_272_cube_7400_binary", 9484),
    ("blender_279_color_sets_7400_binary", 10255),
    ("blender_279_ball_7400_binary", 14303),
    ("blender_279_internal_textures_7400_binary", 13711),
    ("blender_293_textures_7400_binary", 13695),
    ("blender_293_embedded_textures_7400_binary", 13695),
    ("blender_293_material_mapping_7400_binary", 11388),
    ("blender_293x_nonmanifold_subsurf_7400_binary", 10447),
    ("blender_293_ngon_subsurf_7400_binary", 10223),
];

/// Part of `ufbxt_do_file_test` in test/runner.c
fn load_or_fail(data: &[u8], opts: &LoadOpts, allow_error: bool, what: &str) {
    match load_memory(data, opts) {
        Ok(scene) => check_scene(&scene),
        Err(e) if !allow_error => {
            log_error(&e);
            panic!("Failed to parse file {what}");
        }
        Err(_) => {}
    }
}

/// Part of `ufbxt_do_file_test` in test/runner.c
fn load_file_or_fail(path: &str, opts: &LoadOpts, allow_error: bool, what: &str, file_format: FileFormat) {
    match load_file(path, opts) {
        Ok(scene) => {
            check_scene(&scene);
            ufbxt_assert!(scene.metadata.file_format == file_format);
        }
        Err(e) if !allow_error => {
            log_error(&e);
            panic!("Failed to parse file {what}");
        }
        Err(_) => {}
    }
}

/// ufbx's threaded load is not run: threaded parsing is not ported.
pub fn do_file_test(name: &str, test_fn: &dyn Fn(Option<&Scene>, &mut DiffError, &Error), suffix: Option<&str>, user_opts: LoadOpts, flags: u32) {
    let (name, req) = match parse_format(name) {
        Some((n, format, version)) => (n, Some((format, version))),
        None => (name.to_string(), None),
    };

    let obj_path = data_path(&format!("{name}.obj"));
    let obj_file: Option<ObjFile> = read_file(&obj_path).map(|data| load_obj(&data, None));

    let quality = if flags & HEAVY_TO_FUZZ != 0 { heavy_fuzz_quality() } else { fuzz_quality() };

    let allow_error = flags & ALLOW_ERROR != 0;
    let alternative = flags & ALTERNATIVE != 0;
    let allow_strict_error = flags & ALLOW_STRICT_ERROR != 0;
    let skip_opts_checks = flags & SKIP_LOAD_OPTS_CHECKS != 0;
    let no_fuzz = flags & NO_FUZZ != 0;
    let fuzz_always = true;
    let diff_always = flags & DIFF_ALWAYS != 0;

    let fuzz_opts = if flags & FUZZ_OPTS != 0 || alternative { Some(&user_opts) } else { None };

    if obj_file.is_some() {
        eprintln!("{obj_path} [diff target found]");
    }

    if let Some(obj) = &obj_file
        && !alternative
    {
        let obj_opts = LoadOpts { load_external_files: true, ignore_missing_external_files: true, ..Default::default() };
        let obj_scene = match load_file(&obj_path, &obj_opts) {
            Ok(s) => s,
            Err(e) => {
                log_error(&e);
                panic!("Failed to parse .obj file");
            }
        };
        ufbxt_assert!(obj_scene.metadata.file_format == FileFormat::Obj);
        check_scene(&obj_scene);
        let mut err = DiffError::default();
        diff_to_obj(&obj_scene, obj, &mut err, 0);
        if err.num > 0 {
            eprintln!(".. Absolute diff: avg {:.3}, max {:.3} ({} tests)", err.sum / err.num as f64, err.max, err.num);
        }
        let data = read_file(&obj_path).expect("the .obj read a moment ago");
        if (!alternative || fuzz_always) && !no_fuzz {
            do_fuzz(&format!("{name}_obj"), &data, &obj_path, allow_error, FileFormat::Unknown, fuzz_opts, quality);
        }
    }

    let mut obj_file = obj_file;
    if flags & DIFF_SCALE_100 != 0
        && let Some(obj) = &mut obj_file
    {
        obj.fbx_position_scale = 100.0;
    }

    let mut num_opened = 0;
    for fi in 0..4 {
        for &version in &FILE_VERSIONS {
            let (format, ext) = match fi {
                0 => ("binary", "fbx"),
                1 => ("ascii", "fbx"),
                2 => ("mtl", "mtl"),
                _ => ("obj", "obj"),
            };
            if let Some((req_format, req_version)) = &req
                && (format != req_format || version != *req_version)
            {
                continue;
            }
            let (file_name, base_name) = match suffix {
                Some(s) => (format!("{name}_{version}_{format}_{s}.{ext}"), format!("{name}_{version}_{format}_{s}")),
                None => (format!("{name}_{version}_{format}.{ext}"), format!("{name}_{version}_{format}")),
            };
            let path = data_path(&file_name);
            let Some(data) = read_file(&path) else { continue };
            let size = data.len();

            let expect_diff_fail = flags & DIFF_EXPECT_FAIL != 0 || (flags & DIFF_EXPECT_FAIL_POST_7000 != 0 && version >= 7000);

            num_opened += 1;
            eprintln!("{path}");

            let mut load_opts = user_opts.clone();
            load_opts.evaluate_skinning = true;
            load_opts.load_external_files = true;
            if load_opts.filename.is_empty() {
                load_opts.filename = path.clone();
            }
            load_opts.file_format = match fi {
                0 | 1 => FileFormat::Fbx,
                2 => FileFormat::Mtl,
                _ => FileFormat::Obj,
            };

            let (cb, calls) = counting_progress();
            let mut memory_opts = load_opts.clone();
            memory_opts.progress_cb = Some(cb);
            let loaded = load_memory(&data, &memory_opts);
            let (scene, error) = match loaded {
                Ok(s) => (Some(s), no_error()),
                Err(e) => (None, e),
            };

            if let Some(scene) = &scene {
                check_scene(scene);
                if flags & ALLOW_FEWER_PROGRESS_CALLS == 0 {
                    ufbxt_assert!(*calls.lock().unwrap() >= (size / 0x4000 / 2) as u64);
                }
                if flags & ALLOW_WARNINGS == 0 {
                    for warning in &scene.metadata.warnings {
                        if flags & ALLOW_INVALID_UNICODE != 0 && warning.kind == WarningType::BadUnicode {
                            continue;
                        }
                        ufbxt_hintf!("Unexpected warning: {}", warning.description);
                        ufbxt_assert!(false);
                    }
                }
            } else if !allow_error {
                log_error(&error);
                panic!("Failed to parse file");
            }

            // ufbx's stream load writes the error the test is given over the first load's
            let error = {
                let (cb, _calls) = counting_progress();
                let mut stream_opts = load_opts.clone();
                stream_opts.file_format = FileFormat::Unknown;
                stream_opts.read_buffer_size = 1;
                stream_opts.temp_allocator.huge_threshold = 2;
                stream_opts.result_allocator.huge_threshold = 2;
                stream_opts.filename = String::new();
                stream_opts.progress_cb = Some(cb);
                stream_opts.progress_interval_hint = 1;
                stream_opts.retain_dom = true;
                if flags & ALLOW_INVALID_UNICODE == 0 {
                    stream_opts.unicode_error_handling = UnicodeErrorHandling::AbortLoading;
                }
                match load_file(&path, &stream_opts) {
                    Ok(streamed) => {
                        check_scene(&streamed);
                        ufbxt_assert!(streamed.dom_root.is_some());
                        ufbxt_assert!(streamed.metadata.file_format == load_opts.file_format);
                        no_error()
                    }
                    Err(e) if !allow_error => {
                        log_error(&e);
                        panic!("Failed to parse streamed file");
                    }
                    Err(e) => e,
                }
            };

            if begin_fuzz() && (!alternative || fuzz_always) && !no_fuzz {
                let failed = fuzz_steps(0..size, quality, &data, |_, buf_sz| {
                    catch_unwind(AssertUnwindSafe(|| {
                        let opts = LoadOpts { read_buffer_size: buf_sz, ..Default::default() };
                        let buf_scene = load_file(&path, &opts);
                        ufbxt_assert!(buf_scene.is_ok());
                        check_scene(&buf_scene.unwrap());
                    }))
                    .is_ok()
                });
                if let Some(sz) = failed
                    && !allow_error
                {
                    panic!("{base_name}: Failed to parse with: read_buffer_size = {sz}");
                }
            }

            if !skip_opts_checks {
                let variants: [(&str, fn(&mut LoadOpts)); 4] = [
                    ("ignoring geometry", |o| o.ignore_geometry = true),
                    ("ignoring animation", |o| o.ignore_animation = true),
                    ("ignoring embedded files", |o| o.ignore_embedded = true),
                    ("ignoring everything", |o| {
                        o.ignore_geometry = true;
                        o.ignore_animation = true;
                        o.ignore_embedded = true;
                    }),
                ];
                for (what, set) in variants {
                    let mut opts = load_opts.clone();
                    set(&mut opts);
                    load_or_fail(&data, &opts, allow_error, what);
                }

                let mut strict_opts = load_opts.clone();
                strict_opts.disable_quirks = true;
                strict_opts.strict = true;
                strict_opts.no_format_from_content = true;
                strict_opts.no_format_from_extension = true;
                load_file_or_fail(&path, &strict_opts, allow_error || allow_strict_error, "with strict options", load_opts.file_format);

                let mut loose_opts = load_opts.clone();
                loose_opts.allow_missing_vertex_position = true;
                loose_opts.allow_nodes_out_of_root = true;
                loose_opts.connect_broken_elements = true;
                loose_opts.generate_missing_normals = true;
                loose_opts.ignore_missing_external_files = true;
                load_file_or_fail(&path, &loose_opts, allow_error, "with loose options", load_opts.file_format);
            }

            if let Some(scene) = &scene {
                eprintln!(".. Loaded: file {:.1}kB", size as f64 * 1e-3);
                if fi <= 1 {
                    ufbxt_assert!(scene.metadata.file_format == FileFormat::Fbx);
                    ufbxt_assert!(scene.metadata.ascii == (fi == 1));
                    ufbxt_assert!(scene.metadata.version == version);
                } else if fi == 2 {
                    ufbxt_assert!(scene.metadata.file_format == FileFormat::Mtl);
                } else {
                    ufbxt_assert!(scene.metadata.file_format == FileFormat::Obj);
                }
                check_scene(scene);

                let state = evaluate_scene(scene, &scene.anim, 1.0, &EvaluateOpts::default());
                ufbxt_assert!(state.is_ok());
                check_scene(&state.unwrap());
                for stack in scene.anim_stacks.iter().skip(1) {
                    let state = evaluate_scene(scene, &stack.anim, 1.0, &EvaluateOpts::default());
                    ufbxt_assert!(state.is_ok());
                    check_scene(&state.unwrap());
                }
            }

            let mut err = DiffError::default();
            let mut num_failing_diff_checks = 0;
            if let (Some(scene), Some(obj)) = (&scene, &obj_file)
                && (!alternative || diff_always)
            {
                if expect_diff_fail {
                    begin_expect_fail();
                    diff_to_obj(scene, obj, &mut err, 0);
                    num_failing_diff_checks = end_expect_fail();
                } else {
                    diff_to_obj(scene, obj, &mut err, 0);
                }
            }

            if !skip_opts_checks {
                for handling in [GeometryTransformHandling::HelperNodes, GeometryTransformHandling::ModifyGeometry] {
                    let opts = LoadOpts { geometry_transform_handling: handling, ..load_opts.clone() };
                    match load_memory(&data, &opts) {
                        Ok(opt_scene) => {
                            check_scene(&opt_scene);
                            if let (Some(_), Some(obj)) = (&scene, &obj_file)
                                && (!alternative || diff_always)
                                && !expect_diff_fail
                            {
                                let diff_flags = if flags & OPT_HANDLING_IGNORE_NORMALS_IN_DIFF != 0 { OBJ_DIFF_FLAG_IGNORE_NORMALS } else { 0 };
                                diff_to_obj(&opt_scene, obj, &mut err, diff_flags);
                            }
                        }
                        Err(e) if !allow_error => {
                            log_error(&e);
                            panic!("Failed to parse file with {handling:?}");
                        }
                        Err(_) => {}
                    }
                }
            }

            test_fn(scene.as_ref(), &mut err, &error);

            if err.num > 0 {
                let avg = err.sum / err.num as f64;
                if expect_diff_fail {
                    eprintln!(".. Absolute diff: avg {avg:.3}, max {:.3} ({} tests, {num_failing_diff_checks} failing as expected)", err.max, err.num);
                } else {
                    eprintln!(".. Absolute diff: avg {avg:.3}, max {:.3} ({} tests)", err.max, err.num);
                }
            }

            if (!alternative || fuzz_always) && !no_fuzz {
                do_fuzz(&base_name, &data, &path, allow_error, FileFormat::Unknown, fuzz_opts, quality);
            }

            if (!alternative || fuzz_always) && scene.is_some() && !no_fuzz {
                for &(check_name, read_buffer_size) in BUFFER_CHECKS.iter().filter(|(n, _)| *n == base_name) {
                    eprintln!(".. Read buffer limit {read_buffer_size} ({check_name})");
                    let opts = LoadOpts { read_buffer_size, ..Default::default() };
                    match load_file(&path, &opts) {
                        Ok(buf_scene) => check_scene(&buf_scene),
                        Err(e) => {
                            log_error(&e);
                            panic!("read buffer limit {read_buffer_size}");
                        }
                    }
                }
            }
        }
    }

    if num_opened == 0 {
        panic!("File not found: {name}");
    }
}

/// `ufbxt_inflate_opts` in test/runner.c
#[derive(Debug, Clone, Copy, Default)]
pub struct InflateTestOpts {
    pub fast_bits: usize,
    pub force_fast: bool,
    pub primary: bool,
}

pub fn do_deflate_test(name: &str, test_fn: &dyn Fn(&InflateTestOpts)) {
    eprintln!("{name} (opt 0) default");
    test_fn(&InflateTestOpts { primary: true, ..Default::default() });
    for fast_bits in 1..=8 {
        eprintln!("{name} (opt {fast_bits}) fast_bits = {fast_bits}");
        test_fn(&InflateTestOpts { fast_bits, ..Default::default() });
    }
    eprintln!("{name} (opt 9) force_fast = true");
    test_fn(&InflateTestOpts { force_fast: true, ..Default::default() });
}
