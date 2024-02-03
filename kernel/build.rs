#![feature(lazy_cell)]

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::LazyLock,
};

use serde_derive::Deserialize;

pub const OUT_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env::var("OUT_DIR").unwrap()));
pub const BOARD: LazyLock<String> =
    LazyLock::new(|| env::var("BOARD").unwrap_or("qemu".to_string()));

macro_rules! display {
    ($fmt:expr) => (println!("cargo:warning={}", format!($fmt)));
    ($fmt:expr, $($arg:tt)*) => (println!(concat!("cargo:warning=", $fmt), $($arg)*));
}

#[derive(Deserialize, Debug)]
struct Config {
    drivers: Vec<String>,
    cfgs: Option<Vec<String>>,
    ld_file: Option<String>,
}

fn read_config_file() -> Config {
    if let Ok(content) = fs::read_to_string(&format!("../config/{}.toml", BOARD.clone())) {
        if let Ok(config) = toml::from_str(&content) {
            return config;
        }
    }
    panic!("can't find the config file");
}

// write module config to file.
fn write_module_config(driver_list: Vec<String>) {
    let mut module_file_content = String::new();
    module_file_content.push_str("use kheader::macros::module_use;\n");
    driver_list.into_iter().for_each(|module| {
        // check driver if it exists.
        if !Path::new(&format!("../drivers/{module}/Cargo.toml")).exists() {
            panic!("can't find module {}", module);
        }
        // use cargo command to add driver to kernel.
        Command::new("cargo")
            .args(["add", "--path", &format!("../drivers/{module}")])
            .output()
            .expect("failed to execute cargo add");
        // write data file
        module_file_content.push_str("module_use!(");
        module_file_content.push_str(&module.replace("-", "_"));
        module_file_content.push_str(");\n");
    });
    fs::write(OUT_PATH.join("modules.rs"), module_file_content)
        .expect("can't write file to OUT_DIR");
}

fn main() {
    // let ac = AutoCfg::new();
    let config = read_config_file();

    display!("BOARD: {}", BOARD.as_str());
    display!("config: {config:?}");

    println!("cargo:rustc-cfg=board=\"{}\"", BOARD.as_str());

    // write module configuration to OUT_PATH, then it will be included in the main.rs
    write_module_config(config.drivers);

    config.cfgs.unwrap_or_default().iter().for_each(|x| {
        println!("cargo:rustc-cfg={}", x);
    });

    println!(
        "cargo:rustc-link-arg=-Tconfig/{}",
        config.ld_file.unwrap_or(String::from("linker-general.ld"))
    );
    println!("cargo:rerun-if-env-changed=BOARD");
    println!("cargo:rerun-if-changed=../config/{}.toml", BOARD.as_str());
    println!("cargo:rerun-if-changed=build.rs");
    
}
