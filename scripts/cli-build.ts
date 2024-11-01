import { Command, CommandOptions } from "https://deno.land/x/cliffy@v1.0.0-rc.4/command/mod.ts";
import { globalArgType } from "./cli-types.ts";
import { KernelBuilder } from "./kernel.ts";


export const cargoBuild = async function(options: CommandOptions<globalArgType>) {

    const builder = new KernelBuilder(options.arch);
    await builder.buildElf();
    await builder.convertBin();

    console.log("options", options);
}

export const cliCommand = new Command<globalArgType>()
    .description("Build Rust Kernel")
    .action(cargoBuild);
