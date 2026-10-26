use std::collections::HashMap;
use std::fmt::{self, Display, Write as _};

#[derive(Clone, Copy)]
enum Seg {
    F(&'static str),
    I(usize),
}

#[derive(Debug, Clone)]
pub struct Diff {
    pub path: String,
    pub c: String,
    pub p: String,
}

/// C DOM node address -> the child indices leading to it from `dom_root`, the port's `DomRef`
pub struct DomMap {
    map: HashMap<usize, Vec<u32>>,
}

impl DomMap {
    pub fn new(scene: &ufbx::Scene) -> DomMap {
        let mut map = HashMap::new();
        let root = raw_ptr(&scene.dom_root) as *const ufbx::DomNode;
        if !root.is_null() {
            let mut stack = vec![(root, Vec::new())];
            while let Some((n, path)) = stack.pop() {
                let node = unsafe { &*n };
                for (i, &ch) in reflist(&node.children).iter().enumerate() {
                    let mut child = path.clone();
                    child.push(i as u32);
                    stack.push((ch, child));
                }
                map.insert(n as usize, path);
            }
        }
        DomMap { map }
    }

    /// `[u32::MAX]` for a node outside the tree, which ufbx never makes
    pub fn path(&self, p: *const u8) -> Option<Vec<u32>> {
        if p.is_null() {
            return None;
        }
        Some(self.map.get(&(p as usize)).cloned().unwrap_or_else(|| vec![u32::MAX]))
    }
}

pub struct Kx {
    pub dom: DomMap,
}

impl Kx {
    pub fn dom_path(&self, p: *const u8) -> Option<Vec<u32>> {
        self.dom.path(p)
    }
}

pub struct Cx {
    path: Vec<Seg>,
    pub checks: u64,
    pub count: usize,
    pub kept: Vec<Diff>,
    keep: usize,
    pub by_field: HashMap<String, usize>,
    dom: DomMap,
}

impl Cx {
    pub fn new(c: &ufbx::Scene, keep: usize) -> Cx {
        Cx { path: Vec::with_capacity(16), checks: 0, count: 0, kept: Vec::new(), keep, by_field: HashMap::new(), dom: DomMap::new(c) }
    }

    pub fn detached(keep: usize) -> Cx {
        Cx { path: Vec::new(), checks: 0, count: 0, kept: Vec::new(), keep, by_field: HashMap::new(), dom: DomMap { map: HashMap::new() } }
    }

    #[inline]
    pub fn push(&mut self, f: &'static str) {
        self.path.push(Seg::F(f));
    }

    #[inline]
    pub fn idx(&mut self, i: usize) {
        self.path.push(Seg::I(i));
    }

    #[inline]
    pub fn pop(&mut self) {
        self.path.pop();
    }

    fn render(&self, indices: bool) -> String {
        let mut s = String::new();
        for seg in &self.path {
            match seg {
                Seg::F(f) => {
                    if !s.is_empty() {
                        s.push('.');
                    }
                    s.push_str(f);
                }
                Seg::I(i) if indices => {
                    let _ = write!(s, "[{i}]");
                }
                Seg::I(_) => s.push_str("[]"),
            }
        }
        s
    }

    #[cold]
    pub fn diff(&mut self, c: impl Display, p: impl Display) {
        self.count += 1;
        *self.by_field.entry(self.render(false)).or_insert(0) += 1;
        if self.kept.len() < self.keep {
            self.kept.push(Diff { path: self.render(true), c: c.to_string(), p: p.to_string() });
        }
    }

    /// Equal bits, or both NaN, as `common::same`: GCC folds `-a*b + c` to `c - a*b`, which keeps every number
    /// and drops the sign a NaN would take
    #[inline]
    pub fn f64(&mut self, c: f64, p: f64) {
        self.checks += 1;
        if c.to_bits() != p.to_bits() && !(c.is_nan() && p.is_nan()) {
            self.diff(F64(c), F64(p));
        }
    }

    #[inline]
    pub fn f32(&mut self, c: f32, p: f32) {
        self.checks += 1;
        if c.to_bits() != p.to_bits() && !(c.is_nan() && p.is_nan()) {
            self.diff(F32(c), F32(p));
        }
    }

    #[inline]
    pub fn int(&mut self, c: i128, p: i128) {
        self.checks += 1;
        if c != p {
            self.diff(c, p);
        }
    }

    #[inline]
    pub fn bool(&mut self, c: bool, p: bool) {
        self.checks += 1;
        if c != p {
            self.diff(c, p);
        }
    }

    #[inline]
    pub fn flags(&mut self, c: u32, p: u32) {
        self.checks += 1;
        if c != p {
            self.diff(format_args!("{c:#x}"), format_args!("{p:#x} (xor {:#x})", c ^ p));
        }
    }

