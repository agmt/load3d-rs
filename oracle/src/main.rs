use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

use fbx_oracle::SECTIONS;
use fbx_oracle::runner::{Args, Child, run_section, take_panic};

fn usage() -> ! {
    let names: Vec<&str> = SECTIONS.iter().map(|s| s.name).collect();
    eprintln!(
        "usage: fbx-oracle ufbx-version\n       fbx-oracle <{}|all> --corpus DIR [--jobs N] [--timeout SECS] [--keep N] [--filter SUBSTR]... [--limit N]\n\
         \x20                 [--passes a,b] [--self-test] [--mutants N] [--random N] [--verbose]\n\
         DIR is the `data` folder of the ufbx repository at the version the ufbx crate bundles",
        names.join("|")
    );
    std::process::exit(2)
}

fn parse_args(args: &[String]) -> Args {
    let mut a = Args::default();
    let mut it = args.iter();
    let mut corpus = std::env::var_os("FBX_ORACLE_CORPUS").map(PathBuf::from);
    while let Some(k) = it.next() {
        let mut val = || it.next().cloned().unwrap_or_else(|| usage());
        let num = |s: String| s.parse().unwrap_or_else(|_| usage());
        match k.as_str() {
            "--corpus" => corpus = Some(PathBuf::from(val())),
            "--jobs" => a.jobs = num(val()) as usize,
            "--timeout" => a.timeout = num(val()),
            "--keep" => a.keep = num(val()) as usize,
            "--filter" => a.filter.push(val()),
            "--limit" => a.limit = num(val()) as usize,
            "--passes" => a.passes = Some(val().split(',').map(str::to_string).collect()),
            "--self-test" => a.self_test = true,
            "--mutants" => a.mutants = num(val()) as usize,
            "--random" => a.random = num(val()),
            "--verbose" => a.verbose = true,
            "--child" => a.child = Some(val()),
            "--from" => a.from = num(val()) as usize,
            _ => usage(),
        }
    }
    a.corpus = corpus.unwrap_or_else(|| usage());
    a
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else { usage() };
    // the release tag of the ufbx this crate builds, whose `data` folder the sections read
    if cmd == "ufbx-version" {
        let v = ufbx::source_version();
        println!("v{}.{}.{}", v / 1_000_000, v / 1000 % 1000, v % 1000);
        return;
    }
    let a = parse_args(&args[1..]);
    if let Some(item) = &a.child {
        let sec = SECTIONS.iter().find(|s| s.name == cmd).unwrap_or_else(|| usage());
        let mut child = Child::new(&a);
        if catch_unwind(AssertUnwindSafe(|| (sec.run)(&a, item, &mut child))).is_err() {
            eprintln!("{}", take_panic().1);
            std::process::exit(101);
        }
        return;
    }
    let chosen: Vec<_> = if cmd == "all" { SECTIONS.iter().collect() } else { SECTIONS.iter().filter(|s| s.name == cmd).collect() };
    if chosen.is_empty() {
        usage();
    }
    let summaries: Vec<_> = chosen.iter().map(|s| run_section(s, &a)).collect();
    if chosen.len() > 1 {
        println!("== all");
        for s in &summaries {
            println!("{} {}", if s.ok { "ok  " } else { "FAIL" }, s.line);
        }
    }
    if summaries.iter().any(|s| !s.ok) {
        std::process::exit(1);
    }
}
