use std::ffi::c_void;
use std::sync::Mutex;

use crate::common::Rng;
use crate::runner::{Args, Child, Job, Section, Status, Tally, corpus_jobs};

const CASES_PER_BATCH: u64 = 1000;
const BIG: usize = 64 << 20;

#[derive(Clone, Debug)]
struct Reader {
    chunk: usize,
    err_call: Option<usize>,
}

#[derive(Clone, Debug)]
struct Progress {
    hint: u64,
    before: u64,
    after: u64,
    cancel_at: Option<u64>,
}

#[derive(Clone)]
struct Call<'a> {
    src: &'a [u8],
    total_size: usize,
    data_len: usize,
    dst_size: usize,
    reader: Option<Reader>,
    buffer_size: usize,
    progress: Option<Progress>,
    no_header: bool,
    no_checksum: bool,
    fast_bits: usize,
}

impl<'a> Call<'a> {
    fn plain(src: &'a [u8], total_size: usize, dst_size: usize) -> Call<'a> {
        Call {
            src,
            total_size,
            data_len: src.len(),
            dst_size,
            reader: None,
            buffer_size: 0,
            progress: None,
            no_header: false,
            no_checksum: false,
            fast_bits: 0,
        }
    }

    fn describe(&self) -> String {
        format!(
            "total {} data {} dst {} reader {:?} buffer {} progress {:?} no_header {} no_checksum {} fast_bits {}",
            self.total_size, self.data_len, self.dst_size, self.reader, self.buffer_size, self.progress, self.no_header, self.no_checksum, self.fast_bits
        )
    }
}

struct Case<'a> {
    calls: Vec<Call<'a>>,
    shared_retain: bool,
}

#[derive(Default, PartialEq, Debug)]
struct Log {
    count: u64,
    hash: u64,
    head: Vec<(u64, u64)>,
}

impl Log {
    fn push(&mut self, a: u64, b: u64) {
        self.count += 1;
        self.hash = (self.hash ^ a).wrapping_mul(0x100000001b3);
        self.hash = (self.hash ^ b).wrapping_mul(0x100000001b3);
        if self.head.len() < 4096 {
            self.head.push((a, b));
        }
    }

    fn first_difference(&self, o: &Log) -> String {
        let at = self.head.iter().zip(&o.head).position(|(x, y)| x != y).unwrap_or(self.head.len().min(o.head.len()));
        format!("from call {at}: C {:?} of {}, port {:?} of {}", self.head.get(at), self.count, o.head.get(at), o.count)
    }
}

struct Outcome {
    ret: isize,
    dst: Vec<u8>,
    progress: Log,
    reads: Log,
}

fn pattern(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i.wrapping_mul(131).wrapping_add(7) >> 1) as u8).collect()
}

fn read_step(src: &[u8], pos: &mut usize, spec: &Reader, log: &mut Log, out: &mut [u8]) -> usize {
    let n = if spec.err_call == Some(log.count as usize) {
        usize::MAX
    } else {
        let n = out.len().min(spec.chunk).min(src.len() - *pos);
        out[..n].copy_from_slice(&src[*pos..*pos + n]);
        *pos += n;
        n
    };
    log.push(out.len() as u64, n as u64);
    n
}

struct CReader<'a> {
    src: &'a [u8],
    pos: usize,
    spec: Reader,
    log: Log,
}

unsafe extern "C" fn c_read(user: *mut c_void, data: *mut c_void, size: usize) -> usize {
    let r = unsafe { &mut *(user as *mut CReader) };
    let out: &mut [u8] = if size == 0 { &mut [] } else { unsafe { std::slice::from_raw_parts_mut(data as *mut u8, size) } };
    read_step(r.src, &mut r.pos, &r.spec, &mut r.log, out)
}

struct CProgress {
    cancel_at: Option<u64>,
    log: Log,
}

