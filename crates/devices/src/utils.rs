use polyhal::{debug_console::DebugConsole, PageTable};

use crate::MAIN_UART;

/// Translate virtual address into physical address in the current virtual address space
///
#[inline]
pub fn virt_to_phys(vaddr: usize) -> Option<usize> {
    PageTable::current()
        .translate(vaddr.into())
        .map(|x| x.0.raw())
}

pub fn puts(buffer: &[u8]) {
    // Use the main uart as much as possible.
    let main_uart_inited = MAIN_UART.is_init();
    for i in buffer {
        match main_uart_inited {
            true => MAIN_UART.put(*i),
            false => DebugConsole::putchar(*i),
        }
    }
}

/// Get a character from the uart.
///
/// If the uart device was initialized, then use it.
pub fn get_char() -> Option<u8> {
    match MAIN_UART.try_get() {
        Some(uart) => uart.get(),
        None => DebugConsole::getchar(),
    }
}
