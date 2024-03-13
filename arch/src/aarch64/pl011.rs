//! PL011 UART.

use arm_pl011::pl011::Pl011Uart;
use irq_safety::MutexIrqSafe;

use crate::PhysAddr;

const UART_BASE: PhysAddr = PhysAddr(0x0900_0000);

static UART: MutexIrqSafe<Pl011Uart> = MutexIrqSafe::new(Pl011Uart::new(UART_BASE.get_mut_ptr()));

/// Writes a byte to the console.
pub fn console_putchar(c: u8) {
    let mut uart = UART.lock();
    match c {
        b'\n' => {
            uart.putchar(b'\r');
            uart.putchar(b'\n');
        }
        c => uart.putchar(c),
    }
}

/// Reads a byte from the console, or returns [`None`] if no input is available.
pub fn console_getchar() -> Option<u8> {
    UART.lock().getchar()
}

/// Initialize the UART
pub fn init_early() {
    UART.lock().init();
}
