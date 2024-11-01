import { Command, CommandOptions } from "https://deno.land/x/cliffy@v1.0.0-rc.4/command/mod.ts";
import { globalArgType } from "./cli-types.ts";
import { KernelBuilder } from "./kernel.ts";

class QemuRunner {
    arch: string;
    bus: string = "device";
    builder: KernelBuilder;

    constructor(options: CommandOptions<globalArgType>, builder: KernelBuilder) {
        this.arch = options.arch;
        this.builder = builder;
        if(this.arch == "x86_64" || this.arch == "loongarch64")
            this.bus = "pci";
    }

    getQemuArchExec(): string[] {
        return {
            x86_64: [
                "-machine",
                "q35",
                "-kernel",
                this.builder.elfPath,
                "-cpu",
                "IvyBridge-v2"
            ],
            riscv64: [
                "-machine",
                "virt",
                "-kernel",
                this.builder.binPath
            ],
            aarch64: [
                "-cpu", 
                "cortex-a72",
                "-machine",
                "virt",
                "-kernel",
                this.builder.binPath
            ],
            loongarch64: [
                "-kernel",
                this.builder.elfPath
            ]
        }[this.arch] ?? [];
    }

    async run() {
        const qemuCommand = new Deno.Command(`qemu-system-${this.arch}`, {
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
                "virtio-blk-device,drive=x0"
            ]
        });
        await qemuCommand.spawn().status;
    }
}

async function runQemu(options: CommandOptions<globalArgType>) {
    const builder = new KernelBuilder(options.arch);
    await builder.buildElf();
    await builder.convertBin();

    const runner = new QemuRunner(options, builder);
    await runner.run();
}

export const cliCommand = new Command<globalArgType>()
    .description("Run kernel in the qemu")
    .action(runQemu);
