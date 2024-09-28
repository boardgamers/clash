import {EventEmitter} from "events";

export class Control extends EventEmitter {
    constructor() {
        super();
        this.state = null;
        this.player_index = null;

        this.addListener("state", (data) => {
            this.state = data;
        });
        this.addListener("state:updated", () => {
            this.emit("fetchState");
        });
        this.addListener("player", (player) => {
            this.player_index = player.index;
        });
    }

    receive_state() {
        const state = this.state;
        this.state = null;
        return state;
    }

    receive_player_index() {
        const index = this.player_index;
        this.player_index = null;
        return index;
    }

    send_move(move) {
        this.emit("move", move);
    }

    send_ready() {
        this.emit("ready");
    }
}

export function get_control() {
    return window.clash_control;
}



