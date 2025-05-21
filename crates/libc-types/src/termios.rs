//! This module provides the `libc` types for Termios.
//!
//! MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/generic/bits/termios.h>

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// `termios` 结构体用于控制异步通信端口（如串口、终端）的通用终端接口。
/// 它由多个标志位和特殊字符数组组成，用于控制终端的输入、输出、控制和本地模式。
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/generic/bits/termios.h#L1>
pub struct Termios {
    /// 输入模式标志（Input modes），如是否启用回显、处理换行等
    pub iflag: u32,
    /// 输出模式标志（Output modes），如是否自动添加换行符等
    pub oflag: u32,
    /// 控制模式标志（Control modes），如波特率、字符长度、停止位、硬件流控制等
    pub cflag: u32,
    /// 本地模式标志（Local modes），如是否启用 canonical 模式、信号生成等
    pub lflag: u32,
    /// 行控制符，一般用于选择 `cc` 中的哪一个控制字符表示行结束符
    pub line: u8,
    /// 终端特殊字符数组（Control characters），如中断键、结束符、擦除符等，大小通常为 NCCS（一般为 32）
    pub cc: [u8; 32],
    /// 输入速度（Input speed），表示波特率
    pub ispeed: u32,
    /// 输出速度（Output speed），表示波特率
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
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            ispeed: 0,
            ospeed: 0,
        }
    }
}
