//! Each input of a section runs in a child process of this program, so that an abort or a hang in either library is
//! reported, not fatal; a child that dies is started again after the step it died in.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    Equal,
    ErrorEqual,
    KnownC,
    Differ,
    ErrorMismatch,
    Unported,
    PortPanic,
    OraclePanic,
    Crash,
    Timeout,
}

const STATUSES: [Status; 10] = [
    Status::Equal,
    Status::ErrorEqual,
    Status::KnownC,
    Status::Differ,
    Status::ErrorMismatch,
    Status::Unported,
    Status::PortPanic,
    Status::OraclePanic,
    Status::Crash,
    Status::Timeout,
];

impl Status {
    pub fn name(self) -> &'static str {
        match self {
            Status::Equal => "equal",
            Status::ErrorEqual => "error-equal",
            Status::KnownC => "known-c-failure",
            Status::Differ => "differ",
            Status::ErrorMismatch => "error-mismatch",
            Status::Unported => "unported",
            Status::PortPanic => "port-panic",
            Status::OraclePanic => "oracle-panic",
            Status::Crash => "crash",
            Status::Timeout => "timeout",
        }
    }

    pub fn from_name(s: &str) -> Option<Status> {
        STATUSES.into_iter().find(|x| x.name() == s)
    }

    pub fn ok(self) -> bool {
        matches!(self, Status::Equal | Status::ErrorEqual | Status::KnownC)
    }
}

pub struct Args {
    pub corpus: PathBuf,
    pub jobs: usize,
    pub timeout: u64,
    pub keep: usize,
    pub filter: Vec<String>,
    pub limit: usize,
    pub passes: Option<Vec<String>>,
    pub self_test: bool,
    pub mutants: usize,
    pub random: u64,
    pub verbose: bool,
    pub child: Option<String>,
    pub from: usize,
}

impl Default for Args {
    fn default() -> Args {
        Args {
            corpus: PathBuf::new(),
            jobs: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4),
            timeout: 300,
            keep: 10,
            filter: Vec::new(),
            limit: usize::MAX,
            passes: None,
            self_test: false,
            mutants: 10000,
            random: 200,
            verbose: false,
            child: None,
            from: 0,
        }
    }
}

impl Args {
    fn forwarded(&self) -> Vec<String> {
        let mut v = vec!["--corpus".into(), self.corpus.display().to_string(), "--keep".into(), self.keep.to_string()];
        if let Some(p) = &self.passes {
            v.push("--passes".into());
            v.push(p.join(","));
        }
        if self.self_test {
            v.push("--self-test".into());
        }
        v
    }

    pub fn wanted<'a, T>(&self, all: &'a [T], name: impl Fn(&T) -> &str) -> Vec<&'a T> {
        all.iter().filter(|x| self.passes.as_ref().is_none_or(|p| p.iter().any(|n| n == name(x)))).collect()
    }
}

pub struct Job {
    /// What the child is given: a path, or a `gen:` name the section reads
    pub item: String,
    pub label: String,
    pub generated: bool,
}

impl Job {
    pub fn generated(item: String) -> Job {
        Job { label: item.clone(), item, generated: true }
    }
}

pub struct Section {
    pub name: &'static str,
    pub jobs: fn(&Args) -> Vec<Job>,
    pub run: fn(&Args, &str, &mut Child),
    pub header: Option<fn(&Args)>,
}

pub fn corpus_jobs(a: &Args, recursive: bool, exts: &[&str]) -> Vec<Job> {
    fn walk(dir: &Path, recursive: bool, exts: &[&str], out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                if recursive {
                    walk(&p, recursive, exts, out);
                }
            } else if p.extension().and_then(|e| e.to_str()).is_some_and(|e| exts.iter().any(|x| x.eq_ignore_ascii_case(e))) {
                out.push(p);
            }
        }
    }
    let corpus = a.corpus.canonicalize().unwrap_or(a.corpus.clone());
    let mut files = Vec::new();
    walk(&corpus, recursive, exts, &mut files);
    files
        .into_iter()
        .map(|p| Job { label: p.strip_prefix(&corpus).unwrap_or(&p).display().to_string(), item: p.display().to_string(), generated: false })
        .collect()
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n")
}

