const targetMap: Record<string, string> = {
    "riscv64": 'riscv64gc-unknown-none-elf',
    "x86_64": 'x86_64-unknown-none',
    "aarch64": 'aarch64-unknown-none-softfloat',
    "loongarch64": 'loongarch64-unknown-none'
};

export class KernelBuilder {
    arch: string;
    elfPath: string;
    binPath: string;
    rustflags: string;

    constructor(arch: string) {
        this.arch = arch;
        this.elfPath = `${Deno.cwd()}/target/${targetMap[arch]}/release/kernel`;
        this.binPath = `${this.elfPath}.bin`;

        this.rustflags = Deno.env.get('rustflags') || "";
    }

    buildFlags() {
        const rustflags = [
            "-Cforce-frame-pointers=yes",
            "-Clink-arg=-no-pie",
            "-Ztls-model=local-exec",
            `--cfg=root_fs="ext4_rs"`,
            '--cfg=board="qemu"'
        ];
        
        this.rustflags += rustflags.join(" ");
    }

    async buildElf() {
        this.buildFlags();

        const buildProc = new Deno.Command("cargo", {
            args: [
                "build",
                "--release",
                "--target",
                targetMap[this.arch],
            ],
            env: {
                ROOT_MANIFEST_DIR: Deno.cwd() + "/",
                MOUNT_IMG_PATH: "mount.img",
                HEAP_SIZE: "0x0180_0000",
                BOARD: "qemu",
                RUSTFLAGS: this.rustflags
            },
        });        
        const code = await buildProc.spawn().status;
        if(!code.success) {
            console.error("Failed to build the kernel");
            Deno.exit(1);
        }
    }

    async convertBin() {
        const objcopyProc = new Deno.Command("rust-objcopy", {
            args: [
                `--binary-architecture=${this.arch}`,
                this.elfPath,
                "--strip-all",
                "-O",
                "binary",
                this.binPath
            ]
        });
        await objcopyProc.spawn().status;
    }
}
