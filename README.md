# ByteOS

**Due to time constraints, the project will be suspended until 2024**.

## Kernel struct Design

> ByteOS is a posix-compatible kernel.
>
> If you are interested in this project, please contact me.
>
> email: 321353225@qq.com  qq: 321353225

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
