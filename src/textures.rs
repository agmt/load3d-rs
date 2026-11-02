use std::collections::HashMap;

use crate::context::{Context, FileContent};
use crate::finalize::{fetch_dst_element, typed_index};
use crate::paths::resolve_relative_filename;
use crate::search::{lower_bound_eq, stable_sort};
use crate::types::*;

/// `ufbx_find_shader_texture_input` in ufbx.c, returning the index; the inputs stay in property order, as ufbx keeps them
pub(crate) fn input_index(shader: &ShaderTexture, name: &str) -> Option<usize> {
    let inputs = &shader.inputs;
    lower_bound_eq(0, inputs.len(), 4, |i| inputs[i].name.as_bytes() < name.as_bytes(), |i| inputs[i].name == name)
}

/// `ufbxi_remove_suffix_c` and `ufbxi_remove_suffix_len` in ufbx.c
fn remove_suffix(s: &mut &str, suffix: &str) -> bool {
    match s.strip_suffix(suffix) {
        Some(t) => {
            *s = t;
            true
        }
        None => false,
    }
}

fn push_prop_prefix(prefix: &str) -> String {
    if !prefix.is_empty() && !prefix.ends_with('|') { format!("{prefix}|") } else { prefix.to_string() }
}

fn shader_texture_find_prefix(props: &Props, shader: &mut ShaderTexture) {
    let mut suffixes: Vec<String> = vec![" Parameters/Connections".to_string()];
    if !shader.shader_name.is_empty() {
        suffixes.push(shader.shader_name.clone());
    }
    suffixes.push("3dsMax|parameters".to_string());

    for suffix in &suffixes {
        for prop in &props.props {
            if prop.prop_type != PropType::Compound {
                continue;
            }
            if prop.name.as_bytes().ends_with(suffix.as_bytes()) {
                shader.prop_prefix = push_prop_prefix(&prop.name);
                return;
            }
        }
    }
    for suffix in &suffixes {
        for prop in &props.props {
            let bytes = prop.name.as_bytes();
            let mut len = bytes.len();
            while len > 0 && bytes[len - 1] != b'|' {
                len -= 1;
            }
            if len <= 1 {
                continue;
            }
            let name = &bytes[..len - 1];
            if name.ends_with(suffix.as_bytes()) {
                shader.prop_prefix = push_prop_prefix(&String::from_utf8_lossy(name));
                return;
            }
        }
    }
}

const FILE_SHADERS: [(u64, &str, &str); 6] = [
    (0x7e73161fad53b12a, "ai_image", "filename"),
    (0, "OSLBitmap", "Filename"),
    (0, "OSLBitmap2", "Filename"),
    (0, "OSLBitmap3", "Filename"),
    (0, "UberBitmap", "Filename"),
    (0, "UberBitmap2", "Filename"),
];

pub(crate) fn update_shader_texture(scene: &Scene, texture: usize, shader: &mut ShaderTexture) {
    let t = &scene.textures[texture];
    let props = &t.element.props;
    let id = t.element.element_id;
    for input in &mut shader.inputs {
        if !input.prop.is_empty()
            && let Some(prop) = props.find(&input.prop)
        {
            input.value_vec4 = prop.value_vec4();
            input.value_int = prop.value_int;
            input.value_str = prop.value_str.clone();
            input.value_blob = prop.value_blob.clone();
            input.texture = fetch_dst_element(scene, id, false, &prop.name, ElementType::Texture).map(|e| typed_index(scene, e));
        }
        if let Some(name) = &input.texture_prop
            && let Some(prop) = props.find(name)
            && let Some(tex) = fetch_dst_element(scene, id, false, &prop.name, ElementType::Texture)
        {
            input.texture = Some(typed_index(scene, tex));
        }
        input.texture_enabled = input.texture.is_some();
        if let Some(name) = &input.texture_enabled_prop
            && let Some(prop) = props.find(name)
        {
            input.texture_enabled = prop.value_int != 0;
        }
    }

    if shader.shader_type == ShaderTextureType::SelectOutput {
        let map = input_index(shader, "sourceMap");
        if let Some(index) = input_index(shader, "outputChannelIndex") {
            shader.main_texture_output_index = shader.inputs[index].value_int;
        }
        if let Some(map) = map {
            shader.main_texture = shader.inputs[map].texture;
            shader.inputs[map].texture_output_index = shader.main_texture_output_index;
        }
    }
}

