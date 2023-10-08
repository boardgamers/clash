use crate::game_loop;
use crate::game_sync::{ClientFeatures, GameSyncRequest, GameSyncResult};
use macroquad::prelude::next_frame;
use server::game::Game;
use std::sync::{Arc, Mutex};

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

static mut state: Option<Arc<Mutex<RemoteClientState>>> = None;

pub struct RemoteClientState {
    game: Option<Game>,
    sync_result: GameSyncResult,
    control: Box<dyn RemoteClientControl>,
}

//todo js object
pub trait RemoteClientControl {
    fn execute(&self, action: &str);
}

#[wasm_bindgen]
pub struct RemoteClient {
    state: Arc<Mutex<RemoteClientState>>,
}

#[wasm_bindgen(start)]
pub async unsafe fn start() {
    RemoteClient::start().await;
}

pub unsafe fn launch(control: Box<dyn RemoteClientControl>) -> RemoteClient {
    let arc = &state.expect("state not initialized");
    let mut s: RemoteClientState = *arc.lock().expect("state lock failed");

    s.control = control;
    return RemoteClient {
        state: Arc::clone(arc),
    };
}

#[wasm_bindgen]
impl RemoteClient {
    pub async unsafe fn start() {
        let client = RemoteClient {
            state: Arc::new(Mutex::new(RemoteClientState {
                // game: Game::new(2, "a".repeat(32), false),
                sync_result: GameSyncResult::None,
            })),
        };
        remote_client = Some(client);
        client.run().await;
    }

    pub fn state(&self) {
        //todo new game state received
    }

    pub async fn run(&self) {
        let mut state = game_loop::init().await;
        let features = ClientFeatures {
            import_export: false,
        };

        loop {
            {
                let mut sync = self.state.lock().unwrap();
                let game = &sync.game;
                let message =
                    game_loop::render_and_update(game, &mut state, &sync.sync_result, &features);
                if matches!(sync.sync_result, GameSyncResult::Update) {
                    // only notify once to reset active dialog
                    sync.sync_result = GameSyncResult::None;
                }

                if let GameSyncRequest::ExecuteAction(a) = message {
                    self.control
                        .execute(serde_json::to_string(&a).unwrap().as_str());
                    sync.sync_result = GameSyncResult::WaitingForUpdate;
                };
            }
            next_frame().await;
        }
    }
}
