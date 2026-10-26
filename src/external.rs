use std::sync::Arc;

#[cfg(feature = "geometry-cache")]
use crate::cache::load_scene_cache;
use crate::context::Context;
#[cfg(not(feature = "geometry-cache"))]
use crate::error::Error;
#[cfg(feature = "geometry-cache")]
use crate::error::{ErrorType, NO_INDEX, WarningType};
use crate::error::{Res, Warnings};
use crate::types::*;

struct ExternalFile {
    filename: String,
    #[cfg_attr(not(feature = "geometry-cache"), allow(dead_code))]
    absolute_filename: String,
    index: usize,
    data: Option<Arc<GeometryCache>>,
}

#[cfg(feature = "geometry-cache")]
fn load_external_cache(file: &mut ExternalFile, scene: &Scene, opts: &LoadOpts, mirror_axis: MirrorAxis, warnings: &mut Warnings) -> Res<()> {
    let cache_opts = GeometryCacheOpts {
        open_file_cb: opts.open_file_cb.clone(),
        mirror_axis,
        use_scale_factor: true,
        scale_factor: scene.metadata.geometry_scale,
        ..Default::default()
    };
    let fps = scene.settings.frames_per_second;
    let handling = opts.unicode_error_handling;

    let mut result = load_scene_cache(&file.filename, &cache_opts, fps, handling, warnings);
    if let Err(e) = &result
        && e.kind == ErrorType::FileNotFound
    {
        result = load_scene_cache(&file.absolute_filename, &cache_opts, fps, handling, warnings);
    }

    match result {
        Ok(cache) => {
            file.data = Some(Arc::new(cache));
            Ok(())
        }
        Err(mut error) => {
            if error.kind == ErrorType::FileNotFound {
                if opts.ignore_missing_external_files {
                    warnings.warn(WarningType::MissingExternalFile, NO_INDEX, format!("Failed to open geometry cache: {}", file.filename));
                    return Ok(());
                }
                error.kind = ErrorType::ExternalFileNotFound;
                error.description = "External file not found".to_string();
            }
            Err(error)
        }
    }
}

#[cfg(not(feature = "geometry-cache"))]
fn load_external_cache(_file: &mut ExternalFile, _scene: &Scene, opts: &LoadOpts, _mirror_axis: MirrorAxis, _warnings: &mut Warnings) -> Res<()> {
    if opts.ignore_missing_external_files {
        return Ok(());
    }
    Err(Error::new("Feature disabled").set_err_info("geometry-cache"))
}

fn find_external_file<'f>(files: &'f [ExternalFile], name: &str) -> Option<&'f ExternalFile> {
    let ix = files.partition_point(|f| f.filename.as_str() < name);
    files.get(ix).filter(|f| f.filename == name)
}

pub(crate) fn load_external_files(ctx: &mut Context<'_>) -> Res<()> {
    load_external_files_imp(&mut ctx.scene, &ctx.opts, ctx.mirror_axis, &mut ctx.p.warnings)
}

/// `ufbxi_load_external_files` in ufbx.c
pub(crate) fn load_external_files_imp(scene: &mut Scene, opts: &LoadOpts, mirror_axis: MirrorAxis, warnings: &mut Warnings) -> Res<()> {
    let mut files: Vec<ExternalFile> = Vec::new();
    for cache in &scene.cache_files {
        if !cache.filename.is_empty() {
            files.push(ExternalFile { filename: cache.filename.clone(), absolute_filename: cache.absolute_filename.clone(), index: files.len(), data: None });
        }
    }

    // `ufbxi_less_external_file` in ufbx.c
    files.sort_by(|a, b| a.filename.cmp(&b.filename).then(a.index.cmp(&b.index)));

    let mut prev_name: Option<usize> = None;
    for i in 0..files.len() {
        if prev_name.is_some_and(|p| files[p].filename == files[i].filename) {
            continue;
        }
        load_external_cache(&mut files[i], scene, opts, mirror_axis, warnings)?;
        prev_name = Some(i);
    }

    for cache in &mut scene.cache_files {
        if let Some(file) = find_external_file(&files, &cache.filename)
            && let Some(data) = &file.data
        {
            cache.external_cache = Some(data.clone());
        }
    }

    let Scene { cache_files, cache_deformers, .. } = scene;
    for deformer in cache_deformers.iter_mut() {
        let Some(cache) = deformer.file.and_then(|f| cache_files.get(f)).and_then(|f| f.external_cache.clone()) else { continue };

        // It seems like channels may be connected even if the name is wrong and they work when exporting from
        // Marvelous to Maya
        if cache.channels.len() == 1 {
            deformer.external_channel = Some(0);
        } else {
            let ix = cache.channels.partition_point(|c| c.name < deformer.channel);
            if cache.channels.get(ix).is_some_and(|c| c.name == deformer.channel) {
                deformer.external_channel = Some(ix);
            }
        }
        deformer.external_cache = Some(cache);
    }

    Ok(())
}
