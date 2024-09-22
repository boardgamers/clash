use server::game::Game;

use macroquad::prelude::next_frame;

use client::client::{init, render_and_update, Features, GameSyncRequest, GameSyncResult};
use server::action::Action;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen(module = "/remote_client/src/control.js")]
extern "C" {
    type Control;

    fn get_control() -> Control;

    #[wasm_bindgen(method)]
    fn get_and_reset_state(this: &Control) -> JsValue;

    #[wasm_bindgen(method)]
    fn execute_action(this: &Control, action: &str);
}

#[derive(Debug)]
pub enum State {
    None,
    WaitingForUpdate,
    GameUpdated(String),
}

#[wasm_bindgen]
pub struct Client {
    control: Control,
    state: State,
    game: Option<Game>,
}

#[macroquad::main("Clash")]
async fn main() {
    Client::start().await;
}

#[wasm_bindgen]
impl Client {
    pub async fn start() {
        let mut client = Client {
            control: get_control(),
            state: State::WaitingForUpdate,
            game: None,
        };
        client.run().await;
    }

    pub async fn run(&mut self) {
        let mut client_state = init().await;
        let features = Features {
            import_export: false,
        };

        loop {
            let sync_result = &mut self.update_game();

            if let Some(game) = &self.game {
                let message = render_and_update(game, &mut client_state, sync_result, &features);

                if let GameSyncRequest::ExecuteAction(a) = message {
                    let _ = &mut self.execute_action(&a);
                };
            }
            next_frame().await;
        }
    }

    fn execute_action(&mut self, a: &Action) {
        if !matches!(self.state, State::None) {
            log(format!("cannot execute action - state is {:?}", self.state).as_str());
            return;
        }

        self.control
            .execute_action(serde_json::to_string(&a).unwrap().as_str());
        self.state = State::WaitingForUpdate;
    }

    fn update_game(&mut self) -> GameSyncResult {
        let sync_result = match &self.state {
            State::None => GameSyncResult::None,
            State::WaitingForUpdate => GameSyncResult::WaitingForUpdate,
            State::GameUpdated(_) => GameSyncResult::Update,
        };

        if let State::GameUpdated(s) = &self.state {
            let game = Game::from_data(serde_json::from_str(&s).unwrap());
            self.game = Some(game);
            self.state = State::None;
        }
        sync_result
    }
}
