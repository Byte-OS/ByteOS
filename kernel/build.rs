#![feature(lazy_cell)]

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
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut module_file_content = String::new();
    driver_list.into_iter().for_each(|module| {
        // check driver if it exists.
        if !Path::new(&format!("../drivers/{module}/Cargo.toml")).exists() {
            panic!("can't find module {}", module);
        }
        // use cargo command to add driver to kernel.
        // Command::new("cargo")
        //     .args(["add", "--path", &format!("../drivers/{module}")])
        //     .output()
        //     .expect("failed to execute cargo add");
        
        module_file_content.push_str(&format!("extern crate {};\n", module.replace("-", "_")))
    });
    fs::write(out_path.join("drivers.rs"), module_file_content)
        .expect("can't write file to OUT_DIR");
}

fn main() {
    let drivers = std::env::var("CARGO_CFG_DRIVER")
        .expect("can't find any drivers")
        .split(",")
        .map(|x| x.trim().to_owned())
        .collect();

    // write module configuration to OUT_PATH, then it will be included in the main.rs
    write_module_config(drivers);
    println!("cargo:rerun-if-env-changed=CARGO_CFG_DRIVER");
    println!("cargo:rerun-if-changed=build.rs");
}
