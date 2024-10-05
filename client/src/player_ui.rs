use crate::client::Features;
use crate::client_state::{
    ActiveDialog, ShownPlayer, State, StateUpdate, StateUpdates, OFFSET, ZOOM,
};
use crate::dialog_ui::show_window;
use crate::happiness_ui::start_increase_happiness;
use macroquad::hash;
use macroquad::math::vec2;
use macroquad::prelude::*;
use macroquad::ui::widgets::Window;
use macroquad::ui::Ui;
use server::action::Action;
use server::game::{Game, GameState};
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::resource_pile::ResourcePile;

pub fn show_globals(game: &Game, player: &ShownPlayer) -> StateUpdate {
    let y = 0.;
    let window = Window::new(hash!(), vec2(10., 5.), vec2(1200., 20.))
        .titlebar(false)
        .movable(false)
        .close_button(false);

    let mut updates = StateUpdates::new();
    let (update, _open) = show_window(window, |ui| {
        ui.label(vec2(10., y), &format!("Age {}", game.age));
        ui.label(vec2(40., y), &format!("Round {}", game.round));

        show_player_status(game, player, ui);
        show_wonders(game, player, ui);
        StateUpdate::None
    });
    
    updates.add(update);
    let window = Window::new(hash!(), vec2(10., 50.), vec2(1200., 20.))
        .titlebar(false)
        .movable(false)
        .close_button(false);
    let (update, _open) = show_window(window, |ui| {
        let i = game
            .players
            .iter()
            .position(|p| p.index == game.starting_player_index)
            .unwrap();
        let mut players: Vec<_> = game.players.iter().map(|p| p.index).collect();
        players.rotate_left(i);

        for (i, &p) in players.iter().enumerate() {
            let p = game.get_player(p);
            let shown = player.index == p.index;
            let prefix = if shown { "* " } else { "" };
            let suffix = &player_suffix(game, p);
            let name = p.get_name();
            let y = 0.;
            let x = i as f32 * 500.;
            let label = format!("{prefix}{name}{suffix}");
            if shown {
                ui.label(vec2(x, y), &label);
            } else if ui.button(vec2(x, y), label) {
                return StateUpdate::SetShownPlayer(p.index);
            }
        }
        StateUpdate::None
    });
    updates.add(update);
    updates.result()
}

fn player_suffix(game: &Game, player: &Player) -> String {
    let actions_left = if game.current_player_index == player.index {
        match &game.state {
            GameState::StatusPhase(_) | GameState::Finished => "",
            _ => &format!("{} actions Left", game.actions_left),
        }
    } else {
        ""
    };

    let moves_left = if game.current_player_index == player.index {
        match moves_left(&game.state) {
            None => "",
            Some(m) => &format!("{m} moves left"),
        }
    } else {
        ""
    };

    let active_player = if player.index == game.active_player() {
        match &game.state {
            GameState::Playing => "Play Actions",
            GameState::StatusPhase(ref p) => &format!("Status Phase: {p:?}"),
            GameState::Movement { .. } => "Movement",
            GameState::CulturalInfluenceResolution(_) => "Cultural Influence Resolution",
            GameState::Combat(c) => &format!("Combat Round {} Phase {:?}", c.round, c.phase),
            GameState::PlaceSettler { .. } => "Place Settler",
            GameState::Finished => "Finished",
        }
    } else {
        ""
    };

    let status = vec![active_player, actions_left, moves_left]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ");

    if status.is_empty() {
        String::new()
    } else {
        format!(" ({status})")
    }
}

fn moves_left(state: &GameState) -> Option<u32> {
    match state {
        GameState::Combat(c) => moves_left(&c.initiation),
        GameState::Movement {
            movement_actions_left,
            ..
        } => Some(*movement_actions_left),
        GameState::PlaceSettler {
            player_index: _player_index,
            movement_actions_left,
            ..
        } => Some(*movement_actions_left),
        _ => None,
    }
}

pub fn show_wonders(game: &Game, player: &ShownPlayer, ui: &mut Ui) {
    let player = game.get_player(player.index);
    for (i, name) in player.wonders.iter().enumerate() {
        ui.label(vec2(500. + i as f32 * 30.0, 0.), &format!("Wonder {name}"));
    }
    for (i, card) in player.wonder_cards.iter().enumerate() {
        let req = match card.required_advances[..] {
            [] => String::from("no advances"),
            _ => card.required_advances.join(", "),
        };
        ui.label(
            vec2(900. + i as f32 * 30.0, 0.),
            &format!(
                "Wonder Card {} cost {} requires {}",
                &card.name, card.cost, req
            ),
        );
    }
}

