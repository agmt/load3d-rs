use super::*;
use crate::error::{fail, is_utf8_valid};
use crate::inflate::{InflateInput, inflate};

pub(crate) const MAX_NON_ARRAY_VALUES: usize = 8;

/// The name as the readers take it, and its bytes where ufbx would find them no valid UTF-8
fn node_name(bytes: &[u8]) -> (Cow<'_, str>, Option<Cow<'_, [u8]>>) {
    match std::str::from_utf8(bytes) {
        Ok(s) => (Cow::Borrowed(s), s.contains('\0').then_some(Cow::Borrowed(bytes))),
        Err(_) => (String::from_utf8_lossy(bytes), Some(Cow::Borrowed(bytes))),
    }
}
pub(crate) const MAX_NODE_DEPTH: u32 = 32;
const BINARY_HEADER_SIZE: usize = 27;
const MAX_SKIP_SIZE: u64 = 0x4000_0000;
impl<'a> PNode<'a> {
    pub fn find_child(&self, name: &str) -> Option<&PNode<'a>> {
        self.children.iter().find(|c| c.name == name)
    }

    pub fn vals(&self) -> &[PVal<'a>] {
        match &self.data {
            PData::Vals(v) => v,
            PData::Arr(_) => &[],
        }
    }

    pub fn get_array(&self) -> Option<&PArr<'a>> {
        match &self.data {
            PData::Arr(a) => Some(a),
            PData::Vals(_) => None,
        }
    }

    /// Part of `ufbxi_get_val_at` in ufbx.c
    pub fn num(&self, ix: usize) -> Option<(f64, i64)> {
        match self.vals().get(ix)? {
            PVal::Num { f, i } => Some((*f, *i)),
            _ => None,
        }
    }

    /// Part of `ufbxi_get_val_at` in ufbx.c
    pub fn i32_at(&self, ix: usize) -> Option<i32> {
        self.num(ix).map(|(_, i)| i as i32)
    }
    /// Part of `ufbxi_get_val_at` in ufbx.c
    pub fn i64_at(&self, ix: usize) -> Option<i64> {
        self.num(ix).map(|(_, i)| i)
    }
    /// Part of `ufbxi_get_val_at` in ufbx.c
    pub fn f64_at(&self, ix: usize) -> Option<f64> {
        self.num(ix).map(|(f, _)| f)
    }
    /// Part of `ufbxi_get_val_at` in ufbx.c
    pub fn bool_at(&self, ix: usize) -> Option<bool> {
        self.num(ix).map(|(_, i)| i != 0)
    }
    /// Part of `ufbxi_get_val_at` in ufbx.c
    pub fn size_at(&self, ix: usize) -> Option<usize> {
        let (_, i) = self.num(ix)?;
        if i < 0 { None } else { usize::try_from(i).ok() }
    }

    /// Part of `ufbxi_get_val_at` in ufbx.c
    fn sstr(&self, ix: usize) -> Option<&SStr<'a>> {
        match self.vals().get(ix)? {
            PVal::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Part of `ufbxi_get_val_at` in ufbx.c: `None` for strings the parser kept raw
    pub fn str_at(&self, ix: usize) -> Option<&str> {
        self.sstr(ix)?.utf8()
    }
    /// Part of `ufbxi_get_val_at` in ufbx.c
    pub fn raw_at(&self, ix: usize) -> Option<&[u8]> {
        Some(self.sstr(ix)?.raw())
    }

    /// Part of `ufbxi_find_val1` in ufbx.c
    pub fn find_i32(&self, name: &str) -> Option<i32> {
        self.find_child(name)?.i32_at(0)
    }
    /// Part of `ufbxi_find_val1` in ufbx.c
    pub fn find_i64(&self, name: &str) -> Option<i64> {
        self.find_child(name)?.i64_at(0)
    }
    /// Part of `ufbxi_find_val1` in ufbx.c
    pub fn find_f64(&self, name: &str) -> Option<f64> {
        self.find_child(name)?.f64_at(0)
    }
    /// Part of `ufbxi_find_val1` in ufbx.c
    pub fn find_str(&self, name: &str) -> Option<&str> {
        self.find_child(name)?.str_at(0)
    }
    pub fn find_array(&self, name: &str) -> Option<&PArr<'a>> {
        self.find_child(name)?.get_array()
    }
}
pub(crate) fn update_parse_state(parent: State, name: &str) -> State {
    use State::*;
    match parent {
        Root => match name {
            "FBXHeaderExtension" => FbxHeaderExtension,
            "Definitions" => Definitions,
            "Objects" => Objects,
            "Connections" => Connections,
            "Takes" => Takes,
            "Model" => LegacyModel,
            "References" => References,
            "Relations" => Relations,
            "Media" => LegacyMedia,
            "Switcher" => LegacySwitcher,
            "SceneGenericPersistence" => LegacyScenePersistence,
            _ => Unknown,
        },
        FbxHeaderExtension => match name {
            "FBXVersion" => FbxVersion,
            "SceneInfo" => SceneInfo,
            _ => Unknown,
        },
        SceneInfo => {
            if name == "Thumbnail" {
                Thumbnail
            } else {
                Unknown
            }
        }
        Objects => match name {
            "Model" => Model,
            "Geometry" => Geometry,
            "NodeAttribute" => NodeAttribute,
            "AnimationCurve" => AnimationCurve,
            "Deformer" => Deformer,
            "Pose" => Pose,
            "Texture" => Texture,
            "Video" => Video,
            "LayeredTexture" => LayeredTexture,
            "SelectionNode" => SelectionNode,
            "Collection" => Collection,
            "Audio" => Audio,
            _ => UnknownObject,
        },
        Model | Geometry => {
            if name.starts_with('L') {
                match name {
                    "LayerElementNormal" => return LayerElementNormal,
                    "LayerElementBinormal" => return LayerElementBinormal,
                    "LayerElementTangent" => return LayerElementTangent,
                    "LayerElementUV" => return LayerElementUv,
                    "LayerElementColor" => return LayerElementColor,
                    "LayerElementVertexCrease" => return LayerElementVertexCrease,
                    "LayerElementEdgeCrease" => return LayerElementEdgeCrease,
                    "LayerElementSmoothing" => return LayerElementSmoothing,
                    "LayerElementVisibility" => return LayerElementVisibility,
                    "LayerElementPolygonGroup" => return LayerElementPolygonGroup,
                    "LayerElementHole" => return LayerElementHole,
                    "LayerElementMaterial" => return LayerElementMaterial,
                    _ => {}
                }
                if name.starts_with("LayerElement") {
                    return LayerElementOther;
                }
            }
            if name == "Shape" { Shape } else { Unknown }
        }
        Deformer => {
            if name == "AssociateModel" {
                AssociateModel
            } else {
                Unknown
            }
        }
        LegacyMedia => {
            if name == "Video" {
                LegacyVideo
            } else {
                Unknown
            }
        }
        LegacyVideo => Video,
        LegacyModel => match name {
            "GeometryUVInfo" => GeometryUvInfo,
            "Link" => LegacyLink,
            "Channel" => Channel,
            "Shape" => Shape,
            _ => Unknown,
        },
        Pose => {
            if name == "PoseNode" {
                PoseNode
            } else {
                Unknown
            }
        }
        Takes => {
            if name == "Take" {
                Take
            } else {
                Unknown
            }
        }
        Take => TakeObject,
        TakeObject | Channel => {
            if name == "Channel" {
                Channel
            } else {
                Unknown
            }
        }
        References => Reference,
        _ => Unknown,
    }
}
#[derive(Debug, Clone, Copy)]
struct ArrayInfo {
    /// b, c, i, l, f, d, r (real), s/S (strings), C (content) or '-' (ignore)
    kind: u8,
    // ufbx's table as it stands: where the array is kept and its leading zero, which owned arrays need not
    #[allow(dead_code)]
    result: bool,
    #[allow(dead_code)]
    pad_begin: bool,
    accurate_f32: bool,
    /// Beyond ufbx: the integer values kept beside the booleans (a mesh's smoothing groups)
    keep_ints: bool,
}
pub(crate) fn f64_to_i32(v: f64) -> i32 {
    if v.abs() <= i32::MAX as f64 {
        v as i32
    } else if v >= 0.0 {
        i32::MAX
    } else {
        i32::MIN
    }
}
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\r' | b'\n')
}

