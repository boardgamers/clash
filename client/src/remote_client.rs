#![allow(clippy::missing_safety_doc)]
#![allow(clippy::missing_panics_doc)]

use macroquad::prelude::next_frame;
use std::sync::Mutex;

use crate::client;
use crate::client::{Features, GameSyncRequest, GameSyncResult};
use server::action::Action;
use server::game::Game;
use wasm_bindgen::prelude::*;

// todo not tested yet

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

lazy_static! {
    static ref STATE: Mutex<State> = Mutex::new(State::None);
    static ref EMITTER: Mutex<Option<Box<dyn Emitter>>> = Mutex::new(None);
}

#[derive(Debug)]
pub enum State {
    None,
    WaitingForUpdate,
    GameUpdated(String),
}

//todo js object
pub trait Emitter: Send {
    fn execute(&self, action: &str);
}

#[wasm_bindgen]
pub struct Control {}

#[wasm_bindgen]
pub struct Client {
    game: Option<Game>,
}

// #[wasm_bindgen(start)]
#[wasm_bindgen]
pub async fn start() {
    Client::start().await;
}

#[must_use]
// #[wasm_bindgen]
pub fn launch(emitter: Box<dyn Emitter>) -> Box<Control> {
    EMITTER
        .lock()
        .expect("emitter lock failed")
        .replace(emitter);
    Box::new(Control {})
}

impl Control {
    pub fn state(&self, data: String) {
        let mut state = STATE.lock().expect("state lock failed");
        *state = State::GameUpdated(data);
    }
}

#[wasm_bindgen]
impl Client {
    pub async fn start() {
        let mut client = Client { game: None };
        client.run().await;
    }

    pub async fn run(&mut self) {
        let mut client_state = client::init().await;
        let features = Features {
            import_export: false,
        };

        loop {
            let sync_result = &mut self.update_game();

            if let Some(game) = &self.game {
                let message =
                    client::render_and_update(game, &mut client_state, sync_result, &features);

                if let GameSyncRequest::ExecuteAction(a) = message {
                    Self::execute_action(&a);
                };
            }
            next_frame().await;
        }
    }

    fn execute_action(a: &Action) {
        let mut state = STATE.lock().expect("state lock failed");

        if !matches!(*state, State::None) {
            log(format!("cannot execute action - state is {:?}", *state).as_str());
            return;
        }

        (*EMITTER.lock().expect("emitter lock failed"))
            .as_ref()
            .expect("emitter not initialized")
            .execute(serde_json::to_string(&a).unwrap().as_str());
        *state = State::WaitingForUpdate;
    }

    fn update_game(&mut self) -> GameSyncResult {
        let mut state = STATE.lock().expect("state lock failed");

        let sync_result = match &*state {
            State::None => GameSyncResult::None,
            State::WaitingForUpdate => GameSyncResult::WaitingForUpdate,
            State::GameUpdated(_) => GameSyncResult::Update,
        };

        if let State::GameUpdated(s) = &*state {
            let game = Game::from_data(serde_json::from_str(s).unwrap());
            self.game = Some(game);
            *state = State::None;
        }
        sync_result
    }
}
