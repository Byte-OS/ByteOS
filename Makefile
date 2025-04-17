SHELL := /bin/bash
include scripts/config.mk
export BOARD := qemu
export ROOT_MANIFEST_DIR := $(shell pwd)
SMP := 1
NVME := off
NET  := off
RELEASE := release
GDB  ?= gdb-multiarch
KERNEL_ELF = target/$(TARGET)/$(RELEASE)/kernel
KERNEL_BIN = target/$(TARGET)/$(RELEASE)/kernel.bin

BUS  := device
QEMU_EXEC := qemu-system-$(ARCH) 
ifeq ($(ARCH), x86_64)
  QEMU_EXEC += -machine q35 \
				-cpu IvyBridge-v2 \
				-kernel $(KERNEL_ELF)
  BUS := pci
else ifeq ($(ARCH), riscv64)
  QEMU_EXEC += -machine virt \
				-kernel $(KERNEL_BIN)
else ifeq ($(ARCH), aarch64)
  QEMU_EXEC += -cpu cortex-a72 \
				-machine virt \
				-kernel $(KERNEL_BIN)
else ifeq ($(ARCH), loongarch64)
  QEMU_EXEC += -kernel $(KERNEL_ELF)
  BUS := pci
else
  $(error "ARCH"($(ARCH)) must be one of "x86_64", "riscv64", "aarch64" or "loongarch64")
endif

FS_IMG  := mount.img
features:= 
QEMU_EXEC += -m 1G\
			-nographic \
			-smp $(SMP)
ifeq ($(QEMU_LOG), on)
QEMU_EXEC += -D qemu.log -d in_asm,int,pcall,cpu_reset,guest_errors
endif

TESTCASE := testcase-$(ARCH)
ifeq ($(NVME), on)
QEMU_EXEC += -drive file=$(FS_IMG),if=none,id=nvm \
				-device nvme,serial=deadbeef,drive=nvm
else
QEMU_EXEC += -drive file=$(FS_IMG),if=none,format=raw,id=x0
	QEMU_EXEC += -device virtio-blk-$(BUS),drive=x0
endif

ifeq ($(NET), on)
QEMU_EXEC += -netdev user,id=net0,hostfwd=tcp::6379-:6379,hostfwd=tcp::2222-:2222,hostfwd=tcp::2000-:2000,hostfwd=tcp::8487-:8487,hostfwd=tcp::5188-:5188,hostfwd=tcp::12000-:12000 -object filter-dump,id=net0,netdev=net0,file=packets.pcap \
	-device virtio-net-$(BUS),netdev=net0
features += net
endif

all: build
test: 
	@echo $(TARGET)
	@echo $(PLATFORM)
	@echo $(ARCH)
	@echo $(ROOT_FS)
	@echo $(CONFIGS)

offline:
	cargo build --features "$(features)" --offline
	rust-objcopy --binary-architecture=riscv64 $(KERNEL_ELF) --strip-all -O binary os.bin

fs-img:
	@echo "TESTCASE: $(TESTCASE)"
	@echo "ROOT_FS: $(ROOT_FS)"
	rm -f $(FS_IMG)
	dd if=/dev/zero of=$(FS_IMG) bs=1M count=96
	sync
ifeq ($(ROOT_FS), fat32)
	mkfs.vfat -F 32 $(FS_IMG)
	mkdir mount/ -p
	sudo mount $(FS_IMG) mount/ -o uid=1000,gid=1000
	sudo rm -rf mount/*
else ifeq ($(ROOT_FS), ext4_rs)
	mkfs.ext4 -b 4096 $(FS_IMG)
	mkdir mount/ -p
	sudo mount $(FS_IMG) mount/
else 
	mkfs.ext4 -b 4096 -F -O ^metadata_csum_seed $(FS_IMG)
	mkdir mount/ -p
	sudo mount $(FS_IMG) mount/
endif
	sudo cp -rf tools/$(TESTCASE)/* mount/
	sync
	sudo umount $(FS_IMG)

build:
	cargo build --target $(TARGET) --features "$(features)" --release
	rust-objcopy --binary-architecture=$(ARCH) $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)

justbuild: fs-img build 

run: fs-img build
	time $(QEMU_EXEC)

justrun: fs-img
	$(QEMU_EXEC)

tftp-build: build
	sudo ./tftp-burn.sh

debug: fs-img build
	@tmux new-session -d \
	"$(QEMU_EXEC) -s -S && echo '按任意键继续' && read -n 1" && \
	tmux split-window -h "$(GDB) $(KERNEL_ELF) -ex 'target remote localhost:1234' -ex 'disp /16i $pc' " && \
	tmux -2 attach-session -d
	# $(QEMU_EXEC) -s -S &
	# sleep 1
	# $(GDB) $(KERNEL_ELF) \
	# 	-ex 'target remote localhost:1234' \
	# 	-ex 'disp /16i $pc'

clean:
	rm -rf target/

gdb:
	riscv64-elf-gdb \
        -ex 'file $(KERNEL_ELF)' \
        -ex 'set arch riscv:rv64' \
        -ex 'target remote localhost:1234'

addr2line:
	addr2line -sfipe $(KERNEL_ELF) | rustfilt

iso: build
	cp $(KERNEL_ELF) tools/iso/example
	grub-mkrescue -o bootable.iso tools/iso

boot-iso: iso
	qemu-system-x86_64 -cdrom bootable.iso -serial stdio

.PHONY: all run build clean gdb justbuild iso boot-iso
