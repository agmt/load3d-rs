use std::collections::HashMap;
use std::sync::Arc;

use crate::context::{Context, DomAlias, SYNTHETIC_ID_START};
use crate::error::{Error, ErrorType, Res, check};
use crate::matcher::{is_format, match_pattern};
use crate::math::*;
use crate::parse::{Io, PArr, PData, PNode, PVal, ParseOpts, Parser, first_described};
use crate::types::*;

#[cfg(feature = "format-fbx")]
use crate::{error::WarningType, legacy::supports_version};

/// Part of `ufbxi_load` in ufbx.c
fn complete_opts(mut opts: LoadOpts) -> LoadOpts {
    if opts.ignore_all_content {
        opts.ignore_geometry = true;
        opts.ignore_animation = true;
        opts.ignore_embedded = true;
    }
    if opts.read_buffer_size == 0 {
        opts.read_buffer_size = 0x4000;
    }
    if opts.read_buffer_size <= 32 {
        opts.read_buffer_size = 32;
    }
    if opts.file_format_lookahead == 0 {
        opts.file_format_lookahead = 0x4000;
    } else if opts.file_format_lookahead < 32 {
        opts.file_format_lookahead = 32;
    }
    if opts.path_separator == 0 {
        opts.path_separator = crate::paths::DEFAULT_PATH_SEPARATOR;
    }
    opts
}

fn determine_format(p: &mut Parser, opts: &LoadOpts) -> Res<FileFormat> {
    let mut format = opts.file_format;
    if format == FileFormat::Unknown && !opts.no_format_from_content {
        p.pause_progress();
        let mut lookahead = 32usize;
        while format == FileFormat::Unknown && lookahead <= opts.file_format_lookahead {
            if lookahead > p.io.data_size {
                if p.io.eof {
                    break;
                }
                p.refill(lookahead, false)?;
            }
            let head = p.buffered();
            let size = lookahead.min(head.len());
            check!(size > 0, "Empty file");
            for f in [FileFormat::Fbx, FileFormat::Obj, FileFormat::Mtl] {
                if is_format(&head[..size], f) {
                    format = f;
                    break;
                }
            }
            if lookahead >= opts.file_format_lookahead {
                break;
            } else if lookahead < usize::MAX / 2 {
                lookahead = (lookahead * 2).min(opts.file_format_lookahead);
            } else {
                lookahead = usize::MAX;
            }
        }
        // after ufbx's own formats, so that it reads every file it knows as it reads it
        #[cfg(feature = "format-3ds")]
        if format == FileFormat::Unknown && is_format(p.buffered(), FileFormat::Tds) {
            format = FileFormat::Tds;
        }
        p.resume_progress()?;
    }

    if format == FileFormat::Unknown && !opts.no_format_from_extension && !opts.filename.is_empty() {
        let name = opts.filename.as_bytes();
        let ext = name.iter().rposition(|&c| c == b'.').map_or(name, |i| &name[i..]);
        if match_pattern(ext, "\\c\\.fbx") {
            format = FileFormat::Fbx;
        } else if match_pattern(ext, "\\c\\.obj") {
            format = FileFormat::Obj;
        } else if match_pattern(ext, "\\c\\.mtl") {
            format = FileFormat::Mtl;
        } else if cfg!(feature = "format-3ds") && match_pattern(ext, "\\c\\.3ds") {
            format = FileFormat::Tds;
        }
    }

    check!(format != FileFormat::Unknown, "Unrecognized file format");
    Ok(format)
}

impl Default for PNode<'_> {
    fn default() -> Self {
        PNode { name: Default::default(), raw_name: None, dom_name: None, data: PData::Vals(Vec::new()), children: Vec::new() }
    }
}

fn retain_dom_node(node: &PNode) -> DomNode {
    let name = node.dom_name.clone().unwrap_or_else(|| node.name.to_string());
    let mut dst = DomNode { name, ..Default::default() };
    match &node.data {
        PData::Arr(arr) => {
            let size = arr.len();
            let (value_type, blob, array) = match arr {
                PArr::U8(v) => (DomValueType::Blob, v.clone(), DomArray::U8(v.clone())),
                PArr::Bool(v) | PArr::BoolInts(v, _) => (DomValueType::Blob, v.iter().map(|&b| b as u8).collect(), DomArray::Bool(v.clone())),
                PArr::I32(v) => (DomValueType::ArrayI32, v.iter().flat_map(|x| x.to_le_bytes()).collect(), DomArray::I32(v.clone())),
                PArr::I64(v) => (DomValueType::ArrayI64, v.iter().flat_map(|x| x.to_le_bytes()).collect(), DomArray::I64(v.clone())),
                PArr::F32(v) => (DomValueType::ArrayF32, v.iter().flat_map(|x| x.to_le_bytes()).collect(), DomArray::F32(v.clone())),
                PArr::F64(v) => (DomValueType::ArrayF64, v.iter().flat_map(|x| x.to_le_bytes()).collect(), DomArray::F64(v.clone())),
                PArr::Str(v) => (DomValueType::ArrayBlob, Vec::new(), DomArray::Blob(v.iter().map(|s| s.raw().to_vec()).collect())),
                PArr::Content(v) => (DomValueType::ArrayBlob, Vec::new(), DomArray::Blob(v.clone())),
                PArr::Ignored => (DomValueType::ArrayIgnored, Vec::new(), DomArray::U8(Vec::new())),
            };
            dst.values.push(DomValue {
                value_type,
                value_blob: blob,
                value_int: size as i64,
                value_float: size as f64,
                array: Some(array),
                ..Default::default()
            });
        }
        PData::Vals(vals) => {
            for v in vals {
                match v {
                    PVal::None => break,
                    PVal::Str(s) => dst.values.push(DomValue {
                        value_type: DomValueType::String,
                        // `ufbxi_get_val_at` with 'S' gives nothing for a string kept raw
                        value_str: s.utf8().unwrap_or_default().to_string(),
                        value_blob: s.raw().to_vec(),
                        ..Default::default()
                    }),
                    PVal::Num { f, i } => dst.values.push(DomValue { value_type: DomValueType::Number, value_int: *i, value_float: *f, ..Default::default() }),
                }
            }
        }
    }
    dst.children = node.children.iter().map(retain_dom_node).collect();
    dst
}

