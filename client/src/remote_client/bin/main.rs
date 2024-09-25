use server::game::Game;

use macroquad::prelude::next_frame;

use client::client::{init, render_and_update, Features, GameSyncRequest, GameSyncResult};
use server::action::Action;
use server::map::Terrain;
use server::position::Position;
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
    fn receive_state(this: &Control) -> JsValue;

    #[wasm_bindgen(method)]
    fn receive_player(this: &Control) -> JsValue;

    #[wasm_bindgen(method)]
    fn send_move(this: &Control, action: &str);

    #[wasm_bindgen(method)]
    fn ready(this: &Control);
}

#[derive(Debug)]
pub enum State {
    None,
    WaitingForUpdate,
    GameUpdated(JsValue),
}

#[wasm_bindgen]
pub struct Client {
    control: Control,
    state: State,
    player: Option<usize>,
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
            player: None,
            game: None,
        };
        client.run().await;
    }

    pub async fn run(&mut self) {
        let features = Features {
            import_export: false,
            local_assets: false,
        };

        let mut client_state = init(&features).await;

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
            .send_move(serde_json::to_string(&a).unwrap().as_str());
        self.state = State::WaitingForUpdate;
    }

    fn update_game(&mut self) -> GameSyncResult {
        let p = self.control.receive_player().as_f64();
        if let Some(p) = p {
            log(format!("received player: {}", p).as_str());
            self.player = Some(p as usize);
        }
        let s = self.control.receive_state();
        if s.is_object() {
            log("received state");
            self.state = State::GameUpdated(s);
            self.control.ready();
        }

        let sync_result = match &self.state {
            State::None => GameSyncResult::None,
            State::WaitingForUpdate => GameSyncResult::WaitingForUpdate,
            State::GameUpdated(_) => GameSyncResult::Update,
        };

        if let State::GameUpdated(s) = &self.state {
            let game = Game::from_data(
                serde_wasm_bindgen::from_value(s.into()).expect("game should be of type game data"),
            );
            self.game = Some(game);
            self.state = State::None;
        }
        sync_result
    }
}
