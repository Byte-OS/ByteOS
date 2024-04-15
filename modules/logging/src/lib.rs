#![no_std]

extern crate alloc;

use arch::{console_getchar, console_putchar};
use core::fmt::{self, Write};
use devices::MAIN_UART;
use log::{self, info, Level, LevelFilter, Log, Metadata, Record};

pub struct Logger;

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let file = record.file();
        let line = record.line();

        let color_code = match record.level() {
            Level::Error => 31u8, // Red
            Level::Warn => 93,    // BrightYellow
            Level::Info => 34,    // Blue
            Level::Debug => 32,   // Green
            Level::Trace => 90,   // BrightBlack
        };
        write!(
            Logger,
            "\u{1B}[{}m\
            [{}] {}:{} {}\
            \u{1B}[0m\n",
            color_code,
            record.level(),
            file.unwrap(),
            line.unwrap(),
            record.args()
        )
        .expect("can't write color string in logging module.");
    }

    fn flush(&self) {}
}

impl Write for Logger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut buffer = [0u8; 4];
        for c in s.chars() {
            puts(c.encode_utf8(&mut buffer).as_bytes())
        }
        Ok(())
    }
}

pub fn init(level: Option<&str>) {
    log::set_logger(&Logger).unwrap();
    log::set_max_level(match level {
        Some("error") => LevelFilter::Error,
        Some("warn") => LevelFilter::Warn,
        Some("info") => LevelFilter::Info,
        Some("debug") => LevelFilter::Debug,
        Some("trace") => LevelFilter::Trace,
        _ => LevelFilter::Off,
    });
    info!("logging module initialized");
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::print(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}

#[inline]
pub fn print(args: fmt::Arguments) {
    Logger
        .write_fmt(args)
        .expect("can't write string in logging module.");
}

// #[inline]
// pub fn puts(buffer: &[u8]) {
//     console_putchar(b'2');
//     static LOG_BUFFER: MutexIrqSafe<Vec<Vec<u8>>> = MutexIrqSafe::new(Vec::new());
//     console_putchar(b'4');
//     let mut log_buffer = LOG_BUFFER.lock();
//     loop {
//         if hart_id() < log_buffer.len() {
//             break;
//         }
//         log_buffer.push(Vec::new());
//     }
//     let current_buffer = &mut log_buffer[hart_id()];
//     let r_pos = buffer.into_iter().rposition(|&x| x == b'\n');
//     if let Some(r_pos) = r_pos {
//         real_puts(current_buffer);
//         current_buffer.clear();
//         real_puts(&buffer[..r_pos + 1]);
//         current_buffer.extend_from_slice(&buffer[r_pos + 1..]);
//     } else {
//         current_buffer.extend_from_slice(buffer);
//     }
//     console_putchar(b'1');
//     drop(log_buffer);
//     console_putchar(b'5');
// }

pub fn puts(buffer: &[u8]) {
    // Use the main uart as much as possible.
    let main_uart_inited = MAIN_UART.is_init();
    for i in buffer {
        match main_uart_inited {
            true => MAIN_UART.put(*i),
            false => console_putchar(*i),
        }
    }
}

/// Get a character from the uart.
///
/// If the uart device was initialized, then use it.
pub fn get_char() -> Option<u8> {
    match MAIN_UART.try_get() {
        Some(uart) => uart.get(),
        None => console_getchar(),
    }
}
