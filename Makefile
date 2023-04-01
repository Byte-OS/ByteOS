ARCH := riscv64imac
LOG := info
KERNEL_ELF = target/$(ARCH)-unknown-none-elf/release/kernel
# SBI	:= tools/rustsbi-qemu.bin
SBI := tools/opensbi-qemu.bin
QEMU_EXEC := qemu-system-riscv64 \
				-machine virt \
				-kernel $(KERNEL_ELF) \
				-m 128M \
				-bios $(SBI) \
				-nographic \
				-smp 2

all:

build:
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
