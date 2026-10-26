use std::collections::HashMap;
use std::sync::Arc;

use crate::context::{Context, ElementInfo};
use crate::error::{Error, Res, WarningType, check, clean_string_utf8, fail};
use crate::legacy::setup_root_node;
use crate::matcher::match_pattern;
use crate::mesh::finalize_mesh;
use crate::parse::{Io, ParseOpts, Parser, f64_to_i64, parse_double};
use crate::paths::{default_open_file, init_file_paths, open_file, resolve_relative_filename};
use crate::props::{deduplicate_properties, name_key, sort_properties};
use crate::read::update_face_groups;
use crate::types::*;

const POSITION: usize = 0;
const UV: usize = 1;
const NORMAL: usize = 2;
const COLOR: usize = 3;
const NUM_ATTRIBS: usize = 3;
const ATTRIB_STRIDE: [usize; 4] = [3, 2, 3, 4];

const fn obj_cmd1(a: u8) -> u32 {
    (a as u32) << 24
}

const fn obj_cmd2(a: u8, b: u8) -> u32 {
    (a as u32) << 24 | (b as u32) << 16
}

#[derive(Debug, Clone, Copy)]
struct IndexRange {
    min_ix: u64,
    max_ix: u64,
}

/// `fbx_mesh` indexes `Scene::meshes`
#[derive(Debug, Clone)]
struct ObjMesh {
    num_faces: usize,
    num_indices: usize,
    vertex_range: [IndexRange; NUM_ATTRIBS],
    fbx_mesh: usize,
    fbx_node_id: u64,
    usemtl_base: u32,
    num_groups: u32,
}

/// `ufbxi_obj_group_entry` in ufbx.c
#[derive(Debug, Clone, Copy, Default)]
struct GroupEntry {
    local_id: u32,
    mesh_id: u32,
}

/// `line` always ends in '\n', `tokens` are (start, length) in it
#[derive(Debug, Default)]
struct ObjContext {
    line: Vec<u8>,
    tokens: Vec<(usize, usize)>,
    read_progress: usize,

    vertex_count: [usize; 4],
    tmp_vertices: [Vec<Real>; 4],
    tmp_indices: [Vec<u64>; NUM_ATTRIBS],
    tmp_color_valid: Vec<bool>,
    tmp_faces: Vec<Face>,
    tmp_face_smoothing: Vec<bool>,
    tmp_face_group: Vec<u32>,
    tmp_face_group_infos: Vec<FaceGroup>,
    tmp_face_material: Vec<u32>,
    tmp_meshes: Vec<ObjMesh>,
    tmp_props: Vec<Prop>,

    group_map: HashMap<String, GroupEntry>,

    mesh: Option<usize>,

    usemtl_fbx_id: u64,
    usemtl_index: u32,

    face_material: u32,

    face_group: u32,
    has_face_group: bool,

    face_smoothing: bool,
    has_face_smoothing: bool,

    has_vertex_color: bool,
    // never raised, in ufbx either: an MRGB comment pops every colour counted in `vertex_count[COLOR]`
    mrgb_vertex_count: usize,

    eof: bool,

    mtllib_relative_path: Vec<u8>,

    object: String,
    group: String,
    material_dirty: bool,
    object_dirty: bool,
    group_dirty: bool,
    face_group_dirty: bool,
}

const OPTION_PATTERN: &str = "-[A-Za-z][\\-A-Za-z0-9_]*";

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\r' | b'\n')
}

/// Other characters read as zero digits
fn parse_hex(digits: &[u8]) -> u32 {
    let mut value = 0u32;
    for &c in digits {
        let v = match c {
            b'0'..=b'9' => (c - b'0') as u32,
            b'A'..=b'F' => (c - b'A') as u32 + 10,
            b'a'..=b'f' => (c - b'a') as u32 + 10,
            _ => 0,
        };
        value = (value << 4) | v;
    }
    value
}

/// The zero-based index (`u64::MAX` where missing) and the bytes taken, up to and with the next '/'
fn obj_parse_index(s: &[u8], count: usize) -> Res<(u64, usize)> {
    let mut ptr = 0;
    // ufbx reads the byte past an empty rest here too, which is never '-' as a token ends at a space or a '\'
    let negative = s.first() == Some(&b'-');
    if negative {
        ptr += 1;
    }

    let mut index = 0u64;
    while ptr != s.len() {
        let c = s[ptr];
        if c.is_ascii_digit() {
            check!(index < u64::MAX / 10 - 10);
            index = index * 10 + (c - b'0') as u64;
        } else if c == b'/' {
            ptr += 1;
            break;
        }
        ptr += 1;
    }

    let index = if negative {
        let count = count as u64;
        if index <= count { count - index } else { u64::MAX }
    } else {
        index.wrapping_sub(1)
    };
    Ok((index, ptr))
}

fn split_last<T>(v: &mut Vec<T>, count: usize) -> Vec<T> {
    v.split_off(v.len().saturating_sub(count))
}

impl ObjContext {
    fn tok(&self, index: usize) -> &[u8] {
        let (start, len) = self.tokens[index];
        &self.line[start..start + len]
    }

    fn obj_pop_props(&mut self, count: usize) -> Vec<Prop> {
        let mut props = split_last(&mut self.tmp_props, count);
        for prop in &mut props {
            prop.internal_key = name_key(prop.name.as_bytes());
            if prop.value_int == 0 {
                prop.value_int = f64_to_i64(prop.value_real_arr[0]);
            }
            if prop.value_blob.is_empty() && !prop.value_str.is_empty() {
                prop.value_blob = prop.value_str.as_bytes().to_vec();
            }
        }
        if props.len() > 1 {
            sort_properties(&mut props);
            deduplicate_properties(&mut props);
        }
        props
    }

    fn obj_push_mesh(&mut self, ctx: &mut Context) -> usize {
        let name = if ctx.opts.obj_split_groups && !self.group.is_empty() {
            self.group.clone()
        } else if !ctx.opts.obj_merge_objects && !self.object.is_empty() {
            self.object.clone()
        } else if !ctx.opts.obj_merge_groups && !self.group.is_empty() {
            self.group.clone()
        } else {
            String::new()
        };

        let (fbx_node, fbx_node_id) = ctx.push_synthetic_element(None, Some(&name), ElementType::Node);
        let (fbx_mesh, fbx_mesh_id) = ctx.push_synthetic_element(None, Some(&name), ElementType::Mesh);

        ctx.scene.meshes[fbx_mesh].vertex_position.unique_per_vertex = true;

        let node_element_id = ctx.scene.nodes[fbx_node].element.element_id;
        ctx.tmp_node_ids.push(node_element_id);

        self.tmp_meshes.push(ObjMesh {
            num_faces: 0,
            num_indices: 0,
            vertex_range: [IndexRange { min_ix: u64::MAX, max_ix: 0 }; NUM_ATTRIBS],
            fbx_mesh,
            fbx_node_id,
            usemtl_base: 0,
            num_groups: 0,
        });
        let mi = self.tmp_meshes.len() - 1;
        self.mesh = Some(mi);

        self.face_material = NO_INDEX;
        self.face_group = 0;
        self.face_group_dirty = true;
        self.material_dirty = true;

        ctx.connect_oo(fbx_mesh_id, fbx_node_id);
        ctx.connect_oo(fbx_node_id, 0);

        mi
    }

