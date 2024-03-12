//! Uart 16550.

use irq_safety::MutexIrqSafe;
use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

const UART_CLOCK_FACTOR: usize = 16;
const OSC_FREQ: usize = 1_843_200;

static COM1: MutexIrqSafe<Uart16550> = MutexIrqSafe::new(Uart16550::new(0x3f8));

bitflags::bitflags! {
    /// Line status flags
    struct LineStsFlags: u8 {
        const INPUT_FULL = 1;
        // 1 to 4 unknown
        const OUTPUT_EMPTY = 1 << 5;
        // 6 and 7 unknown
    }
}

struct Uart16550 {
    data: Port<u8>,
    int_en: PortWriteOnly<u8>,
    fifo_ctrl: PortWriteOnly<u8>,
    line_ctrl: PortWriteOnly<u8>,
    modem_ctrl: PortWriteOnly<u8>,
    line_sts: PortReadOnly<u8>,
}

impl Uart16550 {
    const fn new(port: u16) -> Self {
        Self {
            data: Port::new(port),
            int_en: PortWriteOnly::new(port + 1),
            fifo_ctrl: PortWriteOnly::new(port + 2),
            line_ctrl: PortWriteOnly::new(port + 3),
            modem_ctrl: PortWriteOnly::new(port + 4),
            line_sts: PortReadOnly::new(port + 5),
        }
    }

    fn init(&mut self, baud_rate: usize) {
        unsafe {
            // Disable interrupts
            self.int_en.write(0x00);

            // Enable DLAB
            self.line_ctrl.write(0x80);

            // Set maximum speed according the input baud rate by configuring DLL and DLM
            let divisor = OSC_FREQ / (baud_rate * UART_CLOCK_FACTOR);
            self.data.write((divisor & 0xff) as u8);
            self.int_en.write((divisor >> 8) as u8);

            // Disable DLAB and set data word length to 8 bits
            self.line_ctrl.write(0x03);

            // Enable FIFO, clear TX/RX queues and
            // set interrupt watermark at 14 bytes
            self.fifo_ctrl.write(0xC7);

            // Mark data terminal ready, signal request to send
            // and enable auxilliary output #2 (used as interrupt line for CPU)
            self.modem_ctrl.write(0x0B);

            // Enable interrupts
            self.int_en.write(0x00);
        }
    }

    fn line_sts(&mut self) -> LineStsFlags {
        unsafe { LineStsFlags::from_bits_truncate(self.line_sts.read()) }
    }

    fn putchar(&mut self, c: u8) {
        while !self.line_sts().contains(LineStsFlags::OUTPUT_EMPTY) {}
        unsafe { self.data.write(c) };
    }

    fn getchar(&mut self) -> Option<u8> {
        if self.line_sts().contains(LineStsFlags::INPUT_FULL) {
            unsafe { Some(self.data.read()) }
        } else {
            None
        }
    }
}

pub fn console_putchar(c: u8) {
    COM1.lock().putchar(c);
}

pub fn console_getchar() -> Option<u8> {
    COM1.lock().getchar()
}

pub fn init_early() {
    COM1.lock().init(115200);
}

pub fn init() {}