    #[inline]
    pub fn str(&mut self, c: &[u8], p: &[u8]) {
        self.checks += 1;
        if c != p {
            let at = first_difference(c, p);
            self.diff(Str(c, at), Str(p, at));
        }
    }

    #[inline]
    pub fn bytes(&mut self, c: &[u8], p: &[u8]) {
        self.checks += 1;
        if c != p {
            let at = first_difference(c, p);
            self.diff(format_args!("{} bytes", c.len()), format_args!("{} bytes, first difference at byte {at}", p.len()));
        }
    }

    #[inline]
    pub fn len(&mut self, c: usize, p: usize) {
        self.checks += 1;
        if c != p {
            self.push("len");
            self.diff(c, p);
            self.pop();
        }
    }

    #[inline]
    pub fn id(&mut self, c: Option<u64>, p: Option<u64>) {
        self.checks += 1;
        if c != p {
            self.diff(Id(c), Id(p));
        }
    }

    pub fn ids<T, P: PortId>(&mut self, c: &[*const T], p: &[P], read: fn(*const u8) -> Option<u64>) {
        self.len(c.len(), p.len());
        for i in 0..c.len().min(p.len()) {
            self.checks += 1;
            let (a, b) = (read(c[i] as *const u8), p[i].port_id());
            if a != b {
                self.idx(i);
                self.diff(Id(a), Id(b));
                self.pop();
            }
        }
    }

    #[inline]
    pub fn same(&mut self, ok: bool) -> bool {
        self.checks += 1;
        ok
    }

    pub fn presence(&mut self, c: bool, p: bool) {
        self.checks += 1;
        let show = |x: bool| if x { "value" } else { "none" };
        self.diff(show(c), show(p));
    }

    pub fn opt_str(&mut self, c: Option<&[u8]>, p: Option<&[u8]>) {
        match (c, p) {
            (Some(a), Some(b)) => self.str(a, b),
            (None, None) => self.checks += 1,
            (a, b) => {
                self.checks += 1;
                self.diff(OptStr(a), OptStr(b))
            }
        }
    }

    pub fn vec4_arr(&mut self, c: &ufbx::Vec4, p: &[f64; 4]) {
        for (name, a, b) in [("x", c.x, p[0]), ("y", c.y, p[1]), ("z", c.z, p[2]), ("w", c.w, p[3])] {
            self.push(name);
            self.f64(a, b);
            self.pop();
        }
    }

    pub fn dom_ref(&mut self, c: *const u8, p: Option<&[u32]>) {
        self.checks += 1;
        let a = self.dom.path(c);
        if a.as_deref() != p {
            self.diff(format_args!("{a:?}"), format_args!("{p:?}"));
        }
    }
}

fn first_difference(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()))
}

struct F64(f64);

impl Display for F64 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.is_nan() { write!(f, "NaN({:#018x})", self.0.to_bits()) } else { write!(f, "{:?}", self.0) }
    }
}

struct F32(f32);

impl Display for F32 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.is_nan() { write!(f, "NaN({:#010x})", self.0.to_bits()) } else { write!(f, "{:?}f32", self.0) }
    }
}

struct Str<'a>(&'a [u8], usize);

impl Display for Str<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = self.0;
        if s.len() <= 60 {
            return write!(f, "{:?}", String::from_utf8_lossy(s));
        }
        let start = self.1.saturating_sub(20).min(s.len());
        let end = (start + 60).min(s.len());
        write!(
            f,
            "{}{:?}{} ({} bytes)",
            if start > 0 { "…" } else { "" },
            String::from_utf8_lossy(&s[start..end]),
            if end < s.len() { "…" } else { "" },
            s.len()
        )
    }
}

struct OptStr<'a>(Option<&'a [u8]>);

impl Display for OptStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Some(s) => Str(s, 0).fmt(f),
            None => f.write_str("none"),
        }
    }
}

struct Id(Option<u64>);

impl Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Some(i) => write!(f, "#{i}"),
            None => f.write_str("null"),
        }
    }
}

pub trait PortId {
    fn port_id(&self) -> Option<u64>;
}

impl PortId for usize {
    fn port_id(&self) -> Option<u64> {
        Some(*self as u64)
    }
}

impl PortId for u32 {
    fn port_id(&self) -> Option<u64> {
        Some(*self as u64)
    }
}

impl<T: PortId> PortId for Option<T> {
    fn port_id(&self) -> Option<u64> {
        self.as_ref().and_then(|x| x.port_id())
    }
}

pub trait FromId {
    fn from_id(v: Option<u64>) -> Self;
}

impl FromId for usize {
    fn from_id(v: Option<u64>) -> usize {
        v.map_or(usize::MAX, |x| x as usize)
    }
}

