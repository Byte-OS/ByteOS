pub const CLOCK_FREQ: usize = 403000000 / 62;

pub fn init_device(hartid: usize, device_tree: usize) -> (usize, usize) {
    (hartid, device_tree)
}
