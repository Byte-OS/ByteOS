# 华山派开发板SD卡驱动设计

为在华山派CV1811h上运行**ByteOS**并顺利运行测试，我们编写了一应用于华山派开发板的SD卡驱动，以下是详细设计

```
├── cv1811-sd	//支持华山派cv1811h的SD卡驱动
│   ├── Cargo.toml
│   └── src
│       ├── consts.rs	//相关常量的定义
│       ├── lib.rs	//驱动主体
│       └── utils.rs	//寄存器读写操作定义
```

在`consts.rs`中我们定义了一些编写驱动所需的常量和寄存器

例如：

```rust
pub const TOP_BASE: usize = 0xffff_ffc0_030_00000;
pub const SD_DRIVER_ADDR: usize = 0xffff_ffc0_0431_0000;
pub const SOFT_REST_BASE_ADDR: usize = 0xffff_ffc0_0300_3000;
pub const PINMUX_BASE: usize = 0xffff_ffc0_0300_1000;
```

- `TOP_BASE`：顶层基地址，用于与其他组件进行通信。
- `SD_DRIVER_ADDR`：SD卡驱动的基地址。
- `SOFT_REST_BASE_ADDR`：软件复位基地址，用于执行软件复位操作。
- `PINMUX_BASE`：引脚复用基地址，用于配置引脚的功能。

```rust
pub struct EmmcCtrl(u32) {
        emmc_func_en: u1,
        latancy_1t: u1,
        clk_free_en: u1,
        disable_data_crc_check: u1,
        reserved: u1,
        emmc_rstn: u1,
        emmc_rstn_oen: u1,
        reserved1: u2,
        cqe_algo_sel: u1,
        cqe_prefetch_disable: u1,
        reserved2: u2,
        timer_clk_sel: u1,
        reserved3: u18
    }
```

`EmmcCtrl`：这个结构体表示eMMC控制器的控制寄存器。它使用了一个32位的整数来存储以下字段：

- `emmc_func_en`、`latancy_1t`、`clk_free_en`、`disable_data_crc_check`、`reserved`、`emmc_rstn`、`emmc_rstn_oen`、`reserved1`、`cqe_algo_sel`、`cqe_prefetch_disable`、`reserved2`、`timer_clk_sel`和`reserved3`：这些字段是控制寄存器中的各个位字段，用于控制eMMC控制器的不同功能和选项。

同时还定义了错误类型，指令类型和信息输出

```rust
#[derive(Debug)]
pub enum CmdError {
    IntError,
}

pub enum CommandType {
    CMD(u8),
    ACMD(u8),
}

impl CommandType {
    pub fn num(&self) -> u8 {
        match self {
            CommandType::CMD(t) => *t,
            CommandType::ACMD(t) => *t,
        }
    }
}
```

这段代码定义了两个枚举类型，一个用于表示命令执行过程中的错误，另一个用于表示命令的类型。并且为 `CmdError` 枚举类型自动实现了 `Debug` trait，以便于调试和打印错误信息。

`utils.rs`设计了针对寄存器的读写操作

```rust
use crate::consts::{PresentState, SD_DRIVER_ADDR};
use bit_struct::*;

pub fn reg_transfer<T>(offset: usize) -> &'static mut T {
    unsafe { ((SD_DRIVER_ADDR + offset) as *mut T).as_mut().unwrap() }
}

/// check the sdcard that was inserted
pub fn check_sd() -> bool {
    let present_state = reg_transfer::<PresentState>(0x24);
    present_state.card_inserted().get() == u1!(1)
}

pub fn mmio_clrsetbits_32(addr: *mut u32, clear: u32, set: u32) {
    unsafe {
        *addr = (*addr & !clear) | set;
    }
}

pub fn mmio_clearbits_32(addr: *mut u32, clear: u32) {
    unsafe {
        *addr = *addr & !clear;
    }
}

pub fn mmio_setbits_32(addr: *mut u32, set: u32) {
    unsafe {
        *addr = *addr | set;
    }
}

pub fn mmio_write_32(addr: *mut u32, value: u32) {
    unsafe {
        *addr = value;
    }
}

pub fn mmio_read_32(addr: *mut u32) -> u32 {
    unsafe { *addr }
}
```