/// `None` where they share no root
fn absolute_to_relative_path(rel: &[u8], src: &[u8]) -> Option<Vec<u8>> {
    if rel.is_empty() || src.is_empty() || rel[0] != src[0] {
        return None;
    }
    let mut rel_length = rel.len();
    while rel_length > 0 && rel[rel_length - 1] != b'/' && rel[rel_length - 1] != b'\\' {
        rel_length -= 1;
    }
    if rel_length == 0 {
        return None;
    }
    let separator = rel[rel_length - 1];
    // `ufbxi_next_path_segment` in ufbx.c
    let next = |d: &[u8], begin: usize, length: usize| (begin..length).find(|&i| d[i] == b'/' || d[i] == b'\\').unwrap_or(length);

    let (mut rel_begin, mut src_begin) = (0usize, 0usize);
    while rel_begin < rel_length && src_begin < src.len() {
        let rel_end = next(rel, rel_begin, rel_length);
        let src_end = next(src, src_begin, src.len());
        if rel_end != src_end || rel[rel_begin..rel_end] != src[src_begin..src_end] {
            break;
        }
        rel_begin = rel_end + 1;
        src_begin = src_end + 1;
    }

    let mut out = Vec::new();
    while rel_begin < rel_length {
        let rel_end = next(rel, rel_begin, rel_length);
        out.extend_from_slice(&[b'.', b'.', separator]);
        rel_begin = rel_end + 1;
    }
    while src_begin < src.len() {
        let src_end = next(src, src_begin, src.len());
        out.extend_from_slice(&src[src_begin..src_end]);
        if src_end < src.len() {
            out.push(separator);
        }
        src_begin = src_end + 1;
    }
    Some(out)
}

