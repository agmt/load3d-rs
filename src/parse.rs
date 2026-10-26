use std::sync::Arc;

use crate::error::{Error, Res, WarningType, Warnings, check, fail, fail_msg, utf8_valid_length};
use crate::inflate::{InflateInput, InflateRetain, inflate};
use crate::types::{Progress, ProgressFn, ProgressResult, UnicodeErrorHandling};

pub(crate) const MAX_NON_ARRAY_VALUES: usize = 8;
pub(crate) const MAX_NODE_DEPTH: u32 = 32;
pub const BINARY_MAGIC: &[u8; 22] = b"Kaydara FBX Binary  \x00\x1a";
const BINARY_HEADER_SIZE: usize = 27;
const MAX_SKIP_SIZE: u64 = 0x4000_0000;

/// `ufbxi_sanitized_string` in ufbx.c: The bytes as in the file and, unless the node keeps its strings raw, the UTF-8 repaired as `unicode_error_handling` says
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct SStr {
    pub raw: Vec<u8>,
    pub utf8: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PVal {
    /// A slot whose value was an array among plain values: ufbx reads past it and types the slot as none
    None,
    Num {
        f: f64,
        i: i64,
    },
    Str(SStr),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PArr {
    Bool(Vec<bool>),
    U8(Vec<u8>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    /// 's' (raw) and 'S' (sanitized) string arrays
    Str(Vec<SStr>),
    /// 'C': embedded content, decoded from base64 in ASCII files
    Content(Vec<Vec<u8>>),
    /// '-': read past, as the options ignore it
    Ignored,
    /// Beyond ufbx: booleans as ufbx reads them, with the integers they were read from
    BoolInts(Vec<bool>, Vec<i64>),
}

impl PArr {
    pub fn len(&self) -> usize {
        match self {
            PArr::Bool(v) => v.len(),
            PArr::U8(v) => v.len(),
            PArr::I32(v) => v.len(),
            PArr::I64(v) => v.len(),
            PArr::F32(v) => v.len(),
            PArr::F64(v) => v.len(),
            PArr::Str(v) => v.len(),
            PArr::Content(v) => v.len(),
            PArr::Ignored => 0,
            PArr::BoolInts(v, _) => v.len(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PData {
    Vals(Vec<PVal>),
    Arr(PArr),
}

/// `ufbxi_node` in ufbx.c
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PNode {
    pub name: String,
    /// The name's bytes where they are no valid UTF-8, which the DOM repairs as ufbx's string pool does
    pub raw_name: Option<Vec<u8>>,
    /// That repair, made where ufbx keeps the node in its DOM, as the node is parsed
    pub dom_name: Option<String>,
    pub data: PData,
    pub children: Vec<PNode>,
}

impl PNode {
    pub fn find_child(&self, name: &str) -> Option<&PNode> {
        self.children.iter().find(|c| c.name == name)
    }

    pub fn vals(&self) -> &[PVal] {
        match &self.data {
            PData::Vals(v) => v,
            PData::Arr(_) => &[],
        }
    }

    pub fn get_array(&self) -> Option<&PArr> {
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
    fn sstr(&self, ix: usize) -> Option<&SStr> {
        match self.vals().get(ix)? {
            PVal::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Part of `ufbxi_get_val_at` in ufbx.c: `None` for strings the parser kept raw
    pub fn str_at(&self, ix: usize) -> Option<&str> {
        self.sstr(ix)?.utf8.as_deref()
    }
    /// Part of `ufbxi_get_val_at` in ufbx.c
    pub fn raw_at(&self, ix: usize) -> Option<&[u8]> {
        Some(&self.sstr(ix)?.raw)
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
    pub fn find_array(&self, name: &str) -> Option<&PArr> {
        self.find_child(name)?.get_array()
    }
}

/// `ufbxi_parse_state` in ufbx.c
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Root,
    FbxHeaderExtension,
    SceneInfo,
    Thumbnail,
    Definitions,
    Objects,
    Connections,
    Relations,
    Takes,
    FbxVersion,
    Model,
    Geometry,
    NodeAttribute,
    LegacyModel,
    LegacyMedia,
    LegacyVideo,
    LegacySwitcher,
    LegacyScenePersistence,
    References,
    Reference,
    AnimationCurve,
    Deformer,
    AssociateModel,
    LegacyLink,
    Pose,
    PoseNode,
    Texture,
    Video,
    LayeredTexture,
    SelectionNode,
    Collection,
    Audio,
    UnknownObject,
    LayerElementNormal,
    LayerElementBinormal,
    LayerElementTangent,
    LayerElementUv,
    LayerElementColor,
    LayerElementVertexCrease,
    LayerElementEdgeCrease,
    LayerElementSmoothing,
    LayerElementVisibility,
    LayerElementPolygonGroup,
    LayerElementHole,
    LayerElementMaterial,
    LayerElementOther,
    GeometryUvInfo,
    Shape,
    Take,
    TakeObject,
    Channel,
    Unknown,
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

#[derive(Debug, Clone, Default)]
pub(crate) struct ParseOpts {
    pub ignore_geometry: bool,
    pub ignore_animation: bool,
    pub ignore_embedded: bool,
    pub retain_dom: bool,
    pub retain_vertex_w: bool,
    pub strict: bool,
    pub unicode_error_handling: UnicodeErrorHandling,
    /// 0 for none: the bytes all arrays together may unpack to
    pub memory_limit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokKind {
    End,
    Name,
    BareWord,
    Int,
    Float,
    Str,
    Char(u8),
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokKind,
    text: Vec<u8>,
    negative: bool,
    i64: i64,
    f64: f64,
}

impl Default for Token {
    fn default() -> Token {
        Token { kind: TokKind::End, text: Vec::new(), negative: false, i64: 0, f64: 0.0 }
    }
}

#[derive(Debug, Default)]
struct Ascii {
    token: Token,
    prev: Token,
    read_first_comment: bool,
    found_version: bool,
    parse_as_f32: bool,
}

/// ufbx's reading around the parser's position, replayed over the input held whole, so that refills and their failures and
/// the progress reports fall where ufbx's do
#[derive(Default)]
pub(crate) struct Io {
    pub stream: bool,
    /// The stream can skip without reading (a file's `fseek`, past its end without failing)
    pub skip_fn: bool,
    pub io_error_at: Option<usize>,
    pub stream_pos: usize,
    pub data_size: usize,
    pub yield_size: usize,
    pub read_buffer_size: usize,
    pub opts_read_buffer_size: usize,
    pub eof: bool,
    pub data_offset: u64,
    /// The position `data_begin` stands for: the read offset is `data_offset + (pos - buf_begin)`
    pub buf_begin: usize,
    pub progress: Option<Arc<ProgressFn>>,
    pub progress_interval: usize,
    pub latest_progress_bytes: u64,
    pub progress_bytes_total: u64,
    pub src_yield: usize,
    pub src_end: usize,
    /// A failure ufbx records and reads on past, the text reader's cancel or IO error: the load's error if it fails
    pub pending: Option<Error>,
}

impl Io {
    /// Part of `ufbx_load_memory` in ufbx.c
    pub fn memory(len: usize) -> Io {
        Io { data_size: len, stream_pos: len, progress_bytes_total: len as u64, opts_read_buffer_size: 0x4000, progress_interval: usize::MAX, ..Io::default() }
    }

    /// Part of `ufbx_load_stream_prefix` in ufbx.c
    pub fn stream(prefix: usize, skip_fn: bool) -> Io {
        Io { stream: true, skip_fn, data_size: prefix, stream_pos: prefix, opts_read_buffer_size: 0x4000, progress_interval: usize::MAX, ..Io::default() }
    }

    /// Part of `ufbxi_load` in ufbx.c
    pub fn set_opts(&mut self, read_buffer_size: usize, progress: Option<Arc<ProgressFn>>, interval_hint: u64) {
        self.opts_read_buffer_size = read_buffer_size;
        self.progress_interval = match &progress {
            None => usize::MAX,
            Some(_) if interval_hint >= usize::MAX as u64 => usize::MAX,
            Some(_) if interval_hint > 0 => interval_hint as usize,
            Some(_) => 0x4000,
        };
        self.progress = progress;
    }

    fn stream_read(&mut self, len: usize, n: usize) -> Res<usize> {
        let end = self.io_error_at.map_or(len, |at| at.min(len));
        if self.io_error_at.is_some_and(|at| self.stream_pos >= at) {
            return fail_msg("IO error");
        }
        let got = n.min(end.saturating_sub(self.stream_pos));
        self.stream_pos += got;
        Ok(got)
    }
}

/// ufbx's error keeps the first description it is given
pub(crate) fn first_described(first: Error, then: Error) -> Error {
    if first.description.is_empty() && !then.description.is_empty() { then } else { first }
}

/// With ufbx's lazy top-level reading: the reader asks for a top-level node by name and gets it, the nodes before it kept for
/// later requests
pub(crate) struct Parser<'a> {
    pub data: &'a [u8],
    pos: usize,
    pub version: u32,
    pub from_ascii: bool,
    pub big_endian: bool,
    pub sure_fbx: bool,
    pub ascii_blender: bool,
    pub opts: ParseOpts,
    pub blender_full_weights: bool,
    pub warnings: Warnings,
    ascii: Ascii,
    has_next_child: bool,
    pub top_nodes: Vec<PNode>,
    pub parsed_to_end: bool,
    array_bytes: usize,
    pub io: Io,
    lazy: Option<(usize, State)>,
    pub dom_children: std::collections::HashMap<usize, Vec<PNode>>,
    inflate_retain: InflateRetain,
}

pub(crate) fn f64_to_i64(v: f64) -> i64 {
    if v.abs() <= i64::MAX as f64 {
        v as i64
    } else if v >= 0.0 {
        i64::MAX
    } else {
        i64::MIN
    }
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
    pub fn new(data: &'a [u8], opts: ParseOpts) -> Parser<'a> {
        Parser {
            data,
            pos: 0,
            version: 0,
            from_ascii: false,
            big_endian: false,
            sure_fbx: false,
            ascii_blender: false,
            opts,
            blender_full_weights: false,
            warnings: Warnings::default(),
            ascii: Ascii::default(),
            has_next_child: false,
            top_nodes: Vec::new(),
            parsed_to_end: false,
            array_bytes: 0,
            io: Io::memory(data.len()),
            lazy: None,
            dom_children: std::collections::HashMap::new(),
            inflate_retain: InflateRetain::new(),
        }
    }

    /// No later `FBXVersion` changes the version
    pub fn set_found_version(&mut self) {
        self.ascii.found_version = true;
    }

    fn push_sanitized_string(&mut self, raw: &[u8], keep_raw: bool) -> Res<SStr> {
        if raw.is_empty() {
            return Ok(SStr { raw: Vec::new(), utf8: Some(String::new()) });
        }
        let utf8 = if keep_raw { None } else { Some(self.utf8(raw)?) };
        Ok(SStr { raw: raw.to_vec(), utf8 })
    }

    /// Part of `ufbxi_push_string_imp` in ufbx.c
    pub fn utf8(&mut self, raw: &[u8]) -> Res<String> {
        let valid = utf8_valid_length(raw);
        if valid == raw.len() {
            return Ok(String::from_utf8(raw.to_vec()).unwrap_or_default());
        }
        push_sanitized_string_entry(raw, valid, self.opts.unicode_error_handling, &mut self.warnings)
    }

    pub fn get_read_offset(&self) -> u64 {
        self.io.data_offset.wrapping_add(self.pos.wrapping_sub(self.io.buf_begin) as u64)
    }

    fn byte(&self, at: usize) -> u8 {
        self.data.get(at).copied().unwrap_or(0)
    }

    fn window(&self, from: usize, to: usize) -> &'a [u8] {
        let to = to.min(self.data.len());
        &self.data[from.min(to)..to]
    }

    pub fn buffered(&self) -> &'a [u8] {
        let end = (self.pos + self.io.data_size).min(self.data.len());
        &self.data[self.pos.min(end)..end]
    }

    pub fn report_progress(&mut self) -> Res<()> {
        let Some(cb) = self.io.progress.clone() else { return Ok(()) };
        let bytes_read = self.get_read_offset();
        self.io.latest_progress_bytes = bytes_read;
        let progress = Progress { bytes_read, bytes_total: self.io.progress_bytes_total.max(bytes_read) };
        check!(cb(&progress) != ProgressResult::Cancel, "Cancelled");
        Ok(())
    }

    fn stream_read(&mut self, n: usize) -> Res<usize> {
        self.io.stream_read(self.data.len(), n)
    }

    pub fn refill(&mut self, size: usize, require_size: bool) -> Res<()> {
        check!(!self.io.eof);
        if require_size {
            check!(self.io.stream || self.io.data_size > 0, "Empty file");
            check!(self.io.stream, "Truncated file");
        } else if !self.io.stream {
            self.io.eof = true;
            return Ok(());
        }
        if size > self.io.read_buffer_size {
            self.io.read_buffer_size = size.max(self.io.opts_read_buffer_size).max(self.io.read_buffer_size.saturating_mul(2));
        }
        let mut data_size = self.io.data_size;
        let capacity = self.io.read_buffer_size;
        while data_size < capacity {
            let read = self.stream_read(capacity - data_size)?;
            data_size += read;
            if read == 0 {
                self.io.eof = true;
                break;
            }
        }
        if require_size {
            if self.io.data_offset == 0 {
                check!(data_size > 0, "Empty file");
            }
            check!(data_size >= size, "Truncated file");
        }
        self.io.data_offset = self.get_read_offset();
        self.io.buf_begin = self.pos;
        self.io.data_size = data_size;
        Ok(())
    }

    /// `ufbxi_yield` in ufbx.c
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

    /// `yield_size` is left as it is
    #[cfg(feature = "format-obj")]
    pub fn take_buffered(&mut self, size: usize) {
        self.pos += size;
        self.io.data_size -= size;
    }

    fn consume_bytes(&mut self, size: usize) {
        self.io.yield_size -= size;
        self.pos += size;
    }

    pub fn pause_progress(&mut self) {
        self.io.data_size += self.io.yield_size;
        self.io.yield_size = 0;
    }

    pub fn resume_progress(&mut self) -> Res<()> {
        self.io.yield_size = self.io.data_size.min(self.io.progress_interval);
        self.io.data_size -= self.io.yield_size;
        if self.get_read_offset().wrapping_sub(self.io.latest_progress_bytes) >= self.io.progress_interval as u64 {
            self.report_progress()?;
        }
        Ok(())
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
        let a = [b[0], b[1], b[2], b[3]];
        if self.big_endian { u32::from_be_bytes(a) } else { u32::from_le_bytes(a) }
    }

    /// With `ufbxi_swap_endian` in ufbx.c
    fn read_u64(&self, b: &[u8]) -> u64 {
        let a = [b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]];
        if self.big_endian { u64::from_be_bytes(a) } else { u64::from_le_bytes(a) }
    }

    /// With `ufbxi_swap_endian` in ufbx.c
    fn read_i16(&self, b: &[u8]) -> i16 {
        let a = [b[0], b[1]];
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
    fn binary_parse_node(&mut self, depth: u32, parent: State, recursive: bool) -> Res<Option<PNode>> {
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
        let name = String::from_utf8_lossy(name_bytes).into_owned();
        let raw_name = (utf8_valid_length(name_bytes) < name_bytes.len()).then(|| name_bytes.to_vec());
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
    fn binary_tail(&mut self, name: String, data: PData, values_end: u64, end_offset: u64, depth: u32, parent: State, recursive: bool) -> Res<Option<PNode>> {
        let offset = self.get_read_offset();
        check!(offset <= values_end);
        if offset < values_end {
            self.skip_bytes(values_end - offset)?;
        }
        let mut children = Vec::new();
        if recursive {
            let state = update_parse_state(parent, &name);
            loop {
                let current = self.get_read_offset();
                if current >= end_offset {
                    check!(current == end_offset || end_offset == 0);
                    break;
                }
                match self.binary_parse_node(depth + 1, state, true)? {
                    Some(child) => children.push(child),
                    None => break,
                }
            }
        } else {
            self.has_next_child = self.get_read_offset() < end_offset;
        }
        Ok(Some(PNode { name, raw_name: None, dom_name: None, data, children }))
    }

    /// Part of `ufbxi_binary_parse_node` in ufbx.c
    fn binary_array(&mut self, info: ArrayInfo, num_values: u32) -> Res<PArr> {
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
                let bytes: std::borrow::Cow<[u8]> = match encoding {
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
                            std::borrow::Cow::Borrowed(bytes)
                        } else {
                            std::borrow::Cow::Borrowed(self.read_to(encoded_size)?)
                        }
                    }
                    1 => std::borrow::Cow::Owned(self.inflate_array(encoded_size, decoded_size, arr_begin, arr_end)?),
                    _ => return fail(),
                };
                let arr = self.binary_convert_array(src, dst, &bytes, size, info.kind)?;
                if info.keep_ints
                    && let PArr::Bool(bools) = &arr
                    && let PArr::I64(ints) = self.binary_convert_array(src, b'l', &bytes, size, b'l')?
                {
                    return Ok(PArr::BoolInts(bools.clone(), ints));
                }
                Ok(arr)
            }
            // An empty boolean array keeps the byte type, which the readers asking for booleans do not take
            b'0' | b'-' => Ok(if c == b'-' { PArr::Ignored } else { empty_array(dst, dst) }),
            _ => self.binary_parse_multivalue_array(dst, info.kind, num_values as usize),
        }
    }

    /// Part of `ufbxi_binary_parse_node` in ufbx.c: Its DEFLATE branch; the buffer is handed to the inflater, the rest read from
    /// the stream as it asks, the stream left where its reads stopped
    fn inflate_array(&mut self, encoded_size: usize, decoded_size: usize, arr_begin: u64, arr_end: u64) -> Res<Vec<u8>> {
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
        let mut decoded = vec![0u8; decoded_size.min(inflate_bound(available))];
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
        let res = inflate(&mut decoded, &mut input, &mut self.inflate_retain);
        check!(res != -28, "Cancelled");
        check!(res == decoded_size as isize, "Bad DEFLATE data");
        Ok(decoded)
    }

    fn binary_convert_array(&self, src: u8, dst: u8, b: &[u8], size: usize, kind: u8) -> Res<PArr> {
        macro_rules! read_all {
            ($n:expr, $f:expr) => {{
                let mut v = Vec::with_capacity(size);
                for i in 0..size {
                    v.push($f(&b[i * $n..i * $n + $n]));
                }
                v
            }};
        }
        // `ufbxi_swap_endian_array` in ufbx.c
        let i32s = |p: &[u8]| self.read_u32(p) as i32;
        let i64s = |p: &[u8]| self.read_u64(p) as i64;
        let f32s = |p: &[u8]| self.read_f32(p);
        let f64s = |p: &[u8]| self.read_f64(p);
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
    fn binary_parse_multivalue_array(&mut self, dst: u8, kind: u8, size: usize) -> Res<PArr> {
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
                    let s = if raw { SStr { raw: s.to_vec(), utf8: None } } else { self.push_sanitized_string(s, false)? };
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
        let prev = std::mem::take(&mut self.ascii.token);
        self.ascii.prev = prev;
        let mut tok = Token::default();
        let read = self.read_token(&mut tok);
        self.ascii.token = tok;
        read
    }

    /// Part of `ufbxi_ascii_next_token` in ufbx.c: `tok` filled as ufbx fills its token in place, so that a failure leaves it as
    /// far as it got
    fn read_token(&mut self, tok: &mut Token) -> Res<()> {
        let mut c = self.ascii_skip_whitespace();
        if c.is_ascii_alphabetic() || c == b'_' {
            tok.kind = TokKind::BareWord;
            while c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'(' | b')') {
                tok.text.push(c);
                c = self.ascii_next();
            }
            c = self.ascii_skip_whitespace();
            if c == b':' {
                tok.kind = TokKind::Name;
                self.ascii_next();
            }
        } else if c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.') {
            tok.kind = TokKind::Int;
            tok.negative = c == b'-';
            while c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E') {
                if matches!(c, b'.' | b'e' | b'E') {
                    tok.kind = TokKind::Float;
                }
                tok.text.push(c);
                c = self.ascii_next();
            }
            let mut nan_like = false;
            while c.is_ascii_alphanumeric() || matches!(c, b'#' | b'(' | b')') {
                nan_like = true;
                tok.text.push(c);
                c = self.ascii_next();
            }
            if nan_like {
                tok.kind = TokKind::Float;
            }
            if tok.kind == TokKind::Int {
                let (v, end) = parse_int64(&tok.text);
                tok.i64 = v;
                check!(end == Some(tok.text.len()));
            } else {
                let (v, end) = parse_double(&tok.text, self.ascii.parse_as_f32);
                tok.f64 = v;
                check!(end == Some(tok.text.len()));
            }
        } else if c == b'"' {
            tok.kind = TokKind::Str;
            c = self.ascii_next();
            while c != b'"' {
                // ufbx copies the run up to the next '"' or '&' out of its buffer whole, NULs and all, while two bytes
                // or more are left in the window
                if self.pos + 1 < self.io.src_yield {
                    let end = (self.pos..self.io.src_yield).find(|&at| matches!(self.byte(at), b'"' | b'&')).unwrap_or(self.io.src_yield);
                    if self.pos < end {
                        tok.text.extend((self.pos..end).map(|at| self.byte(at)));
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
                        tok.text.push(replacement);
                    } else {
                        tok.text.extend_from_slice(&entity[..step]);
                    }
                    continue;
                }
                check!(c != 0);
                tok.text.push(c);
                c = self.ascii_next();
            }
            let next = self.ascii_next();
            if next == b':' {
                tok.kind = TokKind::Name;
                self.ascii_next();
            }
        } else if c == 0 {
            // ufbx's end is a token of one character too, stepped past like any other
            tok.kind = TokKind::End;
            self.ascii_next();
        } else {
            tok.kind = TokKind::Char(c);
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

    fn ascii_parse_node(&mut self, depth: u32, parent: State, recursive: bool) -> Res<Option<PNode>> {
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
        let name = String::from_utf8_lossy(&self.ascii.prev.text).into_owned();
        let raw_name = (utf8_valid_length(&self.ascii.prev.text) < self.ascii.prev.text.len()).then(|| self.ascii.prev.text.clone());

        let mut in_array = false;
        let mut num_values: i64 = 0;
        let mut vals: Vec<PVal> = Vec::new();

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
                let text = std::mem::take(&mut self.ascii.prev.text);
                if arr_type != 0 {
                    match arr_type {
                        b's' | b'S' => {
                            let s = if arr_type == b's' { SStr { utf8: None, raw: text } } else { self.push_sanitized_string(&text, false)? };
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
                    let s = self.push_sanitized_string(&text, raw)?;
                    vals.push(PVal::Str(s));
                }
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

        let data = if arr_type != 0 {
            if arr_type == b'-' {
                PData::Arr(PArr::Ignored)
            } else if arr_error {
                PData::Arr(empty_array(arr_type, info.map(|i| i.kind).unwrap_or(arr_type)))
            } else {
                PData::Arr(acc.finish(info.map(|i| i.kind).unwrap_or(arr_type)))
            }
        } else {
            PData::Vals(vals)
        };

        let mut children = Vec::new();
        if self.ascii_accept(TokKind::Char(b'{'))? {
            if recursive {
                while let Some(child) = self.ascii_parse_node(depth + 1, state, recursive)? {
                    children.push(child);
                }
            }
            self.has_next_child = true;
        } else {
            self.has_next_child = false;
        }
        Ok(Some(PNode { name, raw_name, dom_name: None, data, children }))
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
    fn toplevel_head(&mut self) -> Res<Option<PNode>> {
        if self.from_ascii { self.ascii_parse_node(0, State::Root, false) } else { self.binary_parse_node(0, State::Root, false) }
    }

    fn parse_toplevel_child_imp(&mut self, state: State) -> Res<Option<PNode>> {
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
    pub fn next_lazy_child(&mut self) -> Res<Option<PNode>> {
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
    pub fn keep_child(&mut self, ix: usize, child: PNode) {
        if self.opts.retain_dom {
            self.dom_children.entry(ix).or_default().push(child);
        }
    }

    /// Part of `ufbxi_parse_toplevel` in ufbx.c: Reading on
    pub fn next_toplevel(&mut self) -> Res<Option<PNode>> {
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
    pub fn parse_legacy_toplevel(&mut self) -> Res<Option<PNode>> {
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

struct Acc {
    kind: u8,
    ints: Option<Vec<i64>>,
    bools: Vec<bool>,
    u8s: Vec<u8>,
    i32s: Vec<i32>,
    i64s: Vec<i64>,
    f32s: Vec<f32>,
    f64s: Vec<f64>,
    strs: Vec<SStr>,
    content: Vec<Vec<u8>>,
}

impl Acc {
    fn new(kind: u8) -> Acc {
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

    fn finish(self, info_kind: u8) -> PArr {
        let ints = self.ints.clone();
        let arr = self.finish_typed(info_kind);
        match (arr, ints) {
            (PArr::Bool(b), Some(i)) if b.len() == i.len() => PArr::BoolInts(b, i),
            (arr, _) => arr,
        }
    }

    /// Part of `ufbxi_ascii_parse_node` in ufbx.c
    fn finish_typed(self, info_kind: u8) -> PArr {
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

fn empty_array(dst: u8, kind: u8) -> PArr {
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

/// `ufbxi_push_sanitized_string_entry` from the first invalid byte on
pub(crate) fn push_sanitized_string_entry(raw: &[u8], valid: usize, handling: UnicodeErrorHandling, warnings: &mut Warnings) -> Res<String> {
    check!(handling != UnicodeErrorHandling::AbortLoading, "Invalid UTF-8");
    warnings.warn(WarningType::BadUnicode, crate::error::NO_INDEX, "Bad UTF-8 string");
    let mut out: Vec<u8> = raw[..valid].to_vec();
    let mut index = valid;
    while index < raw.len() {
        let n = utf8_valid_length(&raw[index..]);
        if n > 0 {
            out.extend_from_slice(&raw[index..index + n]);
            index += n;
            continue;
        }
        // `ufbxi_add_replacement_char` in ufbx.c
        match handling {
            UnicodeErrorHandling::ReplacementCharacter => out.extend_from_slice(&[0xef, 0xbf, 0xbd]),
            UnicodeErrorHandling::Underscore => out.push(b'_'),
            UnicodeErrorHandling::QuestionMark => out.push(b'?'),
            UnicodeErrorHandling::Remove => {}
            UnicodeErrorHandling::UnsafeIgnore => out.push(raw[index]),
            UnicodeErrorHandling::AbortLoading => {}
        }
        index += 1;
    }
    Ok(match String::from_utf8(out) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
    })
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

fn scan_ignorecase(s: &[u8], at: usize, word: &[u8]) -> bool {
    word.iter().enumerate().all(|(k, w)| s.get(at + k).is_some_and(|c| (c | 0x20) == *w))
}

pub(crate) fn parse_inf_nan(s: &[u8]) -> (f64, Option<usize>) {
    let mut p = 0;
    let mut negative = false;
    if let Some(&c) = s.first()
        && (c == b'+' || c == b'-')
    {
        negative = c == b'-';
        p += 1;
    }
    let top: u64;
    if s.len() - p >= 3 && s[p].is_ascii_digit() && s[p + 1] == b'.' && s[p + 2] == b'#' {
        p += 3;
        if scan_ignorecase(s, p, b"inf") {
            p += 3;
            top = 0x7ff0;
        } else if scan_ignorecase(s, p, b"nan") || scan_ignorecase(s, p, b"ind") {
            p += 3;
            top = 0x7ff8;
        } else {
            return (0.0, None);
        }
        while p < s.len() && s[p].is_ascii_digit() {
            p += 1;
        }
    } else if scan_ignorecase(s, p, b"nan") {
        p += 3;
        top = 0x7ff8;
        if p < s.len() && s[p] == b'(' {
            p += 1;
            while p < s.len() && s[p] != b')' {
                if !s[p].is_ascii_alphanumeric() {
                    return (0.0, None);
                }
                p += 1;
            }
            if p == s.len() {
                return (0.0, None);
            }
            p += 1;
        }
    } else if scan_ignorecase(s, p, b"inf") {
        p += if scan_ignorecase(s, p + 3, b"inity") { 8 } else { 3 };
        top = 0x7ff0;
    } else {
        top = 0;
    }
    let bits = (top | if negative { 0x8000 } else { 0 }) << 48;
    (f64::from_bits(bits), Some(p))
}

/// Clinger's fast path: where the digits and the power of ten are both exact in the type, its one multiplication or
/// division rounds as Rust's parser rounds
fn parse_exact(mantissa: &[u8], exp: i64, as_f32: bool) -> Option<f64> {
    let (mut m, mut digits, mut fraction, mut dot) = (0u64, 0, 0i64, false);
    for &c in mantissa {
        if c == b'.' {
            dot = true;
            continue;
        }
        digits += 1;
        if digits > 19 {
            return None;
        }
        m = m * 10 + (c - b'0') as u64;
        fraction += dot as i64;
    }
    let e = exp - fraction;
    const P64: [f64; 23] = [1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16, 1e17, 1e18, 1e19, 1e20, 1e21, 1e22];
    const P32: [f32; 11] = [1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10];
    if as_f32 {
        if m > 1 << 24 || e.unsigned_abs() > 10 {
            return None;
        }
        let (v, p) = (m as f32, P32[e.unsigned_abs() as usize]);
        return Some(if e < 0 { v / p } else { v * p } as f64);
    }
    if m > 1 << 53 || e.unsigned_abs() > 22 {
        return None;
    }
    let (v, p) = (m as f64, P64[e.unsigned_abs() as usize]);
    Some(if e < 0 { v / p } else { v * p })
}

/// ufbx's grammar, the digits rounded correctly (to binary32 where `as_f32`)
pub(crate) fn parse_double(s: &[u8], as_f32: bool) -> (f64, Option<usize>) {
    let mut p = 0;
    let mut negative = false;
    if let Some(&c) = s.first()
        && (c == b'+' || c == b'-')
    {
        negative = c == b'-';
        p += 1;
    }
    let mantissa_begin = p;
    let mut has_dot = false;
    let mut any_digit = false;
    while p < s.len() {
        let c = s[p];
        if c.is_ascii_digit() {
            any_digit = true;
            p += 1;
        } else if c == b'.' && !has_dot {
            has_dot = true;
            p += 1;
        } else {
            break;
        }
    }
    let mantissa_end = p;
    let mut exp: i64 = 0;
    let mut exp_negative = false;
    if p < s.len() && (s[p] == b'e' || s[p] == b'E') {
        p += 1;
        if p < s.len() && (s[p] == b'+' || s[p] == b'-') {
            exp_negative = s[p] == b'-';
            p += 1;
        }
        while p < s.len() {
            let c = s[p];
            if c.is_ascii_digit() {
                p += 1;
                exp = exp * 10 + (c - b'0') as i64;
                if exp >= 10000 {
                    break;
                }
            } else {
                break;
            }
        }
    }
    if p < s.len() && matches!(s[p], b'#' | b'i' | b'I' | b'n' | b'N') {
        let (v, end) = parse_inf_nan(s);
        if end.is_some() {
            return (v, end);
        }
    }
    if !any_digit {
        return (if negative { -0.0 } else { 0.0 }, Some(p));
    }
    if let Some(v) = parse_exact(&s[mantissa_begin..mantissa_end], if exp_negative { -exp } else { exp }, as_f32) {
        return (if negative { -v } else { v }, Some(p));
    }
    // the digits rewritten as Rust's parser takes them, on the stack unless they are many
    let mantissa = &s[mantissa_begin..mantissa_end];
    let mut stack = [0u8; 64];
    let mut heap = Vec::new();
    let text: &mut [u8] = if mantissa.len() + 16 <= stack.len() {
        &mut stack
    } else {
        heap.resize(mantissa.len() + 16, 0);
        &mut heap
    };
    let mut n = 0;
    let mut put = |bytes: &[u8]| {
        text[n..n + bytes.len()].copy_from_slice(bytes);
        n += bytes.len();
    };
    if negative {
        put(b"-");
    }
    if mantissa.first() == Some(&b'.') {
        put(b"0");
    }
    put(mantissa);
    if mantissa.last() == Some(&b'.') {
        put(b"0");
    }
    let exp = if exp_negative { -exp } else { exp };
    if exp != 0 {
        put(b"e");
        let mut digits = [0u8; 8];
        let mut k = digits.len();
        let mut e = exp.unsigned_abs();
        while e > 0 || k == digits.len() {
            k -= 1;
            digits[k] = b'0' + (e % 10) as u8;
            e /= 10;
        }
        if exp < 0 {
            put(b"-");
        }
        put(&digits[k..]);
    }
    let text = std::str::from_utf8(&text[..n]).unwrap_or("0");
    let v = if as_f32 { text.parse::<f32>().map(|v| v as f64).unwrap_or(0.0) } else { text.parse::<f64>().unwrap_or(0.0) };
    (v, Some(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_as_ufbx_reads_them() {
        assert_eq!(parse_double(b"1.5", false), (1.5, Some(3)));
        assert_eq!(parse_double(b"-0", false).0.to_bits(), (-0.0f64).to_bits());
        assert_eq!(parse_double(b"1.", false), (1.0, Some(2)));
        assert_eq!(parse_double(b".25", false), (0.25, Some(3)));
        assert_eq!(parse_double(b"1e3", false), (1000.0, Some(3)));
        assert_eq!(parse_double(b"0.1", true).0, 0.1f32 as f64);
        assert!(parse_double(b"1.#INF", false).0.is_infinite());
        assert!(parse_double(b"-1.#IND", false).0.is_nan());
        assert_eq!(parse_int64(b"-12"), (-12, Some(3)));
        assert_eq!(parse_int64(b"-"), (0, None));
    }

    // Held to Rust's own correctly rounded parsing where ufbx's tests take `strtod` and `strtof`; its big integer tests are left
    // out, as the port has no big integers

    fn reference<T: std::str::FromStr>(s: &str) -> Option<T> {
        (1..=s.len()).rev().find_map(|n| s.get(..n).and_then(|p| p.parse::<T>().ok()))
    }

    fn check_float(s: &str) {
        let ref_d = reference::<f64>(s).unwrap_or(0.0);
        let ref_f = reference::<f32>(s).unwrap_or(0.0);
        let slow_d = parse_double(s.as_bytes(), false).0;
        let slow_f = parse_double(s.as_bytes(), true).0 as f32;
        if ref_d.is_finite() {
            assert!(slow_d == ref_d, "strtod() mismatch: '{s}': reference {ref_d:e}, port {slow_d:e}");
        } else {
            assert!(!slow_d.is_finite() && !slow_f.is_finite(), "{s}");
        }
        if ref_f.is_finite() {
            assert!(slow_f == ref_f, "strtof() mismatch: '{s}': reference {ref_f:e}, port {slow_f:e}");
        } else {
            assert!(!slow_f.is_finite(), "{s}");
        }
    }

    fn check_nan(s: &str) {
        let (d, end_d) = parse_double(s.as_bytes(), false);
        let (f, end_f) = parse_double(s.as_bytes(), true);
        assert!(d.is_nan() && (f as f32).is_nan(), "{s}");
        assert_eq!((end_d, end_f), (Some(s.len()), Some(s.len())), "{s}");
    }

    fn check_inf(s: &str, sign: i32) {
        let (d, end_d) = parse_double(s.as_bytes(), false);
        let (f, end_f) = parse_double(s.as_bytes(), true);
        let f = f as f32;
        assert!(d.is_infinite() && f.is_infinite(), "{s}");
        assert!((d < 0.0) == (sign < 0) && (f < 0.0) == (sign < 0), "{s}");
        assert_eq!((end_d, end_f), (Some(s.len()), Some(s.len())), "{s}");
    }

    #[test]
    fn test_double_parse() {
        let nines = "9".repeat(200);
        let zeros = "0".repeat(200);
        for s in [
            "1",
            "123.0",
            "123.456",
            "123e-6",
            "-1.5",
            "1112223333444.555666777888999",
            "0",
            "-0",
            ".5",
            "-.5",
            "1e100",
            "1e1000",
            "1e-1000",
            "1e10000",
            "1e-10000",
            "7.67844768714563e-239",
        ] {
            check_float(s);
        }
        check_float(&nines);
        check_float(&format!("{nines}.999999999999999999999999999999999999999"));
        for e in ["e+108", "e+109", "e+118", "e+119", "e+120", "e+300", "e-200", "e-400", "e-520", "e-523", "e-524"] {
            check_float(&format!("{nines}{e}"));
        }
        check_float(&format!("{zeros}123"));
        check_float(&format!("{zeros}.123"));
        check_float(&format!("{zeros}.{zeros}123"));
        check_float(&format!("{zeros}.{zeros}123{zeros}"));
        check_float(&format!("{zeros}.{zeros}{zeros}123"));
        check_float("241309881603643e20");
        check_float(".5.57999999993498");
        check_float("-71862.4328795732984723456847839347829321867347892347893274982374982349872136217381623872E-273");
        check_float("4656612873077392578125e-8");
    }

    #[test]
    fn test_double_parse_nan() {
        for s in ["nan", "NAN", "NaN", "-nan", "+nan", "nan(1234)", "nan(0x123456789abcdef)", "nan(nan)", "nan(ind)", "nan(nans)"] {
            check_nan(s);
        }
        check_inf("inf", 1);
        check_inf("-inf", -1);
        check_inf("INF", 1);
        check_inf("INFINITY", 1);
        for s in ["1.#NAN", "0.#NAN12345678", "1.#IND", "-7.#NAN00"] {
            check_nan(s);
        }
        check_inf("1.#INF", 1);
        check_inf("-1.#INF", -1);
    }

    /// Values about every pattern of their top `bits` bits, printed as `%.*f` (`exponent` false) or `%.*e`
    fn test_double_parse_fmt(exponent: bool, width: usize, bits: u32) {
        for hi in 0..1u32 << bits {
            for delta in -2i32..=2 {
                let val_f = f32::from_bits((hi << (32 - bits)).wrapping_add(delta as u32));
                let val_d = f64::from_bits(((hi as u64) << (64 - bits)).wrapping_add(delta as i64 as u64));
                if val_f.is_finite() {
                    check_float(&if exponent { format!("{val_f:.width$e}") } else { format!("{val_f:.width$}") });
                }
                if val_d.is_finite() {
                    check_float(&if exponent { format!("{val_d:.width$e}") } else { format!("{val_d:.width$}") });
                }
            }
        }
    }

    #[test]
    fn test_double_parse_bits() {
        for width in 4..=12 {
            test_double_parse_fmt(false, width, 12);
        }
        for width in 4..=12 {
            test_double_parse_fmt(true, width, 12);
        }
    }

    /// Decimal digits, the least significant first
    struct BigDecimal(Vec<u8>);

    impl BigDecimal {
        fn bigdecimal_mul(&mut self, k: u32) {
            let mut carry = 0;
            for d in &mut self.0 {
                let product = *d as u32 * k + carry;
                *d = (product % 10) as u8;
                carry = product / 10;
            }
            if carry > 0 {
                self.0.push(carry as u8);
            }
        }

        fn bigdecimal_add(&mut self, addend: i32) {
            let mut carry = addend;
            for d in &mut self.0 {
                let sum = *d as i32 + carry;
                if (0..10).contains(&sum) {
                    *d = sum as u8;
                    carry = 0;
                    break;
                } else if sum < 0 {
                    *d = (sum + 10) as u8;
                    carry = -1;
                } else {
                    *d = (sum % 10) as u8;
                    carry = 1;
                }
            }
            assert!(carry >= 0, "negative bigdecimal");
            if carry > 0 {
                self.0.push(carry as u8);
            }
        }

        /// `bigdecimal_string` and `bigdecimal_suffixf` in test/unit_tests.c
        fn text(&self, suffix: &str) -> String {
            let Some(top) = self.0.iter().rposition(|&d| d != 0) else { return String::new() };
            let mut s: String = self.0[..=top].iter().rev().map(|&d| (b'0' + d) as char).collect();
            s.push_str(suffix);
            s
        }
    }

    /// Long (about 18 million numbers), so ignored, and run in release with the oracle's checks
    #[test]
    #[ignore]
    fn test_double_parse_decimal() {
        let (max_pow2, max_pow5, min_exp, max_exp, max_delta) = (128, 128, -32, 32, 8);
        let mut pow2 = BigDecimal(vec![1]);
        for _ in 0..max_pow2 {
            let mut pow5 = BigDecimal(pow2.0.clone());
            for _ in 0..max_pow5 {
                if pow5.0.len() >= 2 {
                    pow5.bigdecimal_add(-max_delta);
                    for _ in -max_delta..=max_delta {
                        for exp in min_exp..=max_exp {
                            check_float(&pow5.text(&format!("e{exp:+}")));
                        }
                        pow5.bigdecimal_add(1);
                    }
                    pow5.bigdecimal_add(-max_delta - 1);
                } else {
                    for exp in min_exp..=max_exp {
                        check_float(&pow5.text(&format!("e{exp:+}")));
                    }
                }
                pow5.bigdecimal_mul(5);
            }
            pow2.bigdecimal_mul(2);
        }
    }
}
