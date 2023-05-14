cfg_if::cfg_if! {
    if #[cfg(feature = "board-k210")] {
        mod k210;
        pub use k210::*;
    } else if #[cfg(feature = "board-qemu")] {
        mod qemu;
        pub use qemu::*;
    } else {
        pub const CLOCK_FREQ: usize = 12500000;

        pub fn init_device(hartid: usize, device_tree: usize) -> (usize, usize) {
            warn!("use default board config");
            (hartid, device_tree)
        }
    }
}
