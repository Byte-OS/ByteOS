# 河南科技大学-你说对不队

## 内核 ByteOS

## Kernel struct Design

```plain
crates --> arch --> modules --> kernel
```

## TODO List
- [x] higher half kernel
- [x] Modular skeleton
- [x] global allocator
- [x] RTC device support
- [x] Timestamp --> actual Date/Time [timestamp crate](crates/timestamp/)
- [x] frame allocator, use bit_field to store page usage
- [x] Interrupt support
- [x] backtrace support
- [x] timer interrupt support
- [x] page mapping support
- [x] get devices info and memory info from device_tree
- [x] VIRTIO blk device support
- [x] Add a banner for os. use tool [banner生成工具](http://patorjk.com/software/taag/#p=display&f=Big&t=ByteOS)
- [x] vfs support, contains Inode
- [x] fatfs support
- [x] fs mount support (a temporary solution)
- [x] ramfs support
- [x] devfs support
- [x] async/await support (simple version)
- [x] process support
- [ ] syscalls [syscalls](./docs/step1-progress.md)
- [ ] VIRTIO net device support
- [ ] smp support

# 运行

> 内含一个简单的 `shell`, 可以执行 `help`, `ls`, `clear`, `exit`, `brk`, `run_all` 命令或者执行 `elf` 文件
>
> `brk` 是执行一个 `brk` 程序.

```shell
make run
```

![](./run.png)
