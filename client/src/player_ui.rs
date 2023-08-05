use macroquad::color::BLACK;
use macroquad::math::vec2;
use macroquad::prelude::*;
use macroquad::text::draw_text;
use macroquad::ui::root_ui;

use crate::ui::State;
use server::game::{Action, Game};
use server::playing_actions::PlayingAction;
use server::resource_pile::ResourcePile;

pub fn show_globals(game: &Game) {
    draw_text(&format!("Age {}", game.age), 600., 20., 20., BLACK);
    draw_text(&format!("Round {}", game.round), 600., 50., 20., BLACK);
    draw_text(
        &format!("Actions Left {}", game.actions_left),
        600.,
        80.,
        20.,
        BLACK,
    );
}

pub fn show_resources(game: &Game, player_index: usize) {
    let player = &game.players[player_index];
    let r: &ResourcePile = player.resources();

    let mut i: f32 = 0.;
    let mut res = |label: String| {
        draw_text(
            &label,
            600.,
            200. + player_index as f32 * 200. + i,
            20.,
            BLACK,
        );
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

pub fn show_global_controls(game: &mut Game, player_index: usize, state: &mut State) {
    if game.actions_left > 0
        && root_ui().button(vec2(600., 480.), "Increase Happiness")
        && !state.happiness_selection_active()
    {
        state.clear();
        state.increase_happiness_cities = game.players[player_index]
            .cities
            .iter()
            .map(|c| (c.position.clone(), 0))
            .collect();
    }
    if state.happiness_selection_active() && root_ui().button(vec2(750., 480.), "Cancel") {
        state.clear();
    }

    if game.can_undo() && root_ui().button(vec2(600., 510.), "Undo") {
        game.execute_action(Action::Undo, player_index);
    }
    if game.can_redo() && root_ui().button(vec2(650., 510.), "Redo") {
        game.execute_action(Action::Redo, player_index);
    }
    if game.actions_left == 0 && root_ui().button(vec2(700., 510.), "End Turn") {
        game.execute_action(Action::PlayingAction(PlayingAction::EndTurn), player_index);
    }
}
