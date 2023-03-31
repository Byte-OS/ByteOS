mod entry;
mod sbi;

pub use sbi::*;

#[no_mangle]
extern "C" fn rust_main(hartid: usize) {
    extern "Rust" {
        fn main(hartid: usize);
    }

    unsafe {
        main(hartid);
    }

    shutdown();
}
