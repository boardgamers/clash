import { Control } from "./control";
import { run } from "./run";

const viewerScriptUrl = document.currentScript?.src;
window.clash = {
  launch(selector) {
    const control = new Control();
    if (viewerScriptUrl)
      control.assets_url = new URL("assets/", viewerScriptUrl).href;
    window.clash_control = control;

    run({ selector, control });

    return control;
  },
};
