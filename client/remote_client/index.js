import {Control} from "./control";
import {run} from "./run";

function launch(selector) {
    const control = new Control();

    window.clash_control = control;

    window.clash = {
        launch(selector) {
            run(selector);

            return control;
        },
    };

    return control;
}

window.clash = {launch};

export default launch;

