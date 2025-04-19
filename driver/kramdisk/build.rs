use std::{env, fs, path::PathBuf};

#[allow(unused_macros)]
macro_rules! display {
    ($fmt:expr) => (println!("cargo:warning={}", format!($fmt)));
    ($fmt:expr, $($arg:tt)*) => (println!(concat!("cargo:warning=", $fmt), $($arg)*));
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("can't find manifest dir"));
    let img_relative_path = env::var("MOUNT_IMG_PATH").unwrap_or("mount.img".into());
    let project_dir =
        PathBuf::from(env::var("ROOT_MANIFEST_DIR").expect("can't find manifest directory"));

    let img_path = project_dir.join(img_relative_path);
    let img_path = img_path.to_str().expect("can't build a valid img path");
    fs::write(
        out_dir.join("inc.S"),
        format!(
            ".section .data
    .global ramdisk_start
    .global ramdisk_end
    .p2align 12
    ramdisk_start:
    .incbin \"{img_path}\"
    ramdisk_end:"
        ),
    )
    .expect("can't write ram file to out_dir");

    // fs::write(path, contents)

    // write module configuration to OUT_PATH, then it will be included in the main.rs
    println!("cargo:rerun-if-env-changed=MOUNT_IMG_PATH");
    println!("cargo:rerun-if-changed=mount.img");
    println!("cargo:rerun-if-changed=build.rs");
}
