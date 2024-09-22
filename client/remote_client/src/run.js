import init, {set_wasm} from "../../dist/remote_client";

async function impl_run() {
    let wbg = await init();
    miniquad_add_plugin({
        register_plugin: (a) => (a.wbg = wbg),
        on_init: () => set_wasm(wasm_exports),
        version: "0.0.1",
        name: "wbg",
    });
    load("./remote_client_bg.wasm");
}

export async function run() {
    document.getElementById("glcanvas").removeAttribute("hidden");
    document.getElementById("glcanvas").focus();
    await impl_run();
}