fn unesc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(ch) = it.next() {
        if ch == '\\' {
            match it.next() {
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some(o) => out.push(o),
                None => {}
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn emit(line: std::fmt::Arguments) {
    let mut o = std::io::stdout().lock();
    let _ = o.write_fmt(line);
    let _ = o.write_all(b"\n");
    let _ = o.flush();
}

pub struct Tally {
    idx: usize,
    keep: usize,
    pub checks: u64,
    pub diffs: u64,
    pub kept: Vec<String>,
    /// Differing field paths without their indices -> differences there
    pub fields: HashMap<String, u64>,
    counts: Vec<(&'static str, u64)>,
    status: Option<(Status, String)>,
    place: Option<String>,
}

impl Tally {
    fn new(idx: usize, keep: usize) -> Tally {
        Tally { idx, keep, checks: 0, diffs: 0, kept: Vec::new(), fields: HashMap::new(), counts: Vec::new(), status: None, place: None }
    }

    pub fn check(&mut self, ok: bool, what: impl FnOnce() -> String) -> bool {
        self.checks += 1;
        if !ok {
            self.diffs += 1;
            if self.kept.len() < self.keep {
                self.kept.push(what());
            }
        }
        ok
    }

    pub fn keep(&mut self, what: impl FnOnce() -> String) {
        if self.kept.len() < self.keep {
            self.kept.push(what());
        }
    }

    pub fn count(&mut self, what: &'static str, n: u64) {
        match self.counts.iter_mut().find(|(k, _)| *k == what) {
            Some(e) => e.1 += n,
            None => self.counts.push((what, n)),
        }
    }

    pub fn status(&mut self, s: Status, detail: impl Into<String>) {
        self.status = Some((s, detail.into()));
    }

    pub fn known_failure(&mut self, why: &str, place: &str) {
        self.status(Status::KnownC, why);
        self.place = Some(place.to_string());
    }

    pub fn known_c<R>(&mut self, why: &str, place: &str, f: impl FnOnce() -> R) -> R {
        emit(format_args!("C\t{}\t{}\t{}", self.idx, esc(why), esc(place)));
        let r = f();
        emit(format_args!("c\t{}", self.idx));
        r
    }
}

pub struct Child {
    from: usize,
    next: usize,
    keep: usize,
    fault: Option<(String, String)>,
}

impl Child {
    pub fn new(a: &Args) -> Child {
        install_panic_hook();
        // for the runner's own tests: `abort:<step kind>` or `hang:<step kind>`
        let fault = std::env::var("FBX_ORACLE_TEST_FAULT").ok().and_then(|f| f.split_once(':').map(|(a, b)| (a.to_string(), b.to_string())));
        Child { from: a.from, next: 0, keep: a.keep, fault }
    }

    /// A step before `--from` was run by an earlier child
    pub fn step(&mut self, kind: &str, f: impl FnOnce(&mut Tally)) {
        let idx = self.next;
        self.next += 1;
        if idx < self.from {
            return;
        }
        emit(format_args!("B\t{idx}\t{}", esc(kind)));
        if let Some((what, at)) = &self.fault
            && at == kind
        {
            match what.as_str() {
                "abort" => std::process::abort(),
                "hang" => loop {
                    std::thread::park();
                },
                _ => {}
            }
        }
        let mut t = Tally::new(idx, self.keep);
        if catch_unwind(AssertUnwindSafe(|| f(&mut t))).is_err() {
            let (status, detail) = take_panic();
            t.status(status, detail);
        }
        let status = t.status.take().unwrap_or_else(|| (if t.diffs > 0 { Status::Differ } else { Status::Equal }, String::new()));
        let mut o = std::io::stdout().lock();
        for d in &t.kept {
            let _ = writeln!(o, "D\t{idx}\t{}", esc(d));
        }
        for (k, n) in &t.fields {
            let _ = writeln!(o, "F\t{idx}\t{}\t{n}", esc(k));
        }
        for (k, n) in &t.counts {
            let _ = writeln!(o, "N\t{idx}\t{}\t{n}", esc(k));
        }
        if let Some(p) = &t.place {
            let _ = writeln!(o, "P\t{idx}\t{}", esc(p));
        }
        let _ = writeln!(o, "R\t{idx}\t{}\t{}\t{}\t{}", status.0.name(), t.checks, t.diffs, esc(&status.1));
        let _ = o.flush();
    }
}

thread_local! {
    static LAST_PANIC: std::cell::RefCell<Option<(String, String)>> = const { std::cell::RefCell::new(None) };
}

pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "panic".to_string()
        };
        let at = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        LAST_PANIC.with(|p| *p.borrow_mut() = Some((msg, at)));
    }));
}

