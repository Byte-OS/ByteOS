#![feature(lazy_cell)]

use std::io::Result;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[allow(unused_macros)]
macro_rules! display {
    ($fmt:expr) => (println!("cargo:warning={}", format!($fmt)));
    ($fmt:expr, $($arg:tt)*) => (println!(concat!("cargo:warning=", $fmt), $($arg)*));
}

// write module config to file.
fn write_module_config(driver_list: Vec<String>) {
    let manifest_path =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("can't find manifest dir"));
    let mut module_file_content = String::new();
    driver_list.into_iter().for_each(|module| {
        // check driver if it exists.
        if !Path::new(&format!("../drivers/{module}/Cargo.toml")).exists() {
            panic!("can't find module {}", module);
        }
        module_file_content.push_str(&format!("extern crate {};\n", module.replace("-", "_")))
    });
    fs::write(manifest_path.join("src/drivers.rs"), module_file_content)
        .expect("can't write file to manifest dir");
}

fn main() {
    let drivers = std::env::var("CARGO_CFG_DRIVER")
        .expect("can't find any drivers")
        .split(",")
        .map(|x| x.trim().to_owned())
        .collect();

    // write module configuration to OUT_PATH, then it will be included in the main.rs
    write_module_config(drivers);
    gen_linker_script(&env::var("CARGO_CFG_BOARD").expect("can't find board"))
        .expect("can't generate linker script");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ARCH");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_KERNEL_BASE");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_BOARD");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_DRIVER");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=linker.lds.S");
}

fn gen_linker_script(platform: &str) -> Result<()> {
    let arch = env::var("CARGO_CFG_TARGET_ARCH").expect("can't find target");
    let fname = format!("linker_{}_{}.lds", arch, platform);
    let (output_arch, kernel_base) = if arch == "x86_64" {
        ("i386:x86-64", "0xffffff8000200000")
    } else if arch.contains("riscv64") {
        ("riscv", "0xffffffc080200000") // OUTPUT_ARCH of both riscv32/riscv64 is "riscv"
    } else if arch.contains("aarch64") {
        // ("aarch64", "0x40080000")
        ("aarch64", "0xffffff8040080000")
        // ("aarch64", "0xffff000040080000")
    } else if arch.contains("loongarch64") {
        ("loongarch64", "0x9000000090000000")
    } else {
        (arch.as_str(), "0")
    };
    let ld_content = std::fs::read_to_string("linker.lds.S")?;
    let ld_content = ld_content.replace("%ARCH%", output_arch);
    // let ld_content = ld_content.replace(
    //     "%KERNEL_BASE%",
    //     &env::var("CARGO_CFG_KERNEL_BASE").expect("can't find KERNEL_BASE cfg"),
    // );
    let ld_content = ld_content.replace("%KERNEL_BASE%", kernel_base);
    let ld_content = ld_content.replace("%SMP%", "4");

    std::fs::write(&fname, ld_content)?;
    println!("cargo:rustc-link-arg=-Tkernel/{}", fname);
    println!("cargo:rerun-if-env-changed=CARGO_CFG_KERNEL_BASE");
    Ok(())
}
