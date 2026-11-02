//! `use load3d::compat as ufbx;` lets a program written for the `ufbx` crate build unchanged.
//!
//! The only module with `unsafe`: pointers into what a `SceneRoot` owns, callbacks that borrow, and the C function pointers
//! and `FILE *` that ufbx-rust's API names.
// ufbx-rust's signatures, its `Error` returned by value
#![allow(clippy::result_large_err)]

use std::alloc::Layout;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::fmt::{self, Debug, Display, Formatter};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Deref, Index};
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Weak};
use std::{ptr, slice};

use crate as f;

macro_rules! flags_impl {
    ($name:ident, [$(($text:expr, $value:expr)),* $(,)?]) => {
        impl $name {
            pub fn any(self) -> bool { self.0 != 0 }
            pub fn has_any(self, bits: Self) -> bool { (self.0 & bits.0) != 0 }
            pub fn has_all(self, bits: Self) -> bool { (self.0 & bits.0) == bits.0 }
            #[doc(hidden)]
            pub fn bits(self) -> u32 { self.0 }
        }
        impl Default for $name {
            fn default() -> Self { Self(0) }
        }
        impl Debug for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                format_flags(f, &[$(($text, $value)),*], self.0)
            }
        }
        impl BitAnd for $name { type Output = Self; fn bitand(self, o: Self) -> Self { Self(self.0 & o.0) } }
        impl BitAndAssign for $name { fn bitand_assign(&mut self, o: Self) { self.0 &= o.0 } }
        impl BitOr for $name { type Output = Self; fn bitor(self, o: Self) -> Self { Self(self.0 | o.0) } }
        impl BitOrAssign for $name { fn bitor_assign(&mut self, o: Self) { self.0 |= o.0 } }
        impl BitXor for $name { type Output = Self; fn bitxor(self, o: Self) -> Self { Self(self.0 ^ o.0) } }
        impl BitXorAssign for $name { fn bitxor_assign(&mut self, o: Self) { self.0 ^= o.0 } }
    };
}

mod generated;
pub use generated::*;
pub(crate) use generated::{Arenas, Bases};

#[cfg(test)]
mod api_check;

pub type Real = f64;
pub type ThreadPoolContext = usize;
pub type OpenFileContext = usize;
pub type Result<T> = std::result::Result<T, Error>;

pub fn format_flags(f: &mut Formatter<'_>, names: &[(&str, u32)], value: u32) -> fmt::Result {
    let mut has_any = false;
    for (name, v) in names {
        if (value & v) != 0 {
            write!(f, "{}{}", if has_any { "|" } else { "" }, name)?;
            has_any = true;
        }
    }
    if !has_any {
        write!(f, "NONE")?;
    }
    Ok(())
}

/// A value of this crate's that a value of ufbx-rust's shape was made from, alive as long as it
pub(crate) struct Port<T>(*const T);

impl<T> Port<T> {
    pub(crate) fn new(r: &T) -> Port<T> {
        Port(r)
    }
}

impl<T> Deref for Port<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.0 }
    }
}

/// Where ufbx's pointer can be null though ufbx-rust's type says it cannot, reading it panics.
#[repr(transparent)]
pub struct Ref<T> {
    ptr: *const T,
    _marker: PhantomData<T>,
}

impl<T> Ref<T> {
    pub(crate) fn new(ptr: *const T) -> Ref<T> {
        Ref { ptr, _marker: PhantomData }
    }
    pub(crate) fn opt(ptr: Option<*const T>) -> Ref<T> {
        Ref::new(ptr.unwrap_or(ptr::null()))
    }
    /// For what must not read one: ufbx leaves a few of these null where ufbx-rust's type says they cannot be
    #[doc(hidden)]
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }
}

// what a scene holds is read and never changed, so a panic leaves nothing half-made to see
impl<T> panic::UnwindSafe for Ref<T> {}
impl<T> panic::RefUnwindSafe for Ref<T> {}

impl<T> AsRef<T> for Ref<T> {
    fn as_ref(&self) -> &T {
        assert!(!self.ptr.is_null(), "a reference ufbx leaves null");
        unsafe { &*self.ptr }
    }
}

impl<T> Deref for Ref<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.as_ref()
    }
}

unsafe fn slice_from_ptr<'a, T>(data: *const T, len: usize) -> &'a [T] {
    if len > 0 { unsafe { slice::from_raw_parts(data, len) } } else { &[] }
}

#[repr(C)]
pub struct List<T> {
    pub data: *const T,
    pub count: usize,
    _marker: PhantomData<T>,
}

impl<T> List<T> {
    pub(crate) fn view(s: &[T]) -> List<T> {
        List { data: s.as_ptr(), count: s.len(), _marker: PhantomData }
    }
}

impl<T> AsRef<[T]> for List<T> {
    fn as_ref(&self) -> &[T] {
        unsafe { slice_from_ptr(self.data, self.count) }
    }
}

impl<T> Deref for List<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.as_ref()
    }
}

impl<'a, T> IntoIterator for &'a List<T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.as_ref().iter()
    }
}

impl<T> Index<usize> for List<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        &self.as_ref()[index]
    }
}

#[repr(C)]
pub struct RefList<T> {
    data: *const Ref<T>,
    pub count: usize,
    _marker: PhantomData<T>,
}

impl<T> AsRef<[Ref<T>]> for RefList<T> {
    fn as_ref(&self) -> &[Ref<T>] {
        unsafe { slice_from_ptr(self.data, self.count) }
    }
}

impl<T> Deref for RefList<T> {
    type Target = [Ref<T>];
    fn deref(&self) -> &[Ref<T>] {
        self.as_ref()
    }
}

pub struct RefIter<'a, T> {
    inner: slice::Iter<'a, Ref<T>>,
}

impl<'a, T> Iterator for RefIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        self.inner.next().map(|v| v.as_ref())
    }
}

impl<'a, T> IntoIterator for &'a RefList<T> {
    type Item = &'a T;
    type IntoIter = RefIter<'a, T>;
    fn into_iter(self) -> RefIter<'a, T> {
        RefIter { inner: self.as_ref().iter() }
    }
}

impl<T> Index<usize> for RefList<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        &self.as_ref()[index]
    }
}

#[repr(C)]
pub struct VoidList {
    pub data: *mut c_void,
    pub count: usize,
}

#[repr(C)]
pub struct String {
    data: *const u8,
    pub length: usize,
    _marker: PhantomData<u8>,
}

impl String {
    pub(crate) fn view(s: &str) -> String {
        String { data: s.as_ptr(), length: s.len(), _marker: PhantomData }
    }
}

impl AsRef<str> for String {
    fn as_ref(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(slice_from_ptr(self.data, self.length)) }
    }
}

impl Deref for String {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_ref()
    }
}

impl Default for String {
    fn default() -> String {
        String { data: ptr::null(), length: 0, _marker: PhantomData }
    }
}

impl Display for String {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}

impl<'a> PartialEq<&'a str> for String {
    fn eq(&self, rhs: &&'a str) -> bool {
        &self.as_ref() == rhs
    }
}

impl Debug for String {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_ref())
    }
}

#[repr(C)]
pub struct Blob {
    data: *const u8,
    pub size: usize,
    _marker: PhantomData<u8>,
}

impl Blob {
    pub(crate) fn view(b: &[u8]) -> Blob {
        Blob { data: b.as_ptr(), size: b.len(), _marker: PhantomData }
    }
}

impl Deref for Blob {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        unsafe { slice_from_ptr(self.data, self.size) }
    }
}

#[repr(C)]
pub struct ExternalRef<'a, T> {
    data: T,
    _marker: PhantomData<&'a T>,
}

impl<'a, T> ExternalRef<'a, T> {
    /// # Safety
    /// As ufbx-rust's: what `t` points at outlives the reference. Nothing here relies on it.
    pub unsafe fn new(t: T) -> Self {
        Self { data: t, _marker: PhantomData }
    }
}

impl<T> AsRef<T> for ExternalRef<'_, T> {
    fn as_ref(&self) -> &T {
        &self.data
    }
}

impl<T> Deref for ExternalRef<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.data
    }
}

#[repr(transparent)]
#[derive(Default)]
pub struct Unsafe<T>(pub(crate) T);

impl<T> Unsafe<T> {
    /// # Safety
    /// As ufbx-rust's: the value is taken for what it claims to be.
    pub unsafe fn new(t: T) -> Self {
        Self(t)
    }
}

impl<T: Default> Unsafe<T> {
    pub fn take(&mut self) -> T {
        std::mem::take(&mut self.0)
    }
}

#[repr(transparent)]
pub struct InlineBuf<T> {
    pub data: MaybeUninit<T>,
}

impl<T> Default for InlineBuf<T> {
    fn default() -> Self {
        Self { data: MaybeUninit::uninit() }
    }
}

#[derive(Default)]
pub enum StringOpt<'a> {
    #[default]
    Unset,
    Ref(&'a str),
    Owned(std::string::String),
}

impl<'a> From<&'a str> for StringOpt<'a> {
    fn from(v: &'a str) -> Self {
        StringOpt::Ref(v)
    }
}

