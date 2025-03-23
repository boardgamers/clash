use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::Action;
use crate::action_card::ActionCard;
use crate::content::builtin::Builtin;
use crate::content::custom_actions::{CustomAction, CustomActionType};
use crate::content::incidents::great_persons::{
    great_person_action_card, great_person_description, GREAT_PERSON_DESCRIPTION,
};
use crate::incident::PermanentIncidentEffect;
use crate::playing_actions::{ActionType, PlayingAction, PlayingActionType};
use crate::utils::{remove_element, remove_element_by};

pub(crate) fn great_engineer() -> ActionCard {
    let groups = &["Construction"];
    great_person_action_card(
        26,
        "Great Engineer",
        &format!(
            "{} Then, you may build a building in a city without spending an action and without activating it.",
            great_person_description(groups)
        ),
        ActionType::regular(),
        groups,
        |_game, _player| true,
    )
        .add_bool_request(
            |e| &mut e.on_play_action_card,
            0,
            |_, _, _| Some("Build a building in a city without spending an action and without activating it?".to_string()),
            |game, s, _| {
                if s.choice {
                    game.permanent_incident_effects.push(PermanentIncidentEffect::GreatEngineer);
                    game.actions_left += 1; // to offset the action spent for building
                    game.add_info_log_item("Great Engineer: You may build a building in a city without spending an action and without activating it.");
                } else {
                    game.add_info_log_item("Great Engineer: You decided not to use the ability.");
                }
            },
        )
    .build()
}

pub(crate) fn use_great_engineer() -> Builtin {
    Builtin::builder("great_engineer", "-")
        .add_transient_event_listener(
            |event| &mut event.is_playing_action_available,
            2,
            |available, game, i| {
                if game
                    .permanent_incident_effects
                    .contains(&PermanentIncidentEffect::GreatEngineer)
                    && !matches!(i.action_type, PlayingActionType::Construct)
                {
                    *available = false;
                }
            },
        )
        .add_transient_event_listener(
            |event| &mut event.construct_cost,
            1,
            |c, _, game| {
                if game
                    .permanent_incident_effects
                    .contains(&PermanentIncidentEffect::GreatEngineer)
                {
                    c.activate_city = false;
                }
            },
        )
        .add_simple_persistent_event_listener(
            |event| &mut event.on_construct,
            2,
            |game, _, _, _| {
                remove_element(
                    &mut game.permanent_incident_effects,
                    &PermanentIncidentEffect::GreatEngineer,
                );
            },
        )
        .build()
}

pub(crate) fn great_architect() -> ActionCard {
    great_person_action_card::<_, String>(
        55,
        "Great Engineer",
        &format!(
            "{GREAT_PERSON_DESCRIPTION} When constructing a wonder, you may ignore \
            the requirement advances (but not Engineering). \
            In addition, the cost of constructing the wonder is reduced by 3 culture tokens.",
        ),
        ActionType::regular(),
        &[],
        |_game, _player| true,
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.on_play_action_card,
        0,
        |game, player, _, _| {
            game.permanent_incident_effects
                .push(PermanentIncidentEffect::GreatArchitect(player));
        },
    )
    .build()
}

pub(crate) fn use_great_architect() -> Builtin {
    let b = Builtin::builder("Great Architect", "-");
    let key = b.get_key();
    b.add_ability_initializer(move |game, player_index| {
        let player = &mut game.players[player_index];
        if game
            .permanent_incident_effects
            .iter()
            .any(|e| matches!(e, PermanentIncidentEffect::GreatArchitect(p) if *p == player_index))
        {
            player
                .custom_actions
                .insert(CustomActionType::GreatArchitect, key.clone());
        }
    })
    //todo remove effect after building
    // todo may discount 3 culture tokens
    // todo may ignore requirement advances (but not Engineering)
        .add_transient_event_listener(
            |event| &mut event.wonder_cost,
            0,
            |c, _, game| {
                if game
                    .permanent_incident_effects
                    .iter()
                    .any(|e| matches!(e, PermanentIncidentEffect::GreatArchitect(_)))
                {
                    c.cost.culture -= 3;
                }
            },
        )
    .add_simple_persistent_event_listener(
        |event| &mut event.on_construct_wonder,
        0,
        |game, _, _, _| {
            let architect_used = game.action_log.iter().rev().find_map(|action| {
                match action.action {
                    Action::Playing(PlayingAction::ConstructWonder(_)) => Some(false),
                    Action::Playing(PlayingAction::Custom(CustomAction::GreatArchitect(_))) => Some(true),
                    _ => None
                }
            }).expect("No wonder constructed");
            
            if architect_used {
                remove_element_by(
                    &mut game.permanent_incident_effects,
                    |e| matches!(e, PermanentIncidentEffect::GreatArchitect(_)),
                );
            }
        },
    )
    .build()
}
