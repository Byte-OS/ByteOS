#![feature(lazy_cell)]

#[allow(unused_macros)]
macro_rules! display {
    ($fmt:expr) => (println!("cargo:warning={}", format!($fmt)));
    ($fmt:expr, $($arg:tt)*) => (println!(concat!("cargo:warning=", $fmt), $($arg)*));
}

fn main() {
    // write module configuration to OUT_PATH, then it will be included in the main.rs
    println!("cargo:rerun-if-changed=mount.img");
    println!("cargo:rerun-if-changed=build.rs");
}
