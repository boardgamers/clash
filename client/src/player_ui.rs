use crate::client::Features;
use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate, OFFSET, ZOOM};
use crate::happiness_ui::start_increase_happiness;
use crate::layout_ui::{
    bottom_left_texture, bottom_right_texture, icon_pos, left_mouse_button, top_center_texture,
    top_right_texture, ICON_SIZE,
};
use crate::resource_ui::{resource_name, ResourceType};
use macroquad::math::{u32, vec2};
use macroquad::prelude::*;
use macroquad::ui::{root_ui, Ui};
use server::action::Action;
use server::game::{Game, GameState};
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::resource_pile::ResourcePile;
use server::unit::MovementAction;

pub fn player_select(game: &Game, player: &ShownPlayer, state: &State) -> StateUpdate {
    let i = game
        .players
        .iter()
        .position(|p| p.index == game.starting_player_index)
        .unwrap();
    let mut players: Vec<_> = game.players.iter().map(|p| p.index).collect();
    players.rotate_left(i);

    let mut y = (players.len() as f32 * -ICON_SIZE) / 2.;

    for p in players {
        let pl = game.get_player(p);
        let shown = player.index == pl.index;
        let pos = vec2(player.screen_size.x, player.screen_size.y / 2.0) + vec2(-20., y);

        let color = player_color(pl.index);

        let w = if shown { ICON_SIZE + 10. } else { ICON_SIZE };
        let x = pos.x - w + ICON_SIZE;
        draw_rectangle(x, pos.y, w, ICON_SIZE, color);
        draw_rectangle_lines(x, pos.y, w, ICON_SIZE, 2.0, BLACK);
        let text = format!("{}", pl.victory_points());

        state.draw_text(&text, pos.x + 5., pos.y + 20.);

        let active = game.active_player();
        if active == pl.index {
            draw_texture_ex(
                &state.assets.active_player,
                x - 25.,
                pos.y + 5.,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(20., 20.)),
                    ..Default::default()
                },
            );
        }

        if !shown && left_mouse_button(Rect::new(x, pos.y, w, ICON_SIZE)) {
            return StateUpdate::SetShownPlayer(pl.index);
        }

        y += ICON_SIZE;
    }

    StateUpdate::None
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
        resource_name(resource_type),
    );
}

pub fn top_icon_with_label(
    player: &ShownPlayer,
    state: &State,
    label: &str,
    texture: &Texture2D,
    p: Vec2,
    tooltip: &str,
) {
    let dimensions = state.measure_text(label);
    let x = (ICON_SIZE - dimensions.width) / 2.0;
    state.draw_text(
        label,
        player.screen_size.x / 2.0 + p.x + x,
        p.y + ICON_SIZE + 30.,
    );
    top_center_texture(state, texture, p, tooltip);
}

pub fn show_top_center(game: &Game, player: &ShownPlayer, state: &State) {
    let p = game.get_player(player.index);

    top_icon_with_label(
        player,
        state,
        &format!("{}", &p.victory_points()),
        &state.assets.victory_points,
        icon_pos(3, 0),
        "Victory Points",
    );
    resource_label(
        player,
        state,
        &resource_ui(p, |r| r.culture_tokens),
        ResourceType::CultureTokens,
        icon_pos(2, 0),
    );
    resource_label(
        player,
        state,
        &resource_ui(p, |r| r.mood_tokens),
        ResourceType::MoodTokens,
        icon_pos(1, 0),
    );
    resource_label(
        player,
        state,
        &resource_ui(p, |r| r.gold as u32),
        ResourceType::Gold,
        icon_pos(0, 0),
    );
    resource_label(
        player,
        state,
        &resource_ui(p, |r| r.ideas),
        ResourceType::Ideas,
        icon_pos(-1, 0),
    );
    resource_label(
        player,
        state,
        &resource_ui(p, |r| r.ore),
        ResourceType::Ore,
        icon_pos(-2, 0),
    );
    resource_label(
        player,
        state,
        &resource_ui(p, |r| r.wood),
        ResourceType::Wood,
        icon_pos(-3, 0),
    );
    resource_label(
        player,
        state,
        &resource_ui(p, |r| r.food),
        ResourceType::Food,
        icon_pos(-4, 0),
    );

    show_wonders(game, player, &mut root_ui());
}

pub fn show_top_left(game: &Game, player: &ShownPlayer, state: &State) {
    let mut y = 0.;
    let mut label = |label: &str| {
        let p = vec2(10., y * 25. + 20.);
        y += 1.;
        state.draw_text(label, p.x, p.y);
    };

    match &game.state {
        GameState::Finished => label("Finished"),
        _ => label(&format!("Age {}", game.age)),
    }
    match &game.state {
        GameState::StatusPhase(ref p) => label(&format!("Status Phase: {p:?}")),
        _ => label(&format!("Round {}", game.round)),
    }

    let p = game.get_player(player.index);

    label(&p.get_name());

    label(&format!("Civ {}", p.civilization.name));

    label(&format!(
        "Leader {}",
        if let Some(l) = &p.active_leader {
            &l.name
        } else {
            "-"
        }
    ));

    if game.current_player_index == p.index {
        match &game.state {
            GameState::StatusPhase(_) | GameState::Finished => {}
            _ => label(&format!("{} actions left", game.actions_left)),
        }

        match &game.state {
            GameState::Movement {
                movement_actions_left,
                ..
            } => label(&format!("Move units: {movement_actions_left} moves left")),
            GameState::CulturalInfluenceResolution(_) => {
                label("Cultural Influence Resolution");
            }
            GameState::Combat(c) => label(&format!(
                "Combat Round {} Phase {:?}{}",
                c.round,
                c.phase,
                moves_left(&game.state)
                    .map(|m| format!(", {m} moves left"))
                    .unwrap_or_default()
            )),
            GameState::PlaceSettler {
                player_index: _,
                movement_actions_left,
                ..
            } => label(&format!(
                "Place Settler: {movement_actions_left} moves left"
            )),
            _ => {}
        }
        state.active_dialog.help_message().map(label);
    }
}

