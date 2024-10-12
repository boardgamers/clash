import init, {set_wasm} from "../../dist/remote_client";

function dynamicallyLoadScript(url, onload) {
    const script = document.createElement("script");
    script.onload = onload;
    script.src = url;

    document.head.appendChild(script);
}

export async function run({selector, control}) {
        let wbg = await init();
         miniquad_add_plugin({
             register_plugin: (a) => {
                 console.log("register_plugin", a);
                 return (a.wbg = wbg)
             },
             on_init: () => {
                 console.log("on_init", wasm_exports);
                 window.clash_control.send_ready()
                 return set_wasm(wasm_exports)
             },
             version: "0.0.1",
             name: "wbg",
         });
         const src = document.body.getElementsByTagName("script")[1].src;
         control.assets_url = src.replace("client.js", "assets/");
         const url = src.replace("client.js", "client.wasm");
         console.log("Loading wasm from", url);
         await load(url);
         console.log("Loaded wasm");
}