impl<'a> Parser<'a> {
    /// No later `FBXVersion` changes the version
    pub fn set_found_version(&mut self) {
        self.ascii.found_version = true;
    }
    fn push_sanitized_string(&mut self, raw: &'a [u8], keep_raw: bool) -> Res<SStr<'a>> {
        if keep_raw && !raw.is_empty() {
            return Ok(SStr::raw_only(raw));
        }
        if let Ok(text) = std::str::from_utf8(raw)
            && !raw.contains(&0)
        {
            return Ok(SStr::text(text));
        }
        self.repair_string(Cow::Borrowed(raw))
    }
    /// Part of `ufbxi_push_sanitized_string` in ufbx.c: for bytes the parser owns
    fn push_sanitized_string_owned(&mut self, raw: Vec<u8>, keep_raw: bool) -> Res<SStr<'a>> {
        if keep_raw && !raw.is_empty() {
            return Ok(SStr::raw_only(raw));
        }
        match String::from_utf8(raw) {
            Ok(text) if !text.as_bytes().contains(&0) => Ok(SStr::text(text)),
            Ok(text) => self.repair_string(Cow::Owned(text.into_bytes())),
            Err(e) => self.repair_string(Cow::Owned(e.into_bytes())),
        }
    }
    /// Part of `ufbxi_push_sanitized_string` in ufbx.c
    fn repair_string(&mut self, raw: Cow<'a, [u8]>) -> Res<SStr<'a>> {
        let utf8 = push_sanitized_string_entry(&raw, utf8_valid_length(&raw), self.opts.unicode_error_handling, &mut self.warnings)?;
        Ok(SStr::new(raw, utf8))
    }
    /// A string token as `push_sanitized_string` takes it: from the input where it is the input's bytes
    fn push_ascii_string(&mut self, text: &[u8], keep_raw: bool) -> Res<SStr<'a>> {
        match self.ascii_in_input(text) {
            Some(raw) => self.push_sanitized_string(raw, keep_raw),
            None => self.push_sanitized_string_owned(text.to_vec(), keep_raw),
        }
    }
    /// The previous token's `text` in the input, unless an entity of it was replaced
    fn ascii_in_input(&self, text: &[u8]) -> Option<&'a [u8]> {
        let input: &'a [u8] = self.data;
        let (start, end) = self.ascii.prev.span;
        if self.ascii.prev.replaced {
            return None;
        }
        debug_assert_eq!(&input[start..end], text);
        Some(&input[start..end])
    }
    fn byte(&self, at: usize) -> u8 {
        self.data.get(at).copied().unwrap_or(0)
    }
    fn window(&self, from: usize, to: usize) -> &'a [u8] {
        let to = to.min(self.data.len());
        &self.data[from.min(to)..to]
    }
    /// `ufbxi_yield` in ufbx.c
    #[cold]
    fn yield_bytes(&mut self, size: usize) -> Res<()> {
        self.io.data_size += self.io.yield_size;
        let refilled = if self.io.data_size >= size { Ok(()) } else { self.refill(size, true) };
        self.io.yield_size = self.io.data_size.min(size.max(self.io.progress_interval));
        self.io.data_size -= self.io.yield_size;
        let reported = self.report_progress();
        match (refilled, reported) {
            (Err(e), Err(r)) => Err(first_described(e, r)),
            (Err(e), Ok(())) | (Ok(()), Err(e)) => Err(e),
            (Ok(()), Ok(())) => Ok(()),
        }
    }
    fn peek_bytes(&mut self, size: usize) -> Res<&'a [u8]> {
        if self.io.yield_size < size {
            self.yield_bytes(size)?;
        }
        Ok(&self.data[self.pos..self.pos + size])
    }
    fn read_bytes(&mut self, size: usize) -> Res<&'a [u8]> {
        let s = self.peek_bytes(size)?;
        self.consume_bytes(size);
        Ok(s)
    }
    fn consume_bytes(&mut self, size: usize) {
        self.io.yield_size -= size;
        self.pos += size;
    }
    fn skip_bytes(&mut self, size: u64) -> Res<()> {
        if self.io.skip_fn {
            self.pause_progress();
            if size > self.io.data_size as u64 {
                let mut rest = size - self.io.data_size as u64;
                // past the buffer the stream itself moves on, the read offset with it
                let far = usize::try_from(rest).ok().and_then(|r| self.pos.checked_add(self.io.data_size)?.checked_add(r));
                let Some(far) = far else { return fail() };
                self.pos += self.io.data_size;
                self.io.data_size = 0;
                self.io.data_offset += rest;
                self.io.buf_begin += rest as usize;
                self.pos = far;
                while rest >= MAX_SKIP_SIZE {
                    rest -= MAX_SKIP_SIZE;
                    self.io.stream_pos = self.io.stream_pos.saturating_add((MAX_SKIP_SIZE - 1) as usize);
                    // the one byte ufbx reads after each long skip, as `fseek` does not tell of the end
                    check!(self.stream_read(1)? == 1, "Truncated file");
                }
                self.io.stream_pos = self.io.stream_pos.saturating_add(rest as usize);
            } else {
                self.pos += size as usize;
                self.io.data_size -= size as usize;
            }
            self.resume_progress()
        } else {
            let step = self.io.read_buffer_size.max(self.io.opts_read_buffer_size) as u64;
            let mut size = size;
            while size > 0 {
                let n = size.min(step);
                self.read_bytes(n as usize)?;
                size -= n;
            }
            Ok(())
        }
    }
    fn read_to(&mut self, size: usize) -> Res<&'a [u8]> {
        let start = self.pos;
        self.pause_progress();
        let len = self.io.data_size.min(size);
        self.pos += len;
        self.io.data_size -= len;
        let mut left = size - len;
        if left > 0 {
            self.io.data_offset = self.get_read_offset();
            self.io.buf_begin = self.pos;
            self.io.data_size = 0;
            check!(self.io.stream);
            while left > 0 {
                let read = self.stream_read(left)?;
                check!(read != 0);
                left -= read;
                self.pos += read;
                self.io.buf_begin += read;
                self.io.data_offset += read as u64;
            }
        }
        self.resume_progress()?;
        Ok(&self.data[start..start + size])
    }
    /// With `ufbxi_swap_endian` in ufbx.c
    fn read_u32(&self, b: &[u8]) -> u32 {
        let a: [u8; 4] = b[..4].try_into().unwrap_or_default();
        if self.big_endian { u32::from_be_bytes(a) } else { u32::from_le_bytes(a) }
    }
    /// With `ufbxi_swap_endian` in ufbx.c
    fn read_u64(&self, b: &[u8]) -> u64 {
        let a: [u8; 8] = b[..8].try_into().unwrap_or_default();
        if self.big_endian { u64::from_be_bytes(a) } else { u64::from_le_bytes(a) }
    }
    /// With `ufbxi_swap_endian` in ufbx.c
    fn read_i16(&self, b: &[u8]) -> i16 {
        let a: [u8; 2] = b[..2].try_into().unwrap_or_default();
        if self.big_endian { i16::from_be_bytes(a) } else { i16::from_le_bytes(a) }
    }
    fn read_f32(&self, b: &[u8]) -> f32 {
        f32::from_bits(self.read_u32(b))
    }
    fn read_f64(&self, b: &[u8]) -> f64 {
        f64::from_bits(self.read_u64(b))
    }
    pub fn begin_parse(&mut self) -> Res<()> {
        let header = self.peek_bytes(BINARY_HEADER_SIZE)?;
        if &header[..22] == BINARY_MAGIC {
            self.big_endian = header[22] != 0;
            self.version = self.read_u32(&header[23..27]);
            self.sure_fbx = true;
            self.consume_bytes(BINARY_HEADER_SIZE);
        } else {
            self.from_ascii = true;
            self.ascii = Ascii::default();
            self.io.src_yield = self.pos + self.io.yield_size;
            self.io.src_end = self.pos + self.io.data_size + self.io.yield_size;
            self.ascii_next_token()?;
            if self.version > 0 {
                self.sure_fbx = true;
            } else {
                if !self.opts.strict {
                    self.version = 7400;
                }
                check!(self.version > 0, "Not an FBX file");
            }
        }
        Ok(())
    }
    fn is_array_node(&self, parent: State, name: &str) -> Option<ArrayInfo> {
        use State::*;
        let ig = self.opts.ignore_geometry;
        let ia = self.opts.ignore_animation;
        let dom = self.opts.retain_dom;
        let g = |kind: u8| if ig { b'-' } else { kind };
        let a = |kind: u8| if ia { b'-' } else { kind };
        let info = |kind: u8, result: bool, pad: bool| ArrayInfo { kind, result: result || dom, pad_begin: pad, accurate_f32: false, keep_ints: false };
        let tmp = |kind: u8| ArrayInfo { kind, result: dom, pad_begin: false, accurate_f32: false, keep_ints: false };
        let embedded = if self.opts.ignore_embedded { b'-' } else { b'C' };
        match parent {
            Thumbnail if name == "ImageData" => Some(ArrayInfo { kind: b'c', result: true, pad_begin: false, accurate_f32: false, keep_ints: false }),
            Geometry | Model => match name {
                "Vertices" => Some(info(g(b'r'), true, true)),
                "PolygonVertexIndex" => Some(info(g(b'i'), true, false)),
                "Edges" => Some(tmp(g(b'i'))),
                "Indexes" => Some(info(g(b'i'), true, false)),
                "Points" => Some(info(g(b'r'), true, false)),
                "KnotVector" | "KnotVectorU" | "KnotVectorV" => Some(info(g(b'r'), true, false)),
                "PointsIndex" => Some(info(g(b'i'), true, false)),
                "Normals" => Some(info(g(b'r'), true, true)),
                _ => None,
            },
            LegacyModel => match name {
                "Vertices" | "Normals" => Some(info(g(b'r'), true, true)),
                "Materials" | "PolygonVertexIndex" => Some(info(g(b'i'), true, false)),
                "Children" => Some(tmp(b's')),
                _ => None,
            },
            AnimationCurve => match name {
                "KeyTime" => Some(tmp(a(b'l'))),
                "KeyValueFloat" => Some(tmp(a(b'r'))),
                "KeyAttrFlags" | "KeyAttrRefCount" => Some(tmp(a(b'i'))),
                "KeyAttrDataFloat" => {
                    // ASCII 7200 and later write these as integers, as some of the elements aren't floats at all
                    let mut kind = if self.from_ascii && self.version >= 7200 { b'i' } else { b'f' };
                    if ia {
                        kind = b'-';
                    }
                    let mut info = tmp(kind);
                    info.accurate_f32 = self.from_ascii && self.version < 7200;
                    Some(info)
                }
                _ => None,
            },
            Texture if matches!(name, "ModelUVTranslation" | "ModelUVScaling" | "Cropping") => Some(tmp(if dom { b'r' } else { b'-' })),
            Video if name == "Content" => Some(tmp(embedded)),
            LayeredTexture if matches!(name, "BlendModes") => Some(tmp(b'i')),
            LayeredTexture if matches!(name, "Alphas") => Some(tmp(b'r')),
            SelectionNode if matches!(name, "VertexIndexArray" | "EdgeIndexArray" | "PolygonIndexArray") => {
                Some(ArrayInfo { kind: b'i', result: true, pad_begin: false, accurate_f32: false, keep_ints: false })
            }
            LayerElementNormal => match name {
                "Normals" => Some(info(g(b'r'), true, true)),
                "NormalsIndex" => Some(info(g(b'i'), true, false)),
                "NormalsW" => Some(info(if self.opts.retain_vertex_w { b'r' } else { b'-' }, true, true)),
                _ => None,
            },
            LayerElementBinormal => match name {
                "Binormals" => Some(info(g(b'r'), true, true)),
                "BinormalsIndex" => Some(info(g(b'i'), true, false)),
                "BinormalsW" => Some(info(if self.opts.retain_vertex_w { b'r' } else { b'-' }, true, true)),
                _ => None,
            },
            LayerElementTangent => match name {
                "Tangents" => Some(info(g(b'r'), true, true)),
                "TangentsIndex" => Some(info(g(b'i'), true, false)),
                "TangentsW" => Some(info(if self.opts.retain_vertex_w { b'r' } else { b'-' }, true, true)),
                _ => None,
            },
            LayerElementUv => match name {
                "UV" => Some(info(g(b'r'), true, true)),
                "UVIndex" => Some(info(g(b'i'), true, false)),
                _ => None,
            },
            LayerElementColor => match name {
                "Colors" => Some(info(g(b'r'), true, true)),
                "ColorIndex" => Some(info(g(b'i'), true, false)),
                _ => None,
            },
            LayerElementVertexCrease => match name {
                "VertexCrease" => Some(info(g(b'r'), true, true)),
                "VertexCreaseIndex" => Some(info(g(b'i'), true, false)),
                _ => None,
            },
            LayerElementEdgeCrease if name == "EdgeCrease" => Some(info(g(b'r'), true, false)),
            LayerElementSmoothing if name == "Smoothing" => Some(ArrayInfo { keep_ints: true, ..info(g(b'b'), true, false) }),
            LayerElementVisibility if name == "Visibility" => Some(info(g(b'b'), true, false)),
            LayerElementPolygonGroup if name == "PolygonGroup" => Some(info(g(b'i'), true, false)),
            LayerElementHole if name == "Hole" => Some(info(g(b'b'), true, false)),
            LayerElementMaterial if name == "Materials" => Some(info(g(b'i'), true, false)),
            LayerElementOther => match name {
                "TextureId" => Some(tmp(g(b'i'))),
                "UV" => Some(tmp(if dom { b'r' } else { b'-' })),
                "UVIndex" => Some(tmp(if dom { b'i' } else { b'-' })),
                _ => None,
            },
            GeometryUvInfo => match name {
                "TextureUV" => Some(info(g(b'r'), true, true)),
                "TextureUVVerticeIndex" => Some(info(g(b'i'), true, true)),
                _ => None,
            },
            Shape => match name {
                "Indexes" => Some(info(g(b'i'), true, false)),
                "Vertices" | "Normals" => Some(info(g(b'r'), true, true)),
                _ => None,
            },
            Deformer => match name {
                "Transform" | "TransformLink" => Some(tmp(b'r')),
                "Indexes" => Some(info(g(b'i'), true, false)),
                "Weights" | "BlendWeights" => Some(info(g(b'r'), true, false)),
                "FullWeights" => {
                    Some(ArrayInfo { kind: b'r', result: dom || self.blender_full_weights, pad_begin: false, accurate_f32: false, keep_ints: false })
                }
                "TransformAssociateModel" => Some(tmp(if dom { b'r' } else { b'-' })),
                _ => None,
            },
            AssociateModel if name == "Transform" => Some(tmp(if dom { b'r' } else { b'-' })),
            LegacyLink => match name {
                "Transform" | "TransformLink" => Some(tmp(b'r')),
                "Indexes" => Some(info(g(b'i'), true, false)),
                "Weights" => Some(info(g(b'r'), true, false)),
                _ => None,
            },
            PoseNode if name == "Matrix" => Some(tmp(b'r')),
            Channel if name == "Key" => Some(tmp(a(b'd'))),
            Audio if name == "Content" => Some(tmp(embedded)),
            _ if name == "BinaryData" && !matches!(parent, Thumbnail | Video | Audio) => Some(tmp(embedded)),
            _ => None,
        }
    }
    fn is_raw_string(&self, parent: State, name: &str) -> bool {
        use State::*;
        match parent {
            Root => name == "Model" || name == "FileId",
            FbxHeaderExtension => name == "SceneInfo",
            Objects => true,
            Connections | Relations => self.version < 7000,
            Model | Geometry | NodeAttribute | UnknownObject => name == "NodeAttributeName" || name == "Name",
            Video | Audio => name == "Content",
            Texture => name == "TextureName" || name == "Media",
            PoseNode | SelectionNode => name == "Node",
            Collection => name == "Member",
            LegacyModel => name == "Material" || name == "Link" || name == "Name",
            LegacySwitcher => name == "CameraIndexName",
            LegacyScenePersistence => name == "SceneInfo",
            Reference => name == "Object",
            Take => name == "Model",
            _ => false,
        }
    }
    /// `None` for the NULL record that ends a list of nodes
    fn binary_parse_node(&mut self, depth: u32, parent: State, recursive: bool) -> Res<Option<PNode<'a>>> {
        check!(depth < MAX_NODE_DEPTH);
        let wide = self.version >= 7500;
        let header = self.read_bytes(if wide { 25 } else { 13 })?;
        let (end_offset, num_values64, values_len, name_len) = if wide {
            (self.read_u64(&header[0..]), self.read_u64(&header[8..]), self.read_u64(&header[16..]), header[24])
        } else {
            (self.read_u32(&header[0..]) as u64, self.read_u32(&header[4..]) as u64, self.read_u32(&header[8..]) as u64, header[12])
        };
        check!(num_values64 <= u32::MAX as u64);
        let num_values = num_values64 as u32;
        if end_offset == 0 && name_len == 0 {
            return Ok(None);
        }
        if end_offset > self.io.progress_bytes_total {
            self.io.progress_bytes_total = end_offset;
        }
        let name_bytes = self.read_bytes(name_len as usize)?;
        let (name, raw_name) = node_name(name_bytes);
        let values_end = self.get_read_offset().wrapping_add(values_len);

        let data = if let Some(info) = self.is_array_node(parent, &name) {
            PData::Arr(self.binary_array(info, num_values)?)
        } else {
            let count = (num_values as usize).min(MAX_NON_ARRAY_VALUES);
            let mut vals = Vec::with_capacity(count);
            for _ in 0..count {
                let head = self.peek_bytes(13)?;
                let t = head[0];
                let v = &head[1..];
                match t {
                    b'C' | b'B' | b'Z' => {
                        let i = v[0] as i64;
                        vals.push(PVal::Num { f: i as f64, i });
                        self.consume_bytes(2);
                    }
                    b'Y' => {
                        let i = self.read_i16(v) as i64;
                        vals.push(PVal::Num { f: i as f64, i });
                        self.consume_bytes(3);
                    }
                    b'I' => {
                        let i = self.read_u32(v) as i32 as i64;
                        vals.push(PVal::Num { f: i as f64, i });
                        self.consume_bytes(5);
                    }
                    b'L' => {
                        let i = self.read_u64(v) as i64;
                        vals.push(PVal::Num { f: i as f64, i });
                        self.consume_bytes(9);
                    }
                    b'F' => {
                        let f = self.read_f32(v) as f64;
                        vals.push(PVal::Num { f, i: f64_to_i64(f) });
                        self.consume_bytes(5);
                    }
                    b'D' => {
                        let f = self.read_f64(v);
                        vals.push(PVal::Num { f, i: f64_to_i64(f) });
                        self.consume_bytes(9);
                    }
                    b'S' | b'R' => {
                        let length = self.read_u32(v) as usize;
                        self.consume_bytes(5);
                        let s = self.read_bytes(length)?;
                        let raw = self.is_raw_string(parent, &name);
                        vals.push(PVal::Str(self.push_sanitized_string(s, raw)?));
                    }
                    b'c' | b'b' | b'i' | b'l' | b'f' | b'd' => {
                        let encoded = self.read_u32(&v[8..]) as u64;
                        self.consume_bytes(13);
                        self.skip_bytes(encoded)?;
                        vals.push(PVal::None);
                    }
                    _ => return fail(),
                }
            }
            PData::Vals(vals)
        };

        let mut node = self.binary_tail(name, data, values_end, end_offset, depth, parent, recursive)?;
        if let Some(n) = &mut node {
            n.raw_name = raw_name;
        }
        Ok(node)
    }
    /// Part of `ufbxi_binary_parse_node` in ufbx.c
    #[allow(clippy::too_many_arguments)]
    fn binary_tail(
        &mut self,
        name: Cow<'a, str>,
        data: PData<'a>,
        values_end: u64,
        end_offset: u64,
        depth: u32,
        parent: State,
        recursive: bool,
    ) -> Res<Option<PNode<'a>>> {
        let offset = self.get_read_offset();
        check!(offset <= values_end);
        if offset < values_end {
            self.skip_bytes(values_end - offset)?;
        }
        let base = self.node_stack.len();
        if recursive {
            let read = self.binary_children(update_parse_state(parent, &name), depth, end_offset);
            let children = self.node_stack.drain(base..).collect();
            read?;
            return Ok(Some(PNode { name, raw_name: None, dom_name: None, data, children }));
        }
        self.has_next_child = self.get_read_offset() < end_offset;
        Ok(Some(PNode { name, raw_name: None, dom_name: None, data, children: Vec::new() }))
    }
    /// Part of `ufbxi_binary_parse_node` in ufbx.c: The children onto `node_stack`, from which the caller takes them whole,
    /// allocated once
    fn binary_children(&mut self, state: State, depth: u32, end_offset: u64) -> Res<()> {
        loop {
            let current = self.get_read_offset();
            if current >= end_offset {
                check!(current == end_offset || end_offset == 0);
                return Ok(());
            }
            match self.binary_parse_node(depth + 1, state, true)? {
                Some(child) => self.node_stack.push(child),
                None => return Ok(()),
            }
        }
    }
    /// Part of `ufbxi_binary_parse_node` in ufbx.c
    fn binary_array(&mut self, info: ArrayInfo, num_values: u32) -> Res<PArr<'a>> {
        // 'r' is ufbx_real, a double here; booleans are read as bytes and turned to bools after
        // `ufbxi_normalize_array_type` in ufbx.c
        let dst = match info.kind {
            b'r' => b'd',
            b'b' => b'c',
            k => k,
        };
        let head = self.peek_bytes(13)?;
        let mut c = head[0];
        if num_values == 0 {
            c = b'0';
        }
        if dst == b'-' {
            c = b'-';
        }
        match c {
            b'c' | b'b' | b'i' | b'l' | b'f' | b'd' => {
                // `ufbxi_normalize_array_type` in ufbx.c
                let src = if c == b'b' { b'c' } else { c };
                let size = self.read_u32(&head[1..]) as usize;
                let encoding = self.read_u32(&head[5..]);
                let encoded_size = self.read_u32(&head[9..]) as usize;
                self.consume_bytes(13);
                let elem = array_type_size(src);
                let decoded_size = elem.checked_mul(size).ok_or_else(|| crate::error::Error::new(""))?;
                if self.opts.memory_limit > 0 {
                    self.array_bytes = self.array_bytes.saturating_add(decoded_size);
                    check!(self.array_bytes <= self.opts.memory_limit, "Memory limit exceeded");
                }
                let arr_begin = self.get_read_offset();
                check!((u64::MAX - encoded_size as u64) > arr_begin);
                let arr_end = arr_begin + encoded_size as u64;
                if arr_end > self.io.progress_bytes_total {
                    self.io.progress_bytes_total = arr_end;
                }
                let mut decoded = std::mem::take(&mut self.decode_buffer);
                let bytes: &[u8] = match encoding {
                    0 => {
                        check!(encoded_size == decoded_size);
                        // ufbx converts out of its buffer where the bytes are there, and reads straight into the array
                        // where they need no converting
                        if (src != dst || self.big_endian) && self.io.yield_size + self.io.data_size >= encoded_size {
                            if encoded_size > self.io.yield_size {
                                self.io.data_size += self.io.yield_size;
                                self.io.yield_size = encoded_size;
                                self.io.data_size -= self.io.yield_size;
                            }
                            let bytes = &self.data[self.pos..self.pos + encoded_size];
                            self.consume_bytes(encoded_size);
                            bytes
                        } else {
                            self.read_to(encoded_size)?
                        }
                    }
                    1 => {
                        let n = self.inflate_array(&mut decoded, encoded_size, decoded_size, arr_begin, arr_end)?;
                        &decoded[..n]
                    }
                    _ => return fail(),
                };
                let mut arr = self.binary_convert_array(src, dst, bytes, size, info.kind)?;
                if info.keep_ints
                    && let PArr::Bool(bools) = &arr
                    && let PArr::I64(ints) = self.binary_convert_array(src, b'l', bytes, size, b'l')?
                {
                    arr = PArr::BoolInts(bools.clone(), ints);
                }
                self.decode_buffer = decoded;
                Ok(arr)
            }
            // An empty boolean array keeps the byte type, which the readers asking for booleans do not take
            b'0' | b'-' => Ok(if c == b'-' { PArr::Ignored } else { empty_array(dst, dst) }),
            _ => self.binary_parse_multivalue_array(dst, info.kind, num_values as usize),
        }
    }
    /// Part of `ufbxi_binary_parse_node` in ufbx.c: Its DEFLATE branch; the buffer is handed to the inflater, the rest read from
    /// the stream as it asks, the stream left where its reads stopped. The output goes to `decoded`, kept from array to array
    /// and grown where it is short
    fn inflate_array(&mut self, decoded: &mut Vec<u8>, encoded_size: usize, decoded_size: usize, arr_begin: u64, arr_end: u64) -> Res<usize> {
        self.pause_progress();
        let in_buffer = self.io.data_size;
        let buffered = &self.data[self.pos..self.pos + in_buffer];
        let streamed = encoded_size > in_buffer;
        if streamed {
            let rest = encoded_size - in_buffer;
            self.pos += in_buffer;
            self.io.data_size = 0;
            let Some(far) = self.pos.checked_add(rest) else { return fail() };
            self.io.data_offset += rest as u64;
            self.io.buf_begin += rest;
            self.pos = far;
        } else {
            self.pos += encoded_size;
            self.io.data_size -= encoded_size;
            self.resume_progress()?;
        }

        let reads = streamed && self.io.stream;
        let file = self.data;
        let available = encoded_size.min(in_buffer + if reads { file.len().saturating_sub(self.io.stream_pos) } else { 0 });
        let out_size = decoded_size.min(inflate_bound(available));
        if decoded.len() < out_size {
            decoded.resize(out_size, 0);
        }
        let progress = self.io.progress.clone();
        let (before, after, hint) = match progress {
            Some(_) => (arr_begin, self.io.progress_bytes_total.wrapping_sub(arr_end), self.io.progress_interval as u64),
            None => (0, 0, 0),
        };
        let buffer_size = if streamed { self.io.read_buffer_size } else { 0 };
        let io = &mut self.io;
        let mut read = |buf: &mut [u8]| {
            let from = io.stream_pos;
            match io.stream_read(file.len(), buf.len()) {
                Ok(n) => {
                    buf[..n].copy_from_slice(&file[from..from + n]);
                    n
                }
                Err(_) => usize::MAX,
            }
        };
        let mut input = InflateInput {
            total_size: encoded_size,
            data: buffered,
            buffer_size,
            read_fn: if reads { Some(&mut read) } else { None },
            progress_cb: progress.as_deref(),
            progress_interval_hint: hint,
            progress_size_before: before,
            progress_size_after: after,
            ..Default::default()
        };
        let res = inflate(&mut decoded[..out_size], &mut input, &mut self.inflate_retain);
        check!(res != -28, "Cancelled");
        check!(res == decoded_size as isize, "Bad DEFLATE data");
        Ok(decoded_size)
    }
    fn binary_convert_array(&self, src: u8, dst: u8, b: &[u8], size: usize, kind: u8) -> Res<PArr<'a>> {
        macro_rules! read_all {
            ($n:expr, $f:expr) => {{
                let f = $f;
                b[..size * $n].chunks_exact($n).map(|p| f(p.try_into().unwrap_or([0; $n]))).collect::<Vec<_>>()
            }};
        }
        // `ufbxi_swap_endian_array` in ufbx.c
        let be = self.big_endian;
        let i32s = |p: [u8; 4]| if be { i32::from_be_bytes(p) } else { i32::from_le_bytes(p) };
        let i64s = |p: [u8; 8]| if be { i64::from_be_bytes(p) } else { i64::from_le_bytes(p) };
        let f32s = |p: [u8; 4]| if be { f32::from_be_bytes(p) } else { f32::from_le_bytes(p) };
        let f64s = |p: [u8; 8]| if be { f64::from_be_bytes(p) } else { f64::from_le_bytes(p) };
        let arr = match dst {
            b'c' => {
                let v: Vec<u8> = match src {
                    b'c' => b[..size].to_vec(),
                    b'i' => read_all!(4, |p| i32s(p) as u8),
                    b'l' => read_all!(8, |p| i64s(p) as u8),
                    b'f' => read_all!(4, |p| f32s(p) as u8),
                    b'd' => read_all!(8, |p| f64s(p) as u8),
                    _ => return fail(),
                };
                // `ufbxi_postprocess_bool_array` in ufbx.c
                if kind == b'b' { PArr::Bool(v.into_iter().map(|x| x != 0).collect()) } else { PArr::U8(v) }
            }
            b'i' => PArr::I32(match src {
                b'c' => b[..size].iter().map(|&x| x as i8 as i32).collect(),
                b'i' => read_all!(4, i32s),
                b'l' => read_all!(8, |p| i64s(p) as i32),
                b'f' => read_all!(4, |p| f64_to_i32(f32s(p) as f64)),
                b'd' => read_all!(8, |p| f64_to_i32(f64s(p))),
                _ => return fail(),
            }),
            b'l' => PArr::I64(match src {
                b'c' => b[..size].iter().map(|&x| x as i8 as i64).collect(),
                b'i' => read_all!(4, |p| i32s(p) as i64),
                b'l' => read_all!(8, i64s),
                b'f' => read_all!(4, |p| f64_to_i64(f32s(p) as f64)),
                b'd' => read_all!(8, |p| f64_to_i64(f64s(p))),
                _ => return fail(),
            }),
            b'f' => PArr::F32(match src {
                b'c' => b[..size].iter().map(|&x| x as i8 as f32).collect(),
                b'i' => read_all!(4, |p| i32s(p) as f32),
                b'l' => read_all!(8, |p| i64s(p) as f32),
                b'f' => read_all!(4, f32s),
                b'd' => read_all!(8, |p| f64s(p) as f32),
                _ => return fail(),
            }),
            b'd' => PArr::F64(match src {
                b'c' => b[..size].iter().map(|&x| x as i8 as f64).collect(),
                b'i' => read_all!(4, |p| i32s(p) as f64),
                b'l' => read_all!(8, |p| i64s(p) as f64),
                b'f' => read_all!(4, |p| f32s(p) as f64),
                b'd' => read_all!(8, f64s),
                _ => return fail(),
            }),
            _ => return fail(),
        };
        Ok(arr)
    }
    /// A pre-7000 array written as one plain value each
    fn binary_parse_multivalue_array(&mut self, dst: u8, kind: u8, size: usize) -> Res<PArr<'a>> {
        if self.opts.memory_limit > 0 {
            self.array_bytes = self.array_bytes.saturating_add(size.saturating_mul(array_type_size(dst)));
            check!(self.array_bytes <= self.opts.memory_limit, "Memory limit exceeded");
        }
        // ufbx allocates the count the file states; room is made only for the values the rest of the file can hold,
        // two bytes each at least, as a 32-bit build cannot allocate a count of four billion
        let room = size.min(self.data.len().saturating_sub(self.pos) / 2);
        if matches!(dst, b's' | b'S' | b'C') {
            let raw = dst == b's';
            let mut strs = Vec::with_capacity(room);
            let mut content = Vec::new();
            for _ in 0..size {
                let head = self.peek_bytes(13)?;
                let t = head[0];
                check!(t == b'S' || t == b'R');
                let len = self.read_u32(&head[1..]) as usize;
                self.consume_bytes(5);
                let s = self.read_bytes(len)?;
                if dst == b'C' {
                    content.push(s.to_vec());
                } else {
                    let s = if raw { SStr::raw_only(s) } else { self.push_sanitized_string(s, false)? };
                    strs.push(s);
                }
            }
            return Ok(if dst == b'C' { PArr::Content(content) } else { PArr::Str(strs) });
        }
        let mut ints: Vec<i64> = Vec::with_capacity(room);
        let mut floats: Vec<f64> = Vec::with_capacity(room);
        let mut is_float: Vec<bool> = Vec::with_capacity(room);
        for _ in 0..size {
            let head = self.peek_bytes(13)?;
            let t = head[0];
            let v = &head[1..];
            let (n, int, float) = match t {
                b'C' | b'B' => (1, Some(v[0] as i8 as i64), None),
                b'Y' => (2, Some(self.read_i16(v) as i64), None),
                b'I' => (4, Some(self.read_u32(v) as i32 as i64), None),
                b'L' => (8, Some(self.read_u64(v) as i64), None),
                b'F' => (4, None, Some(self.read_f32(v) as f64)),
                b'D' => (8, None, Some(self.read_f64(v))),
                _ => return fail(),
            };
            self.consume_bytes(n + 1);
            ints.push(int.unwrap_or(0));
            floats.push(float.unwrap_or(0.0));
            is_float.push(float.is_some());
        }
        let n = size;
        let arr = match dst {
            b'c' => {
                let v: Vec<u8> = (0..n).map(|k| if is_float[k] { floats[k] as u8 } else { ints[k] as u8 }).collect();
                // `ufbxi_postprocess_bool_array` in ufbx.c
                if kind == b'b' { PArr::Bool(v.into_iter().map(|x| x != 0).collect()) } else { PArr::U8(v) }
            }
            b'i' => PArr::I32((0..n).map(|k| if is_float[k] { f64_to_i32(floats[k]) } else { ints[k] as i32 }).collect()),
            b'l' => PArr::I64((0..n).map(|k| if is_float[k] { f64_to_i64(floats[k]) } else { ints[k] }).collect()),
            b'f' => PArr::F32((0..n).map(|k| if is_float[k] { floats[k] as f32 } else { ints[k] as f32 }).collect()),
            b'd' => PArr::F64((0..n).map(|k| if is_float[k] { floats[k] } else { ints[k] as f64 }).collect()),
            _ => return fail(),
        };
        Ok(arr)
    }
    /// A stream's next read, or past the end of memory a buffer of one NUL
    fn ascii_refill(&mut self) -> u8 {
        self.io.data_offset = self.io.data_offset.wrapping_add(self.pos.wrapping_sub(self.io.buf_begin) as u64);
        if self.io.stream {
            if self.io.read_buffer_size < self.io.opts_read_buffer_size {
                self.io.read_buffer_size = self.io.opts_read_buffer_size;
            }
            let read = match self.stream_read(self.io.read_buffer_size) {
                Ok(n) => n,
                Err(e) => {
                    self.keep_failure(e);
                    return 0;
                }
            };
            if read == 0 {
                return 0;
            }
            self.io.buf_begin = self.pos;
            self.io.src_end = self.pos + read;
            self.byte(self.pos)
        } else {
            self.io.buf_begin = self.pos;
            self.io.src_end = self.pos + 1;
            0
        }
    }
    /// A cancel is kept and read past as NUL
    fn ascii_yield(&mut self) -> u8 {
        let ret = if self.pos == self.io.src_end { self.ascii_refill() } else { self.byte(self.pos) };
        self.io.src_yield = if self.io.src_end - self.pos < self.io.progress_interval { self.io.src_end } else { self.pos + self.io.progress_interval };
        if let Err(e) = self.report_progress() {
            self.keep_failure(e);
            return 0;
        }
        ret
    }
    fn ascii_peek(&mut self) -> u8 {
        if self.pos == self.io.src_yield {
            return self.ascii_yield();
        }
        self.byte(self.pos)
    }
    fn ascii_next(&mut self) -> u8 {
        if self.pos == self.io.src_yield {
            return self.ascii_yield();
        }
        self.pos += 1;
        if self.pos == self.io.src_yield {
            return self.ascii_yield();
        }
        self.byte(self.pos)
    }
    /// " FBX ?.?.?" after the ';' of the first comment
    fn ascii_parse_version(&mut self) -> u32 {
        let mut digits = [0u32; 3];
        let mut n = 0;
        let mut c = self.ascii_next();
        let fmt = b" FBX ?.?.?";
        let mut ix = 0;
        while n < 3 {
            let r = fmt[ix];
            ix += 1;
            match r {
                b'?' => {
                    if !c.is_ascii_digit() {
                        return 0;
                    }
                    digits[n] = (c - b'0') as u32;
                    n += 1;
                    c = self.ascii_next();
                }
                b' ' => {
                    while c == b' ' || c == b'\t' {
                        c = self.ascii_next();
                    }
                }
                _ => {
                    if c != r {
                        return 0;
                    }
                    c = self.ascii_next();
                }
            }
        }
        1000 * digits[0] + 100 * digits[1] + 10 * digits[2]
    }
    fn ascii_skip_whitespace(&mut self) -> u8 {
        let mut c = self.ascii_peek();
        loop {
            while is_space(c) {
                c = self.ascii_next();
            }
            if c != b';' {
                break;
            }
            let mut read_magic = false;
            if !self.ascii.read_first_comment {
                self.ascii.read_first_comment = true;
                let version = self.ascii_parse_version();
                if version != 0 {
                    self.version = version;
                    self.ascii.found_version = true;
                    read_magic = true;
                }
            }
            c = self.ascii_next();
            while c != b'\n' && c != 0 {
                c = self.ascii_next();
            }
            c = self.ascii_next();
            if read_magic && c == b';' {
                let mut line = Vec::new();
                c = self.ascii_next();
                while c != b'\n' && c != 0 {
                    if line.len() < 32 {
                        line.push(c);
                    }
                    c = self.ascii_next();
                }
                if line.starts_with(b" Created by Blender") {
                    self.ascii_blender = true;
                }
            }
        }
        c
    }
    fn ascii_next_token(&mut self) -> Res<()> {
        std::mem::swap(&mut self.ascii.prev, &mut self.ascii.token);
        let tok = &mut self.ascii.token;
        tok.text.clear();
        (tok.kind, tok.span, tok.replaced, tok.negative, tok.i64, tok.f64) = (TokKind::End, (0, 0), false, false, 0, 0.0);
        self.read_token()
    }
    /// Part of `ufbxi_ascii_next_token` in ufbx.c: The token filled in place as ufbx fills its own, so that a failure leaves it
    /// as far as it got
    fn read_token(&mut self) -> Res<()> {
        let mut c = self.ascii_skip_whitespace();
        if c.is_ascii_alphabetic() || c == b'_' {
            self.ascii.token.kind = TokKind::BareWord;
            let start = self.pos;
            while c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'(' | b')') {
                self.ascii.token.text.push(c);
                c = self.ascii_next();
            }
            self.ascii.token.span = (start, self.pos);
            c = self.ascii_skip_whitespace();
            if c == b':' {
                self.ascii.token.kind = TokKind::Name;
                self.ascii_next();
            }
        } else if c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.') {
            self.ascii.token.kind = TokKind::Int;
            self.ascii.token.negative = c == b'-';
            while c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E') {
                if matches!(c, b'.' | b'e' | b'E') {
                    self.ascii.token.kind = TokKind::Float;
                }
                self.ascii.token.text.push(c);
                c = self.ascii_next();
            }
            let mut nan_like = false;
            while c.is_ascii_alphanumeric() || matches!(c, b'#' | b'(' | b')') {
                nan_like = true;
                self.ascii.token.text.push(c);
                c = self.ascii_next();
            }
            if nan_like {
                self.ascii.token.kind = TokKind::Float;
            }
            if self.ascii.token.kind == TokKind::Int {
                let (v, end) = parse_int64(&self.ascii.token.text);
                self.ascii.token.i64 = v;
                check!(end == Some(self.ascii.token.text.len()));
            } else {
                let (v, end) = parse_double(&self.ascii.token.text, self.ascii.parse_as_f32);
                self.ascii.token.f64 = v;
                check!(end == Some(self.ascii.token.text.len()));
            }
        } else if c == b'"' {
            self.ascii.token.kind = TokKind::Str;
            c = self.ascii_next();
            let start = self.pos;
            while c != b'"' {
                // ufbx copies the run up to the next '"' or '&' out of its buffer whole, NULs and all, while two bytes
                // or more are left in the window
                if self.pos + 1 < self.io.src_yield {
                    let end = (self.pos..self.io.src_yield).find(|&at| matches!(self.byte(at), b'"' | b'&')).unwrap_or(self.io.src_yield);
                    if self.pos < end {
                        let data = self.data;
                        self.ascii.token.text.extend((self.pos..end).map(|at| data.get(at).copied().unwrap_or(0)));
                        self.pos = end;
                        c = self.ascii_peek();
                        continue;
                    }
                }
                if c == b'&' {
                    c = self.ascii_next();
                    let (entity, replacement): (&[u8], u8) = match c {
                        b'q' => (b"&quot;", b'"'),
                        b'c' => (b"&cr;", b'\r'),
                        b'l' => (b"&lf;", b'\n'),
                        _ => (b"&", b'&'),
                    };
                    let mut step = 1;
                    while step < entity.len() {
                        if c != entity[step] {
                            break;
                        }
                        c = self.ascii_next();
                        step += 1;
                    }
                    if step == entity.len() {
                        self.ascii.token.text.push(replacement);
                        self.ascii.token.replaced |= step > 1;
                    } else {
                        self.ascii.token.text.extend_from_slice(&entity[..step]);
                    }
                    continue;
                }
                check!(c != 0);
                self.ascii.token.text.push(c);
                c = self.ascii_next();
            }
            self.ascii.token.span = (start, self.pos);
            let next = self.ascii_next();
            if next == b':' {
                self.ascii.token.kind = TokKind::Name;
                self.ascii_next();
            }
        } else if c == 0 {
            // ufbx's end is a token of one character too, stepped past like any other
            self.ascii.token.kind = TokKind::End;
            self.ascii_next();
        } else {
            self.ascii.token.kind = TokKind::Char(c);
            self.ascii_next();
        }
        Ok(())
    }
    /// A failure to read the next token is "not accepted": the load reads on, the failure kept
    fn ascii_accept(&mut self, kind: TokKind) -> Res<bool> {
        if self.ascii.token.kind != kind {
            return Ok(false);
        }
        match self.ascii_next_token() {
            Ok(()) => Ok(true),
            Err(e) => {
                self.keep_failure(e);
                Ok(false)
            }
        }
    }
    /// A failure ufbx records and reads past: the load's error, if it fails, describes the first with a description
    fn keep_failure(&mut self, e: Error) {
        self.io.pending = Some(match self.io.pending.take() {
            Some(p) => first_described(p, e),
            None => e,
        });
    }
    fn ascii_try_ignore_string(&mut self) -> Res<bool> {
        let c = self.ascii_skip_whitespace();
        if c == b'"' {
            let prev = std::mem::take(&mut self.ascii.token);
            self.ascii.prev = prev;
            self.ascii.token = Token { kind: TokKind::Str, ..Token::default() };
            self.ascii_next();
            self.ascii_skip_until(b'"')?;
            self.ascii_next();
            return Ok(true);
        }
        Ok(false)
    }
    fn ascii_skip_until(&mut self, d: u8) -> Res<()> {
        loop {
            let buffered = self.io.src_yield - self.pos;
            if let Some(p) = (self.pos..self.pos + buffered).position(|at| self.byte(at) == d) {
                self.pos += p;
                return Ok(());
            }
            self.pos += buffered;
            if buffered == 0 {
                check!(self.ascii_yield() != 0);
            }
        }
    }
    fn ascii_parse_node(&mut self, depth: u32, parent: State, recursive: bool) -> Res<Option<PNode<'a>>> {
        if self.ascii.token.kind == TokKind::Char(b'}') {
            self.ascii_next_token()?;
            return Ok(None);
        }
        if self.ascii.token.kind == TokKind::End {
            check!(depth == 0, "Truncated file");
            return Ok(None);
        }
        check!(depth < MAX_NODE_DEPTH);
        if !self.sure_fbx && depth == 0 && self.ascii.token.kind != TokKind::Name {
            return fail_msg("Not an FBX file");
        }
        check!(self.ascii_accept(TokKind::Name)?);
        check!(self.ascii.prev.text.len() <= 0xff);
        let (name, raw_name) = match self.ascii_in_input(&self.ascii.prev.text) {
            Some(bytes) => node_name(bytes),
            None => {
                let bytes = &self.ascii.prev.text;
                (Cow::Owned(String::from_utf8_lossy(bytes).into_owned()), (!is_utf8_valid(bytes)).then(|| Cow::Owned(bytes.clone())))
            }
        };

        let mut in_array = false;
        let mut num_values: i64 = 0;
        // the values gathered in a buffer kept from node to node, then allocated once
        let mut vals = std::mem::take(&mut self.val_stack);
        vals.clear();

        let info = self.is_array_node(parent, &name);
        // `ufbxi_normalize_array_type` in ufbx.c
        let arr_type = info.map(|i| if i.kind == b'r' { b'd' } else { i.kind }).unwrap_or(0);
        let mut acc = Acc::new(arr_type);
        if info.is_some_and(|i| i.keep_ints) {
            acc.ints = Some(Vec::new());
        }
        let mut arr_error = false;
        if let Some(i) = info
            && i.accurate_f32
        {
            self.ascii.parse_as_f32 = true;
        }

        if self.ascii.token.kind == TokKind::Char(b',') {
            if arr_type == b'-' {
                if !self.ascii_try_ignore_string()? {
                    self.ascii_next_token()?;
                }
            } else {
                self.ascii_next_token()?;
            }
        }

        let state = update_parse_state(parent, &name);
        loop {
            if arr_type != 0 {
                let num_read = match arr_type {
                    b'f' | b'd' => self.ascii_read_float_array(&mut acc)?,
                    b'i' | b'l' => self.ascii_read_int_array(&mut acc)?,
                    _ => 0,
                };
                check!(u32::MAX as i64 - num_values > num_read as i64);
                num_values += num_read as i64;
            }
            if self.ascii_accept(TokKind::Str)? {
                // taken for the call and put back, so that the token keeps its buffer
                let text = std::mem::take(&mut self.ascii.prev.text);
                if arr_type != 0 {
                    match arr_type {
                        b's' | b'S' => {
                            let s = if arr_type == b's' {
                                SStr::raw_only(self.ascii_in_input(&text).map_or_else(|| Cow::Owned(text.clone()), Cow::Borrowed))
                            } else {
                                self.push_ascii_string(&text, false)?
                            };
                            acc.strs.push(s);
                        }
                        b'C' => {
                            let decoded = self.decode_base64(&text, &mut arr_error)?;
                            acc.content.push(decoded);
                        }
                        _ => num_values -= 1,
                    }
                } else if (num_values as usize) < MAX_NON_ARRAY_VALUES {
                    let raw = self.is_raw_string(parent, &name);
                    let s = self.push_ascii_string(&text, raw)?;
                    vals.push(PVal::Str(s));
                }
                self.ascii.prev.text = text;
            } else if self.ascii_accept(TokKind::Int)? {
                let val = self.ascii.prev.i64;
                let negative = self.ascii.prev.negative;
                let fsign = if val == 0 && negative { -1.0 } else { 1.0 };
                match arr_type {
                    0 => {
                        if !self.ascii.found_version && state == State::FbxVersion && num_values == 0 && (6000..=10000).contains(&val) {
                            self.ascii.found_version = true;
                            self.version = val as u32;
                        }
                        if (num_values as usize) < MAX_NON_ARRAY_VALUES {
                            vals.push(PVal::Num { f: val as f64 * fsign, i: val });
                        }
                    }
                    b'-' => num_values -= 1,
                    _ => acc.push_int(val, fsign)?,
                }
            } else if self.ascii_accept(TokKind::Float)? {
                let val = self.ascii.prev.f64;
                match arr_type {
                    0 => {
                        if (num_values as usize) < MAX_NON_ARRAY_VALUES {
                            vals.push(PVal::Num { f: val, i: f64_to_i64(val) });
                        }
                    }
                    b'-' => num_values -= 1,
                    _ => acc.push_float(val)?,
                }
            } else if self.ascii_accept(TokKind::BareWord)? {
                let text = &self.ascii.prev.text;
                let mut val: i64 = 0;
                let mut val_f = 0.0;
                if !text.is_empty() {
                    val = text[0] as i8 as i64;
                    val_f = val as f64;
                    if text.len() > 1 && text.len() < 64 {
                        let (f, end) = parse_inf_nan(text);
                        if end == Some(text.len()) {
                            val = 0;
                            val_f = f;
                        }
                    }
                }
                match arr_type {
                    0 => {
                        if (num_values as usize) < MAX_NON_ARRAY_VALUES {
                            vals.push(PVal::Num { f: val_f, i: val });
                        }
                    }
                    b'-' => num_values -= 1,
                    _ => acc.push_word(val, val_f)?,
                }
            } else if self.ascii_accept(TokKind::Char(b'*'))? {
                check!(!in_array);
                check!(self.ascii_accept(TokKind::Int)?);
                if self.ascii_accept(TokKind::Char(b'{'))? {
                    check!(self.ascii_accept(TokKind::Name)?);
                    in_array = true;
                    if arr_type == b'-' {
                        self.ascii_skip_until(b'}')?;
                    }
                }
                continue;
            } else {
                break;
            }
            num_values += 1;
            check!(num_values < u32::MAX as i64);
            if !self.ascii_accept(TokKind::Char(b','))? {
                break;
            }
        }

        if in_array {
            check!(self.ascii_accept(TokKind::Char(b'}'))?);
        }
        self.ascii.parse_as_f32 = false;

        #[allow(clippy::drain_collect)] // allocated to size, the buffer kept for the next node
        let data = if arr_type != 0 {
            if arr_type == b'-' {
                PData::Arr(PArr::Ignored)
            } else if arr_error {
                PData::Arr(empty_array(arr_type, info.map(|i| i.kind).unwrap_or(arr_type)))
            } else {
                PData::Arr(acc.finish(info.map(|i| i.kind).unwrap_or(arr_type)))
            }
        } else {
            PData::Vals(vals.drain(..).collect())
        };
        self.val_stack = vals;

        let mut children = Vec::new();
        if self.ascii_accept(TokKind::Char(b'{'))? {
            if recursive {
                let base = self.node_stack.len();
                let read = self.ascii_children(depth, state);
                children = self.node_stack.drain(base..).collect();
                read?;
            }
            self.has_next_child = true;
        } else {
            self.has_next_child = false;
        }
        Ok(Some(PNode { name, raw_name, dom_name: None, data, children }))
    }
    /// Part of `ufbxi_ascii_parse_node` in ufbx.c, as `binary_children`
    fn ascii_children(&mut self, depth: u32, state: State) -> Res<()> {
        while let Some(child) = self.ascii_parse_node(depth + 1, state, true)? {
            self.node_stack.push(child);
        }
        Ok(())
    }
    /// Read straight from the text while a comma follows each value, the tokenizer taking over after the last
    fn ascii_read_float_array(&mut self, acc: &mut Acc) -> Res<usize> {
        if self.ascii.parse_as_f32 {
            return Ok(0);
        }
        let token = &self.ascii.token;
        let mut val = match token.kind {
            TokKind::Float => token.f64,
            TokKind::Int => token.i64 as f64 * if token.i64 == 0 && token.negative { -1.0 } else { 1.0 },
            _ => return Ok(0),
        };
        let end = self.io.src_yield;
        let (start, mut src, mut scan) = (self.pos, self.pos, self.pos);
        let mut num_read = 0;
        loop {
            if !self.scan_comma(&mut scan) {
                break;
            }
            src = scan;
            acc.push_float(val)?;
            num_read += 1;
            match parse_double(self.window(scan, end), false) {
                (v, Some(n)) if n > 0 && scan + n < end => {
                    val = v;
                    scan += n;
                }
                _ => break,
            }
        }
        if src != start {
            self.pos = src;
            self.ascii_next_token()?;
        }
        Ok(num_read)
    }
    /// As `ascii_read_float_array`, a value read only where 32 bytes are left
    fn ascii_read_int_array(&mut self, acc: &mut Acc) -> Res<usize> {
        if self.ascii.parse_as_f32 || self.ascii.token.kind != TokKind::Int {
            return Ok(0);
        }
        let mut val = self.ascii.token.i64;
        let (start, mut src, mut scan) = (self.pos, self.pos, self.pos);
        let mut num_read = 0;
        loop {
            if !self.scan_comma(&mut scan) {
                break;
            }
            src = scan;
            acc.push_int(val, 1.0)?;
            num_read += 1;
            if self.io.src_yield - scan < 32 {
                break;
            }
            match parse_int64(self.window(scan, self.io.src_yield)) {
                (v, Some(n)) => {
                    val = v;
                    scan += n;
                }
                _ => break,
            }
        }
        if src != start {
            self.pos = src;
            self.ascii_next_token()?;
        }
        Ok(num_read)
    }
    /// Part of `ufbxi_ascii_read_int_array` and `ufbxi_ascii_read_float_array` in ufbx.c
    fn scan_comma(&self, scan: &mut usize) -> bool {
        let end = self.io.src_yield;
        while *scan != end && is_space(self.byte(*scan)) {
            *scan += 1;
        }
        if *scan == end || self.byte(*scan) != b',' {
            return false;
        }
        *scan += 1;
        while *scan != end && is_space(self.byte(*scan)) {
            *scan += 1;
        }
        true
    }
    fn decode_base64(&mut self, src: &[u8], failed: &mut bool) -> Res<Vec<u8>> {
        // `ufbxi_setup_base64` in ufbx.c
        let table = |c: u8| -> u32 {
            match c {
                b'A'..=b'Z' => (c - b'A') as u32,
                b'a'..=b'z' => (26 + c - b'a') as u32,
                b'0'..=b'9' => (52 + c - b'0') as u32,
                b'+' => 62,
                b'/' => 63,
                b'=' => 0x40,
                _ => 0x80,
            }
        };
        let mut out = Vec::with_capacity(src.len() / 4 * 3 + 3);
        let mut error_mask = 0u32;
        let mut pad_error = 0u32;
        let mut i = 0;
        while i + 4 <= src.len() {
            let (a, b, c, d) = (table(src[i]), table(src[i + 1]), table(src[i + 2]), table(src[i + 3]));
            pad_error = error_mask;
            error_mask |= a | b | c | d;
            out.push((a << 2 | b >> 4) as u8);
            out.push((b << 4 | c >> 2) as u8);
            out.push((c << 6 | d) as u8);
            i += 4;
        }
        if src.len() >= 4 {
            let end = &src[src.len() - 4..];
            let mut padding = 0;
            padding |= if end[0] == b'=' { 0x8 } else { 0 };
            padding |= if end[1] == b'=' { 0x4 } else { 0 };
            padding |= if end[2] == b'=' { 0x2 } else { 0 };
            padding |= if end[3] == b'=' { 0x1 } else { 0 };
            if padding <= 0x1 {
                out.truncate(out.len() - padding);
            } else if padding == 0x3 {
                out.truncate(out.len() - 2);
            } else {
                pad_error |= 0x40;
            }
        }
        if ((error_mask & 0x80) != 0 || (pad_error & 0x40) != 0 || !src.len().is_multiple_of(4)) && !*failed {
            self.warnings.warn(WarningType::BadBase64Content, crate::error::NO_INDEX, "Ignored bad base64 embedded content");
            *failed = true;
        }
        Ok(out)
    }
    /// Part of `ufbxi_parse_toplevel` in ufbx.c
    fn toplevel_head(&mut self) -> Res<Option<PNode<'a>>> {
        if self.from_ascii { self.ascii_parse_node(0, State::Root, false) } else { self.binary_parse_node(0, State::Root, false) }
    }
    fn parse_toplevel_child_imp(&mut self, state: State) -> Res<Option<PNode<'a>>> {
        if self.from_ascii { self.ascii_parse_node(0, state, true) } else { self.binary_parse_node(0, state, true) }
    }
    /// Nodes met before the one asked for are parsed whole, that one only up to its children, which `next_lazy_child` parses as
    /// its reader asks, whether its header says it has any or not. Children no reader asks for are parsed later as top-level
    /// nodes, as in ufbx.
    pub fn parse_toplevel(&mut self, name: &str) -> Res<Option<usize>> {
        self.lazy = None;
        if let Some(i) = self.top_nodes.iter().position(|n| n.name == name) {
            return Ok(Some(i));
        }
        loop {
            if self.parsed_to_end {
                return Ok(None);
            }
            let Some(mut node) = self.toplevel_head()? else {
                self.parsed_to_end = true;
                return Ok(None);
            };
            self.retain(&mut node)?;
            let state = update_parse_state(State::Root, &node.name);
            if node.name == name {
                self.top_nodes.push(node);
                let ix = self.top_nodes.len() - 1;
                self.lazy = Some((ix, state));
                return Ok(Some(ix));
            }
            if self.has_next_child {
                while let Some(child) = self.parse_toplevel_child_imp(state)? {
                    node.children.push(child);
                }
            }
            for child in &mut node.children {
                self.retain(child)?;
            }
            self.top_nodes.push(node);
        }
    }
    pub fn is_lazy(&self, ix: usize) -> bool {
        self.lazy.is_some_and(|l| l.0 == ix)
    }
    /// Part of `ufbxi_parse_toplevel_child` in ufbx.c
    pub fn next_lazy_child(&mut self) -> Res<Option<PNode<'a>>> {
        let Some((_, state)) = self.lazy else { return Ok(None) };
        let mut child = self.parse_toplevel_child_imp(state)?;
        match &mut child {
            Some(c) => self.retain(c)?,
            None => self.lazy = None,
        }
        Ok(child)
    }
    /// Part of `ufbxi_retain_dom_node` in ufbx.c
    fn retain(&mut self, node: &mut PNode) -> Res<()> {
        if !self.opts.retain_dom {
            return Ok(());
        }
        if let Some(raw) = &node.raw_name {
            node.dom_name = Some(push_sanitized_string_entry(raw, utf8_valid_length(raw), self.opts.unicode_error_handling, &mut self.warnings)?);
        }
        for child in &mut node.children {
            self.retain(child)?;
        }
        Ok(())
    }
    /// Part of `ufbxi_retain_toplevel_child` in ufbx.c
    pub fn keep_child(&mut self, ix: usize, child: PNode<'a>) {
        if self.opts.retain_dom {
            self.dom_children.entry(ix).or_default().push(child);
        }
    }
    /// Part of `ufbxi_parse_toplevel` in ufbx.c: Reading on
    pub fn next_toplevel(&mut self) -> Res<Option<PNode<'a>>> {
        self.lazy = None;
        if self.parsed_to_end {
            return Ok(None);
        }
        let Some(mut node) = self.toplevel_head()? else {
            self.parsed_to_end = true;
            return Ok(None);
        };
        self.retain(&mut node)?;
        if self.has_next_child {
            let state = update_parse_state(State::Root, &node.name);
            while let Some(child) = self.parse_toplevel_child_imp(state)? {
                node.children.push(child);
            }
        }
        for child in &mut node.children {
            self.retain(child)?;
        }
        Ok(Some(node))
    }
    /// Pre-7000 files are read node by node, each parsed whole
    pub fn parse_legacy_toplevel(&mut self) -> Res<Option<PNode<'a>>> {
        if self.parsed_to_end {
            return Ok(None);
        }
        let node = if self.from_ascii { self.ascii_parse_node(0, State::Root, true)? } else { self.binary_parse_node(0, State::Root, true)? };
        match node {
            Some(mut n) => {
                self.retain(&mut n)?;
                self.top_nodes.push(n.clone());
                Ok(Some(n))
            }
            None => {
                self.parsed_to_end = true;
                Ok(None)
            }
        }
    }
    /// Part of `ufbxi_parse_toplevel` in ufbx.c
    pub fn parse_to_end(&mut self) -> Res<()> {
        while let Some(node) = self.next_toplevel()? {
            self.top_nodes.push(node);
        }
        Ok(())
    }
}