impl Context<'_> {
    fn separator(&self) -> u8 {
        self.opts.path_separator
    }

    /// Part of `ufbxi_resolve_filenames` in ufbx.c: (filename, absolute, relative)
    pub(crate) fn resolve_filenames_str(&self, absolute: &str, relative: &str) -> (String, String, String) {
        let mut relative = relative.to_string();
        if relative.is_empty()
            && let Some(r) = absolute_to_relative_path(self.scene.metadata.original_file_path.as_bytes(), absolute.as_bytes())
        {
            relative = String::from_utf8_lossy(&r).into_owned();
        }
        let filename = resolve_relative_filename(self.separator(), self.scene.metadata.relative_root.as_bytes(), relative.as_bytes());
        (String::from_utf8_lossy(&filename).into_owned(), absolute.to_string(), relative)
    }

    /// Part of `ufbxi_resolve_filenames` in ufbx.c
    pub(crate) fn resolve_filenames_raw(&self, absolute: &[u8], relative: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let mut relative = relative.to_vec();
        if relative.is_empty()
            && let Some(r) = absolute_to_relative_path(&self.scene.metadata.raw_original_file_path, absolute)
        {
            relative = r;
        }
        let filename = resolve_relative_filename(self.separator(), &self.scene.metadata.raw_relative_root, &relative);
        (filename, absolute.to_vec(), relative)
    }

    pub(crate) fn resolve_file_content(&mut self) {
        let mut contents: Vec<FileContent> = Vec::new();
        for v in 0..self.scene.videos.len() {
            let video = &self.scene.videos[v];
            let (f, a, r) = self.resolve_filenames_str(&video.absolute_filename, &video.relative_filename);
            let (rf, ra, rr) = self.resolve_filenames_raw(&video.raw_absolute_filename, &video.raw_relative_filename);
            let video = &mut self.scene.videos[v];
            (video.filename, video.absolute_filename, video.relative_filename) = (f, a, r);
            (video.raw_filename, video.raw_absolute_filename, video.raw_relative_filename) = (rf, ra, rr);
            // `ufbxi_push_file_content` in ufbx.c
            if !video.content.is_empty() && !video.absolute_filename.is_empty() {
                contents.push(FileContent { absolute_filename: video.absolute_filename.clone(), content: video.content.clone() });
            }
        }
        for c in 0..self.scene.audio_clips.len() {
            let clip = &mut self.scene.audio_clips[c];
            let p = &clip.element.props;
            clip.absolute_filename = p.find_string("Path", "").to_string();
            clip.relative_filename = p.find_string("RelPath", "").to_string();
            clip.raw_absolute_filename = p.find_blob("Path", b"").to_vec();
            clip.raw_relative_filename = p.find_blob("RelPath", b"").to_vec();
            let clip = &self.scene.audio_clips[c];
            let (f, a, r) = self.resolve_filenames_str(&clip.absolute_filename, &clip.relative_filename);
            let (rf, ra, rr) = self.resolve_filenames_raw(&clip.raw_absolute_filename, &clip.raw_relative_filename);
            let clip = &mut self.scene.audio_clips[c];
            (clip.filename, clip.absolute_filename, clip.relative_filename) = (f, a, r);
            (clip.raw_filename, clip.raw_absolute_filename, clip.raw_relative_filename) = (rf, ra, rr);
            // `ufbxi_push_file_content` in ufbx.c
            if !clip.content.is_empty() && !clip.absolute_filename.is_empty() {
                contents.push(FileContent { absolute_filename: clip.absolute_filename.clone(), content: clip.content.clone() });
            }
        }
        // `ufbxi_file_content_less` and `ufbxi_sort_file_contents` in ufbx.c
        stable_sort(&mut contents, 32, |a, b| a.absolute_filename.as_bytes() < b.absolute_filename.as_bytes());

        // `ufbxi_fetch_file_content` in ufbx.c
        let fetch = |name: &str, content: &mut Vec<u8>| {
            if !content.is_empty() {
                return;
            }
            if let Some(i) =
                lower_bound_eq(0, contents.len(), 8, |i| contents[i].absolute_filename.as_bytes() < name.as_bytes(), |i| contents[i].absolute_filename == name)
            {
                *content = contents[i].content.clone();
            }
        };
        for video in &mut self.scene.videos {
            fetch(&video.absolute_filename.clone(), &mut video.content);
        }
        for clip in &mut self.scene.audio_clips {
            fetch(&clip.absolute_filename.clone(), &mut clip.content);
        }
        self.file_content = contents;
    }

    fn finalize_shader_texture(&mut self, texture: usize) {
        let t = &self.scene.textures[texture];
        let props = &t.element.props;
        let classid_a = props.find_int("3dsMax|ClassIDa", 0) as u64 as u32;
        let classid_b = props.find_int("3dsMax|ClassIDb", 0) as u64 as u32;
        let classid = (classid_a as u64) << 32 | classid_b as u64;
        let max_texture = props.find_string("3dsMax|MaxTexture", "").to_string();

        let shader_type = if max_texture == "MULTIOUTPUT_TO_OSLMap" || classid == 0x896ef2fc44bd743f {
            ShaderTextureType::SelectOutput
        } else if max_texture == "OSLMap" || classid == 0x7f9a7b9d6fcdf00d {
            ShaderTextureType::Osl
        } else if t.texture_type == TextureType::File && t.relative_filename.is_empty() && t.absolute_filename.is_empty() && t.video.is_none() {
            ShaderTextureType::Unknown
        } else {
            return;
        };

        let mut shader = ShaderTexture { shader_type, ..Default::default() };
        if let Some(p) = props.find("3dsMax|params|OSLShaderName") {
            shader.shader_name = p.value_str.clone();
        }
        if let Some(p) = props.find("3dsMax|params|OSLCode") {
            shader.shader_source = p.value_str.clone();
            shader.raw_shader_source = p.value_blob.clone();
        }
        shader_texture_find_prefix(props, &mut shader);

        if shader.shader_name.is_empty() {
            let mut name = shader.prop_prefix.as_str();
            if remove_suffix(&mut name, " Parameters/Connections|") {
                let begin = name.rfind('|').map_or(0, |i| i + 1);
                shader.shader_name = name[begin..].to_string();
            }
        }
        if shader.shader_name.is_empty() && !max_texture.is_empty() {
            shader.shader_name = max_texture.clone();
        }
        if classid != 0 {
            shader.shader_type_id = classid;
        }
        if shader.prop_prefix.is_empty() {
            return;
        }

        for prop in &props.props {
            let Some(name) = prop.name.strip_prefix(shader.prop_prefix.as_str()) else { continue };
            let mut base_name = name;
            if remove_suffix(&mut base_name, "_map") || remove_suffix(&mut base_name, ".shader") {
                if let Some(base) = input_index(&shader, base_name) {
                    shader.inputs[base].texture_prop = Some(prop.name.clone());
                    continue;
                }
            } else if (remove_suffix(&mut base_name, ".connected") || remove_suffix(&mut base_name, "Enabled"))
                && let Some(base) = input_index(&shader, base_name)
            {
                shader.inputs[base].texture_enabled_prop = Some(prop.name.clone());
                continue;
            }
            shader.inputs.push(ShaderTextureInput { name: name.to_string(), prop: prop.name.clone(), ..Default::default() });
        }

        let disable_quirks = self.opts.disable_quirks;
        self.scene.metadata.num_shader_textures += 1;
        let mut file: Option<(String, Vec<u8>)> = None;
        if !disable_quirks {
            for (id, name, input_name) in FILE_SHADERS {
                if ((id != 0 && shader.shader_type_id == id) || shader.shader_name == name)
                    && let Some(input) = input_index(&shader, input_name)
                {
                    if let Some(p) = props.find(&shader.inputs[input].prop) {
                        file = Some((p.value_str.clone(), p.value_blob.clone()));
                    }
                    break;
                }
            }
        }
        update_shader_texture(&self.scene, texture, &mut shader);
        let t = &mut self.scene.textures[texture];
        t.texture_type = TextureType::Shader;
        if let Some((abs, raw_abs)) = file {
            t.absolute_filename = abs;
            t.raw_absolute_filename = raw_abs;
            t.texture_type = TextureType::File;
        }
        t.shader = Some(Box::new(shader));
    }

    /// Part of `ufbxi_finalize_scene` in ufbx.c: the loop over textures, with `ufbxi_insert_texture_file`
    pub(crate) fn finalize_textures(&mut self) -> Vec<TextureFile> {
        let mut files: Vec<TextureFile> = Vec::new();
        let mut keys: HashMap<(bool, Vec<u8>), usize> = HashMap::new();
        for t in 0..self.scene.textures.len() {
            let id = self.scene.textures[t].element.element_id;
            let uv_set = self.scene.textures[t].element.props.find_prop("UVSet").map(|p| p.value_str.clone()).unwrap_or_default();
            let video = fetch_dst_element(&self.scene, id, false, "", ElementType::Video).map(|e| typed_index(&self.scene, e));
            {
                let content = video.map(|v| self.scene.videos[v].content.clone());
                let tex = &mut self.scene.textures[t];
                tex.uv_set = uv_set;
                tex.video = video;
                if let Some(c) = content {
                    tex.content = c;
                }
            }
            self.finalize_shader_texture(t);

            let tex = &self.scene.textures[t];
            let (f, a, r) = self.resolve_filenames_str(&tex.absolute_filename, &tex.relative_filename);
            let (rf, ra, rr) = self.resolve_filenames_raw(&tex.raw_absolute_filename, &tex.raw_relative_filename);
            let layered = tex.texture_type == TextureType::Layered;
            let layers = if layered { crate::finalize::fetch_texture_layers(&self.scene, id) } else { Vec::new() };
            let extra = self.get_element_extra(id).cloned();
            let tex = &mut self.scene.textures[t];
            (tex.filename, tex.absolute_filename, tex.relative_filename) = (f, a, r);
            (tex.raw_filename, tex.raw_absolute_filename, tex.raw_relative_filename) = (rf, ra, rr);
            if layered {
                tex.layers = layers;
                if let Some(extra) = extra {
                    if let Some(alphas) = &extra.alphas {
                        for i in 0..alphas.len().min(tex.layers.len()) {
                            tex.layers[i].alpha = alphas[i];
                        }
                    }
                    if let Some(modes) = &extra.blend_modes {
                        for i in 0..modes.len().min(tex.layers.len()) {
                            let mode = modes[i];
                            if mode >= 0
                                && mode < BlendMode::Overlay as i32
                                && let Some(m) = BlendMode::from_index(mode as i64)
                            {
                                tex.layers[i].blend_mode = m;
                            }
                        }
                    }
                }
            }

            tex.file_index = NO_INDEX;
            let key = if !tex.raw_absolute_filename.is_empty() {
                Some((false, tex.raw_absolute_filename.clone()))
            } else if !tex.raw_relative_filename.is_empty() {
                Some((true, tex.raw_relative_filename.clone()))
            } else {
                None
            };
            let Some(key) = key else { continue };
            let index = *keys.entry(key).or_insert_with(|| {
                files.push(TextureFile { index: files.len() as u32, ..Default::default() });
                files.len() - 1
            });
            let file = &mut files[index];
            tex.file_index = file.index;
            tex.has_file = true;
            if file.filename.is_empty() {
                file.filename = tex.filename.clone();
            }
            if file.relative_filename.is_empty() {
                file.relative_filename = tex.relative_filename.clone();
            }
            if file.absolute_filename.is_empty() {
                file.absolute_filename = tex.absolute_filename.clone();
            }
            if file.raw_filename.is_empty() {
                file.raw_filename = tex.raw_filename.clone();
            }
            if file.raw_relative_filename.is_empty() {
                file.raw_relative_filename = tex.raw_relative_filename.clone();
            }
            if file.raw_absolute_filename.is_empty() {
                file.raw_absolute_filename = tex.raw_absolute_filename.clone();
            }
            if file.content.is_empty() {
                file.content = tex.content.clone();
            }
        }
        files
    }
}

