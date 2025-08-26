import {EventEmitter} from "events";

export class Control extends EventEmitter {
    constructor() {
        super();
        this.state = null;
        this.player_index = null;
        this.preferences = null;
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
        this.addListener("preferences", (preferences) => {
            this.preferences = preferences;
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
    
    receive_preferences() {
        const prefs = this.preferences;
        this.preferences = null;
        return prefs;
    }

    send_move(move) {
        this.emit("move", move);
    }

    send_ready() {
        console.log("Sending ready");
        this.emit("ready");
    }

    get canvas_size() {
        return document.getElementById("glcanvas").getBoundingClientRect();
    }

    get assets_url() {
        return this._assets_url;
    }

    set assets_url(value) {
        this._assets_url = value;
    }
}