fn dom_node_at<'d>(root: &'d mut DomNode, path: &[u32]) -> Option<&'d mut DomNode> {
    let mut node = root;
    for &i in path {
        node = node.children.get_mut(i as usize)?;
    }
    Some(node)
}

fn set_reals(value: &mut DomValue, new: &[f64]) {
    if let Some(DomArray::F64(v)) = &mut value.array {
        let n = v.len().min(new.len());
        v[..n].copy_from_slice(&new[..n]);
        value.value_blob = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    }
}

fn set_ints(value: &mut DomValue, new: &[u32]) {
    if let Some(DomArray::I32(v)) = &mut value.array {
        let n = v.len().min(new.len());
        for i in 0..n {
            v[i] = new[i] as i32;
        }
        value.value_blob = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    }
}

fn flat3(v: &[Vec3]) -> Vec<f64> {
    v.iter().flat_map(|p| [p.x, p.y, p.z]).collect()
}

fn flat4(v: &[Vec4]) -> Vec<f64> {
    v.iter().flat_map(|p| [p.x, p.y, p.z, p.w]).collect()
}

/// ufbx's DOM holds the arrays its scene lists were read in place into, so it shows every edit a load made to them
fn apply_dom_aliases(root: &mut DomNode, scene: &Scene, aliases: &[(DomRef, DomAlias)]) {
    for (path, alias) in aliases {
        let Some(value) = dom_node_at(root, path).and_then(|n| n.values.first_mut()) else { continue };
        let uv = |m: usize, ix: u32| scene.meshes[m].uv_sets.iter().find(|s| s.index == ix);
        let color = |m: usize, ix: u32| scene.meshes[m].color_sets.iter().find(|s| s.index == ix);
        match *alias {
            DomAlias::MeshVertices(m) => set_reals(value, &flat3(&scene.meshes[m].vertex_position.values)),
            DomAlias::NormalValues(m) => set_reals(value, &flat3(&scene.meshes[m].vertex_normal.values)),
            DomAlias::NormalIndices(m) => set_ints(value, &scene.meshes[m].vertex_normal.indices),
            DomAlias::UvValues(m, ix) => {
                if let Some(s) = uv(m, ix) {
                    set_reals(value, &s.vertex_uv.values.iter().flat_map(|p| [p.x, p.y]).collect::<Vec<_>>());
                }
            }
            DomAlias::UvIndices(m, ix) => {
                if let Some(s) = uv(m, ix) {
                    set_ints(value, &s.vertex_uv.indices);
                }
            }
            DomAlias::TangentValues(m, ix) => {
                if let Some(s) = uv(m, ix) {
                    set_reals(value, &flat3(&s.vertex_tangent.values));
                }
            }
            DomAlias::TangentIndices(m, ix) => {
                if let Some(s) = uv(m, ix) {
                    set_ints(value, &s.vertex_tangent.indices);
                }
            }
            DomAlias::BitangentValues(m, ix) => {
                if let Some(s) = uv(m, ix) {
                    set_reals(value, &flat3(&s.vertex_bitangent.values));
                }
            }
            DomAlias::BitangentIndices(m, ix) => {
                if let Some(s) = uv(m, ix) {
                    set_ints(value, &s.vertex_bitangent.indices);
                }
            }
            DomAlias::ColorValues(m, ix) => {
                if let Some(s) = color(m, ix) {
                    set_reals(value, &flat4(&s.vertex_color.values));
                }
            }
            DomAlias::ColorIndices(m, ix) => {
                if let Some(s) = color(m, ix) {
                    set_ints(value, &s.vertex_color.indices);
                }
            }
            // with one material or none ufbx points the list at its shared zeros instead
            DomAlias::FaceMaterial(m) => {
                if scene.meshes[m].materials.len() > 1 {
                    set_ints(value, &scene.meshes[m].face_material);
                }
            }
            DomAlias::FaceGroup(m) => set_ints(value, &scene.meshes[m].face_group),
            DomAlias::SelectionVertices(s) => set_ints(value, &scene.selection_nodes[s].vertices),
            DomAlias::SelectionEdges(s) => set_ints(value, &scene.selection_nodes[s].edges),
            DomAlias::SelectionFaces(s) => set_ints(value, &scene.selection_nodes[s].faces),
            DomAlias::LinePoints(l) => set_reals(value, &flat3(&scene.line_curves[l].control_points)),
            DomAlias::LineIndices(l) => set_ints(value, &scene.line_curves[l].point_indices),
            DomAlias::ShapeVertices(s) => set_reals(value, &flat3(&scene.blend_shapes[s].position_offsets)),
            DomAlias::ShapeIndices(s) => set_ints(value, &scene.blend_shapes[s].offset_vertices),
            DomAlias::ShapeNormals(s) => set_reals(value, &flat3(&scene.blend_shapes[s].normal_offsets)),
            DomAlias::NurbsCurvePoints(c) => set_reals(value, &flat4(&scene.nurbs_curves[c].control_points)),
            DomAlias::NurbsSurfacePoints(s) => set_reals(value, &flat4(&scene.nurbs_surfaces[s].control_points)),
        }
    }
}

