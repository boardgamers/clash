use crate::city_ui::{IconAction, IconActionVec};
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::{BaseOrCustomAction, BaseOrCustomDialog};
use crate::event_ui::event_help;
use crate::happiness_ui::{can_play_increase_happiness, open_increase_happiness_dialog};
use crate::influence_ui::{can_play_influence_culture, new_cultural_influence_dialog};
use crate::layout_ui::{bottom_left_texture, icon_pos};
use crate::move_ui::MoveIntent;
use crate::payment_ui::Payment;
use crate::render_context::RenderContext;
use server::action::Action;
use server::city::City;
use server::content::advances::culture::{sports_options, theaters_options};
use server::content::advances::economy::tax_options;
use server::content::custom_actions::{CustomAction, CustomActionType};
use server::events::EventOrigin;
use server::game::GameState;
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
    if can_play_influence_culture(rc)
        && bottom_left_texture(
            rc,
            &assets.resources[&ResourceType::CultureTokens],
            icon_pos(1, -2),
            "Cultural Influence",
        )
    {
        return base_or_custom_action(
            rc,
            &PlayingActionType::InfluenceCultureAttempt,
            "Influence culture",
            &[(
                EventOrigin::advance("Arts"),
                CustomActionType::ArtsInfluenceCultureAttempt,
            )],
            |d| new_cultural_influence_dialog(rc.game, rc.shown_player.index, d),
        );
    }
    let mut i = 0;
    for (a, origin) in &game.get_available_custom_actions(rc.shown_player.index) {
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
        .get_available_custom_actions(rc.shown_player.index)
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

pub fn base_or_custom_available(
    rc: &RenderContext,
    action: &PlayingActionType,
    custom: &CustomActionType,
) -> bool {
    let self1 = &rc.game;
    let player_index = rc.shown_player.index;
    rc.can_play_action(action)
        || (rc.game.state == GameState::Playing && custom.is_available(self1, player_index))
}

pub fn base_or_custom_action(
    rc: &RenderContext,
    action: &PlayingActionType,
    title: &str,
    custom: &[(EventOrigin, CustomActionType)],
    execute: impl Fn(BaseOrCustomDialog) -> ActiveDialog,
) -> StateUpdate {
    let base = if rc.can_play_action(action) {
        Some(execute(BaseOrCustomDialog {
            custom: BaseOrCustomAction::Base,
            title: title.to_string(),
        }))
    } else {
        None
    };

    let special = custom
        .iter()
        .find(|(_, a)| {
            let self1 = &rc.game;
            let player_index = rc.shown_player.index;
            a.is_available(self1, player_index)
        })
        .map(|(origin, a)| {
            let dialog = execute(BaseOrCustomDialog {
                custom: BaseOrCustomAction::Custom {
                    custom: a.clone(),
                    origin: origin.clone(),
                },
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