    fn obj_flush_mesh(&mut self, ctx: &mut Context) {
        let Some(mi) = self.mesh else { return };
        let props = self.obj_pop_props(self.tmp_props.len());
        let mesh = &mut ctx.scene.meshes[self.tmp_meshes[mi].fbx_mesh];
        mesh.element.props.props = props;
        mesh.face_groups = std::mem::take(&mut self.tmp_face_group_infos);
    }

    fn obj_init(&mut self, ctx: &mut Context) {
        ctx.p.from_ascii = true;
        // the .obj reader keeps no progress window of its own: ufbx folds the window into the buffer and leaves it set
        ctx.p.io.data_size += ctx.p.io.yield_size;

        let root_info = ElementInfo { fbx_id: ctx.root_id, name: String::new(), ..Default::default() };
        let root = ctx.push_element(&root_info, ElementType::Node);
        setup_root_node(ctx, root);
        let root_element_id = ctx.scene.nodes[root].element.element_id;
        ctx.tmp_node_ids.push(root_element_id);
    }

    /// With its continuations, out of the reader's buffer, refilled while it holds no '\n'; '\n' appended at the end of the file
    fn obj_read_line(&mut self, p: &mut Parser) -> Res<()> {
        debug_assert!(!self.eof);
        let mut offset = 0;
        loop {
            let window = p.buffered();
            let Some(end) = window[offset..].iter().position(|&c| c == b'\n') else {
                if p.io.eof {
                    offset = window.len();
                    self.eof = true;
                    break;
                }
                p.refill((p.io.data_size * 2).max(1), false)?;
                continue;
            };
            let segment = &window[offset..];
            offset += end + 1;

            let mut esc = end;
            if esc > 0 && segment[esc - 1] == b'\r' {
                esc -= 1;
            }
            if esc > 0 && segment[esc - 1] == b'\\' {
                continue;
            }
            break;
        }

        self.line.clear();
        self.line.extend_from_slice(&p.buffered()[..offset]);
        p.take_buffered(offset);
        self.read_progress += offset;
        if self.read_progress >= p.io.progress_interval {
            p.report_progress()?;
            self.read_progress %= p.io.progress_interval;
        }
        if self.eof {
            self.line.push(b'\n');
        }
        Ok(())
    }

    fn obj_tokenize(&mut self) {
        let line = &self.line;
        let end = line.len();
        let at = |i: usize| line.get(i).copied().unwrap_or(b'\n');
        let mut ptr = 0;
        self.tokens.clear();

        loop {
            loop {
                let c = at(ptr);
                if c == b' ' || c == b'\t' || c == b'\r' {
                    ptr += 1;
                    continue;
                }
                if c == b'\\' {
                    let mut p = ptr + 1;
                    if at(p) == b'\r' {
                        p += 1;
                    }
                    if at(p) == b'\n' && p < end - 1 {
                        ptr = p + 1;
                        continue;
                    }
                }
                break;
            }

            let c = at(ptr);
            if c == b'\n' {
                break;
            }
            if c == b'#' && !self.tokens.is_empty() {
                break;
            }

            let start = ptr;
            if c == b'#' {
                ptr += 1;
                self.tokens.push((start, 1));
                continue;
            }

            loop {
                ptr += 1;
                let c = at(ptr);
                if is_space(c) {
                    break;
                }
                if c == b'\\' {
                    let mut p = ptr + 1;
                    if at(p) == b'\r' {
                        p += 1;
                    }
                    if at(p) == b'\n' && p < end - 1 {
                        break;
                    }
                }
            }
            self.tokens.push((start, ptr - start));
        }
    }

    fn obj_tokenize_line(&mut self, p: &mut Parser) -> Res<()> {
        self.obj_read_line(p)?;
        self.obj_tokenize();
        Ok(())
    }

    fn obj_span_token(&self, start: usize, end: usize) -> &[u8] {
        let end = end.min(self.tokens.len() - 1);
        let (begin, _) = self.tokens[start];
        let (last, last_len) = self.tokens[end];
        &self.line[begin..last + last_len]
    }

    fn obj_parse_vertex(&mut self, ctx: &Context, attrib: usize, offset: usize) -> Res<()> {
        if ctx.opts.ignore_geometry {
            return Ok(());
        }

        let num_values = ATTRIB_STRIDE[attrib];
        self.vertex_count[attrib] += 1;

        let mut read_values = num_values;
        if attrib == COLOR && offset + read_values > self.tokens.len() {
            read_values = 3;
        }
        check!(offset + read_values <= self.tokens.len());

        let base = self.tmp_vertices[attrib].len();
        self.tmp_vertices[attrib].resize(base + num_values, 0.0);
        for i in 0..read_values {
            let (start, len) = self.tokens[offset + i];
            let (val, end) = parse_double(&self.line[start..start + len], false);
            check!(end == Some(len));
            self.tmp_vertices[attrib][base + i] = val;
        }

        if read_values < num_values {
            self.tmp_vertices[attrib][base + read_values] = 1.0;
        }

        Ok(())
    }