1. `use crate::consts::{PresentState, SD_DRIVER_ADDR};`：这行代码引入了 `consts` 模块中的 `PresentState` 和 `SD_DRIVER_ADDR` 常量。`PresentState` 是一个寄存器状态的结构体，`SD_DRIVER_ADDR` 是一个寄存器的基地址。
2. `pub fn reg_transfer<T>(offset: usize) -> &'static mut T`：这是一个泛型函数，用于在给定偏移量的基础上返回一个可变引用。该函数将基地址 `SD_DRIVER_ADDR` 与偏移量相加，然后将其转换为一个指向类型 `T` 的可变引用。这个函数使用了 `unsafe` 关键字，因为它涉及到原始指针的操作和不可变引用的可变化。
3. `pub fn check_sd() -> bool`：这个函数用于检查是否插入了SD卡。它通过调用 `reg_transfer()` 函数获取 `PresentState` 寄存器的可变引用，并使用 `card_inserted()` 方法获取 `card_inserted` 字段的值。如果 `card_inserted` 字段的值为1，表示SD卡已插入，函数返回 `true`；否则，返回 `false`。
4. `pub fn mmio_clrsetbits_32(addr: *mut u32, clear: u32, set: u32)`：这个函数用于对32位寄存器进行复位和设置位操作。它使用原始指针 `addr` 来访问寄存器，将寄存器的值与 `clear` 进行按位取反的与操作，再与 `set` 进行按位或操作。最终的结果被存储回寄存器。
5. `pub fn mmio_clearbits_32(addr: *mut u32, clear: u32)`：这个函数用于清除32位寄存器中的指定位。它使用原始指针 `addr` 来访问寄存器，将寄存器的值与 `clear` 进行按位取反的与操作，将指定位清零。
6. `pub fn mmio_setbits_32(addr: *mut u32, set: u32)`：这个函数用于设置32位寄存器中的指定位。它使用原始指针 `addr` 来访问寄存器，将寄存器的值与 `set` 进行按位或操作，将指定位设置为1。
7. `pub fn mmio_write_32(addr: *mut u32, value: u32)`：这个函数用于将指定值写入32位寄存器。它使用原始指针 `addr` 来访问寄存器，将指定值 `value` 直接存储到寄存器中。
8. `pub fn mmio_read_32(addr: *mut u32) -> u32`：这个函数用于从32位寄存器中读取值。它使用原始指针 `addr` 来访问寄存器，并返回寄存器中的值。

`lib.rs`是SD卡驱动的主要部分，实现了对SD卡数据的读取写入等功能

```rust
pub fn read_block(block_id: u32, data: &mut [u8]) -> Result<(), CmdError> {
    cmd_transfer(CommandType::CMD(17), block_id, 1)?;
    read_buff(data)?;
    let res = wait_for_xfer_done();
    mmio_write_32(
        (SD_DRIVER_ADDR + 0x30) as _,
        mmio_read_32((SD_DRIVER_ADDR + 0x30) as _),
    );
    res
}

/// write a block to the sdcard.
pub fn write_block(block_id: u32, data: &[u8]) -> Result<(), CmdError> {
    cmd_transfer(CommandType::CMD(24), block_id, 1)?;
    // read_buff(data)?;
    write_buff(data)?;
    let res = wait_for_xfer_done();
    mmio_write_32(
        (SD_DRIVER_ADDR + 0x30) as _,
        mmio_read_32((SD_DRIVER_ADDR + 0x30) as _),
    );
    res
}

pub fn reset_config() {
    unsafe {
        // disable power
        // NOTE: This will close the bus power, but i don't how to restart again.
        power_config(PowerLevel::Close);

        // reset
        mmio_clearbits_32(
            (SD_DRIVER_ADDR + 0x2c) as *mut u32,
            (1 << 24) | (1 << 25) | (1 << 26),
        );
        for _ in 0..0x1000 {
            asm!("nop")
        }
        // enable power
        power_config(PowerLevel::V33);

        // high_speed and data width 4 bit
        // mmio_setbits_32((SD_DRIVER_ADDR + 0x28) as _, (1 << 1) | (1 << 2));
        mmio_setbits_32((SD_DRIVER_ADDR + 0x28) as _, 1 << 2);

        // *((SD_DRIVER_ADDR + 0x28) as *mut u8) |= 1 << 3;
    }
}

pub fn wait_for_cmd_done() -> Result<(), CmdError> {
    let norm_int_sts = reg_transfer::<NormAndErrIntSts>(0x30);
    loop {
        if norm_int_sts.err_int().get() == true {
            mmio_write_32((SD_DRIVER_ADDR + 0x30) as _, 1 << 15);
            break Err(CmdError::IntError);
        }
        if norm_int_sts.cmd_cmpl().get() == true {
            mmio_write_32((SD_DRIVER_ADDR + 0x30) as _, 1 << 0);
            break Ok(());
        }
        for _ in 0..1 {
            unsafe { asm!("nop") }
        }
    }
}
```

