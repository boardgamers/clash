use itertools::Itertools;
use macroquad::color::BLACK;
use macroquad::math::vec2;
use macroquad::prelude::*;
use macroquad::text::draw_text;
use macroquad::ui::root_ui;

use server::action::Action;
use server::combat::Combat;
use server::game::{Game, GameState};
use server::playing_actions::PlayingAction;
use server::resource_pile::ResourcePile;

use crate::ui_state::{can_play_action, State, StateUpdate};

pub fn show_globals(game: &Game) {
    draw_text(&format!("Age {}", game.age), 1200., 20., 20., BLACK);
    draw_text(&format!("Round {}", game.round), 1200., 50., 20., BLACK);
    draw_text(
        &format!("Player {}", game.active_player()),
        1200.,
        80.,
        20.,
        BLACK,
    );
    let status = match game.state {
        GameState::Playing => String::from("Play Actions"),
        GameState::StatusPhase(ref p) => format!("Status Phase: {p:?}"),
        GameState::Movement { .. } => String::from("Movement"),
        GameState::CulturalInfluenceResolution(_) => String::from("Cultural Influence Resolution"),
        GameState::Combat(Combat {
            round, ref phase, ..
        }) => {
            format!("Combat Round {} Phase {:?}", round, *phase)
        }
        GameState::PlaceSettler { .. } => String::from("Place Settler"),
        GameState::Finished => String::from("Finished"),
    };
    draw_text(&status, 1200., 110., 20., BLACK);
    draw_text(
        &format!("Actions Left {}", game.actions_left),
        1200.,
        140.,
        20.,
        BLACK,
    );
    let rolls = game
        .dice_roll_log
        .iter()
        .map(std::string::ToString::to_string)
        .join(", ");
    draw_text(&format!("Last Dice Rolls {rolls}"), 1200., 600., 20., BLACK);
}

pub fn show_wonders(game: &Game, player_index: usize) {
    let player = game.get_player(player_index);
    for (i, name) in player.wonders.iter().enumerate() {
        draw_text(
            &format!("Wonder {name}"),
            1200.,
            600. + i as f32 * 30.0,
            20.,
            BLACK,
        );
    }
    for (i, card) in player.wonder_cards.iter().enumerate() {
        let req = match card.required_advances[..] {
            [] => String::from("no advances"),
            _ => card.required_advances.join(", "),
        };
        draw_text(
            &format!(
                "Wonder Card {} cost {} requires {}",
                &card.name, card.cost, req
            ),
            1200.,
            800. + i as f32 * 30.0,
            20.,
            BLACK,
        );
    }
}

pub fn show_resources(game: &Game, player_index: usize) {
    let player = game.get_player(player_index);
    let r: &ResourcePile = &player.resources;

    let mut i: f32 = 0.;
    let mut res = |label: String| {
        draw_text(&label, 1200., 200. + i, 20., BLACK);
        i += 30.;
    };

    res(format!("Food {}", r.food));
    res(format!("Wood {}", r.wood));
    res(format!("Ore {}", r.ore));
    res(format!("Ideas {}", r.ideas));
    res(format!("Gold {}", r.gold));
    res(format!("Mood {}", r.mood_tokens));
    res(format!("Culture {}", r.culture_tokens));
}

pub fn show_global_controls(game: &Game, state: &State) -> StateUpdate {
    if game.can_undo() && root_ui().button(vec2(1200., 410.), "Undo") {
        return StateUpdate::Execute(Action::Undo);
    }
    if game.can_redo() && root_ui().button(vec2(1250., 410.), "Redo") {
        return StateUpdate::Execute(Action::Redo);
    }
    match game.state {
        GameState::Playing if root_ui().button(vec2(1200., 540.), "End Turn") => {
            let left = game.actions_left;
            StateUpdate::execute_with_warning(
                Action::Playing(PlayingAction::EndTurn),
                if left > 0 {
                    vec![(format!("{left} actions left"))]
                } else {
                    vec![]
                },
            )
        }
        GameState::Playing
            if !state.has_dialog()
                && can_play_action(game)
                && root_ui().button(vec2(1200., 510.), "Move Units") =>
        {
            StateUpdate::execute(Action::Playing(PlayingAction::MoveUnits))
        }
        _ => StateUpdate::None,
    }
}

pub fn player_color(player_index: usize) -> Color {
    match player_index {
        0 => YELLOW,
        1 => PINK,
        _ => panic!("unexpected player index"),
    }
}
