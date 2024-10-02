import {EventEmitter} from "events";

export class Control extends EventEmitter {
    constructor() {
        super();
        this.state = null;
        this.player_index = null;
        this._assets_url = null;

        this.addListener("state", (data) => {
            this.state = data;
        });
        // When we receive log slices, when executing a move
        this.addListener("gamelog", (logData) => {
            // Ignore the log data and tell the backend we want the new state
            this.emit("fetchState");
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

    get assets_url() {
        return this._assets_url;
    }

    set assets_url(value) {
        this._assets_url = value;
    }
}

export function get_control() {
    return window.clash_control;
}