1. `pub fn read_block(block_id: u32, data: &mut [u8]) -> Result<(), CmdError>`：这个函数用于从SD卡中读取一个数据块。它接受一个 `block_id` 参数表示要读取的数据块编号，以及一个可变引用 `data` 表示存储读取数据的缓冲区。函数内部调用了 `cmd_transfer()` 函数发送读块的命令（使用命令类型 `CommandType::CMD(17)`），然后调用了 `read_buff()` 函数读取数据块内容到缓冲区中。最后，函数调用了 `wait_for_xfer_done()` 函数等待传输完成，并返回传输结果。
2. `pub fn write_block(block_id: u32, data: &[u8]) -> Result<(), CmdError>`：这个函数用于向SD卡写入一个数据块。它接受一个 `block_id` 参数表示要写入的数据块编号，以及一个 `data` 参数表示要写入的数据块内容。函数内部调用了 `cmd_transfer()` 函数发送写块的命令（使用命令类型 `CommandType::CMD(24)`），然后调用了 `write_buff()` 函数将数据块内容写入到SD卡中。最后，函数调用了 `wait_for_xfer_done()` 函数等待传输完成，并返回传输结果。
3. `pub fn reset_config()`：这个函数用于重置SD卡的配置。函数内部使用了 `unsafe` 关键字，因为它涉及到对寄存器的直接操作。函数首先关闭SD卡的电源（通过调用 `power_config()` 函数），然后对SD卡的寄存器进行复位操作，包括清除特定位的状态、延时等待一段时间，最后重新打开SD卡的电源，并设置高速模式和数据宽度为4位。
4. `pub fn wait_for_cmd_done() -> Result<(), CmdError>`：这个函数用于等待命令执行完成。函数内部使用了一个循环来检查命令执行的状态。它通过调用 `reg_transfer()` 函数获取命令状态寄存器的可变引用，并在循环中检查错误状态和命令完成状态。如果出现错误，函数将返回 `Err(CmdError::IntError)`；如果命令完成，函数将返回 `Ok(())`。在每次循环迭代中，函数使用 `mmio_write_32()` 函数清除中断状态位，并使用 `asm!("nop")` 指令进行短暂延时。

