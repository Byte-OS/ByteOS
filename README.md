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
- [ ] add the large page (addr > 0xffffffffc02x0000) to allocator
- [x] RTC device support
- [ ] Timestamp --> actual Date/Time
- [x] frame allocator, use bitmap written by myself or bit_field crate
- [x] Interrupt support
- [ ] timer interrupt support
- [x] get devices info and memory info from device_tree
- [x] VIRTIO blk device support
- [ ] vfs support, contains Inode
- [ ] fatfs support
- [ ] ramfs support
- [ ] devfs support
- [ ] process support
- [ ] syscalls
    - [ ] open
    - [ ] exec
    - [ ] read
    - [ ] write
- [ ] VIRTIO net device support
- [ ] smp support
- [ ] async/await support