impl From<std::string::String> for StringOpt<'_> {
    fn from(v: std::string::String) -> Self {
        StringOpt::Owned(v)
    }
}

impl StringOpt<'_> {
    pub(crate) fn to_port(&self) -> std::string::String {
        match self {
            StringOpt::Unset => std::string::String::new(),
            StringOpt::Ref(s) => s.to_string(),
            StringOpt::Owned(s) => s.clone(),
        }
    }
}

#[derive(Default)]
pub enum BlobOpt<'a> {
    #[default]
    Unset,
    Ref(&'a [u8]),
    Owned(Vec<u8>),
}

impl<'a> From<&'a [u8]> for BlobOpt<'a> {
    fn from(v: &'a [u8]) -> Self {
        BlobOpt::Ref(v)
    }
}

impl From<Vec<u8>> for BlobOpt<'_> {
    fn from(v: Vec<u8>) -> Self {
        BlobOpt::Owned(v)
    }
}

impl BlobOpt<'_> {
    pub(crate) fn to_port(&self) -> Vec<u8> {
        match self {
            BlobOpt::Unset => Vec::new(),
            BlobOpt::Ref(b) => b.to_vec(),
            BlobOpt::Owned(b) => b.clone(),
        }
    }
}

pub enum ListOpt<'a, T> {
    Unset,
    Ref(&'a [T]),
    Mut(&'a mut [T]),
    Owned(Vec<T>),
}

// no bound on `T`, as ufbx-rust's
#[allow(clippy::derivable_impls)]
impl<T> Default for ListOpt<'_, T> {
    fn default() -> Self {
        ListOpt::Unset
    }
}
impl<'a, T> From<&'a [T]> for ListOpt<'a, T> {
    fn from(v: &'a [T]) -> Self {
        ListOpt::Ref(v)
    }
}

impl<T> From<Vec<T>> for ListOpt<'_, T> {
    fn from(v: Vec<T>) -> Self {
        ListOpt::Owned(v)
    }
}

impl<T> ListOpt<'_, T> {
    pub(crate) fn as_slice(&self) -> &[T] {
        match self {
            ListOpt::Unset => &[],
            ListOpt::Ref(s) => s,
            ListOpt::Mut(s) => s,
            ListOpt::Owned(v) => v,
        }
    }
}

pub(crate) fn prop_override_to_port(p: &PropOverrideDesc) -> f::PropOverrideDesc {
    f::PropOverrideDesc { element_id: p.element_id, prop_name: p.prop_name.to_port(), value: p.value, value_str: p.value_str.to_port(), value_int: p.value_int }
}

pub trait AllocatorInterface {
    fn alloc(&mut self, layout: Layout) -> *mut u8;
    fn free(&mut self, ptr: *mut u8, layout: Layout);
    fn realloc(&mut self, ptr: *mut u8, old_layout: Layout, new_layout: Layout) -> *mut u8 {
        self.free(ptr, old_layout);
        self.alloc(new_layout)
    }
    fn free_allocator(&mut self) {}
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawAllocator {
    pub alloc_fn: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub realloc_fn: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize, usize) -> *mut c_void>,
    pub free_fn: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize)>,
    pub free_allocator_fn: Option<unsafe extern "C" fn(*mut c_void)>,
    pub user: *mut c_void,
}

impl Default for RawAllocator {
    fn default() -> Self {
        RawAllocator { alloc_fn: None, realloc_fn: None, free_fn: None, free_allocator_fn: None, user: ptr::null_mut() }
    }
}

/// Which allocator is accepted, and has no effect: this crate's memory is Rust's. `memory_limit` beside it still counts.
#[derive(Default)]
pub enum Allocator {
    Libc,
    #[default]
    Global,
    System,
    Box(Box<dyn AllocatorInterface>),
    Raw(Unsafe<RawAllocator>),
}