unsafe extern "C" fn c_progress(user: *mut c_void, p: *const ufbx::Progress) -> ufbx::ProgressResult {
    let s = unsafe { &mut *(user as *mut CProgress) };
    let p = unsafe { &*p };
    let k = s.log.count;
    s.log.push(p.bytes_read, p.bytes_total);
    if s.cancel_at == Some(k) { ufbx::ProgressResult::Cancel } else { ufbx::ProgressResult::Continue }
}

fn run_c(case: &Case) -> Vec<Outcome> {
    let mut retain = Box::new(ufbx::InflateRetain { initialized: false, data: [0; 1024] });
    let mut outcomes = Vec::new();
    for call in &case.calls {
        if !case.shared_retain {
            retain.initialized = false;
        }
        let data = &call.src[..call.data_len];
        let mut dst = pattern(call.dst_size);
        let spec = call.reader.clone().unwrap_or(Reader { chunk: 0, err_call: None });
        let mut reader = CReader { src: &call.src[call.data_len..], pos: 0, spec, log: Log::default() };
        let p = call.progress.as_ref();
        let mut prog = CProgress { cancel_at: p.and_then(|p| p.cancel_at), log: Log::default() };
        let mut buffer = vec![0u8; call.buffer_size];
        let input = ufbx::InflateInput {
            total_size: call.total_size,
            data: data.as_ptr() as *const c_void,
            data_size: data.len(),
            buffer: if call.buffer_size > 0 { buffer.as_mut_ptr() as *mut c_void } else { std::ptr::null_mut() },
            buffer_size: call.buffer_size,
            read_fn: if call.reader.is_some() { Some(c_read) } else { None },
            read_user: &mut reader as *mut CReader as *mut c_void,
            progress_cb: ufbx::RawProgressCb { fn_: p.map(|_| c_progress as _), user: &mut prog as *mut CProgress as *mut c_void },
            progress_interval_hint: p.map_or(0, |p| p.hint),
            progress_size_before: p.map_or(0, |p| p.before),
            progress_size_after: p.map_or(0, |p| p.after),
            no_header: call.no_header,
            no_checksum: call.no_checksum,
            internal_fast_bits: call.fast_bits,
        };
        let ret = ufbx::inflate(&mut dst, &input, &mut retain);
        outcomes.push(Outcome { ret, dst, progress: prog.log, reads: reader.log });
    }
    outcomes
}

fn run_port(case: &Case) -> Vec<Outcome> {
    let mut retain = load3d::InflateRetain::new();
    let mut outcomes = Vec::new();
    for call in &case.calls {
        if !case.shared_retain {
            retain = load3d::InflateRetain::new();
        }
        let mut dst = pattern(call.dst_size);
        let p = call.progress.as_ref();
        let cancel_at = p.and_then(|p| p.cancel_at);
        let plog = Mutex::new(Log::default());
        let cb = |pr: &load3d::Progress| {
            let mut l = plog.lock().unwrap();
            let k = l.count;
            l.push(pr.bytes_read, pr.bytes_total);
            if cancel_at == Some(k) { load3d::ProgressResult::Cancel } else { load3d::ProgressResult::Continue }
        };
        let spec = call.reader.clone().unwrap_or(Reader { chunk: 0, err_call: None });
        let rest = &call.src[call.data_len..];
        let (mut pos, mut rlog) = (0, Log::default());
        let mut read = |out: &mut [u8]| read_step(rest, &mut pos, &spec, &mut rlog, out);
        let mut input = load3d::InflateInput {
            total_size: call.total_size,
            data: &call.src[..call.data_len],
            buffer_size: call.buffer_size,
            read_fn: if call.reader.is_some() { Some(&mut read) } else { None },
            progress_cb: if p.is_some() { Some(&cb) } else { None },
            progress_interval_hint: p.map_or(0, |p| p.hint),
            progress_size_before: p.map_or(0, |p| p.before),
            progress_size_after: p.map_or(0, |p| p.after),
            no_header: call.no_header,
            no_checksum: call.no_checksum,
            internal_fast_bits: call.fast_bits,
        };
        let ret = load3d::inflate(&mut dst, &mut input, &mut retain);
        outcomes.push(Outcome { ret, dst, progress: plog.into_inner().unwrap(), reads: rlog });
    }
    outcomes
}

