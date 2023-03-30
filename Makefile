ARCH := riscv64imac
LOG := info
KERNEL_ELF = target/$(ARCH)-unknown-none-elf/release/kernel

all:

build:
	LOG=$(LOG) cargo build --release

run: build
	qemu-system-riscv64 \
		-machine virt \
		-kernel $(KERNEL_ELF) \
		-m 128M \
		-bios tools/rustsbi-qemu.bin \
		-nographic \
		-smp 2

clean:
	rm -rf target/

gdb:
	riscv64-elf-gdb \
        -ex 'file $(KERNEL_ELF)' \
        -ex 'set arch riscv:rv64' \
        -ex 'target remote localhost:1234'

.PHONY: all run build clean gdb