impl<'a> Context<'a> {
    pub(crate) fn new(data: &'a [u8], opts: LoadOpts) -> Context<'a> {
        let parse_opts = ParseOpts {
            ignore_geometry: opts.ignore_geometry,
            ignore_animation: opts.ignore_animation,
            ignore_embedded: opts.ignore_embedded,
            retain_dom: opts.retain_dom,
            retain_vertex_w: (opts.retain_dom || opts.retain_vertex_attrib_w) && !opts.ignore_geometry,
            strict: opts.strict,
            unicode_error_handling: opts.unicode_error_handling,
            memory_limit: opts.temp_allocator.memory_limit,
        };
        Context {
            p: Parser::new(data, parse_opts),
            scene: Scene::default(),
            exporter: Exporter::Unknown,
            exporter_version: 0,
            retain_mesh_parts: !opts.ignore_geometry && !opts.skip_mesh_parts,
            read_legacy_settings: false,
            templates: Vec::new(),
            fbx_id_map: HashMap::new(),
            ptr_fbx_id_map: HashMap::new(),
            string_ids: HashMap::new(),
            fbx_attr_map: HashMap::new(),
            anim_stack_map: HashMap::new(),
            tmp_connections: Vec::new(),
            tmp_node_ids: Vec::new(),
            element_fbx_ids: Vec::new(),
            extra: HashMap::new(),
            tmp_full_weights: Vec::new(),
            synthetic_id_counter: SYNTHETIC_ID_START,
            has_geometry_transform_nodes: false,
            has_scale_helper_nodes: false,
            mirror_axis: MirrorAxis::None,
            root_id: 0,
            legacy_implicit_anim_layer_id: 0,
            file_content: Vec::new(),
            ktime_sec: 0,
            ktime_sec_double: 0.0,
            axis_matrix: Matrix::IDENTITY,
            unit_scale: 1.0,
            tmp_element_id: Vec::new(),
            reading_element: None,
            tmp_element_flag: Vec::new(),
            dom_aliases: Vec::new(),
            shared_positions: Vec::new(),
            shared_normals: Vec::new(),
            opts,
        }
    }

    fn transform_to_axes(&mut self, dst_axes: CoordinateAxes) {
        if !coordinate_axes_valid(self.scene.settings.axes) {
            return;
        }
        let Some(axis_matrix) = crate::update::axis_matrix(self.scene.settings.axes, dst_axes) else { return };
        self.axis_matrix = axis_matrix;

        if matrix_determinant(&self.axis_matrix) < 0.0 && self.opts.handedness_conversion_axis != MirrorAxis::None {
            let mirror_axis = self.opts.handedness_conversion_axis;
            self.mirror_axis = mirror_axis;
            self.scene.metadata.mirror_axis = mirror_axis;
            crate::update::mirror_matrix_dst(&mut self.axis_matrix, mirror_axis);
            for node in &mut self.scene.nodes {
                if !node.is_root {
                    node.adjust_mirror_axis = mirror_axis;
                }
            }
        }

        if self.opts.space_conversion == SpaceConversion::TransformRoot {
            let mut axis_mat = self.axis_matrix;
            let root = self.scene.root_node;
            if !is_transform_identity(&self.scene.nodes[root].local_transform) {
                let root_mat = transform_to_matrix(&self.scene.nodes[root].local_transform);
                axis_mat = matrix_mul(&root_mat, &axis_mat);
            }
            crate::update::mirror_matrix(&mut axis_mat, self.mirror_axis);
            let r = &mut self.scene.nodes[root];
            r.local_transform = matrix_to_transform(&axis_mat);
            r.node_to_parent = axis_mat;
        }
    }

    fn scale_units(&mut self, target_meters: Real) {
        if self.scene.settings.unit_meters <= 0.0 {
            return;
        }
        let target_meters = crate::update::round_if_near(&crate::update::POW10_TARGETS, target_meters);
        let ratio = crate::update::round_if_near(&crate::update::POW10_TARGETS, self.scene.settings.unit_meters / target_meters);
        if ratio == 1.0 {
            return;
        }
        self.unit_scale = ratio;
        if self.opts.space_conversion == SpaceConversion::TransformRoot {
            let root = &mut self.scene.nodes[self.scene.root_node];
            root.local_transform.scale.x *= ratio;
            root.local_transform.scale.y *= ratio;
            root.local_transform.scale.z *= ratio;
            let m = &mut root.node_to_parent;
            m.m00 *= ratio;
            m.m01 *= ratio;
            m.m02 *= ratio;
            m.m10 *= ratio;
            m.m11 *= ratio;
            m.m12 *= ratio;
            m.m20 *= ratio;
            m.m21 *= ratio;
            m.m22 *= ratio;
        }
    }

    fn resolve_warning_elements(&mut self) {
        for w in &mut self.scene.metadata.warnings {
            let id = w.element_id;
            if id & 0x8000_0000 != 0
                && id != u32::MAX
                && let Some(&e) = self.tmp_element_id.get((id & !0x8000_0000) as usize)
            {
                w.element_id = e;
            }
        }
    }

    fn load_imp(&mut self) -> Res<()> {
        check!((0x20..=0x7e).contains(&self.opts.path_separator));

        if !self.opts.allow_unsafe {
            check!(self.opts.index_error_handling != IndexErrorHandling::UnsafeIgnore, "Unsafe options");
            check!(self.opts.unicode_error_handling != UnicodeErrorHandling::UnsafeIgnore, "Unsafe options");
        } else {
            self.scene.metadata.is_unsafe = true;
        }
        if self.opts.index_error_handling == IndexErrorHandling::NoIndex {
            self.scene.metadata.may_contain_no_index = true;
        }
        self.scene.metadata.may_contain_missing_vertex_position = self.opts.allow_missing_vertex_position;
        self.scene.metadata.may_contain_broken_elements = self.opts.connect_broken_elements;
        self.scene.metadata.creator = String::new();

        let format = determine_format(&mut self.p, &self.opts)?;
        self.scene.metadata.file_format = format;

        match format {
            FileFormat::Fbx => {
                #[cfg(feature = "format-fbx")]
                {
                    self.p.begin_parse()?;
                    if self.version() < 6000 {
                        crate::legacy::read_legacy_root(self)?;
                    } else {
                        self.read_root()?;
                    }
                    if !supports_version(self.version()) {
                        let v = self.version();
                        self.warnf(WarningType::UnsupportedVersion, format!("Unsupported FBX version ({v})"));
                    }
                    crate::update::update_scene_metadata(&mut self.scene.metadata);
                    crate::paths::init_file_paths(&mut self.p, &mut self.scene.metadata, &self.opts)?;
                }
                #[cfg(not(feature = "format-fbx"))]
                return Err(Error::new("Feature disabled").set_err_info("format-fbx"));
            }
            FileFormat::Obj => {
                #[cfg(feature = "format-obj")]
                {
                    crate::obj::obj_load(self)?;
                    crate::update::update_scene_metadata(&mut self.scene.metadata);
                }
                #[cfg(not(feature = "format-obj"))]
                return Err(Error::new("Feature disabled").set_err_info("format-obj"));
            }
            FileFormat::Mtl => {
                #[cfg(feature = "format-obj")]
                {
                    crate::obj::mtl_load(self)?;
                    crate::update::update_scene_metadata(&mut self.scene.metadata);
                }
                #[cfg(not(feature = "format-obj"))]
                return Err(Error::new("Feature disabled").set_err_info("format-obj"));
            }
            FileFormat::Tds => {
                #[cfg(feature = "format-3ds")]
                crate::tds::tds_load(self)?;
                #[cfg(not(feature = "format-3ds"))]
                return Err(Error::new("Feature disabled").set_err_info("format-3ds"));
            }
            FileFormat::Unknown => {}
        }

        self.pre_finalize_scene()?;
        self.finalize_scene()?;

        crate::update::update_scene_settings(&mut self.scene.settings);
        if self.scene.metadata.file_format == FileFormat::Obj {
            crate::update::update_scene_settings_obj(self);
        }
        if self.scene.metadata.file_format == FileFormat::Tds {
            crate::update::update_scene_settings_tds(self);
        }

        if coordinate_axes_valid(self.opts.target_axes) {
            self.transform_to_axes(self.opts.target_axes);
        }
        if self.opts.target_unit_meters > 0.0 {
            self.scale_units(self.opts.target_unit_meters);
        }
        crate::update::update_adjust_transforms(self);

        self.modify_geometry();
        self.postprocess_scene();

        crate::update::update_scene(&mut self.scene, true, &[]);

        if self.opts.load_external_files {
            crate::external::load_external_files(self)?;
        }

        if self.opts.evaluate_skinning {
            let cache_opts = GeometryCacheDataOpts { open_file_cb: self.opts.open_file_cb.clone(), ..Default::default() };
            let load_caches = self.opts.load_external_files && self.opts.evaluate_caches;
            crate::evaluate::evaluate_skinning(&mut self.scene, 0.0, load_caches, &cache_opts)?;
        }

        // `ufbxi_retain_toplevel` in ufbx.c
        if self.opts.retain_dom && self.scene.dom_root.is_none() {
            let mut children = Vec::with_capacity(self.p.top_nodes.len());
            for (i, n) in self.p.top_nodes.iter().enumerate() {
                let mut node = retain_dom_node(n);
                if let Some(kept) = self.p.dom_children.get(&i) {
                    node.children.extend(kept.iter().map(retain_dom_node));
                }
                children.push(node);
            }
            let mut root = DomNode { children, ..Default::default() };
            apply_dom_aliases(&mut root, &self.scene, &self.dom_aliases);
            self.scene.dom_root = Some(root);
        }

        // `ufbxi_pop_warnings` in ufbx.c
        let warnings = std::mem::take(&mut self.p.warnings.list);
        for w in &warnings {
            self.scene.metadata.has_warning[w.kind as usize] = true;
        }
        self.scene.metadata.warnings = warnings;
        self.resolve_warning_elements();

        let m = &mut self.scene.metadata;
        m.version = self.p.version;
        m.ascii = self.p.from_ascii;
        m.big_endian = self.p.big_endian;
        m.geometry_ignored = self.opts.ignore_geometry;
        m.animation_ignored = self.opts.ignore_animation;
        m.embedded_ignored = self.opts.ignore_embedded;
        Ok(())
    }
}

/// `io` says how the bytes come in, and `size` is what a stream's `size_fn` would tell of it
fn load(data: &[u8], opts: LoadOpts, io: Io, size: Option<u64>) -> Result<Scene, Error> {
    let opts = complete_opts(opts);
    let mut ctx = Context::new(data, opts);
    ctx.p.io = io;
    ctx.p.io.set_opts(ctx.opts.read_buffer_size, ctx.opts.progress_cb.clone(), ctx.opts.progress_interval_hint);
    if ctx.opts.file_size_estimate != 0 {
        ctx.p.io.progress_bytes_total = ctx.opts.file_size_estimate;
    }
    if ctx.opts.progress_cb.is_some()
        && ctx.p.io.progress_bytes_total == 0
        && let Some(size) = size
    {
        ctx.p.io.progress_bytes_total = size;
    }
    match ctx.load_imp() {
        Ok(()) => Ok(ctx.scene),
        Err(e) => {
            // ufbx's error keeps the first description it was given: a failure the text reader read past comes first
            let e = match ctx.p.io.pending.take() {
                Some(pending) => first_described(pending, e),
                None => e,
            };
            let e = e.fix_error_type("Failed to load");
            #[cfg(feature = "format-fbx")]
            let e = if e.kind == ErrorType::Unknown && ctx.scene.metadata.file_format == FileFormat::Fbx && !supports_version(ctx.p.version) {
                let mut unsupported = Error::new("Unsupported version").set_err_info(ctx.p.version.to_string());
                unsupported.kind = ErrorType::UnsupportedVersion;
                unsupported
            } else {
                e
            };
            Err(e)
        }
    }
}

pub fn load_memory(data: &[u8], opts: &LoadOpts) -> Result<Scene, Error> {
    load(data, opts.clone(), Io::memory(data.len()), None)
}

/// Read to its end first, then taken as ufbx takes a file's reads, every read but the last returning all it is asked for;
/// a reader's error is an "IO error" where the load reaches it
pub fn load_stream(stream: &mut dyn std::io::Read, opts: &LoadOpts) -> Result<Scene, Error> {
    load_stream_prefix(stream, &[], opts)
}

pub fn load_stream_prefix(stream: &mut dyn std::io::Read, prefix: &[u8], opts: &LoadOpts) -> Result<Scene, Error> {
    let mut data = prefix.to_vec();
    let mut io = Io::stream(prefix.len(), false);
    if stream.read_to_end(&mut data).is_err() {
        io.io_error_at = Some(data.len());
    }
    load(&data, opts.clone(), io, None)
}

/// Through `open_file_cb`, or from the disk without one
pub fn load_file(filename: &str, opts: &LoadOpts) -> Result<Scene, Error> {
    let mut opts = opts.clone();
    if opts.filename.is_empty() {
        opts.filename = filename.to_string();
    }
    let data = match (&opts.open_file_cb, opts.open_main_file_with_default) {
        (Some(cb), false) => cb(filename, &OpenFileInfo { file_type: OpenFileType::MainModel, original_filename: filename.as_bytes().to_vec() }),
        _ => std::fs::read(filename).ok(),
    };
    let Some(data) = data else {
        let mut e = Error::new("File not found").set_err_info(filename.to_string());
        e.kind = ErrorType::FileNotFound;
        return Err(e);
    };
    // read as ufbx reads an opened file: through `fread` and `fseek`, its size from the stream
    load(&data, opts, Io::stream(0, true), Some(data.len() as u64))
}

impl Context<'_> {
    fn modify_geometry(&mut self) {
        let mut do_mirror = false;
        let do_winding = self.opts.reverse_winding;
        let mut do_scale = false;
        let mut do_geometry_transforms = false;
        if matches!(self.opts.geometry_transform_handling, GeometryTransformHandling::ModifyGeometry | GeometryTransformHandling::ModifyGeometryNoFallback) {
            for n in 0..self.scene.nodes.len() {
                if self.scene.nodes[n].is_root {
                    continue;
                }
                let t = crate::update::get_geometry_transform(&self.scene.nodes[n].element.props, &self.scene.nodes[n]);
                let node = &mut self.scene.nodes[n];
                node.geometry_transform = t;
                if !is_transform_identity(&t) {
                    node.geometry_to_node = transform_to_matrix(&t);
                    node.has_geometry_transform = true;
                } else {
                    node.geometry_to_node = Matrix::IDENTITY;
                    node.has_geometry_transform = false;
                }
            }
            do_geometry_transforms = true;
        }
        if self.mirror_axis != MirrorAxis::None {
            do_mirror = true;
        }
        if self.scene.metadata.geometry_scale != 1.0 {
            do_scale = true;
        }
        let scale = self.scene.metadata.geometry_scale;
        let axis = self.mirror_axis;

        for shape in &mut self.scene.blend_shapes {
            if do_scale {
                scale_vec3_list(&mut shape.position_offsets, scale);
            }
            if do_mirror {
                mirror_vec3_list(&mut shape.position_offsets, axis);
                mirror_vec3_list(&mut shape.normal_offsets, axis);
            }
        }

        let retain_winding = self.opts.handedness_conversion_retain_winding;
        let mut shared = SharedLists::default();
        for m in 0..self.scene.meshes.len() {
            let geo = get_geometry_transform_node(&self.scene, &self.scene.meshes[m].element);
            let geo_matrix = geo.map(|n| self.scene.nodes[n].geometry_to_node);
            let mesh = &mut self.scene.meshes[m];
            shared.take(mesh, &self.shared_positions, &self.shared_normals);
            if do_scale {
                scale_vec3_list(&mut mesh.vertex_position.values, scale);
            }
            let mut flip = do_winding;
            if do_mirror {
                mirror_vec3_list(&mut mesh.vertex_position.values, axis);
                mirror_vec3_list(&mut mesh.vertex_normal.values, axis);
                for set in &mut mesh.uv_sets {
                    mirror_vec3_list(&mut set.vertex_tangent.values, axis);
                    mirror_vec3_list(&mut set.vertex_bitangent.values, axis);
                }
                if !retain_winding {
                    flip = !flip;
                }
            }
            if flip {
                mesh.reversed_winding = true;
                flip_winding(mesh);
            }
            if do_geometry_transforms && let Some(g) = geo_matrix {
                let mut tangent_matrix = g;
                tangent_matrix.m03 = 0.0;
                tangent_matrix.m13 = 0.0;
                tangent_matrix.m23 = 0.0;
                let normal_matrix = matrix_for_normals(&g);
                transform_vec3_list(&mut mesh.vertex_position.values, &g);
                transform_vec3_list(&mut mesh.vertex_normal.values, &normal_matrix);
                normalize_vec3_list(&mut mesh.vertex_normal.values);
                for set in &mut mesh.uv_sets {
                    transform_vec3_list(&mut set.vertex_tangent.values, &tangent_matrix);
                    transform_vec3_list(&mut set.vertex_bitangent.values, &tangent_matrix);
                    normalize_vec3_list(&mut set.vertex_tangent.values);
                    normalize_vec3_list(&mut set.vertex_bitangent.values);
                }
            }
            shared.keep(mesh, &self.shared_positions, &self.shared_normals);
            sync_mesh_aliases(mesh);
        }
        shared.hand_out(&mut self.scene.meshes, &self.shared_positions, &self.shared_normals);

        for c in 0..self.scene.line_curves.len() {
            let geo = get_geometry_transform_node(&self.scene, &self.scene.line_curves[c].element).map(|n| self.scene.nodes[n].geometry_to_node);
            let curve = &mut self.scene.line_curves[c];
            if do_scale {
                scale_vec3_list(&mut curve.control_points, scale);
            }
            if do_mirror {
                mirror_vec3_list(&mut curve.control_points, axis);
            }
            if do_geometry_transforms && let Some(g) = geo {
                transform_vec3_list(&mut curve.control_points, &g);
            }
        }
        for c in 0..self.scene.nurbs_curves.len() {
            let geo = get_geometry_transform_node(&self.scene, &self.scene.nurbs_curves[c].element).map(|n| self.scene.nodes[n].geometry_to_node);
            let points = &mut self.scene.nurbs_curves[c].control_points;
            modify_vec4_points(points, do_scale, scale, do_mirror, axis, if do_geometry_transforms { geo } else { None });
        }
        for s in 0..self.scene.nurbs_surfaces.len() {
            let geo = get_geometry_transform_node(&self.scene, &self.scene.nurbs_surfaces[s].element).map(|n| self.scene.nodes[n].geometry_to_node);
            let points = &mut self.scene.nurbs_surfaces[s].control_points;
            modify_vec4_points(points, do_scale, scale, do_mirror, axis, if do_geometry_transforms { geo } else { None });
        }

        if self.opts.geometry_transform_handling != GeometryTransformHandling::Preserve {
            let mut defaults: Option<Arc<Props>> = None;
            for node in &mut self.scene.nodes {
                if defaults.is_none() {
                    defaults = node.element.props.defaults.clone();
                }
                if node.has_geometry_transform {
                    Self::set_own_prop_vec3_uniform(&mut node.element.props, "GeometricTranslation", 0.0);
                    Self::set_own_prop_vec3_uniform(&mut node.element.props, "GeometricRotation", 0.0);
                    Self::set_own_prop_vec3_uniform(&mut node.element.props, "GeometricScaling", 1.0);
                }
            }
            // ufbx edits the shared template in place: every element holding it sees the change
            if let Some(old) = defaults {
                let mut new = (*old).clone();
                Self::set_own_prop_vec3_uniform(&mut new, "GeometricTranslation", 0.0);
                Self::set_own_prop_vec3_uniform(&mut new, "GeometricRotation", 0.0);
                Self::set_own_prop_vec3_uniform(&mut new, "GeometricScaling", 1.0);
                let new = Arc::new(new);
                for r in 0..self.scene.elements.len() as u32 {
                    let e = self.scene.element_mut(r);
                    if let Some(d) = &e.props.defaults
                        && Arc::ptr_eq(d, &old)
                    {
                        e.props.defaults = Some(new.clone());
                    }
                }
            }
        }
    }

