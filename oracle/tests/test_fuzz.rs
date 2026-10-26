mod ufbxt;
use ufbxt::*;

use std::sync::Arc;

fn measure_progress() -> Arc<ProgressFn> {
    Arc::new(|_: &Progress| ProgressResult::Continue)
}

fn fuzz_load_opts(file_format: FileFormat) -> LoadOpts {
    let mut load_opts = LoadOpts { file_format, ..Default::default() };
    load_opts.temp_allocator.memory_limit = 0x4000000;
    load_opts.result_allocator.memory_limit = 0x4000000;
    load_opts
}

fn stream_load_opts(load_opts: &LoadOpts) -> LoadOpts {
    let mut stream_opts = load_opts.clone();
    stream_opts.read_buffer_size = 1;
    stream_opts.temp_allocator.huge_threshold = 1;
    stream_opts.result_allocator.huge_threshold = 1;
    stream_opts.progress_cb = Some(measure_progress());
    stream_opts.progress_interval_hint = 1;
    stream_opts
}

#[test]
fn fuzz_files() {
    let mut ok = 0;
    let mut i = 0;
    while i < 10000 {
        let name = format!("fuzz_{i:04}");
        let buf = data_path(&format!("fuzz/fuzz_{i:04}.fbx"));

        let Some(data) = read_file(&buf) else { break };

        let load_opts = fuzz_load_opts(FileFormat::Fbx);

        let scene = load_memory(&data, &load_opts);
        if let Ok(scene) = &scene {
            check_scene(scene);
            ok += 1;
        }
        let allow_error = scene.is_err();

        {
            let streamed_scene = load_file(&buf, &stream_load_opts(&load_opts));
            if let Ok(streamed_scene) = &streamed_scene {
                check_scene(streamed_scene);
                ufbxt_assert!(scene.is_ok());
            } else {
                ufbxt_assert!(scene.is_err());
            }
        }

        do_fuzz(&name, &data, &buf, allow_error, FileFormat::Fbx, None, heavy_fuzz_quality());

        i += 1;
    }

    ufbxt_logf!(".. Loaded fuzz files: {i} ({ok} non-errors)");
}

fn fuzz_cache(dir: &str, ext: &str) {
    let mut ok = 0;
    let mut i = 0;
    while i < 10000 {
        let buf = data_path(&format!("cache_fuzz/{dir}/fuzz_{i:04}.{ext}"));

        let cache_opts = GeometryCacheOpts::default();

        let Some(_data) = read_file(&buf) else { break };

        if let Ok(cache) = load_geometry_cache(&buf, &cache_opts) {
            check_scene::check_string(&cache.root_filename);
            ok += 1;
        }

        i += 1;
    }

    ufbxt_logf!(".. Loaded fuzz files: {i} ({ok} non-errors)");
}

#[test]
fn fuzz_cache_xml() {
    fuzz_cache("xml", "xml");
}

#[test]
fn fuzz_cache_mcx() {
    fuzz_cache("mcx", "mcx");
}

fn fuzz_text_files(prefix: &str, dir: &str, ext: &str, file_format: FileFormat) {
    let mut ok = 0;
    let mut i = 0;
    while i < 10000 {
        let name = format!("{prefix}_{i:04}");
        let buf = data_path(&format!("{dir}/fuzz_{i:04}.{ext}"));

        let Some(data) = read_file(&buf) else { break };

        let load_opts = fuzz_load_opts(file_format);

        let scene = load_memory(&data, &load_opts);
        if let Ok(scene) = &scene {
            check_scene(scene);
            ok += 1;
        }

        let allow_error = scene.is_err();

        let streamed_scene = load_file(&buf, &stream_load_opts(&load_opts));
        if let Ok(streamed_scene) = &streamed_scene {
            check_scene(streamed_scene);
            ufbxt_assert!(scene.is_ok());
        } else {
            ufbxt_assert!(scene.is_err());
        }

        do_fuzz(&name, &data, &buf, allow_error, file_format, None, fuzz_quality());

        i += 1;
    }

    ufbxt_logf!(".. Loaded fuzz files: {i} ({ok} non-errors)");
}

#[test]
fn fuzz_obj_files() {
    fuzz_text_files("obj_fuzz", "obj_fuzz", "obj", FileFormat::Obj);
}

#[test]
fn fuzz_mtl_files() {
    fuzz_text_files("mtl_fuzz", "mtl_fuzz", "mtl", FileFormat::Mtl);
}
