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
- [x] frame allocator, use bitmap written by myself or bit_field crate
- [ ] Interrupt support
- [ ] smp support
- [ ] async/await support
- [x] get devices info and memory info from device_tree
- [ ] MMIO device support, eg: blk, (net?)