    fn obj_parse_indices(&mut self, ctx: &mut Context, token_begin: usize, num_tokens: usize) -> Res<()> {
        let mut flush_mesh = false;
        if self.object_dirty {
            if !ctx.opts.obj_merge_objects {
                flush_mesh = true;
            }
            self.object_dirty = false;
        }

        if self.group_dirty {
            if ((self.object.is_empty() || ctx.opts.obj_merge_objects) && !ctx.opts.obj_merge_groups) || ctx.opts.obj_split_groups {
                flush_mesh = true;
            }
            self.group_dirty = false;
            self.face_group_dirty = true;
        }

        let mi = match self.mesh {
            Some(mi) if !flush_mesh => mi,
            _ => {
                self.obj_flush_mesh(ctx);
                self.obj_push_mesh(ctx)
            }
        };

        if self.material_dirty {
            if self.usemtl_fbx_id != 0 {
                let Some(entry) = ctx.find_fbx_id(self.usemtl_fbx_id) else { return fail() };
                let mut user_id = entry.user_id;
                let mesh = &mut self.tmp_meshes[mi];
                if mesh.usemtl_base == 0 || user_id < mesh.usemtl_base {
                    ctx.connect_oo(self.usemtl_fbx_id, mesh.fbx_node_id);

                    self.usemtl_index = self.usemtl_index.wrapping_add(1);
                    let index = self.usemtl_index;
                    check!(index < u32::MAX);
                    user_id = index;
                    if let Some(e) = ctx.fbx_id_map.get_mut(&self.usemtl_fbx_id) {
                        e.user_id = index;
                    }

                    if mesh.usemtl_base == 0 {
                        mesh.usemtl_base = index;
                    }
                }
                self.face_material = user_id.wrapping_sub(mesh.usemtl_base);
            } else {
                self.face_material = NO_INDEX;
            }
        }

        if ctx.opts.ignore_geometry {
            return Ok(());
        }

        if num_tokens == 0 && !ctx.opts.allow_empty_faces {
            ctx.warnf(WarningType::EmptyFaceRemoved, "Empty face has been removed");
            return Ok(());
        }

        if self.face_group_dirty {
            let name = if !self.group.is_empty() && (!self.object.is_empty() || ctx.opts.obj_merge_groups) && !ctx.opts.obj_split_groups {
                self.group.clone()
            } else {
                String::new()
            };

            let mesh_id = ctx.scene.meshes[self.tmp_meshes[mi].fbx_mesh].element.element_id;
            let entry = self.group_map.entry(name.clone()).or_default();
            if entry.mesh_id != mesh_id {
                let mesh = &mut self.tmp_meshes[mi];
                let id = mesh.num_groups;
                mesh.num_groups = mesh.num_groups.wrapping_add(1);
                entry.mesh_id = mesh_id;
                entry.local_id = id;
                self.tmp_face_group_infos.push(FaceGroup { id: 0, name });
            }

            self.face_group = entry.local_id;

            if !self.has_face_group {
                self.has_face_group = true;
                let num_faces = self.tmp_faces.len();
                self.tmp_face_group.resize(self.tmp_face_group.len() + num_faces, 0);
            }

            self.face_group_dirty = false;
        }

        let num_indices = num_tokens;
        let mesh = &mut self.tmp_meshes[mi];
        check!(u32::MAX as usize - mesh.num_indices >= num_indices);

        self.tmp_faces.push(Face { index_begin: mesh.num_indices as u32, num_indices: num_indices as u32 });
        mesh.num_faces += 1;
        mesh.num_indices += num_indices;

        self.tmp_face_material.push(self.face_material);
        if self.has_face_smoothing {
            self.tmp_face_smoothing.push(self.face_smoothing);
        }
        if self.has_face_group {
            self.tmp_face_group.push(self.face_group);
        }

        for ix in 0..num_indices {
            let (mut start, mut len) = self.tokens[token_begin + ix];
            for attrib in 0..NUM_ATTRIBS {
                let (index, used) = obj_parse_index(&self.line[start..start + len], self.vertex_count[attrib])?;
                self.tmp_indices[attrib].push(index);
                if index != u64::MAX {
                    let range = &mut self.tmp_meshes[mi].vertex_range[attrib];
                    range.min_ix = range.min_ix.min(index);
                    range.max_ix = range.max_ix.max(index);
                }
                start += used;
                len -= used;
            }
        }

        Ok(())
    }

    fn obj_parse_multi_indices(&mut self, ctx: &mut Context, window: usize) -> Res<()> {
        let mut begin = 1;
        while begin + window <= self.tokens.len() {
            self.obj_parse_indices(ctx, begin, window)?;
            begin += 1;
        }
        Ok(())
    }

    /// ZBrush's `#MRGB` vertex colours and its file comment
    fn obj_parse_comment(&mut self, ctx: &mut Context) {
        if self.tokens.len() >= 3 && self.tok(1) == b"MRGB" {
            let num_color = self.vertex_count[COLOR];

            if num_color > self.mrgb_vertex_count {
                let num_pop = num_color - self.mrgb_vertex_count;
                let n = self.tmp_color_valid.len().saturating_sub(num_pop);
                self.tmp_color_valid.truncate(n);
                let n = self.tmp_vertices[COLOR].len().saturating_sub(num_pop * 4);
                self.tmp_vertices[COLOR].truncate(n);
                self.vertex_count[COLOR] -= num_pop;
            }

            let (start, len) = self.tokens[2];
            let mut i = 0;
            while i + 8 <= len {
                let hex = parse_hex(&self.line[start + i..start + i + 8]);
                self.tmp_vertices[COLOR].extend_from_slice(&[
                    ((hex >> 16) & 0xff) as Real / 255.0,
                    ((hex >> 8) & 0xff) as Real / 255.0,
                    (hex & 0xff) as Real / 255.0,
                    ((hex >> 24) & 0xff) as Real / 255.0,
                ]);
                self.tmp_color_valid.push(true);
                i += 8;
            }

            self.has_vertex_color = true;
        }

        if !ctx.opts.disable_quirks && match_pattern(&self.line, "\\s*#\\s*File exported by ZBrush.*") && self.mesh.is_none() {
            ctx.opts.obj_merge_groups = true;
        }
    }

    fn obj_parse_material(&mut self, ctx: &mut Context) -> Res<()> {
        self.material_dirty = true;

        if self.tokens.len() < 2 {
            self.usemtl_fbx_id = 0;
            return Ok(());
        }

        let raw = self.obj_span_token(1, usize::MAX).to_vec();
        let name = ctx.p.utf8(&raw)?;

        let fbx_id = ctx.synthetic_id_from_string(name.as_bytes());
        let exists = ctx.find_fbx_id(fbx_id).is_some();

        self.usemtl_fbx_id = fbx_id;

        if !exists {
            let info = ElementInfo { fbx_id, name, ..Default::default() };
            let ix = ctx.push_element(&info, ElementType::Material);
            ctx.scene.materials[ix].shader_type = ShaderType::WavefrontMtl;
        }

        Ok(())
    }

    fn obj_pop_vertices(&mut self, attrib: usize, min_index: u64) -> Res<Vec<Real>> {
        let stride = ATTRIB_STRIDE[attrib];
        let buf = &mut self.tmp_vertices[attrib];
        check!(min_index < (buf.len() / stride) as u64);
        Ok(buf.split_off(min_index as usize * stride))
    }

    #[allow(clippy::too_many_arguments)]
    fn obj_setup_attrib<T>(
        &mut self,
        ctx: &mut Context,
        mesh: &ObjMesh,
        data: &[Real],
        attrib: usize,
        non_disjoint: bool,
        required: bool,
        make: fn(&[Real]) -> T,
    ) -> Res<Option<(Vec<T>, Vec<u32>)>> {
        let num_indices = mesh.num_indices;
        let stride = ATTRIB_STRIDE[attrib];
        let num_values = data.len() / stride;

        let mesh_min_ix = mesh.vertex_range[attrib].min_ix;
        if num_indices == 0 || num_values == 0 || mesh_min_ix == u64::MAX {
            check!(num_indices == 0 || !required);
            split_last(&mut self.tmp_indices[attrib], num_indices);
            return Ok(None);
        }

        let min_index = if non_disjoint { 0 } else { mesh_min_ix };

        let tmp_indices = split_last(&mut self.tmp_indices[attrib], num_indices);
        let mut indices = Vec::with_capacity(num_indices);
        for &index in &tmp_indices {
            let mut ix = index;
            if ix != u64::MAX {
                ix = ix.wrapping_sub(min_index);
                check!(ix < u32::MAX as u64);
            }
            if ix < num_values as u64 {
                indices.push(ix as u32);
            } else {
                indices.push(ctx.fix_index(ix as u32, num_values)?);
            }
        }

        let values = data.chunks_exact(stride).map(make).collect();
        Ok(Some((values, indices)))
    }

    fn obj_pad_colors(&mut self, ctx: &Context, num_vertices: usize) {
        if ctx.opts.ignore_geometry {
            return;
        }

        let num_colors = self.vertex_count[COLOR];
        if num_vertices > num_colors {
            let num_pad = num_vertices - num_colors;
            let n = self.tmp_vertices[COLOR].len() + num_pad * 4;
            self.tmp_vertices[COLOR].resize(n, 0.0);
            let n = self.tmp_color_valid.len() + num_pad;
            self.tmp_color_valid.resize(n, false);
            self.vertex_count[COLOR] += num_pad;
        }
    }

