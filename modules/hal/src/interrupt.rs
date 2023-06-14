use arch::{Context, TrapType};
use sync::LazyInit;


pub fn init() {
    // initialize interrupt
    arch::init_interrupt();
}

static INT_TABLE: LazyInit<fn(&mut Context, TrapType)> = LazyInit::new();

#[inline(always)]
#[no_mangle]
fn interrupt_table() -> Option<fn(&mut Context, TrapType)> {
    INT_TABLE.try_get().copied()
}

#[inline]
pub fn reg_kernel_int(f: fn(&mut Context, TrapType)) {
    INT_TABLE.init_by(f);
}