impl AllocatorOpts {
    /// C fails as the allocation is made; here the finished result, of about `bytes`, is held to the limit
    pub(crate) fn check(&self, bytes: usize, name: &str) -> Result<()> {
        if self.memory_limit != 0 && bytes > self.memory_limit {
            let mut e = f::Error::new("Memory limit exceeded").set_err_info(name.to_string());
            e.kind = ErrorType::MemoryLimit;
            return Err(to_error(e));
        }
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawThreadPool {
    pub init_fn: Option<unsafe extern "C" fn(*mut c_void, ThreadPoolContext, *const ThreadPoolInfo) -> bool>,
    pub run_fn: Option<unsafe extern "C" fn(*mut c_void, ThreadPoolContext, u32, u32, u32)>,
    pub wait_fn: Option<unsafe extern "C" fn(*mut c_void, ThreadPoolContext, u32, u32)>,
    pub free_fn: Option<unsafe extern "C" fn(*mut c_void, ThreadPoolContext)>,
    pub user: *mut c_void,
}

impl Default for RawThreadPool {
    fn default() -> Self {
        RawThreadPool { init_fn: None, run_fn: None, wait_fn: None, free_fn: None, user: ptr::null_mut() }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ThreadPoolInfo {
    pub max_concurrent_tasks: u32,
}

/// Accepted and unused: this crate parses on one thread
#[derive(Default)]
pub enum ThreadPool {
    #[default]
    None,
    Raw(Unsafe<RawThreadPool>),
}

pub fn thread_pool_run_task(_ctx: ThreadPoolContext, _index: u32) {}
pub fn thread_pool_set_user_ptr(_ctx: ThreadPoolContext, _user_ptr: *mut c_void) {}
pub fn thread_pool_get_user_ptr(_ctx: ThreadPoolContext) -> *mut c_void {
    ptr::null_mut()
}

struct Erased<F: ?Sized>(*mut F);
unsafe impl<F: ?Sized> Send for Erased<F> {}
unsafe impl<F: ?Sized> Sync for Erased<F> {}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawProgressCb {
    pub fn_: Option<unsafe extern "C" fn(*mut c_void, *const Progress) -> ProgressResult>,
    pub user: *mut c_void,
}

impl Default for RawProgressCb {
    fn default() -> Self {
        RawProgressCb { fn_: None, user: ptr::null_mut() }
    }
}

#[derive(Default)]
pub enum ProgressCb<'a> {
    #[default]
    Unset,
    Mut(&'a mut dyn FnMut(&Progress) -> ProgressResult),
    Ref(&'a dyn Fn(&Progress) -> ProgressResult),
    Raw(Unsafe<RawProgressCb>),
}

type ProgressMut<'a> = dyn FnMut(&Progress) -> ProgressResult + 'a;
type ProgressRef<'a> = dyn Fn(&Progress) -> ProgressResult + 'a;

impl ProgressCb<'_> {
    /// The callback lives as long as the call of ufbx's that takes it, and this crate calls it within that call alone
    #[allow(clippy::wrong_self_convention)] // named as the converters gen_compat.py writes
    pub(crate) fn to_port(&mut self) -> Option<Arc<f::ProgressFn>> {
        match self {
            ProgressCb::Unset => None,
            ProgressCb::Mut(cb) => {
                let raw: *mut ProgressMut<'_> = &mut **cb;
                let raw: *mut ProgressMut<'static> = unsafe { std::mem::transmute(raw) };
                let cb = Erased(raw);
                Some(Arc::new(move |p: &Progress| {
                    let cb = &cb;
                    to_enum_ProgressResult(unsafe { (*cb.0)(p) })
                }))
            }
            ProgressCb::Ref(cb) => {
                let raw: *const ProgressRef<'_> = &**cb;
                let raw: *const ProgressRef<'static> = unsafe { std::mem::transmute(raw) };
                let cb = Erased(raw as *mut ProgressRef<'static>);
                Some(Arc::new(move |p: &Progress| {
                    let cb = &cb;
                    to_enum_ProgressResult(unsafe { (*cb.0)(p) })
                }))
            }
            ProgressCb::Raw(raw) => {
                let raw = raw.0;
                let user = Erased(raw.user);
                let cb = raw.fn_?;
                Some(Arc::new(move |p: &Progress| {
                    let user = &user;
                    to_enum_ProgressResult(unsafe { cb(user.0, p) })
                }))
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawCloseMemoryCb {
    pub fn_: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize)>,
    pub user: *mut c_void,
}

impl Default for RawCloseMemoryCb {
    fn default() -> Self {
        RawCloseMemoryCb { fn_: None, user: ptr::null_mut() }
    }
}

#[derive(Default)]
pub enum CloseMemoryCb<'a> {
    #[default]
    Unset,
    Mut(&'a mut dyn FnMut(*mut c_void, usize)),
    Ref(&'a dyn Fn(*mut c_void, usize)),
    Raw(Unsafe<RawCloseMemoryCb>),
}

#[repr(C)]
pub struct OpenFileInfo {
    pub context: OpenFileContext,
    pub type_: OpenFileType,
    pub original_filename: Blob,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawOpenFileCb {
    pub fn_: Option<unsafe extern "C" fn(*mut c_void, *mut RawStream, *const u8, usize, *const OpenFileInfo) -> bool>,
    pub user: *mut c_void,
}

impl Default for RawOpenFileCb {
    fn default() -> Self {
        RawOpenFileCb { fn_: None, user: ptr::null_mut() }
    }
}

#[derive(Default)]
pub enum OpenFileCb<'a> {
    #[default]
    Unset,
    Mut(&'a mut dyn FnMut(&str, &OpenFileInfo) -> Option<Stream>),
    Ref(&'a dyn Fn(&str, &OpenFileInfo) -> Option<Stream>),
    Raw(Unsafe<RawOpenFileCb>),
}

type OpenMut<'a> = dyn FnMut(&str, &OpenFileInfo) -> Option<Stream> + 'a;
type OpenRef<'a> = dyn Fn(&str, &OpenFileInfo) -> Option<Stream> + 'a;

fn open_info(info: &f::OpenFileInfo) -> OpenFileInfo {
    OpenFileInfo { context: 0, type_: info.file_type, original_filename: Blob::view(&info.original_filename) }
}

fn open_result(stream: Option<Stream>) -> Option<Vec<u8>> {
    let mut reader = StreamReader(stream?);
    let mut data = Vec::new();
    let ok = reader.read_to_end(&mut data).is_ok();
    reader.close();
    ok.then_some(data)
}

impl OpenFileCb<'_> {
    #[allow(clippy::wrong_self_convention)] // named as the converters gen_compat.py writes
    pub(crate) fn to_port(&mut self) -> Option<Arc<f::OpenFileFn>> {
        match self {
            OpenFileCb::Unset => None,
            OpenFileCb::Mut(cb) => {
                let raw: *mut OpenMut<'_> = &mut **cb;
                let raw: *mut OpenMut<'static> = unsafe { std::mem::transmute(raw) };
                let cb = Erased(raw);
                Some(Arc::new(move |path: &str, info: &f::OpenFileInfo| {
                    let cb = &cb;
                    open_result(unsafe { (*cb.0)(path, &open_info(info)) })
                }))
            }
            OpenFileCb::Ref(cb) => {
                let raw: *const OpenRef<'_> = &**cb;
                let raw: *const OpenRef<'static> = unsafe { std::mem::transmute(raw) };
                let cb = Erased(raw as *mut OpenRef<'static>);
                Some(Arc::new(move |path: &str, info: &f::OpenFileInfo| {
                    let cb = &cb;
                    open_result(unsafe { (*cb.0)(path, &open_info(info)) })
                }))
            }
            OpenFileCb::Raw(raw) => {
                let raw = raw.0;
                let user = Erased(raw.user);
                let cb = raw.fn_?;
                Some(Arc::new(move |path: &str, info: &f::OpenFileInfo| {
                    let user = &user;
                    let mut stream = RawStream::default();
                    let opened = unsafe { cb(user.0, &mut stream, path.as_ptr(), path.len(), &open_info(info)) };
                    opened.then(|| Stream::Raw(Unsafe(stream))).and_then(|s| open_result(Some(s)))
                }))
            }
        }
    }
}

pub trait StreamInterface {
    fn read(&mut self, buf: &mut [u8]) -> Option<usize>;
    fn skip(&mut self, bytes: usize) -> bool {
        let mut local = [0u8; 512];
        let mut left = bytes;
        while left > 0 {
            let to_read = left.min(local.len());
            if self.read(&mut local[..to_read]).unwrap_or(0) != to_read {
                return false;
            }
            left -= to_read;
        }
        true
    }
    fn size(&mut self) -> u64 {
        0
    }
    fn close(&mut self) {}
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawStream {
    pub read_fn: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize>,
    pub skip_fn: Option<unsafe extern "C" fn(*mut c_void, usize) -> bool>,
    pub size_fn: Option<unsafe extern "C" fn(*mut c_void) -> u64>,
    pub close_fn: Option<unsafe extern "C" fn(*mut c_void)>,
    pub user: *mut c_void,
}

impl Default for RawStream {
    fn default() -> Self {
        RawStream { read_fn: None, skip_fn: None, size_fn: None, close_fn: None, user: ptr::null_mut() }
    }
}

pub enum Stream {
    File(File),
    Read(Box<dyn Read>),
    Box(Box<dyn StreamInterface>),
    Raw(Unsafe<RawStream>),
}

/// This crate reads a stream to its end before it loads it, so a stream's `skip` is never called
struct StreamReader(Stream);

impl Read for StreamReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match &mut self.0 {
            Stream::File(file) => file.read(buf),
            Stream::Read(r) => r.read(buf),
            Stream::Box(b) => b.read(buf).ok_or_else(|| io::Error::other("stream")),
            Stream::Raw(raw) => {
                let raw = raw.0;
                let Some(read) = raw.read_fn else { return Ok(0) };
                match unsafe { read(raw.user, buf.as_mut_ptr() as *mut c_void, buf.len()) } {
                    usize::MAX => Err(io::Error::other("stream")),
                    n => Ok(n),
                }
            }
        }
    }
}

impl StreamReader {
    fn size(&mut self) -> u64 {
        match &mut self.0 {
            Stream::File(file) => {
                let Ok(start) = file.stream_position() else { return 0 };
                let Ok(end) = file.seek(SeekFrom::End(0)) else { return 0 };
                if file.seek(SeekFrom::Start(start)).is_err() { u64::MAX } else { end - start }
            }
            Stream::Read(_) => 0,
            Stream::Box(b) => b.size(),
            Stream::Raw(raw) => raw.0.size_fn.map_or(0, |size| unsafe { size(raw.0.user) }),
        }
    }

    fn close(&mut self) {
        match &mut self.0 {
            Stream::Box(b) => b.close(),
            Stream::Raw(raw) => {
                if let Some(close) = raw.0.close_fn {
                    unsafe { close(raw.0.user) }
                }
            }
            _ => {}
        }
    }
}

unsafe extern "C" {
    fn fread(ptr: *mut c_void, size: usize, count: usize, stream: *mut c_void) -> usize;
    fn ferror(stream: *mut c_void) -> i32;
}

struct StdioReader(*mut c_void);

impl Read for StdioReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = unsafe { fread(buf.as_mut_ptr() as *mut c_void, 1, buf.len(), self.0) };
        if n == 0 && unsafe { ferror(self.0) } != 0 { Err(io::Error::other("stdio")) } else { Ok(n) }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct ErrorFrame {
    pub source_line: u32,
    pub function: String,
    pub description: String,
}

pub struct Error {
    pub type_: ErrorType,
    pub description: String,
    pub stack_size: u32,
    pub stack: [ErrorFrame; 8],
    held: Box<str>,
    info: std::string::String,
}

pub(crate) fn to_error(e: f::Error) -> Error {
    let held: Box<str> = e.description.into_boxed_str();
    Error { type_: e.kind, description: String::view(&held), stack_size: 0, stack: Default::default(), held, info: e.info }
}

impl Error {
    pub fn info(&self) -> &str {
        &self.info
    }

    fn text(&self) -> std::string::String {
        if self.info.is_empty() { self.held.to_string() } else { format!("{}: {}", self.held, self.info) }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text())
    }
}

impl std::error::Error for Error {}

pub fn format_error(dst: &mut [u8], error: &Error) -> usize {
    let text = error.text();
    let n = text.len().min(dst.len().saturating_sub(1));
    dst[..n].copy_from_slice(&text.as_bytes()[..n]);
    if n < dst.len() {
        dst[n] = 0;
    }
    n
}

#[repr(C)]
#[derive(Default)]
pub struct Panic {
    pub did_panic: bool,
    message: std::string::String,
}

impl Panic {
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// A panic of this crate's named as ufbx-rust names it: `ufbx::<function>() <message>`
fn named<R>(name: &str, call: impl FnOnce() -> R) -> R {
    match panic::catch_unwind(AssertUnwindSafe(call)) {
        Ok(r) => r,
        Err(e) => {
            let text = e.downcast_ref::<std::string::String>().map(|s| s.as_str()).or(e.downcast_ref::<&str>().copied()).unwrap_or("");
            panic!("ufbx::{name}() {text}");
        }
    }
}

fn field<T, E>(base: *const T, index: usize, offset: usize) -> *const E {
    base.wrapping_add(index).cast::<u8>().wrapping_add(offset).cast::<E>()
}

pub(crate) struct Kx<'p> {
    pub(crate) b: Bases,
    p: &'p f::Scene,
    scene: *const Scene,
    pub(crate) owner: Weak<SceneData>,
    keep: RefCell<Vec<Box<dyn Any>>>,
    cache: RefCell<HashMap<usize, *const ()>>,
    dom: Cell<*const DomNode>,
    current: Cell<*const f::Element>,
}

impl<'p> Kx<'p> {
    fn new(b: Bases, p: &'p f::Scene, scene: *const Scene, owner: Weak<SceneData>) -> Kx<'p> {
        Kx { b, p, scene, owner, keep: RefCell::default(), cache: RefCell::default(), dom: Cell::new(ptr::null()), current: Cell::new(ptr::null()) }
    }

    fn of(scene: &'p Scene) -> Kx<'p> {
        let k = Kx::new(scene.bases, &scene.port, scene, scene.owner.clone());
        if let Some(dom) = &scene.dom_root {
            k.dom.set(&**dom);
        }
        k
    }

    fn take_keep(self) -> Vec<Box<dyn Any>> {
        self.keep.into_inner()
    }

    pub(crate) fn list<T: 'static>(&self, v: Vec<T>) -> List<T> {
        let l = List::view(&v);
        self.keep.borrow_mut().push(Box::new(v));
        l
    }

    pub(crate) fn reflist<T: 'static>(&self, v: Vec<Ref<T>>) -> RefList<T> {
        let l = RefList { data: v.as_ptr(), count: v.len(), _marker: PhantomData };
        self.keep.borrow_mut().push(Box::new(v));
        l
    }

    pub(crate) fn boxed<T: 'static>(&self, t: T) -> Ref<T> {
        let b = Box::new(t);
        let r = Ref::new(&*b as *const T);
        self.keep.borrow_mut().push(b);
        r
    }

    pub(crate) fn cached<S, T: 'static>(&self, key: *const S, make: impl FnOnce() -> T) -> Ref<T> {
        let key = key as usize;
        if let Some(&r) = self.cache.borrow().get(&key) {
            return Ref::new(r as *const T);
        }
        let r = self.boxed(make());
        self.cache.borrow_mut().insert(key, r.ptr as *const ());
        r
    }

    pub(crate) fn arena_reflist<T: 'static>(&self, base: *const T, len: usize) -> RefList<T> {
        self.reflist((0..len).map(|i| Ref::new(base.wrapping_add(i))).collect())
    }

    pub(crate) fn element_id(&self, id: usize) -> *const Element {
        self.b.element_at(self.p.elements[id])
    }

    pub(crate) fn scene_ref(&self) -> Ref<Scene> {
        Ref::new(self.scene)
    }

    pub(crate) fn dom_root(&self) -> Option<Ref<DomNode>> {
        let dom = self.dom.get();
        (!dom.is_null()).then(|| Ref::new(dom))
    }

    pub(crate) fn dom_ref(&self, path: &Option<f::DomRef>) -> Option<Ref<DomNode>> {
        let path = path.as_ref()?;
        let mut node = self.dom_root()?;
        for &i in path {
            node = Ref::new(&*node.children.as_ref()[i as usize] as *const DomNode);
        }
        Some(node)
    }

    pub(crate) fn dom_blobs(&self, p: &f::DomNode) -> List<Blob> {
        self.list(p.as_blob_list().iter().map(|b| Blob::view(b)).collect())
    }

    pub(crate) fn set_current(&self, element: &f::Element) {
        self.current.set(element);
    }

    pub(crate) fn prop_named(&self, name: &str) -> Ref<Prop> {
        let current = self.current.get();
        let found = if current.is_null() { None } else { unsafe { &*current }.props.find(name) };
        match found {
            Some(prop) => self.boxed(from_Prop(self, prop)),
            None => {
                let held = Box::new(f::Prop { name: name.to_string(), ..f::Prop::default() });
                let r = self.boxed(from_Prop(self, &held));
                self.keep.borrow_mut().push(held);
                r
            }
        }
    }

    pub(crate) fn channel(&self, cache: &Option<Arc<f::GeometryCache>>, channel: Option<usize>) -> Option<Ref<CacheChannel>> {
        let cache = cache.as_ref()?;
        let made = self.cached(Arc::as_ptr(cache), || from_GeometryCache(self, cache));
        Some(Ref::new(&made.channels[channel?] as *const CacheChannel))
    }
}

pub(crate) struct SceneData {
    scene: Box<Scene>,
    _arenas: Arenas,
    _keep: Vec<Box<dyn Any>>,
    _port: Arc<f::Scene>,
}

/// Kept alive by reference counts as `ufbx_retain_scene` does
pub struct SceneRoot {
    inner: Arc<SceneData>,
}

impl Clone for SceneRoot {
    fn clone(&self) -> Self {
        SceneRoot { inner: self.inner.clone() }
    }
}

impl Deref for SceneRoot {
    type Target = Scene;
    fn deref(&self) -> &Scene {
        &self.inner.scene
    }
}

// a scene is immutable once made, and every pointer in it stays within what its root holds
unsafe impl Send for SceneRoot {}
unsafe impl Sync for SceneRoot {}

pub(crate) fn scene_root(port: f::Scene) -> SceneRoot {
    let port = Arc::new(port);
    let inner = Arc::new_cyclic(|weak: &Weak<SceneData>| {
        let mut scene: Box<MaybeUninit<Scene>> = Box::new_uninit();
        let (mut arenas, bases) = make_arenas(&port);
        let k = Kx::new(bases, &port, scene.as_ptr(), weak.clone());
        // first, as elements point into it
        if let Some(dom) = &port.dom_root {
            let made = k.boxed(from_DomNode(&k, dom));
            k.dom.set(made.ptr);
        }
        fill_arenas(&k, &port, &mut arenas);
        scene.write(from_Scene(&k, &port));
        let keep = k.take_keep();
        SceneData { scene: unsafe { scene.assume_init() }, _arenas: arenas, _keep: keep, _port: port.clone() }
    });
    SceneRoot { inner }
}

struct Standalone<T, P> {
    value: Box<T>,
    _port: Box<P>,
    _keep: Vec<Box<dyn Any>>,
    _owner: Option<Arc<SceneData>>,
}

macro_rules! root {
    ($name:ident, $t:ty, $p:ty) => {
        #[doc = concat!("What `", stringify!($t), "` a call of ufbx's returns, with what it points into")]
        pub struct $name {
            inner: Arc<Standalone<$t, $p>>,
        }
        impl Clone for $name {
            fn clone(&self) -> Self {
                $name { inner: self.inner.clone() }
            }
        }
        impl Deref for $name {
            type Target = $t;
            fn deref(&self) -> &$t {
                &self.inner.value
            }
        }
        unsafe impl Send for $name {}
        unsafe impl Sync for $name {}
    };
}

root!(MeshRoot, Mesh, f::Mesh);
root!(LineCurveRoot, LineCurve, f::LineCurve);
root!(GeometryCacheRoot, GeometryCache, f::GeometryCache);
root!(AnimRoot, Anim, f::Anim);
root!(BakedAnimRoot, BakedAnim, f::BakedAnim);

fn made_from_scene<P, T>(scene: &Scene, port: P, make: impl FnOnce(&Kx, &P) -> T) -> Arc<Standalone<T, P>> {
    let port = Box::new(port);
    let k = Kx::of(scene);
    let value = Box::new(make(&k, &port));
    let owner = k.owner.upgrade();
    Arc::new(Standalone { value, _port: port, _keep: k.take_keep(), _owner: owner })
}

fn made_alone<P, T>(port: P, make: impl FnOnce(&Kx, &P) -> T) -> Arc<Standalone<T, P>> {
    let port = Box::new(port);
    let empty = f::Scene::default();
    let k = Kx::new(Bases::none(), &empty, ptr::null(), Weak::new());
    let value = Box::new(make(&k, &port));
    Arc::new(Standalone { value, _port: port, _keep: k.take_keep(), _owner: None })
}

pub struct OwnedProps {
    _port: Box<f::Props>,
    _keep: Vec<Box<dyn Any>>,
}

unsafe impl Send for OwnedProps {}
unsafe impl Sync for OwnedProps {}

fn own_prop(prop: f::Prop) -> Prop {
    let prop = Arc::new(prop);
    let empty = f::Scene::default();
    let k = Kx::new(Bases::none(), &empty, ptr::null(), Weak::new());
    let mut made = from_Prop(&k, &prop);
    made.own = Some(prop);
    made
}

fn own_props(props: f::Props) -> Props {
    let port = Box::new(props);
    let empty = f::Scene::default();
    let k = Kx::new(Bases::none(), &empty, ptr::null(), Weak::new());
    let mut made = from_Props(&k, &port);
    made.own = Some(Arc::new(OwnedProps { _port: port, _keep: k.take_keep() }));
    made
}

/// A `&'a` that ufbx-rust's signature leaves unbound to its arguments: what it points into outlives the call
fn unbound<'a, T: ?Sized>(r: &T) -> &'a T {
    unsafe { &*(r as *const T) }
}

fn sub_range<T>(whole: &[T], part: &[T]) -> std::ops::Range<usize> {
    let start = (part.as_ptr() as usize - whole.as_ptr() as usize) / std::mem::size_of::<T>().max(1);
    start..start + part.len()
}

fn position<T>(whole: &[T], part: &T) -> usize {
    (part as *const T as usize - whole.as_ptr() as usize) / std::mem::size_of::<T>().max(1)
}

fn scene_of(element: &Element) -> &Scene {
    &element.scene
}

fn element_of(scene: &Scene, id: u32) -> &Element {
    &scene.elements[id as usize]
}

#[repr(C)]
pub struct DomValue {
    pub type_: DomValueType,
    pub value_str: String,
    pub value_blob: Blob,
    pub value_int: i64,
    pub value_float: f64,
}

#[allow(non_snake_case)]
pub(crate) fn from_DomValue(_k: &Kx, p: &f::DomValue) -> DomValue {
    DomValue {
        type_: p.value_type,
        value_str: String::view(&p.value_str),
        value_blob: Blob::view(&p.value_blob),
        value_int: p.value_int,
        value_float: p.value_float,
    }
}

#[repr(C)]
pub struct VertexAttrib {
    pub exists: bool,
    pub values: VoidList,
    pub indices: List<u32>,
    pub value_reals: usize,
    pub unique_per_vertex: bool,
    pub values_w: List<Real>,
}

macro_rules! vertex_index {
    ($name:ident, $t:ty) => {
        impl Index<usize> for $name {
            type Output = $t;
            fn index(&self, index: usize) -> &$t {
                &self.values[self.indices[index] as usize]
            }
        }
    };
}

vertex_index!(VertexReal, Real);
vertex_index!(VertexVec2, Vec2);
vertex_index!(VertexVec3, Vec3);
vertex_index!(VertexVec4, Vec4);

impl Display for Vec2 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match (f.precision(), f.sign_plus()) {
            (None, false) => write!(f, "({}, {})", self.x, self.y),
            (None, true) => write!(f, "({:+}, {:+})", self.x, self.y),
            (Some(p), false) => write!(f, "({1:.0$}, {2:.0$})", p, self.x, self.y),
            (Some(p), true) => write!(f, "({1:+.0$}, {2:+.0$})", p, self.x, self.y),
        }
    }
}

impl Display for Vec3 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match (f.precision(), f.sign_plus()) {
            (None, false) => write!(f, "({}, {}, {})", self.x, self.y, self.z),
            (None, true) => write!(f, "({:+}, {:+}, {:+})", self.x, self.y, self.z),
            (Some(p), false) => write!(f, "({1:.0$}, {2:.0$}, {3:.0$})", p, self.x, self.y, self.z),
            (Some(p), true) => write!(f, "({1:+.0$}, {2:+.0$}, {3:+.0$})", p, self.x, self.y, self.z),
        }
    }
}

impl Display for Vec4 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match (f.precision(), f.sign_plus()) {
            (None, false) => write!(f, "({}, {}, {}, {})", self.x, self.y, self.z, self.w),
            (None, true) => write!(f, "({:+}, {:+}, {:+}, {:+})", self.x, self.y, self.z, self.w),
            (Some(p), false) => write!(f, "({1:.0$}, {2:.0$}, {3:.0$}, {4:.0$})", p, self.x, self.y, self.z, self.w),
            (Some(p), true) => write!(f, "({1:+.0$}, {2:+.0$}, {3:+.0$}, {4:+.0$})", p, self.x, self.y, self.z, self.w),
        }
    }
}

