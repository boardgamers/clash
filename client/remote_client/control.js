import {run} from "./run";

export class Control extends EventEmitter {
    constructor() {
        super();

        this.addListener("state", (data) => {
            this.state = data;
        });
    }

    get_and_reset_state() {
        const state = this.state;
        this.state = null;
        return state;
    }

    execute_action(action) {
        this.emit("move", action);
    }
}

export function get_control() {
    return window.clash_control;
}



