#![feature(once_cell)]

use std::{
    env, fs,
    sync::LazyLock,
};

use serde_derive::Deserialize;

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

fn main() {
    // let ac = AutoCfg::new();
    let config = read_config_file();

    display!("BOARD: {}", BOARD.as_str());
    display!("config: {config:?}");

    println!("cargo:rustc-cfg=board=\"{}\"", BOARD.as_str());

    config.cfgs.unwrap_or_default().iter().for_each(|x| {
        println!("cargo:rustc-cfg={}", x);
    });

    println!("cargo:rerun-if-env-changed=BOARD");
    println!("cargo:rerun-if-changed=../config/{}.toml", BOARD.as_str());
    println!("cargo:rerun-if-changed=build.rs");
    
}