impl Vec2 {
    pub fn zero() -> Vec2 {
        zero_vec2()
    }
}

impl Vec3 {
    pub fn zero() -> Vec3 {
        zero_vec3()
    }
}

impl Vec4 {
    pub fn zero() -> Vec4 {
        zero_vec4()
    }
}

impl Quat {
    pub fn identity() -> Quat {
        identity_quat()
    }
}

impl Transform {
    pub fn identity() -> Transform {
        identity_transform()
    }
}

impl Matrix {
    pub fn identity() -> Matrix {
        identity_matrix()
    }
}

impl CoordinateAxes {
    pub fn right_handed_y_up() -> CoordinateAxes {
        axes_right_handed_y_up()
    }
    pub fn right_handed_z_up() -> CoordinateAxes {
        axes_right_handed_z_up()
    }
    pub fn left_handed_y_up() -> CoordinateAxes {
        axes_left_handed_y_up()
    }
    pub fn left_handed_z_up() -> CoordinateAxes {
        axes_left_handed_z_up()
    }
}

pub fn identity_matrix() -> Matrix {
    Matrix::IDENTITY
}
pub fn identity_transform() -> Transform {
    Transform::IDENTITY
}
pub fn zero_vec2() -> Vec2 {
    Vec2::ZERO
}
pub fn zero_vec3() -> Vec3 {
    Vec3::ZERO
}
pub fn zero_vec4() -> Vec4 {
    Vec4::default()
}
pub fn identity_quat() -> Quat {
    Quat::IDENTITY
}
pub fn axes_right_handed_y_up() -> CoordinateAxes {
    CoordinateAxes::RIGHT_HANDED_Y_UP
}
pub fn axes_right_handed_z_up() -> CoordinateAxes {
    CoordinateAxes::RIGHT_HANDED_Z_UP
}
pub fn axes_left_handed_y_up() -> CoordinateAxes {
    CoordinateAxes::LEFT_HANDED_Y_UP
}
pub fn axes_left_handed_z_up() -> CoordinateAxes {
    CoordinateAxes::LEFT_HANDED_Z_UP
}
#[allow(clippy::identity_op)] // ufbx-rust's packing, the major version at bit 24
pub fn source_version() -> u32 {
    (0 << 24) | (23 << 12) | 1
}