    fn obj_pop_meshes(&mut self, ctx: &mut Context) -> Res<()> {
        let meshes = std::mem::take(&mut self.tmp_meshes);

        if self.has_vertex_color {
            self.obj_pad_colors(ctx, self.vertex_count[POSITION]);
        }

        let mut non_disjoint = [false; NUM_ATTRIBS];
        let mut next_min = [0u64; NUM_ATTRIBS];
        let mut vertices: [Vec<Real>; 4] = Default::default();
        let mut color_valid: Vec<bool> = Vec::new();

        for mesh in &meshes {
            for attrib in 0..NUM_ATTRIBS {
                let range = mesh.vertex_range[attrib];
                if range.min_ix > range.max_ix {
                    continue;
                }
                if range.min_ix < next_min[attrib] {
                    non_disjoint[attrib] = true;
                }
                next_min[attrib] = range.max_ix + 1;
            }
        }

        for attrib in 0..NUM_ATTRIBS {
            if !non_disjoint[attrib] {
                continue;
            }
            vertices[attrib] = self.obj_pop_vertices(attrib, 0)?;
        }
        if self.has_vertex_color && non_disjoint[POSITION] {
            vertices[COLOR] = self.obj_pop_vertices(COLOR, 0)?;
            color_valid = split_last(&mut self.tmp_color_valid, vertices[COLOR].len() / 4);
        }

        for mesh in meshes.iter().rev() {
            let fbx_mesh = mesh.fbx_mesh;
            let num_faces = mesh.num_faces;

            if !ctx.opts.ignore_geometry {
                for attrib in 0..NUM_ATTRIBS {
                    if non_disjoint[attrib] {
                        continue;
                    }
                    let min_ix = mesh.vertex_range[attrib].min_ix;
                    if min_ix < u64::MAX {
                        vertices[attrib] = self.obj_pop_vertices(attrib, min_ix)?;
                    }
                }
                if self.has_vertex_color && !non_disjoint[POSITION] {
                    let min_ix = mesh.vertex_range[POSITION].min_ix;
                    check!(min_ix < u64::MAX);
                    vertices[COLOR] = self.obj_pop_vertices(COLOR, min_ix)?;
                    color_valid = split_last(&mut self.tmp_color_valid, vertices[COLOR].len() / 4);
                }

                let m = &mut ctx.scene.meshes[fbx_mesh];
                m.faces = split_last(&mut self.tmp_faces, num_faces);
                m.face_material = split_last(&mut self.tmp_face_material, num_faces);

                if self.has_face_smoothing {
                    m.face_smoothing = split_last(&mut self.tmp_face_smoothing, num_faces);
                }

                if self.has_face_group {
                    let face_group = split_last(&mut self.tmp_face_group, num_faces);
                    if mesh.num_groups > 1 {
                        m.face_group = face_group;
                    }
                }

                let position =
                    self.obj_setup_attrib(ctx, mesh, &vertices[POSITION], POSITION, non_disjoint[POSITION], true, |v| Vec3::new(v[0], v[1], v[2]))?;
                let uv = self.obj_setup_attrib(ctx, mesh, &vertices[UV], UV, non_disjoint[UV], false, |v| Vec2::new(v[0], v[1]))?;
                let normal = self.obj_setup_attrib(ctx, mesh, &vertices[NORMAL], NORMAL, non_disjoint[NORMAL], false, |v| Vec3::new(v[0], v[1], v[2]))?;

                let m = &mut ctx.scene.meshes[fbx_mesh];
                if non_disjoint[POSITION] && position.is_some() {
                    ctx.shared_positions.push(m.element.element_id);
                }
                if non_disjoint[NORMAL] && normal.is_some() {
                    ctx.shared_normals.push(m.element.element_id);
                }
                if let Some((values, indices)) = position {
                    m.vertex_position.exists = true;
                    m.vertex_position.values = values;
                    m.vertex_position.indices = indices;
                }
                if let Some((values, indices)) = uv {
                    m.vertex_uv.exists = true;
                    m.vertex_uv.values = values;
                    m.vertex_uv.indices = indices;
                }
                if let Some((values, indices)) = normal {
                    m.vertex_normal.exists = true;
                    m.vertex_normal.values = values;
                    m.vertex_normal.indices = indices;
                }

                if self.has_vertex_color {
                    let mut has_color = false;
                    let mut all_valid = true;
                    let max_index = m.vertex_position.values.len();
                    for &ix in &m.vertex_position.indices {
                        if (ix as usize) < max_index {
                            if color_valid.get(ix as usize).copied().unwrap_or(false) {
                                has_color = true;
                            } else {
                                all_valid = false;
                            }
                        }
                    }

                    if has_color {
                        let values: Vec<Vec4> = vertices[COLOR].as_chunks::<4>().0.iter().map(|v| Vec4::new(v[0], v[1], v[2], v[3])).collect();
                        let num_values = values.len();
                        let mut indices = m.vertex_position.indices.clone();
                        if !all_valid {
                            for p_ix in &mut indices {
                                if *p_ix as usize >= num_values || !color_valid.get(*p_ix as usize).copied().unwrap_or(false) {
                                    *p_ix = ctx.fix_index(*p_ix, num_values)?;
                                }
                            }
                        }

                        let m = &mut ctx.scene.meshes[fbx_mesh];
                        m.vertex_color.exists = true;
                        m.vertex_color.values = values;
                        m.vertex_color.indices = indices;
                        m.vertex_color.unique_per_vertex = true;
                    }
                }
            }

            let m = &mut ctx.scene.meshes[fbx_mesh];
            finalize_mesh(m)?;

            if ctx.retain_mesh_parts {
                m.face_group_parts = vec![MeshPart::default(); mesh.num_groups as usize];
            }

            if mesh.num_groups > 1 {
                update_face_groups(m);
            } else if mesh.num_groups == 1 {
                m.face_group = vec![0; num_faces];
                if !m.face_group_parts.is_empty() {
                    m.face_group_parts[0] = MeshPart {
                        index: 0,
                        num_faces,
                        num_triangles: m.num_triangles,
                        num_empty_faces: m.num_empty_faces,
                        num_point_faces: m.num_point_faces,
                        num_line_faces: m.num_line_faces,
                        face_indices: (0..num_faces as u32).collect(),
                    };
                }
            }
        }

        Ok(())
    }

