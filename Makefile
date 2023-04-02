ARCH := riscv64imac
LOG  := info
KERNEL_ELF = target/$(ARCH)-unknown-none-elf/release/kernel
# SBI	:= tools/rustsbi-qemu.bin
FS_IMG  := mount.img
SBI := tools/opensbi-qemu.bin
QEMU_EXEC := qemu-system-riscv64 \
				-machine virt \
				-kernel $(KERNEL_ELF) \
				-m 128M \
				-bios $(SBI) \
				-drive file=$(FS_IMG),if=none,format=raw,id=x0 \
        		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
				-nographic \
				-smp 2

all:

fs-img:
	rm -f $(FS_IMG)
	dd if=/dev/zero of=$(FS_IMG) bs=1M count=40
	mkfs.vfat -F 32 $(FS_IMG)
	sudo mount $(FS_IMG) mount/ -o uid=1000,gid=1000
	cp -r tools/testcase-step1/* mount/
	sudo umount $(FS_IMG)

build: fs-img
	LOG=$(LOG) cargo build --release

run: build
	$(QEMU_EXEC)

debug: build
	$(QEMU_EXEC) -s -S

clean:
	rm -rf target/

gdb:
	riscv64-elf-gdb \
        -ex 'file $(KERNEL_ELF)' \
        -ex 'set arch riscv:rv64' \
        -ex 'target remote localhost:1234'

.PHONY: all run build clean gdb
