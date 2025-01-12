SHELL := /bin/bash
BOARD:= qemu
BIN   :=
byteos = $(shell kbuild $(1) byteos.yaml $(BIN) $(2))
byteos_config = $(call byteos,config,get_cfg $(1))
byteos_env = $(call byteos,config,get_env $(1))
byteos_meta = $(call byteos,config,get_meta $(1))
byteos_triple = $(call byteos,config,get_triple $(1))
NVME := off
NET  := off
LOG  := error
RELEASE := release
QEMU_EXEC ?= 
GDB  ?= gdb-multiarch
ARCH := $(call byteos_triple,arch)
ROOT_FS := $(call byteos_config,root_fs)
TARGET := $(call byteos_meta,target)

BUS  := device
ifeq ($(ARCH), x86_64)
  QEMU_EXEC += qemu-system-x86_64 \
				-machine q35 \
				-kernel $(KERNEL_ELF) \
				-cpu IvyBridge-v2
  BUS := pci
else ifeq ($(ARCH), riscv64)
  QEMU_EXEC += qemu-system-$(ARCH) \
				-machine virt \
				-bios $(SBI) \
				-kernel $(KERNEL_BIN)
else ifeq ($(ARCH), aarch64)
  QEMU_EXEC += qemu-system-$(ARCH) \
				-cpu cortex-a72 \
				-machine virt \
				-kernel $(KERNEL_BIN)
else ifeq ($(ARCH), loongarch64)
  QEMU_EXEC += qemu-system-$(ARCH) -kernel $(KERNEL_ELF)
  BUS := pci
else
  $(error "ARCH" must be one of "x86_64", "riscv64", "aarch64" or "loongarch64")
endif

KERNEL_ELF = target/$(TARGET)/$(RELEASE)/kernel
KERNEL_BIN = target/$(TARGET)/$(RELEASE)/kernel.bin
BIN_FILE = byteos.bin
# SBI	:= tools/rustsbi-qemu.bin
FS_IMG  := mount.img
SBI := tools/opensbi-$(BOARD).bin
features:= 
K210-SERIALPORT	= /dev/ttyUSB0
K210-BURNER	= tools/k210/kflash.py
QEMU_EXEC += -m 1G\
			-nographic \
			-smp 1 \
			-D qemu.log -d in_asm,int,pcall,cpu_reset,guest_errors

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

ifeq ($(BOARD), k210)
SBI = tools/rustsbi-k210.bin
features += k210
endif

all: build
offline:
	RUST_BACKTRACE=1 LOG=$(LOG) cargo build $(BUILD_ARGS) --features "$(features)" --offline
#	cp $(SBI) sbi-qemu
#	cp $(KERNEL_ELF) kernel-qemu
	rust-objcopy --binary-architecture=riscv64 $(KERNEL_ELF) --strip-all -O binary os.bin

fs-img:
	@echo "TESTCASE: $(TESTCASE)"
	@echo "ROOT_FS: $(ROOT_FS)"
	rm -f $(FS_IMG)
	dd if=/dev/zero of=$(FS_IMG) bs=1M count=128
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
	mkfs.ext4  -F -O ^metadata_csum_seed $(FS_IMG)
	mkdir mount/ -p
	sudo mount $(FS_IMG) mount/
endif
	sudo cp -rf tools/$(TESTCASE)/* mount/
	sync
	sudo umount $(FS_IMG)

build:
	kbuild build byteos.yaml $(BIN)
	rust-objcopy --binary-architecture=$(ARCH) $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)

justbuild: fs-img build 

run: fs-img build
	time $(QEMU_EXEC)

fdt:
	@qemu-system-riscv64 -M 128m -machine virt,dumpdtb=virt.out
	fdtdump virt.out

justrun: fs-img
	rust-objcopy --binary-architecture=$(ARCH) $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)
	$(QEMU_EXEC)

tftp-build: build
	rust-objcopy --binary-architecture=riscv64 $(KERNEL_ELF) --strip-all -O binary $(BIN_FILE)
	sudo ./tftp-burn.sh

k210-build: build
	rust-objcopy --binary-architecture=riscv64 $(KERNEL_ELF) --strip-all -O binary $(BIN_FILE)
	@cp $(SBI) $(SBI).copy
	@dd if=$(BIN_FILE) of=$(SBI).copy bs=131072 seek=1
	@mv $(SBI).copy $(BIN_FILE)

flash: k210-build
	(which $(K`210-BURNER)) || (cd tools && git clone https://github.com/sipeed/kflash.py.git k210)
	@sudo chmod 777 $(K210-SERIALPORT)
	python3 $(K210-BURNER) -p $(K210-SERIALPORT) -b 1500000 $(BIN_FILE)
	python3 -m serial.tools.miniterm --eol LF --dtr 0 --rts 0 --filter direct $(K210-SERIALPORT) 115200

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