    fn postprocess_scene(&mut self) {
        if self.opts.normalize_normals || self.opts.normalize_tangents {
            let mut shared = SharedLists::default();
            for mesh in &mut self.scene.meshes {
                shared.take(mesh, &[], &self.shared_normals);
                if self.opts.normalize_normals {
                    normalize_vec3_list(&mut mesh.vertex_normal.values);
                }
                shared.keep(mesh, &[], &self.shared_normals);
                if self.opts.normalize_tangents {
                    // as ufbx: the first set's tangents, once per set
                    for _ in 0..mesh.uv_sets.len() {
                        normalize_vec3_list(&mut mesh.vertex_tangent.values);
                        normalize_vec3_list(&mut mesh.vertex_bitangent.values);
                    }
                    if let Some(set) = mesh.uv_sets.first_mut() {
                        set.vertex_tangent.values = mesh.vertex_tangent.values.clone();
                        set.vertex_bitangent.values = mesh.vertex_bitangent.values.clone();
                    }
                }
                mesh.skinned_normal.values = mesh.vertex_normal.values.clone();
            }
            shared.hand_out(&mut self.scene.meshes, &[], &self.shared_normals);
        }
        self.scene.metadata.ortho_size_unit = if self.exporter == Exporter::BlenderBinary { 1.0 / self.scene.metadata.geometry_scale } else { 30.0 };
    }
}