    fn obj_parse_file(&mut self, ctx: &mut Context) -> Res<()> {
        while !self.eof {
            self.obj_tokenize_line(&mut ctx.p)?;
            let num_tokens = self.tokens.len();
            if num_tokens == 0 {
                continue;
            }

            let cmd = self.tok(0);
            let key = name_key(cmd);
            if key == obj_cmd1(b'v') {
                self.obj_parse_vertex(ctx, POSITION, 1)?;
                if num_tokens >= 7 {
                    let num_vertices = self.vertex_count[POSITION];
                    self.has_vertex_color = true;
                    self.obj_pad_colors(ctx, num_vertices.wrapping_sub(1));
                    if self.vertex_count[COLOR] < num_vertices {
                        self.obj_parse_vertex(ctx, COLOR, 4)?;
                        self.tmp_color_valid.push(true);
                    }
                }
            } else if key == obj_cmd2(b'v', b't') {
                self.obj_parse_vertex(ctx, UV, 1)?;
            } else if key == obj_cmd2(b'v', b'n') {
                self.obj_parse_vertex(ctx, NORMAL, 1)?;
            } else if key == obj_cmd1(b'f') {
                self.obj_parse_indices(ctx, 1, num_tokens - 1)?;
            } else if key == obj_cmd1(b'p') {
                self.obj_parse_multi_indices(ctx, 1)?;
            } else if key == obj_cmd1(b'l') {
                self.obj_parse_multi_indices(ctx, 2)?;
            } else if key == obj_cmd1(b's') {
                if num_tokens >= 2 {
                    self.has_face_smoothing = true;
                    self.face_smoothing = self.tok(1) != b"off";

                    if self.tmp_face_smoothing.is_empty() && !self.tmp_faces.is_empty() {
                        self.tmp_face_smoothing.resize(self.tmp_faces.len(), false);
                    }
                }
            } else if key == obj_cmd1(b'o') {
                if num_tokens >= 2 {
                    let raw = self.obj_span_token(1, usize::MAX).to_vec();
                    self.object = ctx.p.utf8(&raw)?;
                    self.object_dirty = true;
                }
            } else if key == obj_cmd1(b'g') {
                if num_tokens >= 2 {
                    let raw = self.obj_span_token(1, usize::MAX).to_vec();
                    self.group = ctx.p.utf8(&raw)?;
                } else {
                    self.group = String::new();
                }
                self.group_dirty = true;
            } else if key == obj_cmd1(b'#') {
                self.obj_parse_comment(ctx);
            } else if cmd == b"mtllib" {
                check!(num_tokens >= 2);
                self.mtllib_relative_path = self.obj_span_token(1, usize::MAX).to_vec();
            } else if cmd == b"usemtl" {
                self.obj_parse_material(ctx)?;
            } else if !ctx.opts.disable_quirks && key == 0 {
                // ZBrush ends some files with a NUL
            } else {
                ctx.warnf(WarningType::UnknownObjDirective, "Unknown .obj directive, skipped line");
            }
        }

        self.obj_flush_mesh(ctx);
        self.obj_pop_meshes(ctx)
    }

    fn obj_flush_material(&mut self, ctx: &mut Context) -> Res<()> {
        if self.usemtl_fbx_id == 0 {
            return Ok(());
        }

        let Some(entry) = ctx.find_fbx_id(self.usemtl_fbx_id) else { return fail() };
        let r = ctx.scene.elements[entry.element_id as usize];
        check!(r.element_type == ElementType::Material);

        let props = self.obj_pop_props(self.tmp_props.len());
        ctx.scene.materials[r.index].element.props.props = props;
        Ok(())
    }

    /// The value is the rest of the line where `include_rest`, else the tokens up to the next `-option`; returns the token after it
    fn obj_parse_prop(&mut self, ctx: &mut Context, name: &[u8], start: usize, include_rest: bool) -> Res<usize> {
        let num_tokens = self.tokens.len();
        if start >= num_tokens {
            return Ok(start);
        }

        let mut prop = Prop { name: ctx.p.utf8(name)?, ..Default::default() };

        let mut flags = prop_flags::VALUE_STR;

        let mut num_reals = 0;
        while num_reals < 4 {
            if start + num_reals >= num_tokens {
                break;
            }
            let tok = self.tok(start + num_reals);
            let (val, end) = parse_double(tok, false);
            if end != Some(tok.len()) {
                break;
            }

            prop.value_real_arr[num_reals] = val;
            if num_reals == 0 {
                prop.value_int = f64_to_i64(val);
                flags |= prop_flags::VALUE_INT;
            }
            num_reals += 1;
        }

        let mut num_args = 0;
        if !include_rest {
            while start + num_args < num_tokens - 1 {
                if match_pattern(self.tok(start + num_args), OPTION_PATTERN) {
                    break;
                }
                num_args += 1;
            }
        }

        if num_args > 0 || include_rest {
            let raw = self.obj_span_token(start, if include_rest { usize::MAX } else { start + num_args - 1 }).to_vec();
            prop.value_str = ctx.p.utf8(&raw)?;
            prop.value_blob = raw;
        }

        if num_reals > 0 {
            flags = prop_flags::VALUE_REAL << (num_reals - 1);
        } else if prop.value_str == "on" {
            prop.value_int = 1;
            prop.value_real_arr[0] = 1.0;
            flags |= prop_flags::VALUE_INT;
        } else if prop.value_str == "off" {
            prop.value_int = 0;
            prop.value_real_arr[0] = 0.0;
            flags |= prop_flags::VALUE_INT;
        }

        prop.flags = flags;
        self.tmp_props.push(prop);

        Ok(start + num_args)
    }

    fn obj_parse_mtl_map(&mut self, ctx: &mut Context, prefix_len: usize) -> Res<()> {
        let num_tokens = self.tokens.len();
        if num_tokens < 2 {
            return Ok(());
        }

        let mut num_props = 1;
        self.obj_parse_prop(ctx, b"obj|args", 1, true)?;

        let mut start = 1;
        while start + 1 < num_tokens {
            let tok = self.tok(start);
            if match_pattern(tok, OPTION_PATTERN) {
                let name = tok[1..].to_vec();
                start = self.obj_parse_prop(ctx, &name, start + 1, false)?;
                num_props += 1;
            } else {
                break;
            }
        }

        let tex_raw = self.obj_span_token(start, usize::MAX).to_vec();
        let tex_str = ctx.p.utf8(&tex_raw)?;

        let (texture, fbx_id) = ctx.push_synthetic_element(None, Some(""), ElementType::Texture);

        let props = self.obj_pop_props(num_props);
        let t = &mut ctx.scene.textures[texture];
        t.relative_filename = tex_str;
        t.raw_relative_filename = tex_raw;
        t.element.props.props = props;

        let prop = self.tok(0)[prefix_len..].to_vec();
        let prop = ctx.p.utf8(&prop)?;

        if self.usemtl_fbx_id != 0 {
            ctx.connect_op(fbx_id, self.usemtl_fbx_id, &prop);
        }

        Ok(())
    }

    fn obj_parse_mtl(&mut self, ctx: &mut Context, mut mtl: Option<&mut Parser>) -> Res<()> {
        self.mesh = None;
        self.usemtl_fbx_id = 0;

        while !self.eof {
            match mtl.as_deref_mut() {
                Some(p) => self.obj_tokenize_line(p)?,
                None => self.obj_tokenize_line(&mut ctx.p)?,
            }
            let num_tokens = self.tokens.len();
            if num_tokens == 0 {
                continue;
            }

            let cmd = self.tok(0);
            if cmd == b"newmtl" {
                // ufbx reuses the `usemtl` parsing but wants a name here
                check!(num_tokens >= 2);
                self.obj_flush_material(ctx)?;
                self.obj_parse_material(ctx)?;
            } else if cmd.len() > 4 && cmd.starts_with(b"map_") {
                self.obj_parse_mtl_map(ctx, 4)?;
            } else if cmd == b"bump" || cmd == b"disp" || cmd == b"norm" {
                self.obj_parse_mtl_map(ctx, 0)?;
            } else if cmd == b"#" {
            } else {
                let name = cmd.to_vec();
                self.obj_parse_prop(ctx, &name, 1, true)?;
            }
        }

        self.obj_flush_material(ctx)
    }

