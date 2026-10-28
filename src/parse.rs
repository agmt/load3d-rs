use std::borrow::Cow;
use std::sync::Arc;

use crate::error::{Error, Res, WarningType, Warnings, check, fail_msg, utf8_valid_length};
use crate::inflate::InflateRetain;
use crate::types::{Progress, ProgressFn, ProgressResult, UnicodeErrorHandling};

#[cfg(feature = "format-fbx")]
mod fbx;

pub const BINARY_MAGIC: &[u8; 22] = b"Kaydara FBX Binary  \x00\x1a";

/// `ufbxi_sanitized_string` in ufbx.c: The bytes as in the file and, unless the node keeps its strings raw, the UTF-8 repaired as `unicode_error_handling` says
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct SStr<'a> {
    /// `None` where the bytes are those of `utf8`, held once
    raw: Option<Cow<'a, [u8]>>,
    utf8: Option<Cow<'a, str>>,
}

#[cfg_attr(not(feature = "format-fbx"), allow(dead_code))]
impl<'a> SStr<'a> {
    pub fn raw_only(raw: impl Into<Cow<'a, [u8]>>) -> SStr<'a> {
        SStr { raw: Some(raw.into()), utf8: None }
    }
    pub fn new(raw: Cow<'a, [u8]>, utf8: String) -> SStr<'a> {
        if *raw == *utf8.as_bytes() { SStr::text(utf8) } else { SStr { raw: Some(raw), utf8: Some(utf8.into()) } }
    }
    pub fn text(utf8: impl Into<Cow<'a, str>>) -> SStr<'a> {
        SStr { raw: None, utf8: Some(utf8.into()) }
    }
    pub fn raw(&self) -> &[u8] {
        match (&self.raw, &self.utf8) {
            (Some(raw), _) => raw,
            (None, Some(utf8)) => utf8.as_bytes(),
            (None, None) => &[],
        }
    }
    pub fn utf8(&self) -> Option<&str> {
        self.utf8.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(not(feature = "format-fbx"), allow(dead_code))]
pub(crate) enum PVal<'a> {
    /// A slot whose value was an array among plain values: ufbx reads past it and types the slot as none
    None,
    Num {
        f: f64,
        i: i64,
    },
    Str(SStr<'a>),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(not(feature = "format-fbx"), allow(dead_code))]
pub(crate) enum PArr<'a> {
    Bool(Vec<bool>),
    U8(Vec<u8>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    /// 's' (raw) and 'S' (sanitized) string arrays
    Str(Vec<SStr<'a>>),
    /// 'C': embedded content, decoded from base64 in ASCII files
    Content(Vec<Vec<u8>>),
    /// '-': read past, as the options ignore it
    Ignored,
    /// Beyond ufbx: booleans as ufbx reads them, with the integers they were read from
    BoolInts(Vec<bool>, Vec<i64>),
}

impl PArr<'_> {
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
#[cfg_attr(not(feature = "format-fbx"), allow(dead_code))]
pub(crate) enum PData<'a> {
    Vals(Vec<PVal<'a>>),
    Arr(PArr<'a>),
}

/// `ufbxi_node` in ufbx.c
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PNode<'a> {
    pub name: Cow<'a, str>,
    /// The name's bytes where they are no valid UTF-8, which the DOM repairs as ufbx's string pool does
    pub raw_name: Option<Cow<'a, [u8]>>,
    /// That repair, made where ufbx keeps the node in its DOM, as the node is parsed
    pub dom_name: Option<String>,
    pub data: PData<'a>,
    pub children: Vec<PNode<'a>>,
}

/// `ufbxi_parse_state` in ufbx.c
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(feature = "format-fbx"), allow(dead_code))]
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

#[derive(Debug, Clone, Default)]
#[cfg_attr(not(feature = "format-fbx"), allow(dead_code))]
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
#[cfg_attr(not(feature = "format-fbx"), allow(dead_code))]
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
#[cfg_attr(not(feature = "format-fbx"), allow(dead_code))]
struct Token {
    kind: TokKind,
    text: Vec<u8>,
    /// Where a name's or a string's `text` lies in the input, unless `replaced`
    span: (usize, usize),
    /// An entity of a string, as `&quot;`, was replaced by its character
    replaced: bool,
    negative: bool,
    i64: i64,
    f64: f64,
}

impl Default for Token {
    fn default() -> Token {
        Token { kind: TokKind::End, text: Vec::new(), span: (0, 0), replaced: false, negative: false, i64: 0, f64: 0.0 }
    }
}

#[derive(Debug, Default)]
#[cfg_attr(not(feature = "format-fbx"), allow(dead_code))]
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
#[cfg_attr(not(feature = "format-fbx"), allow(dead_code))]
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
#[cfg_attr(not(feature = "format-fbx"), allow(dead_code))]
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
    pub top_nodes: Vec<PNode<'a>>,
    pub parsed_to_end: bool,
    array_bytes: usize,
    pub io: Io,
    lazy: Option<(usize, State)>,
    pub dom_children: std::collections::HashMap<usize, Vec<PNode<'a>>>,
    /// The children of the nodes being parsed, each node's taken from its own start on
    node_stack: Vec<PNode<'a>>,
    val_stack: Vec<PVal<'a>>,
    decode_buffer: Vec<u8>,
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
            node_stack: Vec::new(),
            val_stack: Vec::new(),
            decode_buffer: Vec::new(),
            inflate_retain: InflateRetain::new(),
        }
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

    /// `yield_size` is left as it is
    #[cfg(any(feature = "format-obj", feature = "format-3ds"))]
    pub fn take_buffered(&mut self, size: usize) {
        self.pos += size;
        self.io.data_size -= size;
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

#[cfg(any(feature = "format-fbx", feature = "format-obj"))]
fn scan_ignorecase(s: &[u8], at: usize, word: &[u8]) -> bool {
    word.iter().enumerate().all(|(k, w)| s.get(at + k).is_some_and(|c| (c | 0x20) == *w))
}

#[cfg(any(feature = "format-fbx", feature = "format-obj"))]
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
#[cfg(any(feature = "format-fbx", feature = "format-obj"))]
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
#[cfg(any(feature = "format-fbx", feature = "format-obj"))]
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

#[cfg(all(test, any(feature = "format-fbx", feature = "format-obj")))]
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
