import { Command, CommandOptions } from "https://deno.land/x/cliffy@v1.0.0-rc.4/command/mod.ts";
import { globalArgType } from "./cli-types.ts";
import { Cargo } from "./cargo.ts";


export const cargoBuild = async function (options: CommandOptions<globalArgType>) {

    const cargo = new Cargo();
    await cargo.build();

    console.log("options", options);
}

export const cliCommand = new Command<globalArgType>()
    .description("Build Rust Kernel")
    .action(cargoBuild);
