use crate::client::Features;
use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate, OFFSET, ZOOM};
use crate::happiness_ui::start_increase_happiness;
use crate::layout_ui::{
    bottom_left_button, bottom_left_texture, bottom_right_texture, icon_pos, top_center_label,
    top_center_texture, top_left_button, top_left_label, top_right_texture,
};
use crate::resource_ui::ResourceType;
use macroquad::math::{u32, vec2};
use macroquad::prelude::*;
use macroquad::ui::{root_ui, Ui};
use server::action::Action;
use server::game::{Game, GameState};
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::resource_pile::ResourcePile;

pub fn show_globals(game: &Game, player: &ShownPlayer, state: &State) -> StateUpdate {
    let update = show_top_left(game, player);
    show_top_center(game, player, state);
    update
}

pub fn resource_label(
    player: &ShownPlayer,
    state: &State,
    label: &str,
    resource_type: ResourceType,
    p: Vec2,
) {
    top_icon_with_label(
        player,
        state,
        label,
        &state.assets.resources[&resource_type],
        p,
    );
}

pub fn top_icon_with_label(
    player: &ShownPlayer,
    state: &State,
    label: &str,
    texture: &Texture2D,
    p: Vec2,
) {
    top_center_texture(state, texture, p);
    top_center_label(player, p + vec2(-30. - label.len() as f32 * 5., 40.), label);
}

fn show_top_center(game: &Game, player: &ShownPlayer, state: &State) {
    let p = game.get_player(player.index);

    resource_label(
        player,
        &state,
        &resource_ui(p, |r| r.food),
        ResourceType::Food,
        icon_pos(-4, 0),
    );
    resource_label(
        player,
        &state,
        &resource_ui(p, |r| r.wood),
        ResourceType::Wood,
        icon_pos(-3, 0),
    );
    resource_label(
        player,
        &state,
        &resource_ui(p, |r| r.ore),
        ResourceType::Ore,
        icon_pos(-2, 0),
    );
    resource_label(
        player,
        &state,
        &resource_ui(p, |r| r.ideas),
        ResourceType::Ideas,
        icon_pos(-1, 0),
    );
    resource_label(
        player,
        &state,
        &resource_ui(p, |r| r.gold as u32),
        ResourceType::Gold,
        icon_pos(0, 0),
    );
    resource_label(
        player,
        &state,
        &resource_ui(p, |r| r.mood_tokens),
        ResourceType::MoodTokens,
        icon_pos(1, 0),
    );
    resource_label(
        player,
        &state,
        &resource_ui(p, |r| r.culture_tokens),
        ResourceType::CultureTokens,
        icon_pos(2, 0),
    );

    top_icon_with_label(
        player,
        &state,
        &format!("{}", &p.victory_points()),
        &state.assets.victory_points,
        icon_pos(3, 0),
    );

    show_wonders(game, player, &mut root_ui());
}

fn show_top_left(game: &Game, player: &ShownPlayer) -> StateUpdate {
    let mut y = 0.;
    let mut show = |label: String, button: bool| -> bool {
        let p = vec2(0., y * 25.);
        y += 1.;
        if button {
            top_left_button(p, &label)
        } else {
            top_left_label(p, &label);
            false
        }
    };
    let mut label = |label: String| {
       show(label, false);
    };
    // let mut button = |label: String| -> bool {
    //     show(label, true)
    // };

    match &game.state {
        GameState::Finished => label("Finished".to_string()),
        _ => label(format!("Age {}", game.age)),
    }
    match &game.state {
        GameState::StatusPhase(ref p) => label(format!("Status Phase: {p:?}")),
        _ => label(format!("Round {}", game.round)),
    }

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
        let l = format!("{prefix}{name}");
        // if shown {
            label(l);
        // } else if button(l) {
        //     return StateUpdate::SetShownPlayer(p.index);
        // }
    }

    let p = game.get_player(player.index);

    label(format!("Civ {}", p.civilization.name));

    label(format!(
        "Leader {}",
        if let Some(l) = &p.active_leader {
            &l.name
        } else {
            "-"
        }
    ));

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
    StateUpdate::None
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

fn resource_ui(player: &Player, f: impl Fn(&ResourcePile) -> u32) -> String {
    let r: &ResourcePile = &player.resources;
    let l: &ResourcePile = &player.resource_limit;
    format!("{}/{}", f(r), f(l))
}

pub fn show_global_controls(game: &Game, state: &mut State, features: &Features) -> StateUpdate {
    let player = &state.shown_player(game);

    let assets = &state.assets;
    if bottom_left_texture(state, &assets.zoom_in, icon_pos(1, -1)) {
        state.zoom *= 1.1;
        return StateUpdate::None;
    }
    if bottom_left_texture(state, &assets.zoom_out, icon_pos(0, -1)) {
        state.zoom /= 1.1;
        return StateUpdate::None;
    }
    if bottom_left_texture(state, &assets.reset, icon_pos(2, -1)) {
        state.zoom = ZOOM;
        state.offset = OFFSET;
        return StateUpdate::None;
    }
    if bottom_left_texture(state, &assets.up, icon_pos(4, -2)) {
        state.offset += vec2(0., 0.1);
        return StateUpdate::None;
    }
    if bottom_left_texture(state, &assets.down, icon_pos(4, -1)) {
        state.offset += vec2(0., -0.1);
        return StateUpdate::None;
    }
    if bottom_left_texture(state, &assets.left, icon_pos(3, -1)) {
        state.offset += vec2(-0.1, 0.);
        return StateUpdate::None;
    }
    if bottom_left_texture(state, &assets.right, icon_pos(5, -1)) {
        state.offset += vec2(0.1, 0.);
        return StateUpdate::None;
    }

    if game.can_undo() && bottom_right_texture(state, &assets.undo, icon_pos(-6, -1)) {
        return StateUpdate::Execute(Action::Undo);
    }
    if game.can_redo() && bottom_right_texture(state, &assets.redo, icon_pos(-5, -1)) {
        return StateUpdate::Execute(Action::Redo);
    }
    if player.can_control
        && matches!(game.state, GameState::Playing)
        && bottom_right_texture(state, &assets.end_turn, icon_pos(-4, -1))
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

    if player.can_play_action && bottom_left_texture(state, &assets.movement, icon_pos(0, -3)) {
        return StateUpdate::execute(Action::Playing(PlayingAction::MoveUnits));
    }
    if player.can_play_action && bottom_left_texture(state, &assets.happy, icon_pos(0, -2)) {
        return start_increase_happiness(game, player);
    }
    if top_right_texture(state, &assets.advances, icon_pos(-2, 0)) {
        return StateUpdate::OpenDialog(ActiveDialog::AdvanceMenu);
    };

    if top_right_texture(state, &assets.log, icon_pos(-1, 0)) {
        return StateUpdate::OpenDialog(ActiveDialog::Log);
    };
    let d = state.game_state_dialog(game, &ActiveDialog::None);
    if !matches!(d, ActiveDialog::None)
        && d.title() != state.active_dialog.title()
        && bottom_left_button(player, vec2(0., -200.), &format!("Back to {}", d.title()))
    {
        return StateUpdate::OpenDialog(d);
    }

    if features.import_export {
        if bottom_right_texture(state, &assets.import, icon_pos(-2, -3)) {
            return StateUpdate::Import;
        };
        if bottom_right_texture(state, &assets.export, icon_pos(-1, -3)) {
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