fn moves_left(state: &GameState) -> Option<u32> {
    match state {
        GameState::Combat(c) => moves_left(&c.initiation),
        GameState::Movement {
            movement_actions_left,
            ..
        }
        | GameState::PlaceSettler {
            player_index: _,
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

    if let Some(tooltip) = can_end_move(game) {
        if player.can_control
            && bottom_right_texture(state, &assets.end_turn, icon_pos(-4, -1), tooltip)
        {
            return end_move(game);
        }
    }
    if game.can_redo() && bottom_right_texture(state, &assets.redo, icon_pos(-5, -1), "Redo") {
        return StateUpdate::Execute(Action::Redo);
    }
    if game.can_undo() && bottom_right_texture(state, &assets.undo, icon_pos(-6, -1), "Undo") {
        return StateUpdate::Execute(Action::Undo);
    }
    let d = state.game_state_dialog(game, &ActiveDialog::None);
    if d.can_restore()
        && d.title() != state.active_dialog.title()
        && bottom_right_texture(
            state,
            &assets.restore_menu,
            icon_pos(-7, -1),
            format!("Restore {}", d.title()).as_str(),
        )
    {
        return StateUpdate::OpenDialog(d);
    }

    if player.can_play_action {
        if bottom_left_texture(state, &assets.movement, icon_pos(0, -3), "Move units") {
            return StateUpdate::execute(Action::Playing(PlayingAction::MoveUnits));
        }
        if bottom_left_texture(state, &assets.happy, icon_pos(0, -2), "Increase happiness") {
            return start_increase_happiness(game, player);
        }
    }

    if top_right_texture(state, &assets.log, icon_pos(-1, 0), "Show log") {
        return StateUpdate::OpenDialog(ActiveDialog::Log);
    };
    if top_right_texture(
        state,
        &assets.advances,
        icon_pos(-2, 0),
        "Choose or show advances",
    ) {
        return StateUpdate::OpenDialog(ActiveDialog::AdvanceMenu);
    };

    if features.import_export {
        if bottom_right_texture(state, &assets.export, icon_pos(-1, -3), "Export") {
            return StateUpdate::Export;
        };
        if bottom_right_texture(state, &assets.import, icon_pos(-2, -3), "Import") {
            return StateUpdate::Import;
        };
    }

    if bottom_left_texture(state, &assets.up, icon_pos(4, -2), "Move up") {
        state.offset += vec2(0., -0.1);
        return StateUpdate::None;
    }
    if bottom_left_texture(state, &assets.right, icon_pos(5, -1), "Move right") {
        state.offset += vec2(-0.1, 0.);
        return StateUpdate::None;
    }
    if bottom_left_texture(state, &assets.down, icon_pos(4, -1), "Move down") {
        state.offset += vec2(0., 0.1);
        return StateUpdate::None;
    }
    if bottom_left_texture(state, &assets.left, icon_pos(3, -1), "Move left") {
        state.offset += vec2(0.1, 0.);
        return StateUpdate::None;
    }

    if bottom_left_texture(
        state,
        &assets.reset,
        icon_pos(2, -1),
        "Reset zoom and offset",
    ) {
        state.zoom = ZOOM;
        state.offset = OFFSET;
        return StateUpdate::None;
    }

    if bottom_left_texture(state, &assets.zoom_in, icon_pos(1, -1), "Zoom in") {
        state.zoom *= 1.1;
        return StateUpdate::None;
    }
    if bottom_left_texture(state, &assets.zoom_out, icon_pos(0, -1), "Zoom out") {
        state.zoom /= 1.1;
        return StateUpdate::None;
    }

    StateUpdate::None
}

fn can_end_move(game: &Game) -> Option<&str> {
    match game.state {
        GameState::Movement { .. } => Some("End movement"),
        GameState::Playing => Some("End turn"),
        _ => None,
    }
}

fn end_move(game: &Game) -> StateUpdate {
    if let GameState::Movement {
        movement_actions_left,
        ..
    } = &game.state
    {
        return StateUpdate::execute_with_warning(
            Action::Movement(MovementAction::Stop),
            if *movement_actions_left > 0 {
                vec![(format!("{movement_actions_left} movement actions left"))]
            } else {
                vec![]
            },
        );
    }

    let left = game.actions_left;
    StateUpdate::execute_with_warning(
        Action::Playing(PlayingAction::EndTurn),
        if left > 0 {
            vec![format!("{left} actions left")]
        } else {
            vec![]
        },
    )
}

pub fn player_color(player_index: usize) -> Color {
    match player_index {
        0 => YELLOW,
        1 => PINK,
        _ => panic!("unexpected player index"),
    }
}
