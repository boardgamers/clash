import {EventEmitter} from "events";

export class Control extends EventEmitter {
    constructor() {
        super();
        this.state = null;
        this.player = null;

        this.addListener("state", (data) => {
            this.state = data;
        });
        this.addListener("state:updated", () => {
            this.emit("fetchState");
        });
        this.addListener("player", (index) => {
            this.player = index;
        });
    }

    receive_state() {
        const state = this.state;
        this.state = null;
        return state;
    }

    receive_player() {
        return this.player;
    }

    send_move(move) {
        this.emit("move", move);
    }

    ready() {
        this.emit("ready");
    }
}

export function get_control() {
    return window.clash_control;
}



