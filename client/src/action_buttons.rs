use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::{BaseOrCustomAction, BaseOrCustomDialog};
use crate::happiness_ui::{can_play_increase_happiness, open_increase_happiness_dialog};
use crate::layout_ui::{bottom_left_texture, icon_pos};
use crate::move_ui::MoveIntent;
use crate::render_context::RenderContext;
use server::action::Action;
use server::content::advances::get_advance_by_name;
use server::content::custom_actions::{CustomAction, CustomActionType};
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::resource::ResourceType;

pub fn action_buttons(rc: &RenderContext) -> StateUpdate {
    let assets = rc.assets();
    let game = rc.game;
    if can_play_increase_happiness(rc)
        && bottom_left_texture(
            rc,
            &assets.resources[&ResourceType::MoodTokens],
            icon_pos(0, -2),
            "Increase happiness",
        )
    {
        return open_increase_happiness_dialog(rc, |h| h);
    }

    if rc.can_play_action(PlayingActionType::MoveUnits)
        && bottom_left_texture(rc, &assets.move_units, icon_pos(0, -3), "Move units")
    {
        return global_move(rc);
    }

    if rc.can_play_action(PlayingActionType::Advance)
        && bottom_left_texture(rc, &assets.advances, icon_pos(1, -3), "Research advances")
    {
        return StateUpdate::OpenDialog(ActiveDialog::AdvanceMenu);
    }
    if rc.can_play_action(PlayingActionType::InfluenceCultureAttempt)
        && bottom_left_texture(
            rc,
            &assets.resources[&ResourceType::CultureTokens],
            icon_pos(1, -2),
            "Cultural Influence",
        )
    {
        return StateUpdate::OpenDialog(ActiveDialog::CulturalInfluence);
    }
    for (i, a) in game
        .get_available_custom_actions(rc.shown_player.index)
        .iter()
        .enumerate()
    {
        if let Some(action) = generic_custom_action(a) {
            if bottom_left_texture(
                rc,
                &assets.custom_actions[a],
                icon_pos(i as i8, -1),
                &custom_action_tooltip(a),
            ) {
                return StateUpdate::execute(Action::Playing(PlayingAction::Custom(action)));
            }
        }
    }
    StateUpdate::None
}

fn global_move(rc: &RenderContext) -> StateUpdate {
    let pos = rc.state.focused_tile;
    StateUpdate::move_units(
        rc,
        pos,
        if pos.is_some_and(|t| rc.game.map.is_water(t)) {
            MoveIntent::Sea
        } else {
            MoveIntent::Land
        },
    )
}

fn custom_action_tooltip(custom_action_type: &CustomActionType) -> String {
    match custom_action_type {
        CustomActionType::ConstructWonder => "Construct a wonder".to_string(),
        CustomActionType::ForcedLabor => get_advance_by_name("Absolute Power").description,
        CustomActionType::VotingIncreaseHappiness => get_advance_by_name("Voting").description,
        CustomActionType::FreeEconomyCollect => get_advance_by_name("Free Economy").description,
    }
}

fn generic_custom_action(custom_action_type: &CustomActionType) -> Option<CustomAction> {
    match custom_action_type {
        CustomActionType::ConstructWonder
        | CustomActionType::VotingIncreaseHappiness
        | CustomActionType::FreeEconomyCollect => {
            // handled explicitly
            None
        }
        CustomActionType::ForcedLabor => Some(CustomAction::ForcedLabor),
    }
}

pub fn base_or_custom_available(
    rc: &RenderContext,
    action: PlayingActionType,
    custom: &CustomActionType,
) -> bool {
    rc.can_play_action(action)
        || rc
            .game
            .get_available_custom_actions(rc.shown_player.index)
            .contains(custom)
}

pub fn base_or_custom_action(
    rc: &RenderContext,
    action: PlayingActionType,
    title: &str,
    custom: &[(&str, CustomActionType)],
    f: impl Fn(BaseOrCustomDialog) -> ActiveDialog,
) -> StateUpdate {
    let base = if rc.can_play_action(action) {
        Some(f(BaseOrCustomDialog {
            custom: BaseOrCustomAction::Base,
            title: title.to_string(),
        }))
    } else {
        None
    };

    let special = rc
        .game
        .get_available_custom_actions(rc.shown_player.index)
        .iter()
        .find(|a| custom.iter().any(|(_, b)| **a == *b))
        .map(|a| {
            let advance = custom.iter().find(|(_, b)| *b == *a).unwrap().0;
            let dialog = f(BaseOrCustomDialog {
                custom: BaseOrCustomAction::Custom {
                    custom: a.clone(),
                    advance: advance.to_string(),
                },
                title: format!("{title} with {advance}"),
            });

            StateUpdate::dialog_chooser(
                &format!("Use special action from {advance}?"),
                Some(dialog),
                base.clone(),
            )
        });
    special.unwrap_or(StateUpdate::OpenDialog(base.unwrap()))
}
