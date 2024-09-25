import {Control} from "./control";
import {run} from "./run";

window.clash = {
    launch(selector) {
        const control = new Control();
        window.clash_control = control;

        run();

        return control;
    },
};


