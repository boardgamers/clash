import init, {set_wasm} from "../../dist/remote_client";

function dynamicallyLoadScript(url, onload) {
    const script = document.createElement("script");
    script.onload = onload;
    script.src = url;

    document.head.appendChild(script);
}

export async function run(selector, control) {
    const root = document.querySelector(selector);
    const canvas = document.createElement("canvas");
    canvas.setAttribute("id", "glcanvas");
    canvas.setAttribute("style", ` 
                margin: 0px;
                padding: 0px;
                width: 100%;
                height: 100%;
                overflow: hidden;
                position: absolute;
                z-index: 0;
    `);
    root.appendChild(canvas);

    dynamicallyLoadScript("https://not-fl3.github.io/miniquad-samples/mq_js_bundle.js", async () => {
        let wbg = await init();
        miniquad_add_plugin({
            register_plugin: (a) => (a.wbg = wbg),
            on_init: () => set_wasm(wasm_exports),
            version: "0.0.1",
            name: "wbg",
        });
        const src = document.head.getElementsByTagName("script")[0].src;
        control.assets_url = src.replace("client.js", "assets/");
        const url = src.replace("client.js", "client.wasm");
        console.log("Loading wasm from", url);
        await load(url);
    });
}
