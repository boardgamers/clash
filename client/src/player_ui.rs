use macroquad::color::BLACK;
use macroquad::math::vec2;
use macroquad::prelude::*;
use macroquad::text::draw_text;
use macroquad::ui::root_ui;
use server::action::Action;
use server::game::{Game, GameState};
use server::playing_actions::PlayingAction;
use server::resource_pile::ResourcePile;
use server::unit::MovementAction;

use crate::ui_state::{can_play_action, State, StateUpdate};

pub fn show_globals(game: &Game) {
    draw_text(&format!("Age {}", game.age), 600., 20., 20., BLACK);
    draw_text(&format!("Round {}", game.round), 600., 50., 20., BLACK);
    draw_text(
        &format!("Player {}", game.current_player_index),
        600.,
        80.,
        20.,
        BLACK,
    );
    let status = match game.state {
        GameState::Playing => String::from("Play Actions"),
        GameState::StatusPhase(_) => String::from("Status Phase"),
        GameState::Movement { .. } => String::from("Movement"),
        GameState::CulturalInfluenceResolution { .. } => {
            String::from("Cultural Influence Resolution")
        }
        GameState::Finished => String::from("Finished"),
    };
    draw_text(&status, 600., 110., 20., BLACK);
    draw_text(
        &format!("Actions Left {}", game.actions_left),
        600.,
        140.,
        20.,
        BLACK,
    );
}

pub fn show_wonders(game: &Game, player_index: usize) {
    let player = game.get_player(player_index);
    for (i, name) in player.wonders.iter().enumerate() {
        draw_text(
            &format!("Wonder {name}"),
            600.,
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
            600.,
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
        draw_text(&label, 600., 200. + i, 20., BLACK);
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
    let y = 540.;
    if game.can_undo() && root_ui().button(vec2(600., y), "Undo") {
        return StateUpdate::Execute(Action::Undo);
    }
    if game.can_redo() && root_ui().button(vec2(650., y), "Redo") {
        return StateUpdate::Execute(Action::Redo);
    }
    match game.state {
        GameState::CulturalInfluenceResolution {
            roll_boost_cost,
            city_piece: _,
            target_player_index: _,
            target_city_position: _,
        } => {
            if root_ui().button(
                vec2(600., 480.),
                format!("Cultural Influence Resolution for {roll_boost_cost}"),
            ) {
                StateUpdate::Execute(Action::CulturalInfluenceResolution(true))
            } else if root_ui().button(vec2(900., 480.), "Decline") {
                return StateUpdate::Execute(Action::CulturalInfluenceResolution(false));
            } else {
                StateUpdate::None
            }
        }
        GameState::Playing if root_ui().button(vec2(700., y), "End Turn") => {
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
                && root_ui().button(vec2(600., 510.), "Move Units") =>
        {
            StateUpdate::execute(Action::Playing(PlayingAction::MoveUnits))
        }
        GameState::Movement {
            movement_actions_left,
            moved_units: _,
        } if root_ui().button(
            vec2(600., 510.),
            format!("End Move Units - {movement_actions_left} actions left"),
        ) =>
        {
            StateUpdate::execute_with_warning(
                Action::Movement(MovementAction::Stop),
                if movement_actions_left > 0 {
                    vec![(format!("{movement_actions_left} movement actions left"))]
                } else {
                    vec![]
                },
            )
        }
        _ => StateUpdate::None,
    }
}

pub fn player_color(player_index: usize) -> Color {
    match player_index {
        0 => RED,
        1 => BLUE,
        2 => YELLOW,
        3 => BLACK,
        _ => panic!("unexpected player index"),
    }
}
