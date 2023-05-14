use riscv::register::sstatus;

pub const CLOCK_FREQ: usize = 12500000;

pub fn init_device(hartid: usize, device_tree: usize) -> (usize, usize) {
    // 开启SUM位 让内核可以访问用户空间  踩坑：
    // only in qemu. eg: qemu is riscv 1.10  NOTE: k210 is riscv 1.9.1
    // in 1.10 is SUM but in 1.9.1 is PUM which is the opposite meaning with SUM
    unsafe {
        sstatus::set_sum();
    }
    (hartid, device_tree)
}
