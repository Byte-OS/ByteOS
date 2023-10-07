SHELL := /bin/bash
NVME := off
NET  := off
ARCH := riscv64imac
LOG  := error
BOARD:= qemu
RELEASE := release
KERNEL_ELF = target/$(ARCH)-unknown-none-elf/$(RELEASE)/kernel
BIN_FILE = byteos.bin
# SBI	:= tools/rustsbi-qemu.bin
FS_IMG  := mount.img
SBI := tools/opensbi-$(BOARD).bin
features:= 
K210-SERIALPORT	= /dev/ttyUSB0
K210-BURNER	= tools/k210/kflash.py
RUST_BUILD_OPTIONS := 
QEMU_EXEC := qemu-system-riscv64 \
				-machine virt \
				-kernel $(KERNEL_ELF) \
				-m 128M \
				-bios $(SBI) \
				-nographic \
				-smp 1
TESTCASE := testcase-gcc
ifeq ($(NVME), on)
QEMU_EXEC += -drive file=$(FS_IMG),if=none,id=nvm \
				-device nvme,serial=deadbeef,drive=nvm 
features += nvme
else
QEMU_EXEC += -drive file=$(FS_IMG),if=none,format=raw,id=x0 \
        		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 
endif

ifeq ($(NET), on)
QEMU_EXEC += -netdev user,id=net0,hostfwd=tcp::6379-:6379,hostfwd=tcp::2222-:2222,hostfwd=tcp::2000-:2000,hostfwd=tcp::8487-:8487,hostfwd=tcp::5188-:5188,hostfwd=tcp::12000-:12000 -object filter-dump,id=net0,netdev=net0,file=packets.pcap \
	-device virtio-net-device,netdev=net0
features += net
endif

ifeq ($(RELEASE), release)
	RUST_BUILD_OPTIONS += --release
endif

ifeq ($(BOARD), k210)
SBI = tools/rustsbi-k210.bin
features += k210
endif

features += board-$(BOARD)

all: 
	RUST_BACKTRACE=1 LOG=$(LOG) cargo build $(RUST_BUILD_OPTIONS) --features "$(features)" --offline
#	cp $(SBI) sbi-qemu
#	cp $(KERNEL_ELF) kernel-qemu
	rust-objcopy --binary-architecture=riscv64 $(KERNEL_ELF) --strip-all -O binary os.bin

fs-img:
	rm -f $(FS_IMG)
	dd if=/dev/zero of=$(FS_IMG) bs=1M count=2000
	mkfs.vfat -F 32 $(FS_IMG)
	mkdir mount/ -p
	sudo mount $(FS_IMG) mount/ -o uid=1000,gid=1000
	rm -rf mount/*
	-cp -rf tools/$(TESTCASE)/* mount/
	sudo umount $(FS_IMG)

build:
	cp .cargo/linker-$(BOARD).ld .cargo/linker-riscv.ld
	RUST_BACKTRACE=1 LOG=$(LOG) cargo build $(RUST_BUILD_OPTIONS) --features "$(features)" $(OFFLINE)

run: fs-img build
	time $(QEMU_EXEC)

fdt:
	@qemu-system-riscv64 -M 128m -machine virt,dumpdtb=virt.out
	fdtdump virt.out

justrun: build
	$(QEMU_EXEC)

cv1811h-build: build
	rust-objcopy --binary-architecture=riscv64 $(KERNEL_ELF) --strip-all -O binary $(BIN_FILE)
	sudo ./cv1811h-burn.sh

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
