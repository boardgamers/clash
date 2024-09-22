import {Control} from "./control";
import {run} from "./run";

function launch(selector) {
    const control = new Control();

    window.clash_control = control;

    window.clash = {
        launch(selector) {
            run();

            return control;
        },
    };

    return control;
}

console.log("set clash");
window.clash = {launch};

export default launch;

