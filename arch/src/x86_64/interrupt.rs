use core::arch::asm;


#[allow(dead_code)]
#[inline(always)]
pub fn enable_irq() {
    unsafe {
        asm!("sti")
    }
}

pub fn close_irq() {
    unsafe {
        asm!("cli")
    }
}

#[inline(always)]
pub fn enable_external_irq() {
    // unsafe {
        
    // }
}

pub fn init_interrupt() {
    enable_irq()
}

pub fn time_to_usec(tiscks: usize) -> usize {
    tiscks
}

pub fn get_time() -> usize {
    unsafe {
        core::arch::x86_64::_rdtsc() as _
    }
}