pub fn show_player_status(game: &Game, player: &ShownPlayer, ui: &mut Ui) {
    let player = game.get_player(player.index);
    let mut i: f32 = 0.;
    let mut res = |label: String| {
        ui.label(vec2(110. + i, 0.), &label);
        i += 70.;
    };

    res(resource_ui(player, "Food", |r| r.food));
    res(resource_ui(player, "Wood", |r| r.wood));
    res(resource_ui(player, "Ore", |r| r.ore));
    res(resource_ui(player, "Ideas", |r| r.ideas));
    res(resource_ui(player, "Gold", |r| r.gold as u32));
    res(resource_ui(player, "Mood", |r| r.mood_tokens));
    res(resource_ui(player, "Culture", |r| r.culture_tokens));

    res(format!("Civ {}", player.civilization.name));
    res(format!("VP {}", player.victory_points()));
    res(format!(
        "Leader {}",
        if let Some(l) = &player.active_leader {
            &l.name
        } else {
            "-"
        }
    ));
}

fn resource_ui(player: &Player, name: &str, f: impl Fn(&ResourcePile) -> u32) -> String {
    let r: &ResourcePile = &player.resources;
    let l: &ResourcePile = &player.resource_limit;
    format!("{name} {}/{}", f(r), f(l))
}

pub fn show_global_controls(game: &Game, state: &mut State, features: &Features) -> StateUpdate {
    let player = state.shown_player(game);
    let y = 0.;
    let window = Window::new(hash!(), vec2(10., 30.), vec2(1200., 20.))
        .titlebar(false)
        .movable(false)
        .close_button(false);

    let (update, _open) = show_window(window, |ui| {
        if ui.button(vec2(10., y), "+") {
            state.zoom *= 1.1;
            return StateUpdate::None;
        }
        if ui.button(vec2(25., y), "-") {
            state.zoom /= 1.1;
            return StateUpdate::None;
        }
        if ui.button(vec2(40., y), "Reset") {
            state.zoom = ZOOM;
            state.offset = OFFSET;
            return StateUpdate::None;
        }
        if ui.button(vec2(100., y), "L") {
            state.offset += vec2(-0.1, 0.);
            return StateUpdate::None;
        }
        if ui.button(vec2(120., y), "R") {
            state.offset += vec2(0.1, 0.);
            return StateUpdate::None;
        }
        if ui.button(vec2(140., y), "U") {
            state.offset += vec2(0., 0.1);
            return StateUpdate::None;
        }
        if ui.button(vec2(160., y), "D") {
            state.offset += vec2(0., -0.1);
            return StateUpdate::None;
        }

        if game.can_undo() && ui.button(vec2(180., y), "Undo") {
            return StateUpdate::Execute(Action::Undo);
        }
        // todo close active dialog button
        if game.can_redo() && ui.button(vec2(220., y), "Redo") {
            return StateUpdate::Execute(Action::Redo);
        }
        if player.can_control
            && matches!(game.state, GameState::Playing)
            && ui.button(vec2(270., y), "End Turn")
        {
            let left = game.actions_left;
            return StateUpdate::execute_with_warning(
                Action::Playing(PlayingAction::EndTurn),
                if left > 0 {
                    vec![format!("{left} actions left")]
                } else {
                    vec![]
                },
            );
        }

        if player.can_play_action && ui.button(vec2(340., y), "Move") {
            return StateUpdate::execute(Action::Playing(PlayingAction::MoveUnits));
        }
        if player.can_play_action && ui.button(vec2(390., y), "Happiness") {
            return start_increase_happiness(game, &player);
        }
        if ui.button(vec2(490., y), "Advances") {
            return StateUpdate::OpenDialog(ActiveDialog::AdvanceMenu);
        };
        if ui.button(vec2(560., y), "Log") {
            return StateUpdate::OpenDialog(ActiveDialog::Log);
        };
        let d = state.game_state_dialog(game, &ActiveDialog::None);
        if !matches!(d, ActiveDialog::None)
            && d.title() != state.active_dialog.title()
            && ui.button(vec2(600., y), format!("Back to {}", d.title()))
        {
            return StateUpdate::OpenDialog(d);
        }

        if features.import_export {
            if ui.button(vec2(1000., y), "Import") {
                return StateUpdate::Import;
            };
            if ui.button(vec2(1100., y), "Export") {
                return StateUpdate::Export;
            };
        }
        StateUpdate::None
    });
    update
}

pub fn player_color(player_index: usize) -> Color {
    match player_index {
        0 => YELLOW,
        1 => PINK,
        _ => panic!("unexpected player index"),
    }
}