fn get_geometry_transform_node(scene: &Scene, element: &Element) -> Option<usize> {
    if element.instances.len() == 1 && scene.nodes[element.instances[0]].has_geometry_transform { Some(element.instances[0]) } else { None }
}

fn mirror_vec3_list(v: &mut [Vec3], axis: MirrorAxis) {
    for p in v {
        match axis {
            MirrorAxis::X => p.x = -p.x,
            MirrorAxis::Y => p.y = -p.y,
            MirrorAxis::Z => p.z = -p.z,
            MirrorAxis::None => {}
        }
    }
}

fn scale_vec3_list(v: &mut [Vec3], scale: Real) {
    for p in v {
        p.x *= scale;
        p.y *= scale;
        p.z *= scale;
    }
}

fn transform_vec3_list(v: &mut [Vec3], m: &Matrix) {
    for p in v {
        *p = transform_position(m, *p);
    }
}

fn normalize_vec3_list(v: &mut [Vec3]) {
    for p in v {
        *p = normalize3(*p);
    }
}

/// Part of `ufbxi_modify_geometry` in ufbx.c
fn modify_vec4_points(points: &mut [Vec4], do_scale: bool, scale: Real, do_mirror: bool, axis: MirrorAxis, geo: Option<Matrix>) {
    for p in points {
        let mut v = Vec3::new(p.x, p.y, p.z);
        if do_scale {
            v.x *= scale;
            v.y *= scale;
            v.z *= scale;
        }
        if do_mirror {
            mirror_vec3_list(std::slice::from_mut(&mut v), axis);
        }
        if let Some(g) = &geo {
            v = transform_position(g, v);
        }
        (p.x, p.y, p.z) = (v.x, v.y, v.z);
    }
}

