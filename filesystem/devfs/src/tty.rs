use core::cmp;

use alloc::collections::VecDeque;
use bitflags::bitflags;
use devices::utils::{get_char, puts};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use sync::Mutex;
use vfscore::{INodeInterface, PollEvent, Stat, StatMode, VfsError, VfsResult};
pub struct Tty {
    buffer: Mutex<VecDeque<u8>>,
    termios: Mutex<Termios>,
    pgid: Mutex<u32>,
    winsize: Mutex<WinSize>,
}

impl Tty {
    pub fn new() -> Tty {
        Tty {
            buffer: Mutex::new(VecDeque::new()),
            termios: Default::default(),
            pgid: Default::default(),
            winsize: Default::default(),
        }
    }
}

impl INodeInterface for Tty {
    fn readat(&self, _offset: usize, buffer: &mut [u8]) -> vfscore::VfsResult<usize> {
        assert!(buffer.len() > 0);
        let mut self_buffer = self.buffer.lock();
        if self_buffer.len() > 0 {
            let rlen = cmp::min(buffer.len(), self_buffer.len());
            for i in 0..rlen {
                buffer[i] = self_buffer.pop_front().unwrap();
            }
            Ok(rlen)
        } else {
            // VfsError::Blocking
            if let Some(c) = get_char() {
                buffer[0] = c as u8;
                Ok(1)
            } else {
                Err(VfsError::Blocking)
            }
        }
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        stat.dev = 1;
        stat.ino = 1; // TODO: convert path to number(ino)
        stat.mode = StatMode::CHAR; // TODO: add access mode
        stat.nlink = 1;
        stat.uid = 1000;
        stat.gid = 1000;
        stat.size = 15;
        stat.blksize = 512;
        stat.blocks = 0;
        stat.rdev = 0; // TODO: add device id
        Ok(())
    }

    fn writeat(&self, _offset: usize, buffer: &[u8]) -> vfscore::VfsResult<usize> {
        puts(buffer);
        Ok(buffer.len())
    }

    fn poll(&self, events: PollEvent) -> VfsResult<PollEvent> {
        let mut res = PollEvent::NONE;
        if events.contains(PollEvent::POLLIN) {
            let buf_len = self.buffer.lock().len();
            if buf_len > 0 {
                res |= PollEvent::POLLIN;
            } else {
                if let Some(c) = get_char() {
                    res |= PollEvent::POLLIN;
                    self.buffer.lock().push_back(c);
                }
            }
        }
        if events.contains(PollEvent::POLLOUT) {
            res |= PollEvent::POLLOUT;
        }
        Ok(res)
    }