pub use crate::{
    coordinate_axes_valid, euler_to_quat, matrix_determinant, matrix_for_normals, matrix_invert, matrix_mul, matrix_to_transform, quat_dot, quat_fix_antipodal,
    quat_mul, quat_normalize, quat_rotate_vec3, quat_slerp, quat_to_euler, transform_direction, transform_position, transform_to_matrix, vec3_normalize,
};

pub fn triangulate_face_vec(indices: &mut Vec<u32>, mesh: &Mesh, face: Face) -> u32 {
    if face.num_indices < 3 {
        indices.clear();
        return 0;
    }
    let num_triangles = face.num_indices as usize - 2;
    indices.resize(num_triangles * 3, 0);
    let num_triangles = triangulate_face(indices, mesh, face);
    indices.shrink_to(num_triangles as usize * 3);
    num_triangles
}

pub fn is_thread_safe() -> bool {
    true
}

fn loaded(result: std::result::Result<f::Scene, f::Error>) -> Result<SceneRoot> {
    result.map(scene_root).map_err(to_error)
}

pub fn load_memory(data: &[u8], mut opts: LoadOpts) -> Result<SceneRoot> {
    loaded(f::load_memory(data, &opts.to_port()))
}

pub fn load_file(filename: &str, mut opts: LoadOpts) -> Result<SceneRoot> {
    loaded(f::load_file(filename, &opts.to_port()))
}

pub fn load_stdio(file: *mut c_void, opts: LoadOpts) -> Result<SceneRoot> {
    load_stdio_prefix(file, &[], opts)
}

pub fn load_stdio_prefix(file: *mut c_void, prefix: &[u8], mut opts: LoadOpts) -> Result<SceneRoot> {
    loaded(f::load_stream_prefix(&mut StdioReader(file), prefix, &opts.to_port()))
}

pub fn load_stream(stream: Stream, opts: LoadOpts) -> Result<SceneRoot> {
    load_stream_prefix(stream, &[], opts)
}

pub fn load_stream_prefix(stream: Stream, prefix: &[u8], mut opts: LoadOpts) -> Result<SceneRoot> {
    let mut port = opts.to_port();
    let mut reader = StreamReader(stream);
    // ufbx asks a stream its size only for the sake of a progress callback that has no estimate
    if port.progress_cb.is_some() && port.file_size_estimate == 0 {
        match reader.size() {
            u64::MAX => {
                reader.close();
                return Err(to_error(f::Error::new("Failed to get the size of the stream")));
            }
            size => port.file_size_estimate = size,
        }
    }
    let result = f::load_stream_prefix(&mut reader, prefix, &port);
    reader.close();
    loaded(result)
}

fn find_in<'a>(props: &'a Props, port: &f::Props, found: &f::Prop) -> Option<&'a Prop> {
    let found: *const f::Prop = found;
    if let Some(i) = port.props.iter().position(|p| ptr::eq(p, found)) {
        return props.props.get(i);
    }
    let defaults = props.defaults.as_ref()?;
    find_in(defaults, port.defaults.as_deref()?, unsafe { &*found })
}

pub fn find_prop<'a>(props: &'a Props, name: &str) -> Option<&'a Prop> {
    find_in(props, &props.port, props.port.find(name)?)
}

impl Props {
    pub fn find_prop<'a>(&'a self, name: &str) -> Option<&'a Prop> {
        find_prop(self, name)
    }
}

pub fn find_blob(props: &Props, name: &str, def: Blob) -> Blob {
    match props.port.find(name) {
        Some(p) => Blob::view(&p.value_blob),
        None => def,
    }
}

pub fn get_prop_element<'a>(element: &'a Element, prop: &Prop, type_: ElementType) -> Option<&'a Element> {
    let s = scene_of(element);
    f::get_prop_element(&s.port, &element.port, &prop.port, type_).map(|id| element_of(s, id))
}

pub fn find_prop_element<'a>(element: &'a Element, name: &str, type_: ElementType) -> Option<&'a Element> {
    let s = scene_of(element);
    f::find_prop_element(&s.port, &element.port, name, type_).map(|id| element_of(s, id))
}

pub fn find_element<'a>(scene: &'a Scene, type_: ElementType, name: &str) -> Option<&'a Element> {
    f::find_element(&scene.port, type_, name).map(|id| element_of(scene, id))
}

pub fn find_node<'a>(scene: &'a Scene, name: &str) -> Option<&'a Node> {
    f::find_node(&scene.port, name).map(|i| &scene.nodes[i])
}

