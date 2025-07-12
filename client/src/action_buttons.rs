use crate::city_ui::{IconAction, IconActionVec};
use crate::client_state::{ActiveDialog, NO_UPDATE, RenderResult, StateUpdate};
use crate::dialog_ui::BaseOrCustomDialog;
use crate::event_ui::event_help_tooltip;
use crate::happiness_ui::open_increase_happiness_dialog;
use crate::influence_ui::new_cultural_influence_dialog;
use crate::layout_ui::{bottom_left_texture, icon_pos};
use crate::log_ui::MultilineText;
use crate::move_ui::MoveIntent;
use crate::render_context::RenderContext;
use itertools::Itertools;
use server::action::Action;
use server::city::City;
use server::content::custom_actions::{CustomAction, CustomActionInfo};
use server::cultural_influence::available_influence_actions;
use server::happiness::available_happiness_actions;
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::resource::ResourceType;

pub(crate) fn action_buttons(rc: &RenderContext) -> RenderResult {
    let assets = rc.assets();
    let game = rc.game;

    let happiness = available_happiness_actions(rc.game, rc.shown_player.index);
    if !happiness.is_empty()
        && bottom_left_texture(
            rc,
            &assets.resources[&ResourceType::MoodTokens],
            icon_pos(0, -2),
            &MultilineText::of(rc, "Increase happiness"),
        )
    {
        return open_increase_happiness_dialog(rc, &happiness, |h| h);
    }

    if rc.can_play_action(&PlayingActionType::MoveUnits)
        && bottom_left_texture(
            rc,
            &assets.move_units,
            icon_pos(0, -3),
            &MultilineText::of(rc, "Move units"),
        )
    {
        return global_move(rc);
    }

    if rc.can_play_action(&PlayingActionType::Advance)
        && bottom_left_texture(
            rc,
            &assets.advances,
            icon_pos(1, -3),
            &MultilineText::of(rc, "Research advances"),
        )
    {
        return StateUpdate::open_dialog(ActiveDialog::AdvanceMenu);
    }

    let influence = available_influence_actions(game, rc.shown_player.index);

    if !influence.is_empty()
        && bottom_left_texture(
            rc,
            &assets.resources[&ResourceType::CultureTokens],
            icon_pos(1, -2),
            &MultilineText::of(rc, "Cultural Influence"),
        )
    {
        return base_or_custom_action(rc, &influence, "Influence culture", |d| {
            new_cultural_influence_dialog(rc.game, rc.shown_player.index, d)
        });
    }
    let mut i = 0;
    for c in game.available_custom_actions(rc.shown_player.index) {
        if let Some(action) = generic_custom_action(rc, &c, None) {
            if bottom_left_texture(
                rc,
                &assets.custom_actions[&c.action],
                icon_pos(i as i8, -1),
                &event_help_tooltip(rc, &c.event_origin),
            ) {
                return action;
            }
            i += 1;
        }
    }
    for (i, icon) in custom_action_buttons(rc, None).iter().enumerate() {
        if icon.with_rc(rc, |rc| {
            bottom_left_texture(rc, icon.texture, icon_pos(i as i8, -1), &icon.tooltip)
        }) {
            return (icon.action)();
        }
    }
    NO_UPDATE
}

pub(crate) fn custom_action_buttons<'a>(
    rc: &'a RenderContext,
    city: Option<&'a City>,
) -> IconActionVec<'a> {
    rc.game
        .available_custom_actions(rc.shown_player.index)
        .into_iter()
        .filter_map(|c| {
            generic_custom_action(rc, &c, city).map(|action| {
                IconAction::new(
                    &rc.assets().custom_actions[&c.action],
                    false,
                    event_help_tooltip(rc, &c.event_origin),
                    Box::new(move || action.clone()),
                )
            })
        })
        .collect()
}

fn global_move(rc: &RenderContext) -> RenderResult {
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
    c: &CustomActionInfo,
    city: Option<&City>,
) -> Option<RenderResult> {
    let custom_action_type = c.action;

    if let Some(city) = city {
        c.is_city_available(rc.game, city)
            .then_some(StateUpdate::execute(Action::Playing(
                PlayingAction::Custom(CustomAction::new(custom_action_type, Some(city.position))),
            )))
    } else {
        c.city_bound()
            .is_none()
            .then_some(StateUpdate::execute(Action::Playing(
                PlayingAction::Custom(CustomAction::new(custom_action_type, None)),
            )))
    }
}

pub(crate) fn base_or_custom_action(
    rc: &RenderContext,
    action_types: &[PlayingActionType],
    title: &str,
    execute: impl Fn(BaseOrCustomDialog) -> ActiveDialog,
) -> RenderResult {
    StateUpdate::dialog_chooser(
        &format!("Choose action: {title}"),
        action_types
            .iter()
            .map(|action_type| match action_type {
                PlayingActionType::Custom(c) => {
                    let origin = &rc.shown_player.custom_action_info(*c).event_origin;
                    (
                        Some(origin.clone()),
                        execute(BaseOrCustomDialog {
                            action_type: action_type.clone(),
                            title: format!("{title} with {}", origin.name(rc.game)),
                        }),
                    )
                }
                _ => (
                    None,
                    execute(BaseOrCustomDialog {
                        action_type: action_type.clone(),
                        title: title.to_string(),
                    }),
                ),
            })
            .collect_vec(),
    )
}
