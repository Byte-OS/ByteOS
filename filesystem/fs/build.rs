fn main() {
    println!(r#"cargo::rustc-check-cfg=cfg(root_fs, values("fat32", "ext4", "ext4_rs"))"#);
    println!("cargo:rerun-if-env-changed=CARGO_CFG_ROOT_FS");
}
