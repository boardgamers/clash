use crate::client::Features;
use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate, OFFSET, ZOOM};
use crate::happiness_ui::start_increase_happiness;
use crate::layout_ui::{
    bottom_left_button, bottom_right_button, right_center_button, right_center_label,
    top_center_label, top_left_label,
};
use macroquad::math::vec2;
use macroquad::prelude::*;
use macroquad::ui::{root_ui, Ui};
use server::action::Action;
use server::game::{Game, GameState};
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::resource_pile::ResourcePile;

pub fn show_globals(game: &Game, player: &ShownPlayer) -> StateUpdate {
    show_top_left(game);
    show_top_center(game, player);

    let mut y = -100.;

    let i = game
        .players
        .iter()
        .position(|p| p.index == game.starting_player_index)
        .unwrap();
    let mut players: Vec<_> = game.players.iter().map(|p| p.index).collect();
    players.rotate_left(i);

    for p in players {
        let p = game.get_player(p);
        let shown = player.index == p.index;
        let prefix = if shown { "* " } else { "" };
        let name = p.get_name();
        let x = -200.;
        let label = format!("{prefix}{name}");
        if shown {
            right_center_label(vec2(x, y), &label);
        } else if right_center_button(vec2(x, y), &label) {
            return StateUpdate::SetShownPlayer(p.index);
        }
        y += 40.;
    }

    StateUpdate::None
}

fn show_top_center(game: &Game, player: &ShownPlayer) {
    let p = game.get_player(player.index);

    top_center_label(vec2(-400., 0.), &resource_ui(p, "Fd", |r| r.food));
    top_center_label(vec2(-320., 0.), &resource_ui(p, "Wd", |r| r.wood));
    top_center_label(vec2(-240., 0.), &resource_ui(p, "Ore", |r| r.ore));
    top_center_label(vec2(-160., 0.), &resource_ui(p, "Id", |r| r.ideas));
    top_center_label(vec2(-80., 0.), &resource_ui(p, "Gld", |r| r.gold as u32));
    top_center_label(vec2(0., 0.), &resource_ui(p, "Md", |r| r.mood_tokens));
    top_center_label(vec2(80., 0.), &resource_ui(p, "Cul", |r| r.culture_tokens));

    top_center_label(vec2(170., 0.), &format!("Civ {}", p.civilization.name));
    top_center_label(vec2(250., 0.), &format!("VP {}", p.victory_points()));
    top_center_label(
        vec2(300., 0.),
        &format!(
            "Ldr {}",
            if let Some(l) = &p.active_leader {
                &l.name
            } else {
                "-"
            }
        ),
    );
    show_wonders(game, player, &mut root_ui());
}

