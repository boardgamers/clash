use crate::game_loop;
use crate::game_sync::{ClientFeatures, GameSyncRequest, GameSyncResult};
use macroquad::prelude::next_frame;
use server::game::Game;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

static mut global_state: Option<Arc<Mutex<RemoteClientState>>> = None;

pub struct RemoteClientState {
    game: Option<Game>,
    sync_result: GameSyncResult,
    control: Option<Box<dyn RemoteClientControl>>,
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
    let arc = &global_state.expect("state not initialized");
    let mut s: RemoteClientState = *arc.lock().expect("state lock failed");

    s.control = Some(control);
    return RemoteClient {
        state: Arc::clone(arc),
    };
}

#[wasm_bindgen]
impl RemoteClient {
    pub async unsafe fn start() {
        let client = RemoteClient {
            state: Arc::new(Mutex::new(RemoteClientState {
                sync_result: GameSyncResult::None,
                game: None,
                control: None,
            })),
        };
        global_state = Some(Arc::clone(&client.state));
        client.run().await;
    }

    pub fn state(&self) {
        //todo new game state received
    }

    pub async fn run(&self) {
        let mut client_state = game_loop::init().await;
        let features = ClientFeatures {
            import_export: false,
        };

        loop {
            {
                let mut state = self.state.lock().unwrap();
                if let Some(game) = &state.game {
                    let message = game_loop::render_and_update(
                        game,
                        &mut client_state,
                        &state.sync_result,
                        &features,
                    );
                    if matches!(state.sync_result, GameSyncResult::Update) {
                        // only notify once to reset active dialog
                        state.sync_result = GameSyncResult::None;
                    }

                    if let GameSyncRequest::ExecuteAction(a) = message {
                        state
                            .control
                            .as_ref()
                            .expect("control not initialized")
                            .execute(serde_json::to_string(&a).unwrap().as_str());
                        state.sync_result = GameSyncResult::WaitingForUpdate;
                    };
                }
            }
            next_frame().await;
        }
    }
}