pub fn take_panic() -> (Status, String) {
    let (msg, at) = LAST_PANIC.with(|p| p.borrow_mut().take()).unwrap_or_else(|| ("panic".into(), String::new()));
    let place = match function_at(&at) {
        Some(f) => format!("{f} ({})", short_path(&at)),
        None => short_path(&at),
    };
    if msg.starts_with("not yet implemented") || msg.starts_with("not implemented") {
        return (Status::Unported, place);
    }
    let port_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src");
    let in_port =
        at.rsplit_once(':').and_then(|(f, _)| Path::new(f).canonicalize().ok()).zip(port_src.canonicalize().ok()).is_some_and(|(f, p)| f.starts_with(p));
    let first = msg.lines().next().unwrap_or("");
    (if in_port { Status::PortPanic } else { Status::OraclePanic }, format!("{first:.120} at {place}"))
}

fn function_at(at: &str) -> Option<String> {
    let (file, line) = at.rsplit_once(':')?;
    let line: usize = line.parse().ok()?;
    let path = Path::new(file);
    let text = std::fs::read_to_string(path).or_else(|_| std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))).ok()?;
    let lines: Vec<&str> = text.lines().take(line).collect();
    lines.iter().rev().find_map(|l| {
        let i = l.find("fn ")?;
        let rest = &l[i + 3..];
        let end = rest.find(|ch: char| !(ch.is_alphanumeric() || ch == '_'))?;
        (end > 0).then(|| rest[..end].to_string())
    })
}

fn short_path(at: &str) -> String {
    match at.rfind("/src/") {
        Some(i) => at[i + 1..].to_string(),
        None => at.to_string(),
    }
}

pub struct StepOut {
    pub kind: String,
    pub place: String,
    pub status: Status,
    pub detail: String,
    pub checks: u64,
    pub diffs: u64,
    pub kept: Vec<String>,
    pub fields: Vec<(String, u64)>,
    pub counts: Vec<(String, u64)>,
}

impl StepOut {
    fn new(kind: String) -> StepOut {
        StepOut {
            kind,
            place: String::new(),
            status: Status::Crash,
            detail: String::new(),
            checks: 0,
            diffs: 0,
            kept: Vec::new(),
            fields: Vec::new(),
            counts: Vec::new(),
        }
    }

    fn line(&self) -> String {
        if self.detail.is_empty() { self.status.name().to_string() } else { format!("{}: {}", self.status.name(), self.detail) }
    }
}

fn run_job(exe: &Path, section: &str, a: &Args, job: &Job) -> Vec<StepOut> {
    let mut steps: Vec<StepOut> = Vec::new();
    let mut from = 0;
    loop {
        let mut cmd = Command::new(exe);
        cmd.arg(section).arg("--child").arg(&job.item).arg("--from").arg(from.to_string()).args(a.forwarded());
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let mut s = StepOut::new("(start)".into());
                s.detail = format!("cannot start the child: {e}");
                steps.push(s);
                return steps;
            }
        };
        let stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let err_reader = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = stderr.read_to_string(&mut s);
            s
        });
        let reader = std::thread::spawn(move || BufReader::new(stdout).lines().map_while(Result::ok).collect::<Vec<String>>());
        let deadline = Instant::now() + Duration::from_secs(a.timeout);
        let mut timed_out = false;
        let status = loop {
            match child.try_wait() {
                Ok(Some(s)) => break Some(s),
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    timed_out = true;
                    break child.wait().ok();
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(2)),
                Err(_) => break None,
            }
        };
        let lines = reader.join().unwrap_or_default();
        let err = err_reader.join().unwrap_or_default();

        let mut open: Option<(usize, StepOut)> = None;
        let mut known_c: Option<(String, String)> = None;
        for line in &lines {
            let f: Vec<&str> = line.split('\t').collect();
            let num = |s: &str| s.parse::<u64>().unwrap_or(0);
            match f.as_slice() {
                ["B", i, kind] => {
                    open = Some((num(i) as usize, StepOut::new(unesc(kind))));
                    known_c = None;
                }
                ["C", _, why, place] => known_c = Some((unesc(why), unesc(place))),
                ["c", _] => known_c = None,
                ["D", _, text] => open.iter_mut().for_each(|(_, s)| s.kept.push(unesc(text))),
                ["F", _, k, n] => open.iter_mut().for_each(|(_, s)| s.fields.push((unesc(k), num(n)))),
                ["N", _, k, n] => open.iter_mut().for_each(|(_, s)| s.counts.push((unesc(k), num(n)))),
                ["P", _, place] => open.iter_mut().for_each(|(_, s)| s.place = unesc(place)),
                ["R", _, st, checks, diffs, detail] => {
                    if let Some((_, mut s)) = open.take() {
                        s.status = Status::from_name(st).unwrap_or(Status::Crash);
                        s.checks = num(checks);
                        s.diffs = num(diffs);
                        s.detail = unesc(detail);
                        steps.push(s);
                    }
                }
                _ => {}
            }
        }
        if !timed_out && status.is_some_and(|s| s.success()) {
            return steps;
        }
        let code = status.map(|s| format!("{s}")).unwrap_or_else(|| "unknown exit".into());
        let last = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").to_string();
        let how = format!("{code} {last:.160}").trim().to_string();
        let Some((idx, mut s)) = open else {
            let mut s = StepOut::new("(outside a step)".into());
            s.detail = how;
            steps.push(s);
            return steps;
        };
        (s.status, s.detail) = match (&known_c, timed_out) {
            (_, true) => (Status::Timeout, format!("no answer within {} s", a.timeout)),
            (Some((why, place)), false) => {
                s.place = place.clone();
                (Status::KnownC, format!("{why}: {how}"))
            }
            (None, false) => (Status::Crash, how),
        };
        steps.push(s);
        from = idx + 1;
    }
}

