#![deny(clippy::all, clippy::pedantic)]
#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions
)]

mod action_buttons;
mod advance_ui;
mod assets;
mod cards_ui;
mod city_ui;
pub mod client;
pub mod client_state;
mod collect_ui;
mod combat_ui;
mod construct_ui;
mod dialog_ui;
mod happiness_ui;
mod hex_ui;
mod influence_ui;
mod layout_ui;
mod log_ui;
mod map_ui;
mod move_ui;
mod payment_ui;
mod player_ui;
mod recruit_unit_ui;
mod render_context;
mod resource_ui;
mod select_ui;
mod status_phase_ui;
mod tooltip;
mod unit_ui;