/// The lists several meshes share in ufbx (`Context::shared_positions`), as each edit in place leaves them
#[derive(Default)]
struct SharedLists {
    positions: Option<Vec<Vec3>>,
    normals: Option<Vec<Vec3>>,
}

impl SharedLists {
    fn take(&self, mesh: &mut Mesh, positions: &[u32], normals: &[u32]) {
        let id = mesh.element.element_id;
        if let Some(p) = &self.positions
            && positions.contains(&id)
        {
            mesh.vertex_position.values = p.clone();
        }
        if let Some(n) = &self.normals
            && normals.contains(&id)
        {
            mesh.vertex_normal.values = n.clone();
        }
    }

    fn keep(&mut self, mesh: &Mesh, positions: &[u32], normals: &[u32]) {
        let id = mesh.element.element_id;
        if positions.contains(&id) {
            self.positions = Some(mesh.vertex_position.values.clone());
        }
        if normals.contains(&id) {
            self.normals = Some(mesh.vertex_normal.values.clone());
        }
    }

    fn hand_out(&self, meshes: &mut [Mesh], positions: &[u32], normals: &[u32]) {
        for mesh in meshes {
            let id = mesh.element.element_id;
            if positions.contains(&id) || normals.contains(&id) {
                self.take(mesh, positions, normals);
                sync_mesh_aliases(mesh);
            }
        }
    }
}

