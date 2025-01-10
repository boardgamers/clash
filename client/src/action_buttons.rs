use crate::client_state::{ActiveDialog, StateUpdate};
use crate::happiness_ui::IncreaseHappiness;
use crate::layout_ui::{bottom_left_texture, icon_pos};
use crate::render_context::RenderContext;
use crate::resource_ui::ResourceType;
use server::action::Action;
use server::content::advances::get_advance_by_name;
use server::content::custom_actions::{CustomAction, CustomActionType};
use server::playing_actions::PlayingAction;

pub fn action_buttons(rc: &RenderContext) -> StateUpdate {
    let assets = rc.assets();
    let player = &rc.shown_player;
    let game = rc.game;
    if player.can_play_action {
        if bottom_left_texture(rc, &assets.move_units, icon_pos(0, -3), "Move units") {
            return StateUpdate::execute(Action::Playing(PlayingAction::MoveUnits));
        }
        if bottom_left_texture(rc, &assets.advances, icon_pos(1, -3), "Research advances") {
            return StateUpdate::OpenDialog(ActiveDialog::AdvanceMenu);
        }
        let p = rc.player;
        if bottom_left_texture(
            rc,
            &assets.resources[&ResourceType::MoodTokens],
            icon_pos(0, -2),
            "Increase happiness",
        ) {
            return StateUpdate::OpenDialog(ActiveDialog::IncreaseHappiness(
                IncreaseHappiness::new(p),
            ));
        }
        if bottom_left_texture(
            rc,
            &assets.resources[&ResourceType::CultureTokens],
            icon_pos(1, -2),
            "Cultural Influence",
        ) {
            return StateUpdate::OpenDialog(ActiveDialog::CulturalInfluence);
        }
    }
    for (i, a) in game.get_available_custom_actions().iter().enumerate() {
        if matches!(a, CustomActionType::ConstructWonder) {
            // handled in city_ui
            continue;
        };
        if bottom_left_texture(
            rc,
            &assets.custom_actions[a],
            icon_pos(i as i8, -1),
            &custom_action_tooltip(a),
        ) {
            return StateUpdate::execute(Action::Playing(PlayingAction::Custom(
                new_custom_action(a),
            )));
        }
    }
    StateUpdate::None
}

fn custom_action_tooltip(custom_action_type: &CustomActionType) -> String {
    match custom_action_type {
        CustomActionType::ConstructWonder => "Construct a wonder".to_string(),
        CustomActionType::ForcedLabor => get_advance_by_name("Absolute Power").unwrap().description,
    }
}

fn new_custom_action(custom_action_type: &CustomActionType) -> CustomAction {
    match custom_action_type {
        CustomActionType::ConstructWonder => {
            panic!("Construct wonder is handled in city_ui")
        }
        CustomActionType::ForcedLabor => CustomAction::ForcedLabor,
    }
}
