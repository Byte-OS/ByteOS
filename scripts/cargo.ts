import { platform } from "./platform.ts";

export class Cargo {
    configs: Map<string, string | undefined> = new Map();
    rustflags: Array<string> = new Array(0);
    env: Map<string, string> = new Map();
    features = [];
    release = true;

    constructor() {
        this.rustflags.push("-Clink-arg=-no-pie");
        this.rustflags.push("-Cforce-frame-pointers=yes");

        this.configs.set("root_fs", "ext4_rs");
        this.configs.set("board", "qemu");
        this.configs.set("driver", "kramdisk");

        this.env.set("ROOT_MANIFEST_DIR", Deno.cwd() + "/")
        this.env.set("MOUNT_IMG_PATH", "mount.img")
        this.env.set("HEAP_SIZE", "0x0180_0000")
        this.env.set("BOARD", "qemu")
    }

    async build() {
        const rustflags = this.rustflags;
        const args = ["build", "--target", platform.target];

        if (this.release) args.push("--release");

        this.configs.forEach((value, key) => {
            if (value == undefined) {
                rustflags.push(`--cfg=${key}`);
            } else {
                rustflags.push(`--cfg=${key}="${value}"`)
            }
        })

        const buildProc = new Deno.Command("cargo", {
            args,
            env: {
                ...Object.fromEntries(this.env),
                RUSTFLAGS: (Deno.env.get('rustflags') || "") + rustflags.join(" ")
            },
        });
        const code = await buildProc.spawn().status;
        if (!code.success) {
            console.error("Failed to build the kernel");
            Deno.exit(1);
        }
    }

    getTargetPath() {
        const mode = this.release ? "release" : "debug";
        return `${Deno.cwd()}/target/${platform.target}/${mode}/kernel`;
    }

    getBinPath() {
        return `${this.getTargetPath()}.bin`;
    }

    async convertBin() {
        const objcopyProc = new Deno.Command("rust-objcopy", {
            args: [
                `--binary-architecture=${platform.arch}`,
                this.getTargetPath(),
                "--strip-all",
                "-O",
                "binary",
                this.getBinPath()
            ]
        });
        await objcopyProc.spawn().status;
    }
}