impl FromId for u32 {
    fn from_id(v: Option<u64>) -> u32 {
        v.map_or(u32::MAX, |x| x as u32)
    }
}

impl<T: FromId> FromId for Option<T> {
    fn from_id(v: Option<u64>) -> Option<T> {
        v.map(|x| T::from_id(Some(x)))
    }
}

#[inline]
pub fn raw_ptr<R>(r: &R) -> *const u8 {
    const { assert!(size_of::<R>() == size_of::<usize>()) };
    unsafe { std::mem::transmute_copy::<R, *const u8>(r) }
}

/// A `RefList`'s pointers, read raw so that a null entry is not a `Ref`
#[inline]
pub fn reflist<T>(l: &ufbx::RefList<T>) -> &[*const T] {
    let s: &[ufbx::Ref<T>] = l;
    unsafe { std::slice::from_raw_parts(s.as_ptr() as *const *const T, s.len()) }
}

pub trait AsSlice {
    type Item;
    fn as_slice_(&self) -> &[Self::Item];
}

impl<T> AsSlice for ufbx::List<T> {
    type Item = T;
    fn as_slice_(&self) -> &[T] {
        if self.data.is_null() || self.count == 0 { &[] } else { unsafe { std::slice::from_raw_parts(self.data, self.count) } }
    }
}

impl<T, const N: usize> AsSlice for [T; N] {
    type Item = T;
    fn as_slice_(&self) -> &[T] {
        self
    }
}

#[inline]
pub fn slice_of<S: AsSlice + ?Sized>(s: &S) -> &[S::Item] {
    s.as_slice_()
}

#[repr(C)]
struct RawPair {
    data: *const u8,
    len: usize,
}

const _: () = assert!(size_of::<ufbx::Blob>() == size_of::<RawPair>() && size_of::<ufbx::String>() == size_of::<RawPair>());

#[inline]
fn pair_bytes<'a>(r: &RawPair) -> &'a [u8] {
    if r.data.is_null() || r.len == 0 { &[] } else { unsafe { std::slice::from_raw_parts(r.data, r.len) } }
}

#[inline]
pub fn blob(b: &ufbx::Blob) -> &[u8] {
    pair_bytes(unsafe { &*(b as *const ufbx::Blob as *const RawPair) })
}

#[inline]
pub fn cstr(s: &ufbx::String) -> &[u8] {
    pair_bytes(unsafe { &*(s as *const ufbx::String as *const RawPair) })
}

pub fn string_array(b: &ufbx::Blob) -> Vec<&[u8]> {
    let raw = blob(b);
    let n = raw.len() / size_of::<RawPair>();
    let items = raw.as_ptr() as *const RawPair;
    (0..n).map(|i| pair_bytes(unsafe { &*items.add(i) })).collect()
}

pub fn to_string(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// # Safety
/// `p` points at a C element, whose header comes first
#[inline]
pub unsafe fn elem<'a>(p: *const u8) -> &'a ufbx::Element {
    unsafe { &*(p as *const ufbx::Element) }
}

#[inline]
pub fn c_typed(p: *const u8) -> Option<u64> {
    if p.is_null() { None } else { Some(unsafe { elem(p) }.typed_id as u64) }
}

#[inline]
pub fn c_eid(p: *const u8) -> Option<u64> {
    if p.is_null() { None } else { Some(unsafe { elem(p) }.element_id as u64) }
}

pub fn prop_name<'a>(p: *const u8) -> Option<&'a [u8]> {
    if p.is_null() { None } else { Some(cstr(unsafe { &(*(p as *const ufbx::Prop)).name })) }
}

pub fn channel_index(cache: *const u8, channel: *const u8) -> Option<u64> {
    if channel.is_null() {
        return None;
    }
    if cache.is_null() {
        return Some(u64::MAX);
    }
    let channels = slice_of(unsafe { &(*(cache as *const ufbx::GeometryCache)).channels });
    let base = channels.as_ptr() as usize;
    let size = size_of::<ufbx::CacheChannel>();
    let at = (channel as usize).wrapping_sub(base);
    if at.is_multiple_of(size) && at / size < channels.len() { Some((at / size) as u64) } else { Some(u64::MAX) }
}

#[inline]
pub fn flag_bits<F: Copy>(f: F) -> u32 {
    const { assert!(size_of::<F>() == 4) };
    unsafe { std::mem::transmute_copy::<F, u32>(&f) }
}

/// # Safety
/// `off` is the offset of a field of type `T` in the `repr(C)` struct `S`
#[inline]
pub unsafe fn private<T: Copy, S>(s: &S, off: usize) -> T {
    unsafe { std::ptr::read_unaligned((s as *const S as *const u8).add(off) as *const T) }
}

pub const fn align(x: usize, a: usize) -> usize {
    x.div_ceil(a) * a
}