pub(crate) fn propagate_main_textures(scene: &mut Scene) {
    {
        let textures = &mut scene.textures;
        let mut mask = scene.metadata.num_shader_textures;
        let main_of = |textures: &Vec<Texture>, t: usize| textures[t].shader.as_ref().and_then(|s| s.main_texture.map(|m| (m, s.main_texture_output_index)));
        while mask != 0 {
            mask >>= 1;
            for t in 0..textures.len() {
                let Some(shader) = &textures[t].shader else { continue };
                let Some(main_tex) = shader.main_texture else { continue };
                if shader.main_texture_output_index != 0 {
                    continue;
                }
                let Some((m, idx)) = main_of(textures, main_tex) else { continue };
                let shader = textures[t].shader.as_mut().unwrap();
                shader.main_texture = Some(m);
                shader.main_texture_output_index = idx;
            }
        }
        for t in 0..textures.len() {
            let Some(shader) = &textures[t].shader else { continue };
            let Some(main_tex) = shader.main_texture else { continue };
            if shader.main_texture_output_index != 0 {
                continue;
            }
            if main_of(textures, main_tex).is_some() {
                textures[t].shader.as_mut().unwrap().main_texture = None;
            }
        }
        for t in 0..textures.len() {
            let Some(shader) = &textures[t].shader else { continue };
            let mut updates = Vec::new();
            for (i, input) in shader.inputs.iter().enumerate() {
                if let Some(it) = input.texture
                    && let Some((m, idx)) = main_of(textures, it)
                {
                    updates.push((i, m, idx));
                }
            }
            let shader = textures[t].shader.as_mut().unwrap();
            for (i, m, idx) in updates {
                shader.inputs[i].texture = Some(m);
                shader.inputs[i].texture_output_index = idx;
            }
        }
        for mat in &mut scene.materials {
            for tex in &mut mat.textures {
                if let Some(shader) = &textures[tex.texture].shader
                    && let Some(m) = shader.main_texture
                    && shader.main_texture_output_index == 0
                {
                    tex.texture = m;
                }
            }
        }
    }
}

