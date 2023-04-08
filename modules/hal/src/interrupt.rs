use arch::IntTable;
use log::trace;

fn timer() {
    trace!("timer interrupt");
}

pub fn init() {
    // initialize interrupt
    arch::init_interrupt();
}

const INT_TABLE: IntTable = IntTable { timer };

#[inline(always)]
#[no_mangle]
const fn interrupt_table() -> IntTable {
    INT_TABLE
}
