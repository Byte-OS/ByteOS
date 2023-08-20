# drivers部分文件结构

```
├── k210-sdcard	//支持k210的SD卡驱动
│   ├── Cargo.toml
│   └── src
│       └── lib.rs
├── kcvitek-sd	//kcvitek-sd驱动
│   ├── Cargo.toml
│   └── src
│       └── lib.rs
├── kgoldfish-rtc	//RTC设备驱动
│   ├── Cargo.toml
│   └── src
│       └── lib.rs
├── knvme	//KNVME驱动
│   ├── Cargo.toml
│   └── src
│       └── lib.rs
└── kvirtio	//虚拟IO驱动
    ├── Cargo.toml
    └── src
        ├── lib.rs
        ├── virtio_blk.rs
        ├── virtio_impl.rs
        └── virtio_net.rs
```

我们在内核中使用 `Linke` 的库实现了对于驱动的简单组合，在一些情况下，我们需要构建针对某个硬件的 OS，而且只需要部分驱动，这个时候我们就可以通过在ByteOS中的 `link` 设计，实现对于驱动的组合。

下面是我们 `kernel/src/modules.rs` 中的内容。
```rust
#[allow(unused_imports)]
use kheader::macros::module_use;

module_use!(kvirtio);
#[cfg(feature = "nvme")]
module_use!(knvme);

module_use!(kgoldfish_rtc);

#[cfg(feature = "board-k210")]
module_use!(k210_sdcard);
#[cfg(feature = "board-cv1811h")]
module_use!(kcvitek_sd);
```
如果我们需要使用某个驱动，我们可以使用 `module_use!` 宏进行驱动的注入。所有的注入的驱动会在检查设备树之前进行挂载。如果我们需要追加设备树上没有的设备也可以通过这个功能进行注入。

所有需要注入的驱动里面需要利用 `driver_define!` 宏进行初始化函数和结构的定义。

将设备挂载到设备树上。

```rust
driver_define!("cvitek,mars-sd", {
    DRIVER_REGS.lock().insert("cvitek,mars-sd", init_rtc);
    None
});
```

直接注入驱动

```rust
driver_define!(DRIVERS_INIT, {
    info!("init k210 sdcard");
    Some(Arc::new(SDCardWrapper::new()))
});
```

如果我们需要直接