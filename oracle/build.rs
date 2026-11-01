use std::process::Command;

fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out = std::env::var("OUT_DIR").unwrap();
    let cargo = std::env::var("CARGO").unwrap();
    let port_src = format!("{dir}/../src");
    for f in ["types.rs", "math.rs", "error.rs"] {
        println!("cargo:rerun-if-changed={port_src}/{f}");
    }
    println!("cargo:rerun-if-changed=gen.py");
    println!("cargo:rerun-if-env-changed=ORACLE_UFBX_SRC");
    println!("cargo:rerun-if-env-changed=CFLAGS");
    // .cargo/config.toml applies only to cargo started inside this directory
    let cflags = std::env::var("CFLAGS").unwrap_or_default();
    assert!(cflags.contains("-ffp-contract=off"), "CFLAGS lacks -ffp-contract=off: run cargo from oracle/, whose .cargo/config.toml sets it");
    let status = Command::new("python3")
        .args(["-I", &format!("{dir}/gen.py"), "--out", &format!("{out}/generated_cmp.rs"), "--port-src", &port_src])
        .args(["--cargo", &cargo, "--manifest", &format!("{dir}/Cargo.toml")])
        .status()
        .expect("python3 runs gen.py");
    assert!(status.success(), "gen.py failed");
    println!("cargo:rerun-if-changed=gen_compat.py");
    let status = Command::new("python3")
        .args(["-I", &format!("{dir}/gen_compat.py"), "--cmp-out", &format!("{out}/generated_compat_cmp.rs"), "--src", &port_src])
        .args(["--cargo", &cargo, "--manifest", &format!("{dir}/Cargo.toml")])
        .status()
        .expect("python3 runs gen_compat.py");
    assert!(status.success(), "gen_compat.py failed");
}
