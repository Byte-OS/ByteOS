use std::{env, fs, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("can't find manifest dir"));
    let heap_size = env::var("HEAP_SIZE").unwrap_or("0x0180_0000".into());
    fs::write(
        out_dir.join("consts.rs"),
        format!("pub const HEAP_SIZE: usize = {heap_size};"),
    )
    .expect("can't write data to temp file in the out_dir");

    println!("cargo:rerun-if-env-changed=HEAP_SIZE");
    println!("cargo:rerun-if-changed=build.rs");
}