impl Context<'_> {
    pub(crate) fn fetch_file_textures(&mut self) {
        const STARTED: u8 = 1;
        const FINISHED: u8 = 2;
        let textures = &mut self.scene.textures;
        let mut stack: Vec<usize> = (0..textures.len()).collect();
        let mut states = vec![0u8; textures.len()];

        // `ufbxi_deduplicate_textures` in ufbx.c
        let dedup = |list: Vec<usize>| -> Vec<usize> {
            let mut ordered: Vec<(usize, usize)> = list.into_iter().enumerate().map(|(o, t)| (t, o)).collect();
            // `ufbxi_ordered_texture_less_texture` in ufbx.c
            stable_sort(&mut ordered, 16, |a, b| a.0 < b.0);
            ordered.dedup_by(|b, a| a.0 == b.0);
            // `ufbxi_ordered_texture_less_order` in ufbx.c
            stable_sort(&mut ordered, 16, |a, b| a.1 < b.1);
            ordered.into_iter().map(|(t, _)| t).collect()
        };

        while let Some(t) = stack.pop() {
            let state = states[textures[t].element.typed_id as usize];
            if state == FINISHED {
                continue;
            }
            let typed_id = textures[t].element.typed_id as usize;
            if state == STARTED {
                states[typed_id] = FINISHED;
                let tex = &textures[t];
                let mut deps = Vec::new();
                if tex.texture_type == TextureType::File {
                    deps.push(t);
                }
                for layer in &tex.layers {
                    if !textures[layer.texture].file_textures.is_empty() {
                        deps.push(layer.texture);
                    }
                }
                if let Some(shader) = &tex.shader {
                    for input in &shader.inputs {
                        if let Some(d) = input.texture
                            && !textures[d].file_textures.is_empty()
                        {
                            deps.push(d);
                        }
                    }
                }
                let deps = dedup(deps);
                let files = if deps.len() == 1 {
                    textures[deps[0]].file_textures.clone()
                } else {
                    let mut all = Vec::new();
                    for &d in &deps {
                        all.extend_from_slice(&textures[d].file_textures);
                    }
                    dedup(all)
                };
                textures[t].file_textures = files;
            } else {
                if textures[t].texture_type == TextureType::File {
                    textures[t].file_textures = vec![t];
                    if textures[t].shader.is_none() {
                        states[typed_id] = FINISHED;
                        continue;
                    }
                }
                states[typed_id] = STARTED;
                stack.push(t);
                for layer in &textures[t].layers {
                    stack.push(layer.texture);
                }
                if let Some(shader) = &textures[t].shader {
                    for input in &shader.inputs {
                        if let Some(d) = input.texture {
                            stack.push(d);
                        }
                    }
                }
            }
        }
    }
}