fn check(t: &mut Tally, field: &str, ok: bool, what: impl FnOnce() -> String) {
    if !t.check(ok, what) {
        *t.fields.entry(field.to_string()).or_default() += 1;
    }
}

/// With `self_test` C against itself; error-equal where every call failed in both
fn compare(t: &mut Tally, case: &Case, self_test: bool) {
    let c = run_c(case);
    let p = if self_test { run_c(case) } else { run_port(case) };
    for (k, (call, (a, b))) in case.calls.iter().zip(c.iter().zip(&p)).enumerate() {
        t.count("calls", 1);
        t.count(if a.ret >= 0 { "decoded" } else { "failed" }, 1);
        t.count("progress calls", a.progress.count);
        t.count("reads", a.reads.count);
        let at = || format!("call {k} ({})", call.describe());
        check(t, "return value", a.ret == b.ret, || format!("{}: C returned {}, port {}", at(), a.ret, b.ret));
        check(t, "destination", a.dst == b.dst, || {
            let i = a.dst.iter().zip(&b.dst).position(|(x, y)| x != y).unwrap_or(0);
            format!("{}: byte {i} of {}: C {:#04x}, port {:#04x}", at(), a.dst.len(), a.dst[i], b.dst[i])
        });
        if call.progress.is_some() {
            check(t, "progress calls", a.progress == b.progress, || format!("{}: progress {}", at(), a.progress.first_difference(&b.progress)));
        }
        if call.reader.is_some() {
            check(t, "reads", a.reads == b.reads, || format!("{}: reads {}", at(), a.reads.first_difference(&b.reads)));
        }
    }
    if t.diffs == 0 && c.iter().all(|o| o.ret < 0) {
        t.status(Status::ErrorEqual, "");
    }
}

/// (offset, encoded size, decoded size) of each array whose encoding word is 1
fn streams(d: &[u8]) -> Vec<(usize, usize, u64)> {
    if !d.starts_with(b"Kaydara FBX Binary") {
        return Vec::new();
    }
    let word = |p: usize, le: bool| {
        let b: [u8; 4] = d[p..p + 4].try_into().unwrap();
        if le { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) }
    };
    let mut out = Vec::new();
    for p in 13..d.len().saturating_sub(1) {
        if d[p] != 0x78 || ![0x01, 0x5e, 0x9c, 0xda].contains(&d[p + 1]) {
            continue;
        }
        let elem = match d[p - 13] {
            b'f' | b'i' => 4u64,
            b'd' | b'l' => 8,
            b'b' | b'c' => 1,
            _ => continue,
        };
        if let Some((count, size)) = [true, false].into_iter().find_map(|le| {
            let (count, encoding, size) = (word(p - 12, le), word(p - 8, le), word(p - 4, le));
            (encoding == 1 && (2..1 << 31).contains(&size)).then_some((count, size as usize))
        }) {
            out.push((p, size, count as u64 * elem));
        }
    }
    out
}

fn one(calls: Vec<Call>) -> Case {
    Case { calls, shared_retain: false }
}