pub fn find_anim_stack<'a>(scene: &'a Scene, name: &str) -> Option<&'a AnimStack> {
    f::find_anim_stack(&scene.port, name).map(|i| &scene.anim_stacks[i])
}

pub fn find_material<'a>(scene: &'a Scene, name: &str) -> Option<&'a Material> {
    f::find_material(&scene.port, name).map(|i| &scene.materials[i])
}

pub fn find_anim_prop<'a>(layer: &'a AnimLayer, element: &'a Element, prop: &str) -> Option<&'a AnimProp> {
    let found = f::find_anim_prop(&layer.port, element.element_id, prop)?;
    layer.anim_props.get(position(&layer.port.anim_props, found))
}

pub fn find_anim_props<'a>(layer: &'a AnimLayer, element: &'a Element) -> &'a [AnimProp] {
    let found = f::find_anim_props(&layer.port, element.element_id);
    if found.is_empty() {
        return &[];
    }
    &layer.anim_props.as_ref()[sub_range(&layer.port.anim_props, found)]
}

impl AnimLayer {
    pub fn find_anim_prop<'a>(&'a self, element: &'a Element, prop: &str) -> Option<&'a AnimProp> {
        find_anim_prop(self, element, prop)
    }
    pub fn find_anim_props<'a>(&'a self, element: &'a Element) -> &'a [AnimProp] {
        find_anim_props(self, element)
    }
}

pub fn get_compatible_matrix_for_normals(node: &Node) -> Matrix {
    f::get_compatible_matrix_for_normals(&node.port)
}

#[repr(C)]
pub struct InflateInput {
    pub total_size: usize,
    pub data: *const c_void,
    pub data_size: usize,
    pub buffer: *mut c_void,
    pub buffer_size: usize,
    pub read_fn: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize>,
    pub read_user: *mut c_void,
    pub progress_cb: RawProgressCb,
    pub progress_interval_hint: u64,
    pub progress_size_before: u64,
    pub progress_size_after: u64,
    pub no_header: bool,
    pub no_checksum: bool,
    pub internal_fast_bits: usize,
}

/// ufbx's trees kept between calls. This crate builds its own in each call, with the same result.
#[repr(C)]
pub struct InflateRetain {
    pub initialized: bool,
    pub data: [u64; 1024],
}

pub fn inflate(dst: &mut [u8], input: &InflateInput, retain: &mut InflateRetain) -> isize {
    let data = unsafe { slice_from_ptr(input.data as *const u8, input.data_size) };
    let (read, user) = (input.read_fn, input.read_user);
    let mut read_all = move |buf: &mut [u8]| match read {
        Some(read) => unsafe { read(user, buf.as_mut_ptr() as *mut c_void, buf.len()) },
        None => 0,
    };
    let progress = input.progress_cb;
    let user = Erased(progress.user);
    let progress_fn = move |p: &Progress| {
        let user = &user;
        match progress.fn_ {
            Some(cb) => to_enum_ProgressResult(unsafe { cb(user.0, p) }),
            None => f::ProgressResult::Continue,
        }
    };
    let mut port = f::InflateInput {
        total_size: input.total_size,
        data,
        buffer_size: input.buffer_size,
        read_fn: input.read_fn.map(|_| &mut read_all as &mut dyn FnMut(&mut [u8]) -> usize),
        progress_cb: progress.fn_.map(|_| &progress_fn as &(dyn Fn(&Progress) -> f::ProgressResult + Send + Sync)),
        progress_interval_hint: input.progress_interval_hint,
        progress_size_before: input.progress_size_before,
        progress_size_after: input.progress_size_after,
        no_header: input.no_header,
        no_checksum: input.no_checksum,
        internal_fast_bits: input.internal_fast_bits,
    };
    retain.initialized = true;
    f::inflate(dst, &mut port, &mut f::InflateRetain::new())
}

pub fn evaluate_curve(curve: &AnimCurve, time: f64, default_value: Real) -> Real {
    f::evaluate_curve(&curve.port, time, default_value)
}

pub fn evaluate_curve_flags(curve: &AnimCurve, time: f64, default_value: Real, flags: u32) -> Real {
    f::evaluate_curve_flags(&curve.port, time, default_value, flags)
}

impl AnimCurve {
    pub fn evaluate(&self, time: f64, default_value: Real) -> Real {
        evaluate_curve(self, time, default_value)
    }
}

pub fn evaluate_anim_value_real(anim_value: &AnimValue, time: f64) -> Real {
    f::evaluate_anim_value_real(&anim_value.element.scene.port, &anim_value.port, time)
}

pub fn evaluate_anim_value_vec3(anim_value: &AnimValue, time: f64) -> Vec3 {
    f::evaluate_anim_value_vec3(&anim_value.element.scene.port, &anim_value.port, time)
}

pub fn evaluate_anim_value_real_flags(anim_value: &AnimValue, time: f64, flags: u32) -> Real {
    f::evaluate_anim_value_real_flags(&anim_value.element.scene.port, &anim_value.port, time, flags)
}

pub fn evaluate_anim_value_vec3_flags(anim_value: &AnimValue, time: f64, flags: u32) -> Vec3 {
    f::evaluate_anim_value_vec3_flags(&anim_value.element.scene.port, &anim_value.port, time, flags)
}

impl AnimValue {
    pub fn evaluate_real(&self, time: f64) -> Real {
        evaluate_anim_value_real(self, time)
    }
    pub fn evaluate_vec3(&self, time: f64) -> Vec3 {
        evaluate_anim_value_vec3(self, time)
    }
}

pub fn evaluate_prop<'a, 'b>(anim: &'a Anim, element: &'a Element, name: &'b str, time: f64) -> ExternalRef<'b, Prop> {
    let prop = f::evaluate_prop(&scene_of(element).port, &anim.port, element.element_id, name, time);
    ExternalRef { data: own_prop(prop), _marker: PhantomData }
}

pub fn evaluate_prop_flags(anim: &Anim, element: &Element, name: &str, time: f64, flags: u32) -> Prop {
    own_prop(f::evaluate_prop_flags(&scene_of(element).port, &anim.port, element.element_id, name, time, flags))
}

pub fn evaluate_props<'a, 'b>(anim: &'a Anim, element: &'a Element, time: f64, buffer: &'b mut [ExternalRef<'b, Prop>]) -> ExternalRef<'b, Props> {
    let mut props = f::evaluate_props(&scene_of(element).port, &anim.port, element.element_id, time);
    props.props.truncate(buffer.len());
    ExternalRef { data: own_props(props), _marker: PhantomData }
}

pub fn evaluate_props_flags(anim: &Anim, element: &Element, time: f64, _buffer: &mut Prop, buffer_size: usize, flags: u32) -> Props {
    let mut props = f::evaluate_props_flags(&scene_of(element).port, &anim.port, element.element_id, time, flags);
    props.props.truncate(buffer_size);
    own_props(props)
}

pub fn evaluate_transform(anim: &Anim, node: &Node, time: f64) -> Transform {
    f::evaluate_transform(&node.element.scene.port, &anim.port, node.element.typed_id as usize, time)
}

pub fn evaluate_transform_flags(anim: &Anim, node: &Node, time: f64, flags: u32) -> Transform {
    f::evaluate_transform_flags(&node.element.scene.port, &anim.port, node.element.typed_id as usize, time, flags)
}

impl Node {
    pub fn get_compatible_matrix_for_normals(&self) -> Matrix {
        get_compatible_matrix_for_normals(self)
    }
    pub fn evaluate_transform(&self, anim: &Anim, time: f64) -> Transform {
        evaluate_transform(anim, self, time)
    }
}

pub fn evaluate_blend_weight(anim: &Anim, channel: &BlendChannel, time: f64) -> Real {
    f::evaluate_blend_weight(&channel.element.scene.port, &anim.port, channel.element.typed_id as usize, time)
}

pub fn evaluate_blend_weight_flags(anim: &Anim, channel: &BlendChannel, time: f64, flags: u32) -> Real {
    f::evaluate_blend_weight_flags(&channel.element.scene.port, &anim.port, channel.element.typed_id as usize, time, flags)
}

impl BlendChannel {
    pub fn evaluate_blend_weight(&self, anim: &Anim, time: f64) -> Real {
        evaluate_blend_weight(anim, self, time)
    }
}

pub fn evaluate_scene(scene: &Scene, anim: &Anim, time: f64, mut opts: EvaluateOpts) -> Result<SceneRoot> {
    loaded(f::evaluate_scene(&scene.port, &anim.port, time, &opts.to_port()))
}

pub fn create_anim(scene: &Scene, mut opts: AnimOpts) -> Result<AnimRoot> {
    let anim = f::create_anim(&scene.port, &opts.to_port()).map_err(to_error)?;
    opts.result_allocator.check(std::mem::size_of::<f::Anim>(), "result")?;
    Ok(AnimRoot { inner: made_from_scene(scene, anim, from_Anim) })
}

