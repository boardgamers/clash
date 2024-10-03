use server::game::Game;

use macroquad::prelude::next_frame;

extern crate console_error_panic_hook;
use client::client::{init, render_and_update, Features, GameSyncRequest, GameSyncResult};
use client::client_state::State;
use server::action::Action;
use std::panic;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen(module = "/js/src/control.js")]
extern "C" {
    type Control;

    fn get_control() -> Control;

    #[wasm_bindgen(method)]
    fn receive_state(this: &Control) -> JsValue;

    #[wasm_bindgen(method)]
    fn receive_player_index(this: &Control) -> JsValue;

    #[wasm_bindgen(method)]
    fn send_move(this: &Control, action: JsValue);

    #[wasm_bindgen(method)]
    fn send_ready(this: &Control);

    #[wasm_bindgen(method, getter)]
    fn assets_url(this: &Control) -> String;
}

enum SyncState {
    New,
    WaitingForUpdate,
    Playing,
}

#[wasm_bindgen]
struct RemoteClient {
    control: Control,
    state: State,
    sync_state: SyncState,
    game: Option<Game>,
    features: Features,
}

#[macroquad::main("Clash")]
async fn main() {
    RemoteClient::start().await;
}

#[wasm_bindgen]
impl RemoteClient {
    pub async fn start() {
        log("starting client");
        panic::set_hook(Box::new(console_error_panic_hook::hook));
        let control = get_control();
        let features = Features {
            import_export: false,
            assets_url: control.assets_url(),
        };
        let state = init(&features).await;

        let mut client = RemoteClient {
            control,
            state,
            sync_state: SyncState::New,
            game: None,
            features,
        };
        client.run().await;
    }

    pub async fn run(&mut self) {
        log("running client");
        loop {
            let p = self.control.receive_player_index().as_f64();
            if let Some(p) = p {
                log(&format!("received player index: {}", p));
                self.state.control_player = Some(p as usize);
                self.state.show_player = p as usize;
            }

            let sync_result = self.update_state();

            if let Some(game) = &self.game {
                let message =
                    render_and_update(game, &mut self.state, &sync_result, &self.features);

                if let GameSyncRequest::ExecuteAction(a) = message {
                    let _ = &mut self.execute_action(&a);
                };
            }
            // async_std::task::sleep(std::time::Duration::from_millis(100)).await;

            next_frame().await;
        }
    }

    fn update_state(&mut self) -> GameSyncResult {
        let s = self.control.receive_state();
        if s.is_object() {
            log("received state");
            let g = Game::from_data(
                serde_wasm_bindgen::from_value(s).expect("game should be of type game data"),
            );
            self.state.show_player = g.active_player();
            self.game = Some(g);
            self.sync_state = SyncState::Playing;
            self.control.send_ready();
            return GameSyncResult::Update;
        }
        match &self.sync_state {
            SyncState::New => GameSyncResult::None,
            SyncState::WaitingForUpdate => GameSyncResult::WaitingForUpdate,
            SyncState::Playing => GameSyncResult::None,
        }
    }

    fn execute_action(&mut self, a: &Action) {
        if let SyncState::Playing = &self.sync_state {
            self.control
                .send_move(serde_wasm_bindgen::to_value(&a).unwrap());
            self.sync_state = SyncState::WaitingForUpdate;
        } else {
            log("cannot execute action");
        }
    }
}