```rust
pub fn wait_for_xfer_done() -> Result<(), CmdError> {
    let norm_int_sts = reg_transfer::<NormAndErrIntSts>(0x30);
    loop {
        if norm_int_sts.xfer_cmpl().get() == true {
            mmio_write_32((SD_DRIVER_ADDR + 0x30) as _, 1 << 1);
            break Ok(());
        }
        if norm_int_sts.err_int().get() == true {
            mmio_write_32((SD_DRIVER_ADDR + 0x30) as _, 1 << 15);
            break Err(CmdError::IntError);
        }
        for _ in 0..1 {
            unsafe { asm!("nop") }
        }
    }
}

pub fn cmd_transfer(cmd: CommandType, arg: u32, blk_cnt: u32) -> Result<(), CmdError> {
    let present_state = reg_transfer::<PresentState>(0x24);

    while present_state.cmd_inhibit_dat().get() == true
        || present_state.cmd_inhibit_dat().get() == true
    {}

    let mut flags: u32 = (cmd.num() as u32) << 24;

    const BLK_CNT_EN: u32 = 1 << 1;
    const XFER_READ: u32 = 1 << 4;
    const DATA_PRESENT: u32 = 1 << 21;
    const L48: u32 = 2 << 16;
    const L48_BUSY: u32 = 2 << 16;
    const L136: u32 = 1 << 16;
    const CRC_CHECK_EN: u32 = 1 << 19;
    const INX_CHECK_EN: u32 = 1 << 20;

    if blk_cnt > 0 {
        // set blk size and blk count
        mmio_write_32((SD_DRIVER_ADDR + 0x04) as _, 0x200 | (blk_cnt << 16));
        flags |= BLK_CNT_EN;
    }

    flags |= match cmd {
        CommandType::CMD(17) => DATA_PRESENT | XFER_READ,
        CommandType::CMD(24) => DATA_PRESENT,
        CommandType::ACMD(51) => DATA_PRESENT | XFER_READ,
        _ => 0,
    };

    flags |= match cmd {
        // R1
        CommandType::ACMD(6)
        | CommandType::ACMD(42)
        | CommandType::ACMD(51)
        | CommandType::CMD(17)
        | CommandType::CMD(24)
        | CommandType::CMD(8)
        | CommandType::CMD(16)
        | CommandType::CMD(7) => L48 | CRC_CHECK_EN | INX_CHECK_EN,
        // R2
        CommandType::CMD(2) | CommandType::CMD(9) => L136 | CRC_CHECK_EN,
        // R3
        CommandType::ACMD(41) | CommandType::CMD(58) => L48,
        // R6
        CommandType::CMD(3) => L48_BUSY | CRC_CHECK_EN | INX_CHECK_EN,
        _ => 0,
    };

    unsafe {
        // set blk cnt
        *((SD_DRIVER_ADDR + 0x06) as *mut u8) = 0;
        // set timeout time
        *((SD_DRIVER_ADDR + 0x2e) as *mut u8) = 0xe;
        *((SD_DRIVER_ADDR + 0x30) as *mut u32) = 0xF3FFFFFF;
        *((SD_DRIVER_ADDR + 0x8) as *mut u32) = arg;
        *((SD_DRIVER_ADDR + 0xc) as *mut u32) = flags;
    }

    wait_for_cmd_done()?;

    let resp1_0 = *reg_transfer::<u32>(0x10);
    let resp3_2 = *reg_transfer::<u32>(0x14);
    let resp5_4 = *reg_transfer::<u32>(0x18);
    let resp7_6 = *reg_transfer::<u32>(0x1c);

    // this is used to print result and consume ptr.
    // There needs to read the resp regs after cmd.
    log::trace!(
        "resp: {:#x} {:#x} {:#x} {:#x}",
        resp1_0,
        resp3_2,
        resp5_4,
        resp7_6
    );

    Ok(())
}
```

1. `pub fn wait_for_xfer_done() -> Result<(), CmdError>`：这个函数用于等待数据传输完成。函数内部使用一个循环来检查传输完成的状态。它首先调用 `reg_transfer()` 函数获取传输状态寄存器的可变引用，并在循环中检查传输完成状态和错误状态。如果传输完成，函数将调用 `mmio_write_32()` 函数写入中断状态位，并返回 `Ok(())`；如果出现错误，函数将调用 `mmio_write_32()` 函数写入中断状态位，并返回 `Err(CmdError::IntError)`。在每次循环迭代中，函数使用 `asm!("nop")` 指令进行短暂延时。
2. `pub fn cmd_transfer(cmd: CommandType, arg: u32, blk_cnt: u32) -> Result<(), CmdError>`：这个函数用于发送命令到SD卡。函数接受一个 `cmd` 参数表示要发送的命令类型，一个 `arg` 参数表示命令参数，以及一个 `blk_cnt` 参数表示块计数（用于读写命令）。函数内部首先通过调用 `reg_transfer()` 函数获取当前的传输状态寄存器，并在一个循环中等待直到命令控制器可用。然后，函数根据命令类型和块计数设置不同的标志位，并将这些标志位与命令号合并为 `flags` 变量。接下来，根据命令类型设置其他的标志位。在 `flags` 设置完成后，函数使用 `unsafe` 关键字访问寄存器，在适当的寄存器中写入命令相关的参数和标志位。然后，函数调用 `wait_for_cmd_done()` 函数等待命令执行完成。最后，函数使用 `reg_transfer()` 函数读取响应寄存器的值，并将其打印出来。

