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
		-nographic \
		-smp 2

clean:
	rm -rf target/
