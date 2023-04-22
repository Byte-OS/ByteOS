ARCH := riscv64imac
LOG  := info
RELEASE := release
KERNEL_ELF = target/$(ARCH)-unknown-none-elf/$(RELEASE)/kernel
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
RUST_BUILD_OPTIONS := 

ifeq ($(RELEASE), release)
	RUST_BUILD_OPTIONS += --release
endif

all: 
	RUST_BACKTRACE=1 LOG=$(LOG) cargo build $(RUST_BUILD_OPTIONS) --offline
	cp $(SBI) sbi-qemu
	cp $(KERNEL_ELF) kernel-qemu

fs-img:
	rm -f $(FS_IMG)
	dd if=/dev/zero of=$(FS_IMG) bs=1M count=40
	mkfs.vfat -F 32 $(FS_IMG)
	sudo mount $(FS_IMG) mount/ -o uid=1000,gid=1000
	cp -r tools/testcase-step3/* mount/
	sudo umount $(FS_IMG)

build: fs-img
	RUST_BACKTRACE=1 LOG=$(LOG) cargo build $(RUST_BUILD_OPTIONS) $(OFFLINE)

run: build
	$(QEMU_EXEC)

debug: build
	@tmux new-session -d \
	"$(QEMU_EXEC) -s -S && echo '按任意键继续' && read -n 1" && \
	tmux split-window -h "riscv64-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'" && \
	tmux -2 attach-session -d

clean:
	rm -rf target/

gdb:
	riscv64-elf-gdb \
        -ex 'file $(KERNEL_ELF)' \
        -ex 'set arch riscv:rv64' \
        -ex 'target remote localhost:1234'

addr2line:
	addr2line -sfipe $(KERNEL_ELF) | rustfilt

.PHONY: all run build clean gdb