pub struct Summary {
    pub line: String,
    pub ok: bool,
}

pub fn run_section(sec: &Section, a: &Args) -> Summary {
    let started = Instant::now();
    let mut jobs = (sec.jobs)(a);
    jobs.retain(|j| a.filter.is_empty() || a.filter.iter().any(|f| j.label.contains(f.as_str())));
    jobs.truncate(a.limit);
    println!("== {}: {} inputs under {}", sec.name, jobs.len(), a.corpus.display());
    if jobs.is_empty() {
        let line = format!("{}: no inputs", sec.name);
        println!("{line}\n");
        return Summary { line, ok: false };
    }
    if let Some(h) = sec.header {
        h(a);
    }
    let exe = std::env::current_exe().expect("own executable");
    let results: Mutex<Vec<Option<Vec<StepOut>>>> = Mutex::new((0..jobs.len()).map(|_| None).collect());
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let shown = Mutex::new(Instant::now());
    std::thread::scope(|s| {
        for _ in 0..a.jobs.max(1) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    if i >= jobs.len() {
                        break;
                    }
                    let steps = run_job(&exe, sec.name, a, &jobs[i]);
                    results.lock().unwrap()[i] = Some(steps);
                    let n = done.fetch_add(1, Ordering::SeqCst) + 1;
                    let mut last = shown.lock().unwrap();
                    if last.elapsed() >= Duration::from_secs(10) {
                        *last = Instant::now();
                        eprintln!("[{}: {n}/{} inputs, {:.0} s]", sec.name, jobs.len(), started.elapsed().as_secs_f64());
                    }
                }
            });
        }
    });
    let results: Vec<Vec<StepOut>> = results.into_inner().unwrap().into_iter().map(Option::unwrap_or_default).collect();
    report(sec.name, a, &jobs, &results, started.elapsed())
}

