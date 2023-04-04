use arch::IntTable;
use log::info;

fn timer() {
    info!("timer interrupt");
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