```rust
pub fn pad_settings() {
    mmio_write_32((TOP_BASE + REG_TOP_SD_PWRSW_CTRL) as _, 0x9);

    // let val: u8 = (bunplug) ? 0x3 : 0x0;
    let reset = false;

    let val = if reset { 0x3 } else { 0x0 };

    mmio_write_32(PAD_SDIO0_CD_REG as _, 0x0);
    mmio_write_32(PAD_SDIO0_PWR_EN_REG as _, 0x0);
    mmio_write_32(PAD_SDIO0_CLK_REG as _, val as _);
    mmio_write_32(PAD_SDIO0_CMD_REG as _, val as _);
    mmio_write_32(PAD_SDIO0_D0_REG as _, val as _);
    mmio_write_32(PAD_SDIO0_D1_REG as _, val as _);
    mmio_write_32(PAD_SDIO0_D2_REG as _, val as _);
    mmio_write_32(PAD_SDIO0_D3_REG as _, val as _);

    if reset {
        mmio_clrsetbits_32(
            REG_SDIO0_PWR_EN_PAD_REG as _,
            REG_SDIO0_PAD_CLR_MASK,
            REG_SDIO0_PWR_EN_PAD_RESET << REG_SDIO0_PAD_SHIFT,
        );

        mmio_clrsetbits_32(
            REG_SDIO0_CD_PAD_REG as _,
            REG_SDIO0_PAD_CLR_MASK,
            REG_SDIO0_CD_PAD_RESET << REG_SDIO0_PAD_SHIFT,
        );

        mmio_clrsetbits_32(
            REG_SDIO0_CLK_PAD_REG as _,
            REG_SDIO0_PAD_CLR_MASK,
            REG_SDIO0_CLK_PAD_RESET << REG_SDIO0_PAD_SHIFT,
        );

        mmio_clrsetbits_32(
            REG_SDIO0_CMD_PAD_REG as _,
            REG_SDIO0_PAD_CLR_MASK,
            REG_SDIO0_CMD_PAD_RESET << REG_SDIO0_PAD_SHIFT,
        );

        mmio_clrsetbits_32(
            REG_SDIO0_DAT1_PAD_REG as _,
            REG_SDIO0_PAD_CLR_MASK,
            REG_SDIO0_DAT1_PAD_RESET << REG_SDIO0_PAD_SHIFT,
        );

        mmio_clrsetbits_32(
            REG_SDIO0_DAT0_PAD_REG as _,
            REG_SDIO0_PAD_CLR_MASK,
            REG_SDIO0_DAT0_PAD_RESET << REG_SDIO0_PAD_SHIFT,
        );

        mmio_clrsetbits_32(
            REG_SDIO0_DAT2_PAD_REG as _,
            REG_SDIO0_PAD_CLR_MASK,
            REG_SDIO0_DAT2_PAD_RESET << REG_SDIO0_PAD_SHIFT,
        );

        mmio_clrsetbits_32(
            REG_SDIO0_DAT3_PAD_REG as _,
            REG_SDIO0_PAD_CLR_MASK,
            REG_SDIO0_DAT3_PAD_RESET << REG_SDIO0_PAD_SHIFT,
        );
    } else {
        mmio_clrsetbits_32(
            REG_SDIO0_PWR_EN_PAD_REG as _,
            REG_SDIO0_PAD_CLR_MASK,
            REG_SDIO0_PWR_EN_PAD_VALUE << REG_SDIO0_PAD_SHIFT,
        );

        mmio_clrsetbits_32(
            REG_SDIO0_CD_PAD_REG as _,
            REG_SDIO0_PAD_CLR_MASK,
            REG_SDIO0_CD_PAD_VALUE << REG_SDIO0_PAD_SHIFT,
        );

        mmio_clrsetbits_32(
            REG_SDIO0_CLK_PAD_REG as _,
            REG_SDIO0_PAD_CLR_MASK,
            REG_SDIO0_CLK_PAD_VALUE << REG_SDIO0_PAD_SHIFT,
        );

        mmio_clrsetbits_32(
            REG_SDIO0_CMD_PAD_REG as _,
            REG_SDIO0_PAD_CLR_MASK,
            REG_SDIO0_CMD_PAD_VALUE << REG_SDIO0_PAD_SHIFT,
        );

        mmio_clrsetbits_32(
            REG_SDIO0_DAT1_PAD_REG as _,
            REG_SDIO0_PAD_CLR_MASK,
            REG_SDIO0_DAT1_PAD_VALUE << REG_SDIO0_PAD_SHIFT,
        );

        mmio_clrsetbits_32(
            REG_SDIO0_DAT0_PAD_REG as _,
            REG_SDIO0_PAD_CLR_MASK,
            REG_SDIO0_DAT0_PAD_VALUE << REG_SDIO0_PAD_SHIFT,
        );

        mmio_clrsetbits_32(
            REG_SDIO0_DAT2_PAD_REG as _,
            REG_SDIO0_PAD_CLR_MASK,
            REG_SDIO0_DAT2_PAD_VALUE << REG_SDIO0_PAD_SHIFT,
        );

        mmio_clrsetbits_32(
            REG_SDIO0_DAT3_PAD_REG as _,
            REG_SDIO0_PAD_CLR_MASK,
            REG_SDIO0_DAT3_PAD_VALUE << REG_SDIO0_PAD_SHIFT,
        );
    }
}
```