fn corpus_file(a: &Args, path: &str, ch: &mut Child) {
    let d = std::fs::read(path).unwrap_or_default();
    for (p, size, decoded) in streams(&d) {
        let src = &d[p..];
        let exact = if decoded as usize > BIG { 1 << 20 } else { decoded as usize };
        let plain = |dst| Call::plain(src, size, dst);
        let mut sizes = vec![plain(exact)];
        if decoded as usize <= BIG {
            sizes.extend([exact.saturating_sub(1), exact / 2, exact + 17].map(plain));
        }
        ch.step("sizes", |t| compare(t, &one(sizes), a.self_test));
        let mut options = vec![Call { no_checksum: true, ..plain(exact) }];
        options.extend([1, 4, 8, 10].map(|fast_bits| Call { fast_bits, ..plain(exact) }));
        ch.step("options", |t| compare(t, &one(options), a.self_test));
        let splits = [
            (0, usize::MAX, None, 0),
            (1, 7, None, 257),
            (63, 4096, None, 4096),
            (64, 1, None, 0),
            (size / 2, usize::MAX, None, 65536),
            (size / 2, 4096, None, 300),
            (size / 3, usize::MAX, Some(0), 0),
            (17, 4096, Some(2), 1000),
        ];
        let reads = splits
            .map(|(data_len, chunk, err_call, buffer_size)| Call {
                data_len: data_len.min(src.len()),
                reader: Some(Reader { chunk, err_call }),
                buffer_size,
                ..plain(exact)
            })
            .to_vec();
        ch.step("reader", |t| compare(t, &one(reads), a.self_test));
        let progress = |hint, cancel_at| Call { progress: Some(Progress { hint, before: p as u64, after: 1000, cancel_at }), ..plain(exact) };
        ch.step("progress", |t| compare(t, &one([0, 1, 100, 4096, u64::MAX].map(|h| progress(h, None)).to_vec()), a.self_test));
        ch.step("cancel", |t| compare(t, &one([0, 1, 3].map(|k| progress(64, Some(k))).to_vec()), a.self_test));
        let streamed = Call {
            data_len: 40.min(src.len()),
            reader: Some(Reader { chunk: 4096, err_call: None }),
            progress: Some(Progress { hint: 512, before: 5, after: 7, cancel_at: None }),
            ..plain(exact)
        };
        ch.step("retain", |t| compare(t, &Case { calls: vec![streamed, Call { fast_bits: 6, ..plain(exact) }], shared_retain: true }, a.self_test));
    }
}

