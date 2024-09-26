import init, {set_wasm} from "../../dist/remote_client";

async function impl_run() {
    let wbg = await init();
    miniquad_add_plugin({
        register_plugin: (a) => (a.wbg = wbg),
        on_init: () => set_wasm(wasm_exports),
        version: "0.0.1",
        name: "wbg",
    });
    load("http://localhost:4000/client.wasm"); //todo
}

export async function run() {
    await impl_run();
}