这段代码是SD卡驱动中的 `pad_settings()` 函数，用于设置SD卡的引脚配置。

首先，代码通过调用 `mmio_write_32()` 函数将值 `0x9` 写入寄存器 `TOP_SD_PWRSW_CTRL`，从而设置SD卡的电源开关控制。

接下来，代码根据 `reset` 变量的值判断是否进行复位操作。如果 `reset` 为真，则将变量 `val` 设置为 `0x3`，否则设置为 `0x0`。

然后，代码使用 `mmio_write_32()` 函数将不同的值写入一系列与SD卡引脚相关的寄存器。具体来说，将 `0x0` 写入 `PAD_SDIO0_CD_REG` 寄存器、`PAD_SDIO0_PWR_EN_REG` 寄存器以及 `PAD_SDIO0_CLK_REG`、`PAD_SDIO0_CMD_REG`、`PAD_SDIO0_D0_REG`、`PAD_SDIO0_D1_REG`、`PAD_SDIO0_D2_REG`、`PAD_SDIO0_D3_REG` 寄存器。这些寄存器控制着SD卡引脚的功能和电平状态。

如果 `reset` 为真，代码将执行复位操作。对于每个引脚，代码使用 `mmio_clrsetbits_32()` 函数将相应的复位值写入寄存器，通过位掩码和位移操作将复位值放置在正确的位置。

如果 `reset` 为假，代码将设置引脚为非复位状态。对于每个引脚，代码使用 `mmio_clrsetbits_32()` 函数将相应的非复位值写入寄存器，通过位掩码和位移操作将非复位值放置在正确的位置。

这样，`pad_settings()` 函数完成了SD卡引脚的配置，根据 `reset` 变量的值选择进行复位或非复位操作，并将相应的值写入对应的寄存器，以控制SD卡的引脚状态。

```rust
pub fn init() -> Result<(), CmdError> {
    // Initialize sd card gpio
    if check_sd() {
        pad_settings();
        reset_config();

        power_config(PowerLevel::V18);
        set_clock(4);

        // sdcard initialize.
        cmd_transfer(CommandType::CMD(0), 0, 0)?;
        cmd_transfer(CommandType::CMD(8), 0x1aa, 0)?;
        // wait for initialization to end.
        loop {
            cmd_transfer(CommandType::CMD(55), 0, 0)?;
            cmd_transfer(
                CommandType::ACMD(41),
                0x4000_0000 | 0x0030_0000 | (0x1FF << 15),
                0,
            )?;

            if *reg_transfer::<u32>(0x10) >> 31 == 1 {
                break;
            }
            for _ in 0..0x100_0000 {
                unsafe { asm!("nop") }
            }
        }
        log::debug!("init finished");
        // // get card and select
        cmd_transfer(CommandType::CMD(2), 0, 0)?;
        cmd_transfer(CommandType::CMD(3), 0, 0)?;
        log::debug!("start to read scd");
        let rsa = *reg_transfer::<u32>(0x10) & 0xffff0000;
        cmd_transfer(CommandType::CMD(9), rsa, 0)?; // get scd reg
        log::debug!("start to select card");
        cmd_transfer(CommandType::CMD(7), rsa, 0)?; // select card

        log::debug!("start to switch to 4 bit bus");
        // support 4 bit bus width.
        cmd_transfer(CommandType::CMD(55), rsa, 0)?;
        cmd_transfer(CommandType::ACMD(6), 2, 0)?;
        unsafe {
            *((SD_DRIVER_ADDR + 0x28) as *mut u8) |= 2;
        }
        clk_en(false);
    }
    Ok(())
}
```

