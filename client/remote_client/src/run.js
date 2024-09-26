import init, {set_wasm} from "../../dist/remote_client";

export async function run() {
    let wbg = await init();
    miniquad_add_plugin({
        register_plugin: (a) => (a.wbg = wbg),
        on_init: () => set_wasm(wasm_exports),
        version: "0.0.1",
        name: "wbg",
    });
    const url = document.head.getElementsByTagName("script")[0].src.replace("client.js", "client.wasm");
    console.log("Loading wasm from", url);
    await load(url);
}
