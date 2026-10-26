use std::path::PathBuf;
use std::process::{Command, Output};

const CUBE: &str = "; FBX 7.4.0 project file
FBXHeaderExtension:  {
\tFBXHeaderVersion: 1003
\tFBXVersion: 7400
\tCreator: \"fbx-oracle tests\"
}
GlobalSettings:  {
\tVersion: 1000
\tProperties70:  {
\t\tP: \"UpAxis\", \"int\", \"Integer\", \"\",1
\t}
}
Objects:  {
\tModel: 1, \"Model::Cube\", \"Mesh\" {
\t\tVersion: 232
\t}
}
Connections:  {
\tC: \"OO\",1,0
}
";

fn corpus(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fbx-oracle-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("cube_7400_ascii.fbx"), CUBE).unwrap();
    dir
}

fn oracle(args: &[&str], fault: &str) -> (Output, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_fbx-oracle")).args(args).env("FBX_ORACLE_TEST_FAULT", fault).output().expect("the oracle runs");
    let s = String::from_utf8(out.stdout.clone()).unwrap();
    (out, s)
}

fn scene_with_fault(fault: &str) -> (Output, String) {
    let dir = corpus(fault.split(':').next().unwrap());
    let r = oracle(&["scene", "--corpus", dir.to_str().unwrap(), "--passes", "default,dom,no_geometry", "--timeout", "2", "--self-test"], fault);
    let _ = std::fs::remove_dir_all(&dir);
    r
}

#[test]
fn an_aborting_child_is_a_crash_and_the_next_step_runs() {
    let (out, s) = scene_with_fault("abort:dom");
    assert!(!out.status.success(), "{s}");
    assert!(s.contains("  dom          crash: "), "{s}");
    assert!(s.contains("  default      equal 1;"), "{s}");
    assert!(s.contains("  no_geometry  equal 1;"), "{s}");
    assert!(s.contains("scene: 1 files, 3 steps,"), "{s}");
}

#[test]
fn a_hanging_child_is_a_timeout() {
    let (out, s) = scene_with_fault("hang:dom");
    assert!(!out.status.success(), "{s}");
    assert!(s.contains("  dom          timeout: no answer within 2 s"), "{s}");
}

#[test]
fn equal_scenes_pass() {
    let (out, s) = scene_with_fault("none:none");
    assert!(out.status.success(), "{s}");
    assert!(s.contains("0 differences, 0 known C failures, 0 failed steps"), "{s}");
}

#[test]
fn a_missing_file_reads_as_empty_on_both_sides() {
    let (_, s) = oracle(&["scene", "--child", "/nonexistent.fbx", "--from", "0", "--corpus", "/nonexistent", "--passes", "default"], "");
    assert!(s.contains("R\t0\terror-equal\t1\t0\tEmptyFile \"Empty file\""), "{s}");
}

#[test]
fn a_child_starts_from_the_step_it_is_given() {
    let dir = corpus("from");
    let file = dir.join("cube_7400_ascii.fbx");
    let (_, s) = oracle(&["scene", "--child", file.to_str().unwrap(), "--from", "2", "--corpus", dir.to_str().unwrap(), "--self-test"], "");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!s.contains("B\t1\t") && s.contains("B\t2\tnormals"), "{s}");
}

#[test]
fn generated_inputs_need_no_corpus() {
    let dir = corpus("generated");
    let (out, s) = oracle(&["subdivide", "--corpus", dir.to_str().unwrap(), "--random", "3", "--filter", "gen:"], "");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(s.contains("subdivide: 0 files + 3 generated inputs"), "{s}");
    assert!(out.status.success(), "{s}");
}