/// `clone_from` writes into the lists' own buffers, which have the length already unless the lists changed
pub(crate) fn sync_mesh_aliases(mesh: &mut Mesh) {
    mesh.vertices.clone_from(&mesh.vertex_position.values);
    if mesh.skinned_is_local {
        mesh.skinned_position.values.clone_from(&mesh.vertex_position.values);
        mesh.skinned_position.indices.clone_from(&mesh.vertex_position.indices);
        mesh.skinned_normal.values.clone_from(&mesh.vertex_normal.values);
        mesh.skinned_normal.indices.clone_from(&mesh.vertex_normal.indices);
    }
    if let Some(set) = mesh.uv_sets.first() {
        sync_attrib(&mut mesh.vertex_uv, &set.vertex_uv);
        sync_attrib(&mut mesh.vertex_tangent, &set.vertex_tangent);
        sync_attrib(&mut mesh.vertex_bitangent, &set.vertex_bitangent);
    }
    if let Some(set) = mesh.color_sets.first() {
        sync_attrib(&mut mesh.vertex_color, &set.vertex_color);
    }
}

fn sync_attrib<T: Clone>(to: &mut VertexAttrib<T>, from: &VertexAttrib<T>) {
    to.exists = from.exists;
    to.values.clone_from(&from.values);
    to.indices.clone_from(&from.indices);
    to.value_reals = from.value_reals;
    to.unique_per_vertex = from.unique_per_vertex;
    to.values_w.clone_from(&from.values_w);
}

