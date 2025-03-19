fn main() {
    println!(r#"cargo::rustc-check-cfg=cfg(root_fs, values("fat32", "ext4", "ext4_rs"))"#);
}
