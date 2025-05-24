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
    pub iflag: InputFlags,
    /// 输出模式标志（Output modes），如是否自动添加换行符等
    pub oflag: OutputFlags,
    /// 控制模式标志（Control modes），如波特率、字符长度、停止位、硬件流控制等
    pub cflag: u32,
    /// 本地模式标志（Local modes），如是否启用 canonical 模式、信号生成等
    pub lflag: LocalFlags,
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
            iflag: InputFlags::IMAXBEL
                | InputFlags::IUTF8
                | InputFlags::IXON
                | InputFlags::IXANY
                | InputFlags::ICRNL
                | InputFlags::BRKINT,
            // OPOST | ONLCR
            oflag: OutputFlags::OPOST | OutputFlags::ONLCR,
            // HUPCL | CREAD | CSIZE | EXTB
            // cflag: 0o2277,
            cflag: ControlFlags::CREAD.bits() | ControlFlags::HUPCL.bits() | 0x77,
            // IEXTEN | ECHOTCL | ECHOKE ECHO | ECHOE | ECHOK | ISIG | ICANON
            lflag: LocalFlags::ISIG
                | LocalFlags::ICANON
                | LocalFlags::ECHO
                | LocalFlags::ECHOE
                | LocalFlags::ECHOK
                | LocalFlags::ECHONL
                | LocalFlags::IEXTEN,
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

/// 控制字符索引（Control Characters Index）
///
/// 用于 termios 结构中 c_cc 数组，表示各种控制字符在数组中的位置。
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/generic/bits/termios.h>
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlChar {
    /// 中断字符，通常是 Ctrl-C，用于发送中断信号
    VINTR = 0,
    /// 退出字符，通常是 Ctrl-\
    VQUIT = 1,
    /// 删除字符，通常是退格键（Backspace）
    VERASE = 2,
    /// 删除整行字符
    VKILL = 3,
    /// 文件结束字符，通常是 Ctrl-D
    VEOF = 4,
    /// 读取时的超时值（定时器）
    VTIME = 5,
    /// 读取时的最小字节数
    VMIN = 6,
    /// 切换字符（不常用）
    VSWTC = 7,
    /// 开始字符，通常是 Ctrl-Q，用于软件流控制
    VSTART = 8,
    /// 停止字符，通常是 Ctrl-S，用于软件流控制
    VSTOP = 9,
    /// 挂起字符，通常是 Ctrl-Z
    VSUSP = 10,
    /// 额外的行结束字符（EOL）
    VEOL = 11,
    /// 重新打印字符，用于重新显示当前输入行
    VREPRINT = 12,
    /// 丢弃输出字符
    VDISCARD = 13,
    /// 删除一个单词字符
    VWERASE = 14,
    /// 下一字符字面量输入（转义下一个字符）
    VLNEXT = 15,
    /// 第二个额外的行结束字符（EOL2）
    VEOL2 = 16,
}