    /// From `obj_mtl_data`, `obj_mtl_path`, the `mtllib` line or the .obj's own name
    fn obj_load_mtl(&mut self, ctx: &mut Context) -> Res<()> {
        self.eof = false;

        if !ctx.opts.obj_mtl_data.is_empty() {
            let data = ctx.opts.obj_mtl_data.clone();
            let mut p = mtl_reader(ctx, &data, false);
            return self.obj_parse_mtl(ctx, Some(&mut p));
        }

        let mut stream: Option<Vec<u8>> = None;
        let mut needs_stream = false;
        let mut stream_path: Vec<u8> = Vec::new();

        let cb: Arc<OpenFileFn> = ctx.opts.open_file_cb.clone().unwrap_or_else(|| Arc::new(default_open_file));
        if !ctx.opts.obj_mtl_path.is_empty() {
            let path = ctx.opts.obj_mtl_path.clone();
            stream = open_file(Some(&cb), &path, None, OpenFileType::ObjMtl);
            needs_stream = true;
            if stream.is_none() {
                ctx.warnf(WarningType::MissingExternalFile, format!("Could not open .mtl file: {path}"));
            }
            stream_path = path.into_bytes();
        }

        if stream.is_none() && ctx.opts.load_external_files && !self.mtllib_relative_path.is_empty() {
            let dst = resolve_relative_filename(ctx.opts.path_separator, &ctx.scene.metadata.raw_relative_root, &self.mtllib_relative_path);
            let dst_path = String::from_utf8_lossy(&dst).into_owned();
            stream = open_file(Some(&cb), &dst_path, Some(&self.mtllib_relative_path), OpenFileType::ObjMtl);
            stream_path = self.mtllib_relative_path.clone();
            needs_stream = true;
            if stream.is_none() {
                ctx.warnf(WarningType::MissingExternalFile, clean_string_utf8(&[b"Could not open .mtl file: ".as_slice(), &dst].concat()));
            }
        }

        let path = ctx.scene.metadata.filename.as_bytes();
        if stream.is_none() && ctx.opts.load_external_files && ctx.opts.obj_search_mtl_by_filename && path.len() > 4 {
            let len = path.len();
            if match_pattern(&path[len - 4..], "\\c.obj") {
                let mut copy = path.to_vec();
                copy[len - 3] = if copy[len - 3] == b'O' { b'M' } else { b'm' };
                copy[len - 2] = if copy[len - 2] == b'B' { b'T' } else { b't' };
                copy[len - 1] = if copy[len - 1] == b'J' { b'L' } else { b'l' };
                let copy = String::from_utf8_lossy(&copy).into_owned();
                stream = open_file(Some(&cb), &copy, None, OpenFileType::ObjMtl);
                if stream.is_some() {
                    ctx.warnf(WarningType::ImplicitMtl, format!("Opened .mtl file derived from .obj filename: {copy}"));
                }
            }
        }

        if let Some(data) = stream {
            let mut p = mtl_reader(ctx, &data, true);
            self.obj_parse_mtl(ctx, Some(&mut p))?;
        } else if needs_stream && !ctx.opts.ignore_missing_external_files {
            return Err(Error::new("External file not found").set_err_info(clean_string_utf8(&stream_path)));
        }

        Ok(())
    }
}

pub(crate) fn obj_load(ctx: &mut Context) -> Res<()> {
    let mut obj = ObjContext::default();
    obj.obj_init(ctx);
    obj.obj_parse_file(ctx)?;
    init_file_paths(&mut ctx.scene.metadata, &ctx.opts);
    obj.obj_load_mtl(ctx)
}

pub(crate) fn mtl_load(ctx: &mut Context) -> Res<()> {
    let mut obj = ObjContext::default();
    obj.obj_init(ctx);
    init_file_paths(&mut ctx.scene.metadata, &ctx.opts);
    obj.obj_parse_mtl(ctx, None)
}