fn show_top_left(game: &Game) {
    let mut y = 0.;
    let mut label = |label: String| {
        top_left_label(vec2(0., y), &label);
        y += 30.;
    };

    match &game.state {
        GameState::Finished => label("Finished".to_string()),
        _ => label(format!("Age {}", game.age)),
    }
    match &game.state {
        GameState::StatusPhase(ref p) => label(format!("Status Phase: {p:?}")),
        _ => label(format!("Round {}", game.round)),
    }

    let current = game.current_player_index;
    label(format!("Playing {}", &game.get_player(current).get_name()));

    match &game.state {
        GameState::StatusPhase(_) | GameState::Finished => {}
        _ => label(format!("Actions {}", game.actions_left)),
    }

    match &game.state {
        GameState::Movement { .. } => label("Movement".to_string()),
        GameState::CulturalInfluenceResolution(_) => {
            label("Cultural Influence Resolution".to_string());
        }
        GameState::Combat(c) => label(format!("Combat Round {} Phase {:?}", c.round, c.phase)),
        GameState::PlaceSettler { .. } => label("Place Settler".to_string()),
        _ => {}
    }

    if let Some(m) = moves_left(&game.state) {
        label(format!("Moves left {m}"));
    }

    let active = game.active_player();
    if active != current {
        label(format!("Active {}", game.get_player(active).get_name()));
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
    //todo move to cards ui
    let player = game.get_player(player.index);
    let y = 5.;
    for (i, name) in player.wonders.iter().enumerate() {
        ui.label(vec2(500. + i as f32 * 30.0, y), &format!("Wonder {name}"));
    }
    for (i, card) in player.wonder_cards.iter().enumerate() {
        let req = match card.required_advances[..] {
            [] => String::from("no advances"),
            _ => card.required_advances.join(", "),
        };
        ui.label(
            vec2(900. + i as f32 * 30.0, y),
            &format!(
                "Wonder Card {} cost {} requires {}",
                &card.name, card.cost, req
            ),
        );
    }
}

fn resource_ui(player: &Player, name: &str, f: impl Fn(&ResourcePile) -> u32) -> String {
    let r: &ResourcePile = &player.resources;
    let l: &ResourcePile = &player.resource_limit;
    format!("{name} {}/{}", f(r), f(l))
}

pub fn show_global_controls(game: &Game, state: &mut State, features: &Features) -> StateUpdate {
    let player = state.shown_player(game);

    if bottom_left_button(vec2(0., -50.), "+") {
        state.zoom *= 1.1;
        return StateUpdate::None;
    }
    if bottom_left_button(vec2(70., -50.), "-") {
        state.zoom /= 1.1;
        return StateUpdate::None;
    }
    if bottom_left_button(vec2(140., -50.), "0") {
        state.zoom = ZOOM;
        state.offset = OFFSET;
        return StateUpdate::None;
    }
    if bottom_left_button(vec2(210., -80.), "L") {
        state.offset += vec2(-0.1, 0.);
        return StateUpdate::None;
    }
    if bottom_left_button(vec2(310., -80.), "R") {
        state.offset += vec2(0.1, 0.);
        return StateUpdate::None;
    }
    if bottom_left_button(vec2(260., -110.), "U") {
        state.offset += vec2(0., 0.1);
        return StateUpdate::None;
    }
    if bottom_left_button(vec2(260., -50.), "D") {
        state.offset += vec2(0., -0.1);
        return StateUpdate::None;
    }

    if game.can_undo() && bottom_right_button(vec2(-400., -50.), "Undo") {
        return StateUpdate::Execute(Action::Undo);
    }
    if game.can_redo() && bottom_right_button(vec2(-300., -50.), "Redo") {
        return StateUpdate::Execute(Action::Redo);
    }
    if player.can_control
        && matches!(game.state, GameState::Playing)
        && bottom_right_button(vec2(-180., -50.), "End Turn")
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

    if player.can_play_action && bottom_left_button(vec2(0., -170.), "Move") {
        return StateUpdate::execute(Action::Playing(PlayingAction::MoveUnits));
    }
    if player.can_play_action && bottom_left_button(vec2(0., -140.), "Inc. Hap.") {
        return start_increase_happiness(game, &player);
    }
    if bottom_left_button(vec2(0., -110.), "Advances") {
        return StateUpdate::OpenDialog(ActiveDialog::AdvanceMenu);
    };
    if bottom_left_button(vec2(0., -80.), "Log") {
        return StateUpdate::OpenDialog(ActiveDialog::Log);
    };
    let d = state.game_state_dialog(game, &ActiveDialog::None);
    if !matches!(d, ActiveDialog::None)
        && d.title() != state.active_dialog.title()
        && bottom_left_button(vec2(0., -50.), &format!("Back to {}", d.title()))
    {
        return StateUpdate::OpenDialog(d);
    }

    if features.import_export {
        if bottom_right_button(vec2(-300., -100.), "Import") {
            return StateUpdate::Import;
        };
        if bottom_right_button(vec2(-150., -100.), "Export") {
            return StateUpdate::Export;
        };
    }
    StateUpdate::None
}

pub fn player_color(player_index: usize) -> Color {
    match player_index {
        0 => YELLOW,
        1 => PINK,
        _ => panic!("unexpected player index"),
    }
}