const LEN_BASE: [u16; 29] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LEN_EXTRA: [u32; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DIST_BASE: [u16; 30] =
    [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577];
const DIST_EXTRA: [u32; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];
const CODE_LENGTH_ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

fn upto(r: &mut Rng, n: usize) -> usize {
    r.below(n as u64 + 1) as usize
}

fn sometimes(r: &mut Rng, n: usize) -> usize {
    if r.below(4) == 0 { upto(r, n) } else { 0 }
}

fn pick<T: Copy>(r: &mut Rng, xs: &[T]) -> T {
    xs[r.below(xs.len() as u64) as usize]
}

#[derive(Default)]
struct Bits {
    out: Vec<u8>,
    acc: u64,
    n: u32,
}

impl Bits {
    fn put(&mut self, v: u32, n: u32) {
        self.acc |= ((v as u64) & ((1u64 << n) - 1)) << self.n;
        self.n += n;
        while self.n >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }

    fn code(&mut self, code: u32, len: u8) {
        let rev = (0..len as u32).fold(0, |acc, i| acc << 1 | (code >> i & 1));
        self.put(rev, len as u32);
    }

    fn align(&mut self) {
        if self.n > 0 {
            self.put(0, 8 - self.n);
        }
    }
}

#[derive(Clone, Copy)]
enum Token {
    Lit(u8),
    Match { sym: usize, extra: u32, dist: usize, dist_extra: u32 },
}

fn tokens(r: &mut Rng, out: &mut Vec<u8>, n: usize) -> Vec<Token> {
    let small = r.below(2) == 0;
    let (base, alphabet, matches) = (r.below(256), 1 + r.below(if small { 8 } else { 256 }), r.below(9));
    let mut v = Vec::with_capacity(n);
    while v.len() < n && out.len() < 1 << 20 {
        if out.is_empty() || r.below(8) >= matches {
            let b = ((base + r.below(alphabet)) & 0xff) as u8;
            out.push(b);
            v.push(Token::Lit(b));
            continue;
        }
        let window = out.len().min(32768);
        let distance = match r.below(4) {
            0 => 1 + upto(r, window.min(8) - 1),
            1 => 1 + upto(r, window.min(300) - 1),
            2 => window - upto(r, window.min(16) - 1),
            _ => 1 + upto(r, window - 1),
        };
        let length = match r.below(5) {
            0 => 258,
            1 => 3 + upto(r, 255),
            _ => 3 + upto(r, 30),
        };
        for _ in 0..length {
            out.push(out[out.len() - distance]);
        }
        let (sym, extra) = if length == 258 && r.below(4) == 0 {
            // 284 with all five extra bits set: 258 the long way, which ufbx reads as zlib does
            (284, 31)
        } else {
            let i = (0..29).rev().find(|&i| LEN_BASE[i] as usize <= length).unwrap();
            (257 + i, (length - LEN_BASE[i] as usize) as u32)
        };
        let d = (0..30).rev().find(|&i| DIST_BASE[i] as usize <= distance).unwrap();
        v.push(Token::Match { sym, extra, dist: d, dist_extra: (distance - DIST_BASE[d] as usize) as u32 });
    }
    v
}

/// A complete code over the used symbols; a lone symbol gets the one-bit code ufbx asks for
fn code_lengths(r: &mut Rng, used: &[bool], limit: u32) -> Vec<u8> {
    let syms: Vec<usize> = (0..used.len()).filter(|&i| used[i]).collect();
    let mut lens = vec![0u8; used.len()];
    match syms.len() {
        0 => {}
        1 => lens[syms[0]] = 1,
        k => {
            let deep = r.below(3) == 0;
            let mut depth = vec![0u32];
            while depth.len() < k {
                let open: Vec<usize> = (0..depth.len()).filter(|&i| depth[i] < limit).collect();
                let i = if deep { *open.iter().max_by_key(|&&i| depth[i]).unwrap() } else { pick(r, &open) };
                depth[i] += 1;
                depth.push(depth[i]);
            }
            for i in (1..k).rev() {
                depth.swap(i, upto(r, i));
            }
            for (&s, d) in syms.iter().zip(depth) {
                lens[s] = d as u8;
            }
        }
    }
    lens
}

fn perturb(r: &mut Rng, lens: &mut [u8], limit: u8) {
    let i = r.below(lens.len() as u64) as usize;
    lens[i] = (lens[i] + 1 + r.below(2) as u8).min(limit);
}

fn canonical(lens: &[u8]) -> Vec<u32> {
    let mut count = [0u32; 16];
    for &l in lens {
        count[l as usize] += 1;
    }
    count[0] = 0;
    let (mut next, mut code) = ([0u32; 16], 0u32);
    for b in 1..16 {
        code = (code + count[b - 1]) << 1;
        next[b] = code;
    }
    lens.iter()
        .map(|&l| {
            let c = next[l as usize];
            next[l as usize] += 1;
            c
        })
        .collect()
}

fn write_tokens(w: &mut Bits, toks: &[Token], lit: &[u8], dist: &[u8]) {
    let (lit_codes, dist_codes) = (canonical(lit), canonical(dist));
    for t in toks {
        match *t {
            Token::Lit(b) => w.code(lit_codes[b as usize], lit[b as usize]),
            Token::Match { sym, extra, dist: d, dist_extra } => {
                w.code(lit_codes[sym], lit[sym]);
                w.put(extra, LEN_EXTRA[sym - 257]);
                w.code(dist_codes[d], dist[d]);
                w.put(dist_extra, DIST_EXTRA[d]);
            }
        }
    }
    w.code(lit_codes[256], lit[256]);
}

/// 16 repeats ufbx's `prev`, which is 0 before any length
fn run_lengths(r: &mut Rng, lens: &[u8]) -> Vec<(usize, u32, u32)> {
    let (mut out, mut i, mut prev) = (Vec::new(), 0, 0u8);
    while i < lens.len() {
        let l = lens[i];
        let run = lens[i..].iter().take_while(|&&x| x == l).count();
        let rle = r.below(5) != 0;
        if rle && l == 0 && run >= 11 {
            let n = 11 + upto(r, run.min(138) - 11);
            out.push((18, n as u32 - 11, 7));
            (i, prev) = (i + n, 0);
        } else if rle && l == 0 && run >= 3 && r.below(2) == 0 {
            let n = run.min(10);
            out.push((17, n as u32 - 3, 3));
            (i, prev) = (i + n, 0);
        } else if rle && l == prev && run >= 3 && (i > 0 || r.below(4) == 0) {
            let n = run.min(6);
            out.push((16, n as u32 - 3, 2));
            i += n;
        } else {
            out.push((l as usize, 0, 0));
            (i, prev) = (i + 1, l);
        }
    }
    out
}

/// One time in 15 one of its three codes is broken on purpose
fn dynamic_block(r: &mut Rng, w: &mut Bits, toks: &[Token]) -> bool {
    let (mut lit_used, mut dist_used) = ([false; 288], [false; 32]);
    lit_used[256] = true;
    for t in toks {
        match *t {
            Token::Lit(b) => lit_used[b as usize] = true,
            Token::Match { sym, dist, .. } => (lit_used[sym], dist_used[dist]) = (true, true),
        }
    }
    for _ in 0..r.below(4) {
        lit_used[r.below(288) as usize] = true;
    }
    for _ in 0..r.below(3) {
        dist_used[r.below(32) as usize] = true;
    }
    let lit_end = 257.max(lit_used.iter().rposition(|&u| u).unwrap() + 1);
    let num_lit = lit_end + sometimes(r, 288 - lit_end);
    let dist_end = 1.max(dist_used.iter().rposition(|&u| u).map_or(0, |i| i + 1));
    let num_dist = dist_end + sometimes(r, 32 - dist_end);
    let mut lit = code_lengths(r, &lit_used[..num_lit], 15);
    let mut dist = code_lengths(r, &dist_used[..num_dist], 15);
    let broken = if r.below(15) == 0 { 1 + r.below(3) } else { 0 };
    match broken {
        1 => perturb(r, &mut lit, 15),
        2 => perturb(r, &mut dist, 15),
        _ => {}
    }
    let all: Vec<u8> = lit.iter().chain(&dist).copied().collect();
    let runs = run_lengths(r, &all);
    let mut cl_used = [false; 19];
    for &(s, ..) in &runs {
        cl_used[s] = true;
    }
    let mut cl = code_lengths(r, &cl_used, 7);
    if broken == 3 {
        perturb(r, &mut cl, 7);
    }
    let cl_end = 4.max(CODE_LENGTH_ORDER.iter().rposition(|&s| cl[s] != 0).map_or(0, |i| i + 1));
    let num_cl = cl_end + sometimes(r, 19 - cl_end);
    w.put(num_lit as u32 - 257, 5);
    w.put(num_dist as u32 - 1, 5);
    w.put(num_cl as u32 - 4, 4);
    for &s in &CODE_LENGTH_ORDER[..num_cl] {
        w.put(cl[s] as u32, 3);
    }
    let cl_codes = canonical(&cl);
    for (s, extra, bits) in runs {
        w.code(cl_codes[s], cl[s]);
        w.put(extra, bits);
    }
    write_tokens(w, toks, &lit, &dist);
    broken == 0
}

/// (decoded bytes, stream, no code broken); `raw` leaves out the zlib header and checksum
fn zlib_stream(r: &mut Rng, raw: bool) -> (Vec<u8>, Vec<u8>, bool) {
    let (mut out, mut w, mut valid) = (Vec::new(), Bits::default(), true);
    if !raw {
        w.out.extend_from_slice(&[0x78, pick(r, &[0x01, 0x5e, 0x9c, 0xda])]);
    }
    let several = r.below(3) == 0;
    let blocks = 1 + r.below(if several { 4 } else { 1 });
    let scale = pick(r, &[3, 50, 50, 1000, 1000, 1000, 8000, 30000]);
    for b in 0..blocks {
        w.put((b + 1 == blocks) as u32, 1);
        match r.below(10) {
            0..=1 => {
                w.put(0, 2);
                w.align();
                let n = upto(r, (scale * 4).min(65535));
                w.put(n as u32, 16);
                w.put(!n as u32 & 0xffff, 16);
                for _ in 0..n {
                    let byte = if out.is_empty() || r.below(2) == 0 { r.below(256) as u8 } else { out[out.len() - 1 - upto(r, out.len().min(64) - 1)] };
                    out.push(byte);
                    w.out.push(byte);
                }
            }
            2..=4 => {
                w.put(1, 2);
                let n = upto(r, scale);
                let toks = tokens(r, &mut out, n);
                let lit: Vec<u8> = (0..288).map(|i| [8, 9, 7, 8][(i >= 144) as usize + (i >= 256) as usize + (i >= 280) as usize]).collect();
                write_tokens(&mut w, &toks, &lit, &[5; 32]);
            }
            _ => {
                w.put(2, 2);
                let n = upto(r, scale);
                let toks = tokens(r, &mut out, n);
                valid &= dynamic_block(r, &mut w, &toks);
            }
        }
    }
    w.align();
    if !raw {
        let (mut a, mut b) = (1u32, 0u32);
        for &x in &out {
            a = (a + x as u32) % 65521;
            b = (b + a) % 65521;
        }
        w.out.extend_from_slice(&(b << 16 | a).to_be_bytes());
    }
    (out, w.out, valid)
}

fn mutate(r: &mut Rng, z: &mut Vec<u8>) {
    let len = z.len();
    let at = |r: &mut Rng| if r.below(2) == 0 { r.below(len.min(48) as u64) as usize } else { r.below(len as u64) as usize };
    match r.below(8) {
        _ if len == 0 => z.push(r.below(256) as u8),
        0 | 1 => {
            let many = r.below(4) == 0;
            for _ in 0..1 + r.below(if many { 64 } else { 4 }) {
                let i = at(r);
                z[i] ^= 1 << r.below(8);
            }
        }
        2 => {
            for _ in 0..1 + r.below(4) {
                let i = at(r);
                z[i] = r.below(256) as u8;
            }
        }
        3 => z.truncate(upto(r, len)),
        4 => z.extend((0..upto(r, 200)).map(|_| r.below(256) as u8)),
        5 => {
            let i = upto(r, len);
            z.drain(i..(i + 1 + upto(r, 16)).min(len));
        }
        6 => {
            let i = upto(r, len);
            let ins: Vec<u8> = (0..1 + upto(r, 16)).map(|_| r.below(256) as u8).collect();
            z.splice(i..i, ins);
        }
        _ => {
            for b in &mut z[len.saturating_sub(4)..] {
                *b = r.below(256) as u8;
            }
        }
    }
}

fn random_call(r: &mut Rng, src: &mut Vec<u8>) -> (Call<'static>, &'static str) {
    let raw = r.below(6) == 0;
    let (data, mut z, _) = zlib_stream(r, raw);
    let no_header = if raw { r.below(10) != 0 } else { r.below(25) == 0 };
    if r.below(2) == 0 {
        mutate(r, &mut z);
    }
    let zlen = z.len();
    let tail = if r.below(3) == 0 { upto(r, 80) } else { 0 };
    z.extend((0..tail).map(|_| r.below(256) as u8));
    let total_size = match r.below(12) {
        0 => zlen + upto(r, 100),
        1 => upto(r, zlen),
        2 => z.len(),
        _ => zlen,
    };
    let n = data.len();
    let dst_size = match r.below(10) {
        0 => n.saturating_sub(1 + upto(r, 16)),
        1 => n + 1 + upto(r, 64),
        2 => upto(r, n),
        _ => n,
    };
    *src = z;
    let mut call = Call::plain(&[], total_size, dst_size);
    call.data_len = src.len();
    call.no_header = no_header;
    call.buffer_size = pick(r, &[0, 0, 0, 1, 100, 256, 257, 300, 1000, 4096, 65536]);
    let mut kind = "random";
    if r.below(3) == 0 {
        let (any_len, any_chunk) = (upto(r, src.len()), 1 + upto(r, 300));
        call.data_len = pick(r, &[0, 1, 7, 63, 64, 65, any_len, any_len]).min(src.len());
        let chunk = pick(r, &[1, 7, 4096, usize::MAX, usize::MAX, any_chunk]);
        let err_call = if r.below(4) == 0 { Some(upto(r, 3)) } else { None };
        call.reader = Some(Reader { chunk, err_call });
        kind = "random reader";
    } else if r.below(8) == 0 {
        call.data_len = upto(r, src.len());
    }
    if r.below(3) == 0 {
        let hint = pick(r, &[0, 1, 7, 8, 9, 16, 64, 100, 1000, 4096, 0x4000, 1 << 40, u64::MAX - 8, u64::MAX - 7, u64::MAX - 1, u64::MAX]);
        let size = |r: &mut Rng| match r.below(4) {
            0 => 0,
            1 => r.below(1 << 20),
            2 => u64::MAX - r.below(1 << 20),
            _ => r.next_u64(),
        };
        let (before, after) = (size(r), size(r));
        let cancel_at = if r.below(2) == 0 { Some(r.below(6)) } else { None };
        call.progress = Some(Progress { hint, before, after, cancel_at });
        kind = if cancel_at.is_some() { "random cancel" } else { "random progress" };
    }
    call.no_checksum = if raw { r.below(10) != 0 } else { r.below(8) == 0 };
    if r.below(3) == 0 {
        call.fast_bits = pick(r, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0x1_0000_000a, usize::MAX]);
    }
    (call, kind)
}