pub fn bake_anim(scene: &Scene, anim: &Anim, mut opts: BakeOpts) -> Result<BakedAnimRoot> {
    let bake = f::bake_anim(&scene.port, &anim.port, &opts.to_port()).map_err(to_error)?;
    opts.result_allocator.check(std::mem::size_of::<f::BakedAnim>(), "result")?;
    Ok(BakedAnimRoot { inner: made_alone(bake, from_BakedAnim) })
}

pub fn find_baked_node_by_typed_id<'a>(bake: &mut BakedAnim, typed_id: u32) -> Option<&'a BakedNode> {
    let i = bake.nodes.iter().position(|n| n.typed_id == typed_id)?;
    Some(unbound(&bake.nodes[i]))
}

pub fn find_baked_node<'a>(bake: &mut BakedAnim, node: &'a mut Node) -> Option<&'a BakedNode> {
    find_baked_node_by_typed_id(bake, node.element.typed_id)
}

pub fn find_baked_element_by_element_id<'a>(bake: &mut BakedAnim, element_id: u32) -> Option<&'a BakedElement> {
    let i = bake.elements.iter().position(|e| e.element_id == element_id)?;
    Some(unbound(&bake.elements[i]))
}

pub fn find_baked_element<'a>(bake: &mut BakedAnim, element: &'a mut Element) -> Option<&'a BakedElement> {
    find_baked_element_by_element_id(bake, element.element_id)
}

pub fn evaluate_baked_vec3(keyframes: &[BakedVec3], time: f64) -> Vec3 {
    let keys: Vec<f::BakedVec3> = keyframes.iter().map(|k| f::BakedVec3 { time: k.time, value: k.value, flags: k.flags.0 }).collect();
    f::evaluate_baked_vec3(&keys, time)
}

pub fn evaluate_baked_quat(keyframes: &[BakedQuat], time: f64) -> Quat {
    let keys: Vec<f::BakedQuat> = keyframes.iter().map(|k| f::BakedQuat { time: k.time, value: k.value, flags: k.flags.0 }).collect();
    f::evaluate_baked_quat(&keys, time)
}

pub fn get_bone_pose<'a>(pose: &'a Pose, node: &'a Node) -> Option<&'a BonePose> {
    let found = f::get_bone_pose(&pose.port, node.element.typed_id as usize)?;
    pose.bone_poses.get(position(&pose.port.bone_poses, found))
}

pub fn find_prop_texture<'a>(material: &'a Material, name: &str) -> Option<&'a Texture> {
    f::find_prop_texture(&material.port, name).map(|i| &material.element.scene.textures[i])
}

impl Material {
    pub fn find_prop_texture<'a>(&'a self, name: &str) -> Option<&'a Texture> {
        find_prop_texture(self, name)
    }
}

pub fn find_shader_prop<'a>(shader: &'a Shader, name: &'a str) -> &'a str {
    f::find_shader_prop(&shader.element.scene.port, &shader.port, name)
}

impl Shader {
    pub fn find_shader_prop<'a>(&'a self, name: &'a str) -> &'a str {
        find_shader_prop(self, name)
    }
}

pub fn find_shader_prop_bindings<'a>(shader: &'a Shader, name: &str) -> &'a [ShaderPropBinding] {
    let found = f::find_shader_prop_bindings(&shader.element.scene.port, &shader.port, name);
    if found.is_empty() {
        return &[];
    }
    for binding in &shader.bindings {
        let list = &binding.port.prop_bindings;
        let range = list.as_ptr_range();
        if range.contains(&found.as_ptr()) {
            return &binding.prop_bindings.as_ref()[sub_range(list, found)];
        }
    }
    &[]
}

pub fn find_shader_texture_input<'a>(shader: &ShaderTexture, name: &str) -> Option<&'a ShaderTextureInput> {
    let found = f::find_shader_texture_input(&shader.port, name)?;
    Some(unbound(&shader.inputs[position(&shader.port.inputs, found)]))
}

pub fn get_skin_vertex_matrix(skin: &SkinDeformer, vertex: usize, fallback: &Matrix) -> Matrix {
    named("get_skin_vertex_matrix", || f::get_skin_vertex_matrix(&skin.element.scene.port, &skin.port, vertex, Some(fallback)))
}

impl SkinDeformer {
    pub fn get_skin_vertex_matrix(&self, vertex: usize, fallback: &Matrix) -> Matrix {
        get_skin_vertex_matrix(self, vertex, fallback)
    }
}

pub fn get_blend_shape_offset_index(shape: &BlendShape, vertex: usize) -> u32 {
    f::get_blend_shape_offset_index(&shape.port, vertex)
}

pub fn get_blend_shape_vertex_offset(shape: &BlendShape, vertex: usize) -> Vec3 {
    f::get_blend_shape_vertex_offset(&shape.port, vertex)
}

pub fn get_blend_vertex_offset(blend: &BlendDeformer, vertex: usize) -> Vec3 {
    f::get_blend_vertex_offset(&blend.element.scene.port, &blend.port, vertex)
}

pub fn add_blend_shape_vertex_offsets(shape: &BlendShape, vertices: &mut [Vec3], weight: Real) {
    f::add_blend_shape_vertex_offsets(&shape.port, vertices, weight)
}

pub fn add_blend_vertex_offsets(blend: &BlendDeformer, vertices: &mut [Vec3], weight: Real) {
    f::add_blend_vertex_offsets(&blend.element.scene.port, &blend.port, vertices, weight)
}

impl BlendDeformer {
    pub fn get_vertex_offset(&self, vertex: usize) -> Vec3 {
        get_blend_vertex_offset(self, vertex)
    }
    pub fn add_vertex_offsets(&self, vertices: &mut [Vec3], weight: Real) {
        add_blend_vertex_offsets(self, vertices, weight)
    }
}

impl BlendShape {
    pub fn get_vertex_offset(&self, vertex: usize) -> Vec3 {
        get_blend_shape_vertex_offset(self, vertex)
    }
    pub fn add_vertex_offsets(&self, vertices: &mut [Vec3], weight: Real) {
        add_blend_shape_vertex_offsets(self, vertices, weight)
    }
}

pub fn evaluate_nurbs_basis(basis: &NurbsBasis, u: Real, weights: &mut [Real], derivatives: &mut [Real]) -> usize {
    f::evaluate_nurbs_basis(&basis.port, u, weights, derivatives)
}

impl NurbsBasis {
    pub fn evaluate(&self, u: Real, weights: &mut [Real], derivatives: &mut [Real]) -> usize {
        evaluate_nurbs_basis(self, u, weights, derivatives)
    }
}

pub fn evaluate_nurbs_curve(curve: &NurbsCurve, u: Real) -> CurvePoint {
    f::evaluate_nurbs_curve(&curve.port, u)
}

pub fn evaluate_nurbs_surface(surface: &NurbsSurface, u: Real, v: Real) -> SurfacePoint {
    f::evaluate_nurbs_surface(&surface.port, u, v)
}

pub fn tessellate_nurbs_curve(curve: &NurbsCurve, mut opts: TessellateCurveOpts) -> Result<LineCurveRoot> {
    let line = f::tessellate_nurbs_curve(&curve.port, &opts.to_port()).map_err(to_error)?;
    opts.result_allocator.check(std::mem::size_of::<f::LineCurve>(), "result")?;
    Ok(LineCurveRoot { inner: made_from_scene(&curve.element.scene, line, from_LineCurve) })
}

pub fn tessellate_nurbs_surface(surface: &NurbsSurface, mut opts: TessellateSurfaceOpts) -> Result<MeshRoot> {
    let mesh = f::tessellate_nurbs_surface(&surface.port, &opts.to_port()).map_err(to_error)?;
    opts.result_allocator.check(mesh_bytes(&mesh), "result")?;
    Ok(MeshRoot { inner: made_from_scene(&surface.element.scene, mesh, from_Mesh) })
}

impl NurbsCurve {
    pub fn evaluate(&self, u: Real) -> CurvePoint {
        evaluate_nurbs_curve(self, u)
    }
    pub fn tessellate(&self, opts: TessellateCurveOpts) -> Result<LineCurveRoot> {
        tessellate_nurbs_curve(self, opts)
    }
}

impl NurbsSurface {
    pub fn evaluate(&self, u: Real, v: Real) -> SurfacePoint {
        evaluate_nurbs_surface(self, u, v)
    }
    pub fn tessellate(&self, opts: TessellateSurfaceOpts) -> Result<MeshRoot> {
        tessellate_nurbs_surface(self, opts)
    }
}

fn mesh_bytes(mesh: &f::Mesh) -> usize {
    std::mem::size_of::<f::Mesh>() + mesh.num_indices * std::mem::size_of::<u32>()
}

pub fn find_face_index(mesh: &mut Mesh, index: usize) -> u32 {
    f::find_face_index(&mesh.port, index)
}

pub fn triangulate_face(indices: &mut [u32], mesh: &Mesh, face: Face) -> u32 {
    named("triangulate_face", || f::triangulate_face(indices, &mesh.port, face))
}

fn topo_to_port(topo: &[TopoEdge]) -> Vec<f::TopoEdge> {
    topo.iter().map(|t| f::TopoEdge { index: t.index, next: t.next, prev: t.prev, twin: t.twin, face: t.face, edge: t.edge, flags: t.flags.0 }).collect()
}