bitflags! {
    /// 输入模式标志（Input Modes Flags），对应 termios 结构体中的 c_iflag 字段。
    ///
    /// 这些标志控制终端输入的行为，如是否忽略断开信号、是否进行流控等。
    /// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/generic/bits/termios.h#L30>
    #[derive(Debug, Clone, Copy)]
    pub struct InputFlags: u32 {
        /// 忽略 BREAK 条件（输入中的断开信号）
        const IGNBRK  = 0o000001;
        /// 在输入 BREAK 时产生中断信号
        const BRKINT  = 0o000002;
        /// 忽略奇偶校验错误的字符
        const IGNPAR  = 0o000004;
        /// 标记奇偶校验错误的字符
        const PARMRK  = 0o000010;
        /// 启用输入奇偶校验检查
        const INPCK   = 0o000020;
        /// 去除输入字符的第 8 位
        const ISTRIP  = 0o000040;
        /// 将输入的换行符 NL 转换为回车符 CR
        const INLCR   = 0o000100;
        /// 忽略输入的回车符 CR
        const IGNCR   = 0o000200;
        /// 将输入的回车符 CR 转换为换行符 NL
        const ICRNL   = 0o000400;
        /// 将大写字母转换为小写字母（已废弃，通常不使用）
        const IUCLC   = 0o001000;
        /// 启用 XON/XOFF 输出流控制
        const IXON    = 0o002000;
        /// 允许任何字符中断输出暂停（XON）
        const IXANY   = 0o004000;
        /// 启用 XON/XOFF 输入流控制
        const IXOFF   = 0o010000;
        /// 当输入缓冲区满时发出响铃
        const IMAXBEL = 0o020000;
        /// UTF-8 输入编码（Linux 特有）
        const IUTF8   = 0o040000;
    }


    /// 输出模式标志（Output Modes Flags），对应 termios 结构体中的 c_oflag 字段。
    ///
    /// 这些标志用于控制终端输出的处理行为，如是否进行后处理、换行符转换等。
    /// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/generic/bits/termios.h#L46>
    #[derive(Debug, Clone, Copy)]
    pub struct OutputFlags: u32 {
        /// 启用输出处理（如换行符转换）
        const OPOST  = 0o000001;
        /// 将输出中的小写字母转换为大写字母（多数系统不使用）
        const OLCUC  = 0o000002;
        /// 将输出中的换行符 NL 转换为回车符 CR 加换行符 NL
        const ONLCR  = 0o000004;
        /// 将输出中的回车符 CR 转换为换行符 NL
        const OCRNL  = 0o000010;
        /// 不输出回车符 CR
        const ONOCR  = 0o000020;
        /// 输出换行符时不回车（不常用）
        const ONLRET = 0o000040;
        /// 用填充字符填充输出（如延迟输出）
        const OFILL  = 0o000100;
        /// 填充字符使用 DEL（0x7f）
        const OFDEL  = 0o000200;

        // 以下标志通常带条件编译，根据平台支持情况可能不同

        /// 新行延迟标志掩码
        const NLDLY  = 0o000400;
        /// 新行延迟设置为0（无延迟）
        const NL0    = 0o000000;
        /// 新行延迟设置为1（延迟）
        const NL1    = 0o000400;

        /// 回车延迟标志掩码
        const CRDLY  = 0o003000;
        /// 回车延迟设置0（无延迟）
        const CR0    = 0o000000;
        /// 回车延迟设置1
        const CR1    = 0o001000;
        /// 回车延迟设置2
        const CR2    = 0o002000;
        /// 回车延迟设置3
        const CR3    = 0o003000;

        /// 制表符延迟标志掩码
        const TABDLY = 0o014000;
        /// 制表符延迟设置0（无延迟）
        const TAB0   = 0o000000;
        /// 制表符延迟设置1
        const TAB1   = 0o004000;
        /// 制表符延迟设置2
        const TAB2   = 0o010000;
        /// 制表符延迟设置3
        const TAB3   = 0o014000;

        /// 退格延迟标志掩码
        const BSDLY  = 0o020000;
        /// 退格延迟设置0（无延迟）
        const BS0    = 0o000000;
        /// 退格延迟设置1
        const BS1    = 0o020000;

        /// 换页延迟标志掩码
        const FFDLY  = 0o100000;
        /// 换页延迟设置0（无延迟）
        const FF0    = 0o000000;
        /// 换页延迟设置1
        const FF1    = 0o100000;

        /// 垂直制表延迟标志掩码
        const VTDLY  = 0o040000;
        /// 垂直制表延迟设置0（无延迟）
        const VT0    = 0o000000;
        /// 垂直制表延迟设置1
        const VT1    = 0o040000;
    }
    /// 控制模式标志（Control Modes Flags），对应 termios 结构体中的 c_cflag 字段。
    ///
    /// 这些标志用于控制终端的硬件相关设置，如字符大小、停止位、校验以及本地连接等。
    /// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/generic/bits/termios.h#L113>
    #[derive(Debug, Clone, Copy)]
    pub struct ControlFlags: u32 {
        /// 字符大小掩码（用于选择 CS5、CS6、CS7 或 CS8）
        const CSIZE  = 0o000060;
        /// 字符大小：5 位
        const CS5    = 0o000000;
        /// 字符大小：6 位
        const CS6    = 0o000020;
        /// 字符大小：7 位
        const CS7    = 0o000040;
        /// 字符大小：8 位
        const CS8    = 0o000060;
        /// 发送两位停止位，默认是一位停止位
        const CSTOPB = 0o000100;
        /// 启用接收器
        const CREAD  = 0o000200;
        /// 启用奇偶校验位
        const PARENB = 0o000400;
        /// 奇偶校验为奇校验，默认是偶校验
        const PARODD = 0o001000;
        /// 关闭调制解调器挂断控制（保持连接）
        const HUPCL  = 0o002000;
        /// 忽略调制解调器状态线，允许本地连接
        const CLOCAL = 0o004000;
    }
    /// 本地模式标志（Local Modes Flags），对应 termios 结构体中的 c_lflag 字段。
    ///
    /// 这些标志控制终端的本地行为，如信号产生、行编辑、回显等。
    /// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/generic/bits/termios.h#L125>
    #[derive(Debug, Clone, Copy)]
    pub struct LocalFlags: u32 {
        /// 允许产生信号，如 INTR, QUIT, SUSP
        const ISIG   = 0o000001;
        /// 启用标准模式（规范模式）输入，即行缓冲和行编辑
        const ICANON = 0o000002;
        /// 启用输入字符回显
        const ECHO   = 0o000010;
        /// 启用 ERASE 字符的回显效果（擦除字符时，光标左移）
        const ECHOE  = 0o000020;
        /// 输入 KILL 字符时回显一个新行
        const ECHOK  = 0o000040;
        /// 在新行输入时回显换行符
        const ECHONL = 0o000100;
        /// 禁止输入或输出时刷新终端队列
        const NOFLSH = 0o000200;
        /// 背景进程尝试写终端时发送 SIGTTOU 信号
        const TOSTOP = 0o000400;
        /// 启用扩展功能处理，如实现特殊控制字符（如 VDISCARD）
        const IEXTEN = 0o100000;
    }

}

/// 波特率常量（Baud Rate Constants），用于设置串口通信的波特率。
#[allow(missing_docs)]
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaudRate {
    B0 = 0o000000,
    B50 = 0o000001,
    B75 = 0o000002,
    B110 = 0o000003,
    B134 = 0o000004,
    B150 = 0o000005,
    B200 = 0o000006,
    B300 = 0o000007,
    B600 = 0o000010,
    B1200 = 0o000011,
    B1800 = 0o000012,
    B2400 = 0o000013,
    B4800 = 0o000014,
    B9600 = 0o000015,
    B19200 = 0o000016,
    B38400 = 0o000017,
    B57600 = 0o100001,
    B115200 = 0o100002,
    B230400 = 0o100003,
    B460800 = 0o100004,
    B500000 = 0o100005,
    B576000 = 0o100006,
    B921600 = 0o100007,
    B1000000 = 0o100010,
    B1152000 = 0o100011,
    B1500000 = 0o100012,
    B2000000 = 0o100013,
    B2500000 = 0o100014,
    B3000000 = 0o100015,
    B3500000 = 0o100016,
    B4000000 = 0o100017,
}