/// Every face's indices but its first reversed, and the edges remapped to match
fn flip_winding(mesh: &mut Mesh) {
    let faces = mesh.faces.clone();
    // `ufbxi_flip_attrib_winding` in ufbx.c
    let flip = |indices: &mut Vec<u32>| {
        if indices.is_empty() {
            return;
        }
        for f in &faces {
            if f.num_indices == 0 {
                continue;
            }
            let (mut begin, mut end) = (f.index_begin as usize + 1, (f.index_begin + f.num_indices - 1) as usize);
            while begin < end {
                indices.swap(begin, end);
                begin += 1;
                end -= 1;
            }
        }
    };
    flip(&mut mesh.vertex_position.indices);
    flip(&mut mesh.vertex_normal.indices);
    flip(&mut mesh.vertex_crease.indices);
    for set in &mut mesh.uv_sets {
        flip(&mut set.vertex_uv.indices);
        flip(&mut set.vertex_tangent.indices);
        flip(&mut set.vertex_bitangent.indices);
    }
    for set in &mut mesh.color_sets {
        flip(&mut set.vertex_color.indices);
    }
    flip(&mut mesh.skinned_position.indices);
    flip(&mut mesh.skinned_normal.indices);
    if let Some(set) = mesh.uv_sets.first() {
        mesh.vertex_uv = set.vertex_uv.clone();
        mesh.vertex_bitangent = set.vertex_bitangent.clone();
        mesh.vertex_tangent = set.vertex_tangent.clone();
    }
    if let Some(set) = mesh.color_sets.first() {
        mesh.vertex_color = set.vertex_color.clone();
    }
    mesh.vertex_indices = mesh.vertex_position.indices.clone();
    crate::mesh::update_vertex_first_index(mesh);

    if !mesh.edges.is_empty() {
        let mut mapping = vec![0u32; mesh.num_indices];
        for f in &faces {
            if f.num_indices == 0 {
                continue;
            }
            let begin = f.index_begin;
            let count = f.num_indices - 1;
            mapping[begin as usize] = begin;
            for i in 0..count {
                mapping[(begin + 1 + i) as usize] = begin + count - i;
            }
        }
        let map = |i: u32| if i == NO_INDEX { NO_INDEX } else { mapping.get(i as usize).copied().unwrap_or(0) };
        for e in &mut mesh.edges {
            let (a, b) = (map(e.a), map(e.b));
            e.a = b;
            e.b = a;
        }
    }
}

#[cfg(all(test, not(feature = "format-fbx")))]
mod fbx_tests {
    use super::*;

    #[test]
    fn an_fbx_file_needs_its_feature() {
        let mut binary = b"Kaydara FBX Binary  \x00\x1a\x00".to_vec();
        binary.extend_from_slice(&7400u32.to_le_bytes());
        for bytes in [&binary[..], b"; FBX 7.4.0 project file\nFBXHeaderExtension:  {\n}\n"] {
            let e = load_memory(bytes, &LoadOpts::default()).unwrap_err();
            assert_eq!((e.kind, e.description.as_str(), e.info.as_str()), (ErrorType::FeatureDisabled, "Feature disabled", "format-fbx"));
        }
    }
}

#[cfg(all(test, not(feature = "format-obj")))]
mod tests {
    use super::*;

    #[test]
    fn an_obj_file_needs_its_feature() {
        let opts = LoadOpts { filename: "a.obj".into(), ..LoadOpts::default() };
        let e = load_memory(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n", &opts).unwrap_err();
        assert_eq!((e.kind, e.description.as_str(), e.info.as_str()), (ErrorType::FeatureDisabled, "Feature disabled", "format-obj"));
    }
}

#[cfg(all(test, not(feature = "format-3ds")))]
mod tds_tests {
    use super::*;

    #[test]
    fn a_3ds_file_is_not_known_without_its_feature() {
        let main = [0x4D, 0x4D, 6, 0, 0, 0];
        let by_name = LoadOpts { filename: "boat.3ds".into(), ..LoadOpts::default() };
        assert_eq!(load_memory(&main, &by_name).unwrap_err().kind, ErrorType::UnrecognizedFileFormat);
        let told = LoadOpts { file_format: FileFormat::Tds, ..LoadOpts::default() };
        let e = load_memory(&main, &told).unwrap_err();
        assert_eq!((e.kind, e.info.as_str()), (ErrorType::FeatureDisabled, "format-3ds"));
    }
}

/// `root` indexes `ctx.scene.nodes`
pub(crate) fn setup_root_node(ctx: &mut Context, root: usize) {
    let (local_transform, node_to_parent) = if ctx.opts.use_root_transform {
        (ctx.opts.root_transform, transform_to_matrix(&ctx.opts.root_transform))
    } else {
        (Transform::IDENTITY, Matrix::IDENTITY)
    };
    let node = &mut ctx.scene.nodes[root];
    node.local_transform = local_transform;
    node.node_to_parent = node_to_parent;
    node.is_root = true;
}
