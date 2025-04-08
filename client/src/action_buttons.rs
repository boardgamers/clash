use crate::city_ui::{IconAction, IconActionVec};
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::BaseOrCustomDialog;
use crate::event_ui::event_help;
use crate::happiness_ui::open_increase_happiness_dialog;
use crate::influence_ui::new_cultural_influence_dialog;
use crate::layout_ui::{bottom_left_texture, icon_pos};
use crate::move_ui::MoveIntent;
use crate::payment_ui::Payment;
use crate::render_context::RenderContext;
use server::action::Action;
use server::available_actions::{
    available_happiness_actions, available_influence_actions, base_and_custom_action,
};
use server::city::City;
use server::content::advances::culture::{sports_options, theaters_options};
use server::content::advances::economy::tax_options;
use server::content::custom_actions::{CustomAction, CustomActionType};
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::resource::ResourceType;

pub fn action_buttons(rc: &RenderContext) -> StateUpdate {
    let assets = rc.assets();
    let game = rc.game;

    let happiness = available_happiness_actions(rc.game, rc.shown_player.index);
    if !happiness.is_empty()
        && bottom_left_texture(
            rc,
            &assets.resources[&ResourceType::MoodTokens],
            icon_pos(0, -2),
            "Increase happiness",
        )
    {
        return open_increase_happiness_dialog(rc, happiness, |h| h);
    }

    if rc.can_play_action(&PlayingActionType::MoveUnits)
        && bottom_left_texture(rc, &assets.move_units, icon_pos(0, -3), "Move units")
    {
        return global_move(rc);
    }

    if rc.can_play_action(&PlayingActionType::Advance)
        && bottom_left_texture(rc, &assets.advances, icon_pos(1, -3), "Research advances")
    {
        return StateUpdate::OpenDialog(ActiveDialog::AdvanceMenu);
    }

    let influence = available_influence_actions(game, rc.shown_player.index);

    if !influence.is_empty()
        && bottom_left_texture(
            rc,
            &assets.resources[&ResourceType::CultureTokens],
            icon_pos(1, -2),
            "Cultural Influence",
        )
    {
        return base_or_custom_action(rc, influence, "Influence culture", |d| {
            new_cultural_influence_dialog(rc.game, rc.shown_player.index, d)
        });
    }
    let mut i = 0;
    for (a, origin) in &game.available_custom_actions(rc.shown_player.index) {
        if let Some(action) = generic_custom_action(rc, a, None) {
            if bottom_left_texture(
                rc,
                &assets.custom_actions[a],
                icon_pos(i as i8, -1),
                &event_help(rc, origin)[0],
            ) {
                return action;
            }
            i += 1;
        }
    }
    for (i, (icon, tooltip, action)) in custom_action_buttons(rc, None).iter().enumerate() {
        if bottom_left_texture(rc, icon, icon_pos(i as i8, -1), tooltip) {
            return action();
        }
    }
    StateUpdate::None
}

pub fn custom_action_buttons<'a>(
    rc: &'a RenderContext,
    city: Option<&'a City>,
) -> IconActionVec<'a> {
    rc.game
        .available_custom_actions(rc.shown_player.index)
        .into_iter()
        .filter_map(|(a, origin)| {
            generic_custom_action(rc, &a, city).map(|action| {
                let a: IconAction<'a> = (
                    &rc.assets().custom_actions[&a],
                    event_help(rc, &origin)[0].clone(),
                    Box::new(move || action.clone()),
                );
                a
            })
        })
        .collect()
}

fn global_move(rc: &RenderContext) -> StateUpdate {
    let pos = rc.state.focused_tile;
    StateUpdate::move_units(
        rc,
        pos,
        if pos.is_some_and(|t| rc.game.map.is_sea(t)) {
            MoveIntent::Sea
        } else {
            MoveIntent::Land
        },
    )
}

fn generic_custom_action(
    rc: &RenderContext,
    custom_action_type: &CustomActionType,
    city: Option<&City>,
) -> Option<StateUpdate> {
    if let Some(city) = city {
        if matches!(custom_action_type, CustomActionType::Sports) {
            if let Some(options) = sports_options(city) {
                return Some(StateUpdate::OpenDialog(ActiveDialog::Sports((
                    Payment::new_gain(&options, "Increase happiness using sports"),
                    city.position,
                ))));
            }
        }
        return None;
    }

    match custom_action_type {
        CustomActionType::ArtsInfluenceCultureAttempt
        | CustomActionType::VotingIncreaseHappiness
        | CustomActionType::FreeEconomyCollect
        | CustomActionType::Sports => {
            // handled explicitly
            None
        }
        CustomActionType::AbsolutePower => Some(StateUpdate::execute(Action::Playing(
            PlayingAction::Custom(CustomAction::AbsolutePower),
        ))),
        CustomActionType::ForcedLabor => Some(StateUpdate::execute(Action::Playing(
            PlayingAction::Custom(CustomAction::ForcedLabor),
        ))),
        CustomActionType::CivilLiberties => Some(StateUpdate::execute(Action::Playing(
            PlayingAction::Custom(CustomAction::CivilRights),
        ))),
        CustomActionType::Taxes => Some(StateUpdate::OpenDialog(ActiveDialog::Taxes(
            Payment::new_gain(&tax_options(rc.shown_player), "Collect taxes"),
        ))),
        CustomActionType::Theaters => Some(StateUpdate::OpenDialog(ActiveDialog::Theaters(
            Payment::new_gain(&theaters_options(), "Convert Resources"),
        ))),
    }
}

pub fn base_or_custom_action(
    rc: &RenderContext,
    actions: Vec<PlayingActionType>,
    title: &str,
    execute: impl Fn(BaseOrCustomDialog) -> ActiveDialog,
) -> StateUpdate {
    let (action, custom) = base_and_custom_action(actions);

    let base = action.map(|action_type| {
        execute(BaseOrCustomDialog {
            action_type,
            title: title.to_string(),
        })
    });

    let special = custom.map(|a| {
        let origin = &rc.shown_player.custom_actions[&a];
        let dialog = execute(BaseOrCustomDialog {
            action_type: PlayingActionType::Custom(a.info()),
            title: format!("{title} with {}", origin.name()),
        });

        StateUpdate::dialog_chooser(
            &format!("Use special action from {}?", origin.name()),
            Some(dialog),
            base.clone(),
        )
    });
    special
        .or_else(|| base.map(StateUpdate::OpenDialog))
        .unwrap_or(StateUpdate::None)
}