/// Part of `ufbxi_obj_load_mtl` in ufbx.c: ufbx's reader reset, its buffer emptied, the offset and the progress counters going on
fn mtl_reader<'d>(ctx: &Context, data: &'d [u8], stream: bool) -> Parser<'d> {
    let mut p = Parser::new(data, ParseOpts::default());
    let main = &ctx.p.io;
    p.io = if stream { Io::stream(0, false) } else { Io::memory(data.len()) };
    p.io.read_buffer_size = main.read_buffer_size;
    p.io.opts_read_buffer_size = main.opts_read_buffer_size;
    p.io.data_offset = main.data_offset;
    p.io.progress = main.progress.clone();
    p.io.progress_interval = main.progress_interval;
    p.io.latest_progress_bytes = main.latest_progress_bytes;
    p.io.progress_bytes_total = main.progress_bytes_total;
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::DEFAULT_PATH_SEPARATOR;

    fn test_context(data: &[u8], mut opts: LoadOpts) -> Context<'_> {
        if opts.path_separator == 0 {
            opts.path_separator = DEFAULT_PATH_SEPARATOR;
        }
        Context::new(data, opts)
    }

    fn load(text: &str, opts: LoadOpts) -> Context<'_> {
        let mut ctx = test_context(text.as_bytes(), opts);
        obj_load(&mut ctx).unwrap();
        ctx
    }

    fn warnings(ctx: &Context) -> Vec<(WarningType, String, usize)> {
        ctx.p.warnings.list.iter().map(|w| (w.kind, w.description.clone(), w.count)).collect()
    }

    fn element_of(ctx: &Context, fbx_id: u64) -> Option<u32> {
        ctx.fbx_id_map.get(&fbx_id).map(|e| e.element_id)
    }

    fn sources(ctx: &Context, dst: u32) -> Vec<(u32, String)> {
        let dst_fbx = ctx.element_fbx_ids[dst as usize];
        ctx.tmp_connections.iter().filter(|c| c.dst == dst_fbx).filter_map(|c| Some((element_of(ctx, c.src)?, c.dst_prop.clone()))).collect()
    }

    fn mesh_node(ctx: &Context, mesh: usize) -> u32 {
        let fbx = ctx.element_fbx_ids[ctx.scene.meshes[mesh].element.element_id as usize];
        let c = ctx.tmp_connections.iter().find(|c| c.src == fbx).unwrap();
        element_of(ctx, c.dst).unwrap()
    }

    fn mesh_materials(ctx: &Context, mesh: usize) -> Vec<usize> {
        let node = mesh_node(ctx, mesh);
        sources(ctx, node)
            .into_iter()
            .map(|(e, _)| ctx.scene.elements[e as usize])
            .filter(|r| r.element_type == ElementType::Material)
            .map(|r| r.index)
            .collect()
    }

    fn tokens_of(line: &[u8]) -> (Vec<Vec<u8>>, bool) {
        let mut obj = ObjContext::default();
        obj.obj_tokenize_line(&mut Parser::new(line, ParseOpts::default())).unwrap();
        (obj.tokens.iter().map(|&(s, l)| obj.line[s..s + l].to_vec()).collect(), obj.eof)
    }

    #[test]
    fn lines_split_into_tokens_as_ufbx_splits_them() {
        let t = |v: &[&str]| v.iter().map(|s| s.as_bytes().to_vec()).collect::<Vec<_>>();
        assert_eq!(tokens_of(b"f 1 2 \\\n 3\nnext"), (t(&["f", "1", "2", "3"]), false));
        assert_eq!(tokens_of(b"f 1 2\\\r\n3\r\n"), (t(&["f", "1", "2", "3"]), false));
        assert_eq!(tokens_of(b"v 1 2 3 # a comment\n"), (t(&["v", "1", "2", "3"]), false));
        assert_eq!(tokens_of(b"#MRGB ff00ff00\n"), (t(&["#", "MRGB", "ff00ff00"]), false));
        assert_eq!(tokens_of(b"f a#b\n"), (t(&["f", "a#b"]), false));
        // a '\' before the line's own end is a token, not a continuation
        assert_eq!(tokens_of(b"f 1 \\"), (t(&["f", "1", "\\"]), true));
        assert_eq!(tokens_of(b""), (t(&[]), true));
    }

    #[test]
    fn indices_parse_as_ufbx_parses_them() {
        assert_eq!(obj_parse_index(b"12/3", 0).unwrap(), (11, 3));
        assert_eq!(obj_parse_index(b"-1/", 5).unwrap(), (4, 3));
        assert_eq!(obj_parse_index(b"-9", 5).unwrap(), (u64::MAX, 2));
        assert_eq!(obj_parse_index(b"0", 5).unwrap(), (u64::MAX, 1));
        assert_eq!(obj_parse_index(b"", 5).unwrap(), (u64::MAX, 0));
        assert_eq!(obj_parse_index(b"-", 5).unwrap(), (5, 1));
        assert_eq!(obj_parse_index(b"1x2", 0).unwrap(), (11, 3));
        assert!(obj_parse_index(b"99999999999999999999999", 0).is_err());
        assert_eq!(parse_hex(b"ff112233"), 0xff112233);
        assert_eq!(parse_hex(b"zZ9"), 0x009);
    }

    const QUAD: &str = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nvn 0 0 1\n";

    #[test]
    fn faces_index_positions_uvs_and_normals() {
        let text = format!("{QUAD}s 1\nf 1/1/1 2/2/1 3/3/1 4/4/1\ns off\nf -4//-1 -2//-1 -1//-1\nf 1/1 3/3 4/4\n");
        let ctx = load(&text, LoadOpts::default());
        assert_eq!(ctx.scene.elements.len(), 3);
        assert!(ctx.scene.nodes[0].is_root && ctx.scene.nodes[0].element.name.is_empty());
        let m = &ctx.scene.meshes[0];
        let faces: Vec<(u32, u32)> = m.faces.iter().map(|f| (f.index_begin, f.num_indices)).collect();
        assert_eq!(faces, [(0, 4), (4, 3), (7, 3)]);
        assert_eq!(m.face_smoothing, [true, false, false]);
        assert_eq!(m.face_material, [NO_INDEX; 3]);
        assert_eq!(m.vertex_position.indices, [0, 1, 2, 3, 0, 2, 3, 0, 2, 3]);
        assert_eq!(m.vertex_position.values[2], Vec3::new(1.0, 1.0, 0.0));
        assert!(m.vertex_position.unique_per_vertex);
        assert_eq!(m.vertex_uv.indices, [0, 1, 2, 3, 3, 3, 3, 0, 2, 3]);
        assert_eq!(m.vertex_normal.indices, [0; 10]);
        assert_eq!(m.vertex_normal.values, [Vec3::new(0.0, 0.0, 1.0)]);
        assert_eq!((m.num_triangles, m.max_face_triangles, m.uv_sets.len()), (4, 2, 1));
        assert_eq!(warnings(&ctx), [(WarningType::IndexClamped, "Clamped index".to_string(), 6)]);
        assert_eq!(m.face_groups, [FaceGroup { id: 0, name: String::new() }]);
        assert_eq!(m.face_group, [0, 0, 0]);
        assert_eq!(m.face_group_parts[0].face_indices, [0, 1, 2]);
    }

    #[test]
    fn objects_make_meshes_and_groups_make_face_groups() {
        let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 1 1 1\no first\ng g1\nf 1 2 3\ng g2\nf 2 3 4\ng g1\nf 1 3 4\no second\ng g3\nf 1 2 4\ng\nf 4 3 2\n";
        let ctx = load(text, LoadOpts::default());
        let names: Vec<&str> = ctx.scene.meshes.iter().map(|m| m.element.name.as_str()).collect();
        assert_eq!(names, ["first", "second"]);
        let m = &ctx.scene.meshes[0];
        assert_eq!(m.face_group, [0, 1, 0]);
        assert_eq!(m.face_groups.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), ["g1", "g2"]);
        assert_eq!(m.face_group_parts.iter().map(|p| p.face_indices.clone()).collect::<Vec<_>>(), [vec![0, 2], vec![1]]);
        assert_eq!(ctx.scene.meshes[1].face_groups.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), ["g3", ""]);
        let ctx = load("v 0 0 0\nv 1 0 0\nv 0 1 0\ng a\nf 1 2 3\ng b\nf 3 2 1\n", LoadOpts::default());
        assert_eq!(ctx.scene.meshes.iter().map(|m| m.element.name.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        let opts = LoadOpts { obj_merge_groups: true, ..Default::default() };
        let ctx = load("v 0 0 0\nv 1 0 0\nv 0 1 0\ng a\nf 1 2 3\ng b\nf 3 2 1\n", opts);
        assert_eq!(ctx.scene.meshes.len(), 1);
        assert_eq!(ctx.scene.meshes[0].face_group, [0, 1]);
        let ctx = load("#File exported by ZBrush\nv 0 0 0\nv 1 0 0\nv 0 1 0\ng a\nf 1 2 3\ng b\nf 3 2 1\n", LoadOpts::default());
        assert_eq!(ctx.scene.meshes.len(), 1);
    }

    #[test]
    fn lines_and_points_are_faces_of_two_and_one() {
        let ctx = load("v 0 0 0\nv 1 0 0\nv 2 0 0\nv 3 0 0\nl 1 2 3 4\np 1 2\np 4\nl 1\nf\nf 1 2 \\\n   3\n", LoadOpts::default());
        let m = &ctx.scene.meshes[0];
        assert_eq!(m.faces.iter().map(|f| f.num_indices).collect::<Vec<_>>(), [2, 2, 2, 1, 1, 1, 3]);
        assert_eq!(m.vertex_position.indices, [0, 1, 1, 2, 2, 3, 0, 1, 3, 0, 1, 2]);
        assert_eq!((m.num_line_faces, m.num_point_faces, m.num_triangles), (3, 3, 1));
        assert_eq!(warnings(&ctx), [(WarningType::EmptyFaceRemoved, "Empty face has been removed".to_string(), 1)]);
    }

    #[test]
    fn colours_after_positions_and_mrgb_comments() {
        let ctx = load("v 0 0 0 1 0 0\nv 1 0 0 0 1 0 0.5\nv 1 1 0\nv 0 1 0 0 0 1\nf 1 2 3 4\n", LoadOpts::default());
        let c = &ctx.scene.meshes[0].vertex_color;
        assert!(c.exists && c.unique_per_vertex);
        assert_eq!(c.values, [Vec4::new(1.0, 0.0, 0.0, 1.0), Vec4::new(0.0, 1.0, 0.0, 0.5), Vec4::default(), Vec4::new(0.0, 0.0, 1.0, 1.0)]);
        assert_eq!(c.indices, [0, 1, 3, 3]);

        let ctx = load("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 2 2 2\n#MRGB ff112233ff445566\n#MRGB 80aabbcc\nf 1 2 3 4\n", LoadOpts::default());
        let c = &ctx.scene.meshes[0].vertex_color;
        assert_eq!(c.values.len(), 7);
        assert_eq!(c.values[0], Vec4::new(17.0 / 255.0, 34.0 / 255.0, 51.0 / 255.0, 1.0));
        assert_eq!(c.values[2].w, 128.0 / 255.0);
        assert_eq!(c.indices, [0, 1, 2, 6]);
    }

    #[test]
    fn materials_are_numbered_in_each_mesh_by_first_use() {
        let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 1 1 0\no one\nusemtl mat a\nf 1 2 3\nusemtl mat b\nf 2 3 4\no two\nusemtl mat a\nf 1 2 4\nusemtl\nf 1 3 4\nusemtl mat b\nf 2 3 4\nusemtl mat c\nf 1 2 3\n";
        let ctx = load(text, LoadOpts::default());
        let names: Vec<&str> = ctx.scene.materials.iter().map(|m| m.element.name.as_str()).collect();
        assert_eq!(names, ["mat a", "mat b", "mat c"]);
        assert!(ctx.scene.materials.iter().all(|m| m.shader_type == ShaderType::WavefrontMtl));
        assert_eq!(ctx.scene.meshes[0].face_material, [0, 1]);
        assert_eq!(ctx.scene.meshes[1].face_material, [0, NO_INDEX, 1, 2]);
        assert_eq!(mesh_materials(&ctx, 0), [0, 1]);
        assert_eq!(mesh_materials(&ctx, 1), [0, 1, 2]);
    }

    #[test]
    fn disjoint_meshes_keep_their_vertex_ranges() {
        let text = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 9 9 9\nv 2 0 0\nv 3 0 0\nv 3 1 0\nvn 0 0 1\nvn 0 1 0\no a\nf 1 2 3\no b\nf 5//2 6//2 7//2\n";
        let ctx = load(text, LoadOpts::default());
        let (a, b) = (&ctx.scene.meshes[0], &ctx.scene.meshes[1]);
        assert_eq!(a.vertex_position.values.len(), 4);
        assert_eq!(b.vertex_position.values, [Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 0.0)]);
        assert_eq!(b.vertex_position.indices, [0, 1, 2]);
        assert!(!a.vertex_normal.exists);
        assert_eq!(b.vertex_normal.values, [Vec3::new(0.0, 1.0, 0.0)]);

        let text = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nvt 1 1\no a\nf 1/1 2/2 3/1\no b\nf 1/2 3/1 4/2\n";
        let ctx = load(text, LoadOpts::default());
        assert_eq!(ctx.scene.meshes[1].vertex_position.values.len(), 4);
        assert_eq!(ctx.scene.meshes[1].vertex_position.indices, [0, 2, 3]);
        assert_eq!(ctx.scene.meshes[1].vertex_uv.indices, [1, 0, 1]);
    }

    #[test]
    fn mtl_lines_become_properties_and_textures() {
        let text = "Ka 0.1 0.2 0.3\nnewmtl first\nKd 0.5 0.5 0.5\nKd 0.25 0.25 0.25\nmap_Kd -blendu off -imfchan r -mm 0 1 diffuse.png\nnewmtl second\nillum on\nTf 1 1 1 1 1\nnorm normal map.png\n";
        let mut ctx = test_context(text.as_bytes(), LoadOpts::default());
        mtl_load(&mut ctx).unwrap();
        let first = &ctx.scene.materials[0].element.props;
        assert_eq!(first.props.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["Ka", "Kd"]);
        let kd = first.find("Kd").unwrap();
        assert_eq!((kd.value_vec3(), kd.flags, kd.value_str.as_str()), (Vec3::new(0.25, 0.25, 0.25), prop_flags::VALUE_VEC3, "0.25 0.25 0.25"));
        let second = &ctx.scene.materials[1].element.props;
        let illum = second.find("illum").unwrap();
        assert_eq!((illum.value_int, illum.value_real(), illum.flags), (1, 1.0, prop_flags::VALUE_STR | prop_flags::VALUE_INT));
        assert_eq!(second.find("Tf").unwrap().flags, prop_flags::VALUE_VEC4);

        let t = &ctx.scene.textures[0];
        assert_eq!(t.relative_filename, "diffuse.png");
        assert_eq!(t.element.props.find("mm").unwrap().value_vec2(), Vec2::new(0.0, 1.0));
        assert_eq!(t.element.props.find("blendu").unwrap().flags, prop_flags::VALUE_STR | prop_flags::VALUE_INT);
        assert_eq!(t.element.props.find("obj|args").unwrap().value_str, "-blendu off -imfchan r -mm 0 1 diffuse.png");
        assert_eq!(ctx.scene.textures[1].relative_filename, "normal map.png");
        assert_eq!(sources(&ctx, ctx.scene.materials[0].element.element_id), [(ctx.scene.textures[0].element.element_id, "Kd".to_string())]);
        assert_eq!(sources(&ctx, ctx.scene.materials[1].element.element_id), [(ctx.scene.textures[1].element.element_id, "norm".to_string())]);
    }

    #[test]
    fn the_mtl_comes_from_the_options_or_fails_where_missing() {
        let obj = "mtllib missing.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl red\nf 1 2 3\n";
        let opts = LoadOpts { obj_mtl_data: b"newmtl red\nKd 1 0 0\n".to_vec(), ..Default::default() };
        let ctx = load(obj, opts);
        assert_eq!(ctx.scene.materials[0].element.props.find("Kd").unwrap().value_vec3(), Vec3::new(1.0, 0.0, 0.0));

        let opts = LoadOpts { load_external_files: true, filename: "/nonexistent/dir/a.obj".into(), ..Default::default() };
        let mut ctx = test_context(obj.as_bytes(), opts);
        let err = obj_load(&mut ctx).unwrap_err();
        assert_eq!((err.description.as_str(), err.info.as_str()), ("External file not found", "missing.mtl"));
        assert_eq!(warnings(&ctx)[0].1, "Could not open .mtl file: /nonexistent/dir/missing.mtl");

        let opts = LoadOpts { load_external_files: true, ignore_missing_external_files: true, ..Default::default() };
        load(obj, opts);

        let mut ctx = test_context(b"v 0 0 0\nv 1 x 0\n", LoadOpts::default());
        assert!(obj_load(&mut ctx).is_err());
        let mut ctx = test_context(b"vt 0 0\nf 1/1 2/1 3/1\n", LoadOpts::default());
        assert!(obj_load(&mut ctx).is_err());
    }
}