这段代码是SD卡驱动中的 `init()` 函数，用于初始化SD卡。

首先，代码通过调用 `check_sd()` 函数检查SD卡是否存在。如果存在，则执行以下初始化步骤：

1. 调用 `pad_settings()` 函数配置SD卡的引脚。
2. 调用 `reset_config()` 函数对SD卡进行复位配置。
3. 调用 `power_config(PowerLevel::V18)` 函数设置SD卡的电源电压为1.8V。
4. 调用 `set_clock(4)` 函数设置SD卡的时钟频率为4。

接下来，代码执行SD卡的初始化过程。具体步骤如下：

1. 发送命令 `CMD(0)` 进行SD卡初始化。

2. 发送命令 `CMD(8)` 并传递参数 `0x1aa`，用于与SD卡进行通信。

3. 进入一个循环，不断发送命令

   ```
   CMD(55)
   ```

   和

   ```
   ACMD(41)
   ```

   直到初始化结束。在发送

   ```
   ACMD(41)
   ```

   命令时，传递了一些参数，包括位掩码和位移操作。循环检查寄存器的值，如果满足条件则跳出循环，否则等待一段时间。

   - 在循环中，通过 `reg_transfer()` 函数读取寄存器的值，并进行一些位操作。
   - 使用 `asm!("nop")` 指令进行空操作，等待一段时间。

初始化完成后，代码输出日志信息并继续执行以下步骤：

1. 发送命令 `CMD(2)` 获取SD卡的CID信息。
2. 发送命令 `CMD(3)` 进入SD卡的数据传输状态。
3. 输出日志信息。
4. 发送命令 `CMD(9)` 并传递参数 `rsa`，用于获取SD卡的SCR寄存器的值。
5. 发送命令 `CMD(7)` 并传递参数 `rsa`，选择SD卡。
6. 输出日志信息。
7. 发送命令 `CMD(55)` 和 `ACMD(6)`，用于支持4位数据总线宽度。
8. 使用 `*((SD_DRIVER_ADDR + 0x28) as *mut u8) |= 2` 指令将某个地址处的值的特定位设置为1。
9. 调用 `clk_en(false)` 函数关闭时钟使能。

最后，函数返回 `Ok(())` 表示初始化成功。



为了更好地完成任务，我们在编写驱动时特别关注了以下工作：

1. **引脚配置和复位设置**：代码通过调用 `pad_settings()` 函数对SD卡的引脚进行配置，以及调用 `reset_config()` 函数对SD卡进行复位设置。这些步骤确保SD卡与系统正确连接，并处于正确的初始状态。
2. **电源配置**：代码通过调用 `power_config()` 函数对SD卡的电源电压进行配置。这允许驱动程序为SD卡提供正确的电源电压，以确保其正常工作。
3. **时钟设置**：代码调用 `set_clock()` 函数设置SD卡的时钟频率。正确的时钟设置对于SD卡的数据传输和通信至关重要。
4. **SD卡初始化过程**：代码执行了SD卡的初始化过程，包括发送特定的命令和参数以与SD卡进行通信和配置。这确保了SD卡能够正确地与系统进行交互，并准备好进行后续的数据传输操作。
5. **日志输出**：代码使用日志输出来提供关键的调试信息，帮助开发人员了解SD卡初始化过程的状态和进展。这对于故障排除和性能优化非常有用。
6. **4位数据总线宽度支持**：代码通过发送命令 `CMD(55)` 和 `ACMD(6)`，以及特定的位操作，支持SD卡的4位数据总线宽度。这可以提高数据传输速度和性能。