fn report(name: &str, a: &Args, jobs: &[Job], results: &[Vec<StepOut>], took: Duration) -> Summary {
    let mut kinds: Vec<String> = Vec::new();
    let mut per_kind: HashMap<String, HashMap<Status, usize>> = HashMap::new();
    let mut kind_checks: HashMap<String, u64> = HashMap::new();
    let (mut steps, mut checks, mut diffs, mut known, mut failed) = (0usize, 0u64, 0u64, 0usize, 0usize);
    let mut counts: Vec<(String, u64)> = Vec::new();
    let mut fields: HashMap<String, (usize, u64)> = HashMap::new();
    let mut why: HashMap<String, usize> = HashMap::new();
    let mut known_at: Vec<String> = Vec::new();
    let mut first: Vec<String> = Vec::new();
    let mut listed = 0;
    for (job, out) in jobs.iter().zip(results) {
        let bad = out.iter().any(|s| !s.status.ok());
        listed += bad as usize;
        if a.verbose || (bad && listed <= a.keep) {
            println!("{}:", job.label);
            for s in out.iter().filter(|s| a.verbose || !s.status.ok()) {
                println!("  {:<12} {}{}", s.kind, s.line(), if s.place.is_empty() { String::new() } else { format!(", at {}", s.place) });
                for d in &s.kept {
                    println!("      {d}");
                }
                if s.diffs > s.kept.len() as u64 && !s.kept.is_empty() {
                    println!("      … {} more", s.diffs - s.kept.len() as u64);
                }
            }
        }
        for s in out {
            steps += 1;
            checks += s.checks;
            diffs += s.diffs;
            if !kinds.contains(&s.kind) {
                kinds.push(s.kind.clone());
            }
            *per_kind.entry(s.kind.clone()).or_default().entry(s.status).or_default() += 1;
            *kind_checks.entry(s.kind.clone()).or_default() += s.checks;
            for (k, n) in &s.counts {
                match counts.iter_mut().find(|(x, _)| x == k) {
                    Some(e) => e.1 += n,
                    None => counts.push((k.clone(), *n)),
                }
            }
            for (k, n) in &s.fields {
                let e = fields.entry(k.clone()).or_default();
                e.0 += 1;
                e.1 += n;
            }
            for d in &s.kept {
                if first.len() < a.keep {
                    first.push(format!("{} [{}]: {d}", job.label, s.kind));
                }
            }
            match s.status {
                Status::KnownC => {
                    known += 1;
                    *why.entry(format!("{}: {}", s.status.name(), s.detail)).or_default() += 1;
                    if known_at.len() < 3 {
                        known_at.push(format!("{} [{}] {}", job.label, s.kind, s.place));
                    }
                }
                st if !st.ok() => {
                    failed += 1;
                    if st != Status::Differ {
                        *why.entry(format!("{}: {}", s.status.name(), s.detail)).or_default() += 1;
                    }
                }
                _ => {}
            }
        }
    }
    let corpus = jobs.iter().filter(|j| !j.generated).count();
    let generated = jobs.len() - corpus;
    let inputs = if generated > 0 { format!("{corpus} files + {generated} generated inputs") } else { format!("{corpus} files") };
    if listed > a.keep && !a.verbose {
        println!("… {} more inputs with failures (--verbose lists every input)", listed - a.keep);
    }
    let line = format!(
        "{name}: {inputs}, {steps} steps, {checks} checks, {diffs} differences, {known} known C failures, {failed} failed steps, {:.1} s",
        took.as_secs_f64()
    );
    println!("{line}");
    let width = kinds.iter().map(|k| k.len()).max().unwrap_or(0);
    for k in &kinds {
        let n = &per_kind[k];
        let parts: Vec<String> = STATUSES.iter().filter(|s| n.contains_key(s)).map(|s| format!("{} {}", s.name(), n[s])).collect();
        println!("  {k:<width$}  {}; {} checks", parts.join(", "), kind_checks[k]);
    }
    if !counts.is_empty() {
        println!("  counted: {}", counts.iter().map(|(k, n)| format!("{k} {n}")).collect::<Vec<_>>().join(", "));
    }
    if !known_at.is_empty() {
        println!("  known C failures at {}{}", known_at.join(", "), if known > known_at.len() { ", …" } else { "" });
    }
    let mut v: Vec<_> = why.into_iter().collect();
    v.sort_by(|x, y| y.1.cmp(&x.1).then(x.0.cmp(&y.0)));
    for (k, n) in v.iter().take(a.keep) {
        println!("  {n:>6}  {k:.200}");
    }
    if !fields.is_empty() {
        let mut v: Vec<_> = fields.into_iter().collect();
        v.sort_by(|x, y| y.1.0.cmp(&x.1.0).then(y.1.1.cmp(&x.1.1)).then(x.0.cmp(&y.0)));
        println!("  most frequent differing fields (steps, differences):");
        for (k, (np, nd)) in v.iter().take(a.keep) {
            println!("    {np:>6} {nd:>10}  {k}");
        }
    }
    if !first.is_empty() {
        println!("  first differences:");
        for d in &first {
            println!("    {d:.300}");
        }
    }
    println!();
    Summary { line, ok: failed == 0 }
}