    fn ioctl(&self, command: usize, arg: usize) -> VfsResult<usize> {
        let cmd = FromPrimitive::from_usize(command).ok_or(VfsError::InvalidInput)?;
        match cmd {
            TeletypeCommand::TCGETS | TeletypeCommand::TCGETA => {
                unsafe {
                    (arg as *mut Termios).write_volatile(*self.termios.lock());
                }
                Ok(0)
            }
            TeletypeCommand::TCSETS | TeletypeCommand::TCSETSW | TeletypeCommand::TCSETSF => {
                // copy_from_user(token, argp as *const Termios, &mut inner.termios);
                unsafe { *self.termios.lock() = *(arg as *mut Termios).as_mut().unwrap() }
                Ok(0)
            }
            TeletypeCommand::TIOCGPGRP => match unsafe { (arg as *mut u32).as_mut() } {
                Some(pgid) => {
                    *pgid = *self.pgid.lock();
                    Ok(0)
                }
                None => Err(VfsError::InvalidInput),
            },
            TeletypeCommand::TIOCSPGRP => match unsafe { (arg as *mut u32).as_mut() } {
                Some(pgid) => {
                    *self.pgid.lock() = *pgid;
                    Ok(0)
                }
                None => Err(VfsError::InvalidInput),
            },
            TeletypeCommand::TIOCGWINSZ => {
                unsafe {
                    *(arg as *mut WinSize).as_mut().unwrap() = *self.winsize.lock();
                }
                Ok(0)
            }
            TeletypeCommand::TIOCSWINSZ => {
                unsafe {
                    *self.winsize.lock() = *(arg as *mut WinSize).as_mut().unwrap();
                }
                Ok(0)
            }
            _ => Err(vfscore::VfsError::NotSupported),
        }
        // Err(VfsError::NotSupported)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// The termios functions describe a general terminal interface that
/// is provided to control asynchronous communications ports.
pub struct Termios {
    /// input modes
    pub iflag: u32,
    /// ouput modes
    pub oflag: u32,
    /// control modes
    pub cflag: u32,
    /// local modes
    pub lflag: u32,
    pub line: u8,
    /// terminal special characters.
    pub cc: [u8; 21],
    pub ispeed: u32,
    pub ospeed: u32,
}

impl Default for Termios {
    fn default() -> Self {
        Termios {
            // IMAXBEL | IUTF8 | IXON | IXANY | ICRNL | BRKINT
            iflag: 0o66402,
            // OPOST | ONLCR
            oflag: 0o5,
            // HUPCL | CREAD | CSIZE | EXTB
            cflag: 0o2277,
            // IEXTEN | ECHOTCL | ECHOKE ECHO | ECHOE | ECHOK | ISIG | ICANON
            lflag: 0o105073,
            line: 0,
            cc: [
                3,   // VINTR Ctrl-C
                28,  // VQUIT
                127, // VERASE
                21,  // VKILL
                4,   // VEOF Ctrl-D
                0,   // VTIME
                1,   // VMIN
                0,   // VSWTC
                17,  // VSTART
                19,  // VSTOP
                26,  // VSUSP Ctrl-Z
                255, // VEOL
                18,  // VREPAINT
                15,  // VDISCARD
                23,  // VWERASE
                22,  // VLNEXT
                255, // VEOL2
                0, 0, 0, 0,
            ],
            ispeed: 0,
            ospeed: 0,
        }
    }
}

bitflags! {
    pub struct LocalModes : u32 {
        const ISIG = 0o000001;
        const ICANON = 0o000002;
        const ECHO = 0o000010;
        const ECHOE = 0o000020;
        const ECHOK = 0o000040;
        const ECHONL = 0o000100;
        const NOFLSH = 0o000200;
        const TOSTOP = 0o000400;
        const IEXTEN = 0o100000;
        const XCASE = 0o000004;
        const ECHOCTL = 0o001000;
        const ECHOPRT = 0o002000;
        const ECHOKE = 0o004000;
        const FLUSHO = 0o010000;
        const PENDIN = 0o040000;
        const EXTPROC = 0o200000;
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Eq, PartialEq, FromPrimitive)]
#[repr(u32)]
pub enum TeletypeCommand {
    // For struct termios
    /// Gets the current serial port settings.
    TCGETS = 0x5401,
    /// Sets the serial port settings immediately.
    TCSETS = 0x5402,
    /// Sets the serial port settings after allowing the input and output buffers to drain/empty.
    TCSETSW = 0x5403,
    /// Sets the serial port settings after flushing the input and output buffers.
    TCSETSF = 0x5404,

    /// For struct termio
    /// Gets the current serial port settings.
    TCGETA = 0x5405,
    /// Sets the serial port settings immediately.
    TCSETA = 0x5406,
    /// Sets the serial port settings after allowing the input and output buffers to drain/empty.
    TCSETAW = 0x5407,
    /// Sets the serial port settings after flushing the input and output buffers.
    TCSETAF = 0x5408,

    /// Get the process group ID of the foreground process group on this terminal.
    TIOCGPGRP = 0x540F,
    /// Set the foreground process group ID of this terminal.
    TIOCSPGRP = 0x5410,

    /// Get window size.
    TIOCGWINSZ = 0x5413,
    /// Set window size.
    TIOCSWINSZ = 0x5414,

    /// Non-cloexec
    FIONCLEX = 0x5450,
    /// Cloexec
    FIOCLEX = 0x5451,

    /// rustc using pipe and ioctl pipe file with this request id
    /// for non-blocking/blocking IO control setting
    FIONBIO = 0x5421,

    /// Read time
    RTC_RD_TIME = 0x80247009,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WinSize {
    ws_row: u16,
    ws_col: u16,
    xpixel: u16,
    ypixel: u16,
}

impl Default for WinSize {
    fn default() -> Self {
        Self {
            ws_row: 24,
            ws_col: 140,
            xpixel: 0,
            ypixel: 0,
        }
    }
}