pub fn compute_topology(mesh: &Mesh, topo: &mut [TopoEdge]) {
    let mut port = topo_to_port(topo);
    named("compute_topology", || f::compute_topology(&mesh.port, &mut port));
    let empty = f::Scene::default();
    let k = Kx::new(Bases::none(), &empty, ptr::null(), Weak::new());
    for (t, p) in topo.iter_mut().zip(&port) {
        *t = from_TopoEdge(&k, p);
    }
}

pub fn topo_next_vertex_edge(topo: &[TopoEdge], index: u32) -> u32 {
    let port = topo_to_port(topo);
    named("topo_next_vertex_edge", || f::topo_next_vertex_edge(&port, index))
}

pub fn topo_prev_vertex_edge(topo: &[TopoEdge], index: u32) -> u32 {
    let port = topo_to_port(topo);
    named("topo_prev_vertex_edge", || f::topo_prev_vertex_edge(&port, index))
}

pub fn get_weighted_face_normal(positions: &VertexVec3, face: Face) -> Vec3 {
    named("get_weighted_face_normal", || f::get_weighted_face_normal(&positions.port, face))
}

pub fn generate_normal_mapping(mesh: &Mesh, topo: &[TopoEdge], normal_indices: &mut [u32], assume_smooth: bool) -> usize {
    let port = topo_to_port(topo);
    named("generate_normal_mapping", || f::generate_normal_mapping(&mesh.port, &port, normal_indices, assume_smooth))
}

pub fn compute_normals(mesh: &Mesh, positions: &VertexVec3, normal_indices: &[u32], normals: &mut [Vec3]) {
    named("compute_normals", || f::compute_normals(&mesh.port, &positions.port, normal_indices, normals))
}

pub fn subdivide_mesh(mesh: &Mesh, level: usize, mut opts: SubdivideOpts) -> Result<MeshRoot> {
    let scene: &Scene = &mesh.element.scene;
    let result = f::subdivide_mesh(&scene.port, &mesh.port, level, &opts.to_port()).map_err(to_error)?;
    opts.result_allocator.check(mesh_bytes(&result), "result")?;
    Ok(MeshRoot { inner: made_from_scene(scene, result, from_Mesh) })
}

impl Mesh {
    pub fn triangulate_face(&self, indices: &mut [u32], face: Face) -> u32 {
        triangulate_face(indices, self, face)
    }
    pub fn subdivide(&self, level: usize, opts: SubdivideOpts) -> Result<MeshRoot> {
        subdivide_mesh(self, level, opts)
    }
}

pub fn load_geometry_cache(filename: &str, mut opts: GeometryCacheOpts) -> Result<GeometryCacheRoot> {
    let cache = f::load_geometry_cache(filename, &opts.to_port()).map_err(to_error)?;
    Ok(GeometryCacheRoot { inner: made_alone(cache, from_GeometryCache) })
}

pub fn read_geometry_cache_real(frame: &CacheFrame, data: &mut [Real], mut opts: GeometryCacheDataOpts) -> usize {
    f::read_geometry_cache_real(&frame.port, data, &opts.to_port())
}

pub fn read_geometry_cache_vec3(frame: &CacheFrame, data: &mut [Vec3], mut opts: GeometryCacheDataOpts) -> usize {
    f::read_geometry_cache_vec3(&frame.port, data, &opts.to_port())
}

pub fn sample_geometry_cache_real(channel: &CacheChannel, time: f64, data: &mut [Real], mut opts: GeometryCacheDataOpts) -> usize {
    f::sample_geometry_cache_real(&channel.port, time, data, &opts.to_port())
}

pub fn sample_geometry_cache_vec3(channel: &CacheChannel, time: f64, data: &mut [Vec3], mut opts: GeometryCacheDataOpts) -> usize {
    f::sample_geometry_cache_vec3(&channel.port, time, data, &opts.to_port())
}

impl CacheFrame {
    pub fn read_real(&self, data: &mut [Real], opts: GeometryCacheDataOpts) -> usize {
        read_geometry_cache_real(self, data, opts)
    }
    pub fn read_vec3(&self, data: &mut [Vec3], opts: GeometryCacheDataOpts) -> usize {
        read_geometry_cache_vec3(self, data, opts)
    }
}

impl CacheChannel {
    pub fn sample_real(&self, time: f64, data: &mut [Real], opts: GeometryCacheDataOpts) -> usize {
        sample_geometry_cache_real(self, time, data, opts)
    }
    pub fn sample_vec3(&self, time: f64, data: &mut [Vec3], opts: GeometryCacheDataOpts) -> usize {
        sample_geometry_cache_vec3(self, time, data, opts)
    }
}

pub fn dom_find<'a>(parent: &DomNode, name: &str) -> Option<&'a DomNode> {
    let i = parent.port.children.iter().position(|c| c.name == name)?;
    Some(unbound(&parent.children[i]))
}

pub fn dom_is_array(node: &DomNode) -> bool {
    node.port.is_array()
}
pub fn dom_array_size(node: &DomNode) -> usize {
    node.port.array_size()
}
pub fn dom_as_int32_list<'a>(node: &DomNode) -> &'a [i32] {
    unbound(node.port.as_int32_list())
}
pub fn dom_as_int64_list<'a>(node: &DomNode) -> &'a [i64] {
    unbound(node.port.as_int64_list())
}
pub fn dom_as_float_list<'a>(node: &DomNode) -> &'a [f32] {
    unbound(node.port.as_float_list())
}
pub fn dom_as_double_list<'a>(node: &DomNode) -> &'a [f64] {
    unbound(node.port.as_double_list())
}
pub fn dom_as_real_list<'a>(node: &DomNode) -> &'a [Real] {
    unbound(node.port.as_real_list())
}
pub fn dom_as_blob_list<'a>(node: &DomNode) -> &'a [Blob] {
    unbound(node.blobs.as_ref())
}

impl DomNode {
    pub fn find<'a>(&'a self, name: &str) -> Option<&'a DomNode> {
        dom_find(self, name)
    }
    pub fn is_array(&self) -> bool {
        dom_is_array(self)
    }
    pub fn array_size(&self) -> usize {
        dom_array_size(self)
    }
    pub fn as_int32_list(&self) -> &[i32] {
        dom_as_int32_list(self)
    }
    pub fn as_int64_list(&self) -> &[i64] {
        dom_as_int64_list(self)
    }
    pub fn as_float_list(&self) -> &[f32] {
        dom_as_float_list(self)
    }
    pub fn as_double_list(&self) -> &[f64] {
        dom_as_double_list(self)
    }
    pub fn as_real_list(&self) -> &[Real] {
        dom_as_real_list(self)
    }
    pub fn as_blob_list(&self) -> &[Blob] {
        dom_as_blob_list(self)
    }
}

impl<'a> f::VertexStream<'a> {
    pub fn new<T: Copy + Sized>(data: &'a mut [T]) -> f::VertexStream<'a> {
        let vertex_count = data.len();
        let vertex_size = std::mem::size_of::<T>();
        let bytes = unsafe { slice::from_raw_parts_mut(data.as_mut_ptr() as *mut u8, std::mem::size_of_val(data)) };
        f::VertexStream { data: bytes, vertex_count, vertex_size }
    }
}

pub use crate::VertexStream;

pub fn generate_indices(streams: &mut [VertexStream], indices: &mut [u32], _allocator: AllocatorOpts) -> Result<usize> {
    f::generate_indices(streams, indices).map_err(to_error)
}

fn vertex_at<T>(v: &List<T>, indices: &List<u32>, index: usize) -> usize {
    let _ = v;
    indices[index] as usize
}

pub fn get_vertex_real(v: &VertexReal, index: usize) -> Real {
    v.values[vertex_at(&v.values, &v.indices, index)]
}
pub fn get_vertex_vec2(v: &VertexVec2, index: usize) -> Vec2 {
    v.values[vertex_at(&v.values, &v.indices, index)]
}
pub fn get_vertex_vec3(v: &VertexVec3, index: usize) -> Vec3 {
    v.values[vertex_at(&v.values, &v.indices, index)]
}
pub fn get_vertex_vec4(v: &VertexVec4, index: usize) -> Vec4 {
    v.values[vertex_at(&v.values, &v.indices, index)]
}
pub fn get_vertex_w_vec3(v: &VertexVec3, index: usize) -> Real {
    if v.values_w.is_empty() { 0.0 } else { v.values_w[vertex_at(&v.values, &v.indices, index)] }
}

impl Scene {
    pub fn find_element<'a>(&'a self, type_: ElementType, name: &str) -> Option<&'a Element> {
        find_element(self, type_, name)
    }
    pub fn find_node<'a>(&'a self, name: &str) -> Option<&'a Node> {
        find_node(self, name)
    }
    pub fn find_anim_stack<'a>(&'a self, name: &str) -> Option<&'a AnimStack> {
        find_anim_stack(self, name)
    }
    pub fn evaluate(&self, anim: &Anim, time: f64, opts: EvaluateOpts) -> Result<SceneRoot> {
        evaluate_scene(self, anim, time, opts)
    }
}
