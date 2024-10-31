import { Command, CommandOptions } from "https://deno.land/x/cliffy@v1.0.0-rc.4/command/mod.ts";
import { globalArgType } from "./cli-types.ts";
import { cargoBuild, getKernelBin, getKernelElf } from "./cli-build.ts";

async function runQemu(options: CommandOptions<globalArgType>) {
    await cargoBuild(options);

    const qemuExecArch = {
        x86_64: [
            "-machine",
            "q35",
            "-kernel",
            getKernelElf(options.arch),
            "-cpu",
            "IvyBridge-v2"
        ],
        riscv64: [
            "-machine",
            "virt",
            "-kernel",
            getKernelBin(options.arch)
        ],
        aarch64: [
            "-cpu", 
            "cortex-a72",
            "-machine",
            "virt",
            "-kernel",
            getKernelBin(options.arch)
        ],
        loongarch64: [
            "-kernel",
            getKernelElf(options.arch)
        ]
    };

    const qemuCommand = new Deno.Command(`qemu-system-${options.arch}`, {
        args: [
            ...qemuExecArch[options.arch],
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

export const cliCommand = new Command<globalArgType>()
    .description("Run kernel in the qemu")
    .action(runQemu);
