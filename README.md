# 河南科技大学-你说对不队

## 内核 ByteOS

## TODO List
- [x] higher half kernel
- [x] Modular skeleton
- [x] global allocator
- [ ] add the large page (addr > 0xffffffffc02x0000) to allocator
- [ ] frame allocator, use bitmap written by myself or bit_field crate
- [ ] Interrupt support
- [ ] smp support
- [ ] async/await support
- [ ] get devices info and memory info from device_tree
- [ ] MMIO device support, eg: blk, (net?)