fn random_batch(a: &Args, batch: u64, ch: &mut Child) {
    for i in batch * CASES_PER_BATCH..(batch + 1) * CASES_PER_BATCH {
        let mut r = Rng::named(&["inflate", &i.to_string()]);
        let calls = if r.below(5) == 0 { 2 + r.below(3) as usize } else { 1 };
        let shared_retain = calls > 1 && r.below(3) != 0;
        let mut srcs: Vec<Vec<u8>> = vec![Vec::new(); calls];
        let drawn: Vec<(Call, &str)> = srcs.iter_mut().map(|s| random_call(&mut r, s)).collect();
        let kind = if calls > 1 { if shared_retain { "random calls, one retain" } else { "random calls, fresh retains" } } else { drawn[0].1 };
        let case = Case { calls: drawn.into_iter().zip(&srcs).map(|((c, _), s)| Call { src: s, ..c }).collect(), shared_retain };
        ch.step(kind, |t| compare(t, &case, a.self_test));
    }
}

fn jobs(a: &Args) -> Vec<Job> {
    let mut v: Vec<Job> = corpus_jobs(a, true, &["fbx"]).into_iter().filter(|j| std::fs::read(&j.item).is_ok_and(|d| !streams(&d).is_empty())).collect();
    v.extend((0..a.mutants.div_ceil(50) as u64).map(|b| Job::generated(format!("gen:inflate:{b}"))));
    v
}

fn run(a: &Args, item: &str, ch: &mut Child) {
    match item.strip_prefix("gen:inflate:") {
        Some(b) => random_batch(a, b.parse().expect("a batch number"), ch),
        None => corpus_file(a, item, ch),
    }
}

pub const INFLATE: Section = Section { name: "inflate", jobs, run, header: None };

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn written_streams_decode_in_c_to_what_they_were_written_from() {
        let mut decoded = 0;
        for i in 0..2000 {
            let mut r = Rng::named(&["inflate writer", &i.to_string()]);
            let raw = i % 5 == 0;
            let (data, z, valid) = zlib_stream(&mut r, raw);
            if !valid {
                continue;
            }
            let call = Call { no_header: raw, no_checksum: raw, ..Call::plain(&z, z.len(), data.len()) };
            let out = run_c(&Case { calls: vec![call], shared_retain: false }).remove(0);
            assert_eq!(out.ret, data.len() as isize, "stream {i}");
            assert!(out.dst == data, "stream {i}");
            decoded += 1;
        }
        assert!(decoded > 1800, "{decoded}");
    }
}
