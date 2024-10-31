import { Command, CommandOptions } from "https://deno.land/x/cliffy@v1.0.0-rc.4/command/mod.ts";
import { globalArgType } from "./cli-types.ts";

const targetMap: Record<string, string> = {
    "riscv64": 'riscv64gc-unknown-none-elf',
    "x86_64": 'x86_64-unknown-none',
    "aarch64": 'aarch64-unknown-none-softfloat',
    "loongarch64": 'loongarch64-unknown-none'
};

/**
 * Get the path of the kernel elf file.
 * @param arch the architecture
 * @returns path to the file
 */
export function getKernelElf(arch: string): string {
    return `${Deno.cwd()}/target/${targetMap[arch]}/release/kernel`;
}

/**
 * Get the path of the kernel Binary file.
 * @param arch the architecture
 * @returns path to the file
 */
export function getKernelBin(arch: string): string {
    return `${getKernelElf(arch)}.bin`;
}

export const cargoBuild = async function(options: CommandOptions<globalArgType>) {

    const rustflags = [
        "-Cforce-frame-pointers=yes",
        "-Clink-arg=-no-pie",
        "-Ztls-model=local-exec",
        `--cfg=root_fs="ext4_rs"`
    ];

    const buildProc = new Deno.Command("cargo", {
        args: [
            "build",
            "--release",
            "--target",
            targetMap[options.arch],
        ],
        env: {
            ROOT_MANIFEST_DIR: Deno.cwd() + "/",
            MOUNT_IMG_PATH: "mount.img",
            HEAP_SIZE: "0x0180_0000",
            RUSTFLAGS: (Deno.env.get("RUSTFLAGS") || "") + ' ' + rustflags.join(' ')
        },
    });        
    const code = await buildProc.spawn().status;
    if(!code.success) {
        console.error("Failed to build the kernel");
        Deno.exit(1);
    }

    const objcopyProc = new Deno.Command("rust-objcopy", {
        args: [
            `--binary-architecture=${options.arch}`,
            getKernelElf(options.arch),
            "--strip-all",
            "-O",
            "binary",
            getKernelBin(options.arch)
        ]
    });
    await objcopyProc.spawn().status;

    console.log("options", options);
    console.log("code: ", code);
}

export const cliCommand = new Command<globalArgType>()
    .description("Build Rust Kernel")
    .action(cargoBuild);
