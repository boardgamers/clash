import init, {set_wasm} from "./remote_client.js";

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

export async function run(selector) {
    // const canvasElement = document.createElement("canvas");
    // canvasElement.id = "glcanvas";
    // document.querySelector(selector).appendChild(canvasElement);
    document.getElementById("glcanvas").removeAttribute("hidden");
    document.getElementById("glcanvas").focus();
    await impl_run();
}
