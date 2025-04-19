import { parse } from "jsr:@std/yaml";

export const platform = {
    package: "kernel",
    arch: "",
    target: "",
    configs: []
};


export function initPlatform(platformStr: string) {
    const data: any = parse(
        new TextDecoder("utf-8").decode(Deno.readFileSync("byteos.yaml")),
    );
    const config = data["bin"][platformStr];

    platform.target = config["target"];
    platform.arch = platform.target.substring(0, platform.target.indexOf("-"));
    platform.configs = config['configs']
}
