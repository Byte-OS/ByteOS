import { EnumType } from "https://deno.land/x/cliffy@v1.0.0-rc.4/command/mod.ts";

export const logLevelEnum = new EnumType(["debug", "info", "warn", "error"]);
export const archEnum = new EnumType(['x86_64', "aarch64", "riscv64", "loongarch64"]);

export type globalArgType = {
    logLevel: typeof logLevelEnum,
    platform: typeof String
};
