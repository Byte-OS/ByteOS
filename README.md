# ByteOS

## How to use this project

Run with make file.

```shell
# riscv64
make PLATFORM=riscv64-qemu run
# aarch64
make PLATFORM=aarch64-qemu run
# x86_64
make PLATFORM=x86_64-qemu run
# loongarch64
make PLATFORM=loongarch64-qemu run
```

## byteos.yaml

> byteos.yaml is a configuration file for the ByteOS.

You can change the config in this file. For example, you want to use the ext4 fielsystem.

Set the root_fs to 'ext4' or 'ext4_rs' will change the root_fs from 'fat32' to 'ext4'.

The 'ext4' and 'ext4_rs' are the different implementation of the ext4.

TIPS: Make ensure that the mkefs version of your system lower than 1.70. If not, you have to use another argument to build the ext4 image.

## Kernel struct Design

> ByteOS is a posix-compatible kernel.
>
> If you are interested in this project, please contact me.
>
> email: <321353225@qq.com>  qq: 321353225

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
- [x] Add a banner for os. use tool [banner generation tool](http://patorjk.com/software/taag/#p=display&f=Big&t=ByteOS)
- [x] vfs support
- [x] fatfs support
- [x] fs mount support (a temporary solution)
- [x] ramfs support
- [x] devfs support
- [x] async/await support (simple version)
- [x] process support
- [x] VIRTIO net device support
- [ ] smp support
- [ ] desktop support. eg: dwm, hyprland.

## Program support

tools/final2023:

- libctest
- libcbench
- busybox
- lua
- lmbench
- iozone
- iperf3
- nerperf
- cyclic
- unixbench

tools/gcc

- gcc
- redis-server
- ssh-simple
- http-server

You can change the `TESTCASE` in the makefile to change the target. You can run other program in the sh or change the default program in the `kernel/src/tasks/initproc.rs` file.

## run busybox sh on qemu platform

```bash
make run BOARD=qemu LOG=info NET=off
```

Changing 'LOG=info' to 'LOG=error' if you don't need any info output.