struct Acc<'a> {
    kind: u8,
    ints: Option<Vec<i64>>,
    bools: Vec<bool>,
    u8s: Vec<u8>,
    i32s: Vec<i32>,
    i64s: Vec<i64>,
    f32s: Vec<f32>,
    f64s: Vec<f64>,
    strs: Vec<SStr<'a>>,
    content: Vec<Vec<u8>>,
}
impl<'a> Acc<'a> {
    fn new(kind: u8) -> Acc<'a> {
        Acc {
            kind,
            ints: None,
            bools: Vec::new(),
            u8s: Vec::new(),
            i32s: Vec::new(),
            i64s: Vec::new(),
            f32s: Vec::new(),
            f64s: Vec::new(),
            strs: Vec::new(),
            content: Vec::new(),
        }
    }

    /// Part of `ufbxi_ascii_parse_node` in ufbx.c
    fn push_int(&mut self, val: i64, fsign: f64) -> Res<()> {
        if let Some(ints) = &mut self.ints {
            ints.push(val);
        }
        match self.kind {
            b'b' => self.bools.push(val != 0),
            b'c' => self.u8s.push(val as u8),
            b'i' => self.i32s.push(val as i32),
            b'l' => self.i64s.push(val),
            b'f' => self.f32s.push(val as f32 * fsign as f32),
            b'd' => self.f64s.push(val as f64 * fsign),
            b's' | b'S' | b'C' => return fail(),
            _ => return fail(),
        }
        Ok(())
    }

    /// Part of `ufbxi_ascii_parse_node` in ufbx.c
    fn push_float(&mut self, val: f64) -> Res<()> {
        if let Some(ints) = &mut self.ints {
            ints.push(f64_to_i64(val));
        }
        match self.kind {
            b'b' => self.bools.push(val != 0.0),
            b'c' => self.u8s.push(val as u8),
            b'i' => self.i32s.push(f64_to_i32(val)),
            b'l' => self.i64s.push(f64_to_i64(val)),
            b'f' => self.f32s.push(val as f32),
            b'd' => self.f64s.push(val),
            _ => return fail(),
        }
        Ok(())
    }

    /// Part of `ufbxi_ascii_parse_node` in ufbx.c
    fn push_word(&mut self, val: i64, val_f: f64) -> Res<()> {
        if let Some(ints) = &mut self.ints {
            ints.push(val);
        }
        match self.kind {
            b'b' => self.bools.push(val != 0),
            b'c' => self.u8s.push(val as u8),
            b'i' => self.i32s.push(val as i32),
            b'l' => self.i64s.push(val),
            b'f' => self.f32s.push(val_f as f32),
            b'd' => self.f64s.push(val_f),
            _ => return fail(),
        }
        Ok(())
    }

    fn finish(self, info_kind: u8) -> PArr<'a> {
        let ints = self.ints.clone();
        let arr = self.finish_typed(info_kind);
        match (arr, ints) {
            (PArr::Bool(b), Some(i)) if b.len() == i.len() => PArr::BoolInts(b, i),
            (arr, _) => arr,
        }
    }

    /// Part of `ufbxi_ascii_parse_node` in ufbx.c
    fn finish_typed(self, info_kind: u8) -> PArr<'a> {
        match self.kind {
            b'b' => PArr::Bool(self.bools),
            b'c' => {
                if info_kind == b'b' {
                    PArr::Bool(self.u8s.into_iter().map(|x| x != 0).collect())
                } else {
                    PArr::U8(self.u8s)
                }
            }
            b'i' => PArr::I32(self.i32s),
            b'l' => PArr::I64(self.i64s),
            b'f' => PArr::F32(self.f32s),
            b'd' => PArr::F64(self.f64s),
            b's' | b'S' => PArr::Str(self.strs),
            b'C' => PArr::Content(self.content),
            _ => PArr::Ignored,
        }
    }
}
fn empty_array<'a>(dst: u8, kind: u8) -> PArr<'a> {
    match dst {
        b'c' if kind == b'b' => PArr::Bool(Vec::new()),
        b'b' => PArr::Bool(Vec::new()),
        b'c' => PArr::U8(Vec::new()),
        b'i' => PArr::I32(Vec::new()),
        b'l' => PArr::I64(Vec::new()),
        b'f' => PArr::F32(Vec::new()),
        b'd' | b'r' => PArr::F64(Vec::new()),
        b's' | b'S' => PArr::Str(Vec::new()),
        b'C' => PArr::Content(Vec::new()),
        _ => PArr::Ignored,
    }
}
fn array_type_size(t: u8) -> usize {
    match t {
        b'c' | b'b' => 1,
        b'i' | b'f' => 4,
        b'l' | b'd' => 8,
        _ => 1,
    }
}
/// Past what ufbx's inflater writes from `input` bytes, 1032 a byte and its zero padding: a larger destination decodes alike
pub(crate) fn inflate_bound(input: usize) -> usize {
    input.saturating_add(256).saturating_mul(1032).saturating_add(1024)
}
/// Up to 29 digits after an optional sign, wrapping as ufbx's 64-bit arithmetic does; the end is `None` where no digit follows
/// or the digits run to 30
pub(crate) fn parse_int64(s: &[u8]) -> (i64, Option<usize>) {
    let negative = s.first() == Some(&b'-');
    let positive = s.first() == Some(&b'+');
    let init = if negative || positive { 1 } else { 0 };
    let mut len = init;
    let mut abs: u64 = 0;
    while len < 30 {
        let Some(&c) = s.get(len) else { break };
        if !c.is_ascii_digit() {
            break;
        }
        abs = abs.wrapping_mul(10).wrapping_add((c - b'0') as u64);
        len += 1;
    }
    if len == 30 || len == init {
        return (0, None);
    }
    (if negative { 0u64.wrapping_sub(abs) as i64 } else { abs as i64 }, Some(len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_read_as_ufbx_reads_them() {
        assert_eq!(parse_int64(b"-12"), (-12, Some(3)));
        assert_eq!(parse_int64(b"-"), (0, None));
    }
}
