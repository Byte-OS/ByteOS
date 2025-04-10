import { Command } from "https://deno.land/x/cliffy@v1.0.0-rc.4/command/mod.ts";
import { globalArgType } from "./cli-types.ts";
import { platform } from "./platform.ts";
import { Cargo } from "./cargo.ts";

class QemuRunner extends Cargo {
    bus: string = "device";

    constructor() {
        super()
        if (platform.arch == "x86_64" || platform.arch == "loongarch64") {
            this.bus = "pci";
        }
    }

    getQemuArchExec(): string[] {
        return {
            x86_64: [
                "-machine",
                "q35",
                "-cpu",
                "IvyBridge-v2",
                "-kernel",
                this.getTargetPath(),
            ],
            riscv64: [
                "-machine",
                "virt",
                "-kernel",
                this.getBinPath()
            ],
            aarch64: [
                "-cpu",
                "cortex-a72",
                "-machine",
                "virt",
                "-kernel",
                this.getBinPath()
            ],
            loongarch64: [
                "-kernel",
                this.getTargetPath()
            ]
        }[platform.arch] ?? [];
    }

    async run() {
        const qemuCommand = new Deno.Command(`qemu-system-${platform.arch}`, {
            args: [
                ...this.getQemuArchExec(),
                "-m",
                "1G",
                "-nographic",
                "-smp",
                "1",
                "-D",
                "qemu.log",
                "-d",
                "in_asm,int,pcall,cpu_reset,guest_errors",

                "-drive",
                "file=mount.img,if=none,format=raw,id=x0",
                "-device",
                `virtio-blk-${this.bus},drive=x0`
            ]
        });
        await qemuCommand.spawn().status;
    }
}

async function runQemu() {
    const cargo = new Cargo();
    await cargo.build();

    const runner = new QemuRunner();
    await runner.run();
}

export const cliCommand = new Command<globalArgType>()
    .description("Run kernel in the qemu")
    .action(runQemu);
