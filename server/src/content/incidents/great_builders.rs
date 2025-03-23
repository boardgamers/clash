use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::ActionCard;
use crate::card::HandCard;
use crate::content::builtin::Builtin;
use crate::content::custom_phase_actions::HandCardsRequest;
use crate::content::incidents::great_persons::{
    great_person_action_card, great_person_description, GREAT_PERSON_DESCRIPTION,
};
use crate::game::Game;
use crate::incident::PermanentIncidentEffect;
use crate::player::Player;
use crate::playing_actions::{ActionType, PlayingActionType};
use crate::utils::remove_element;
use crate::wonder::{cities_for_wonder, play_wonder_card, WonderCardInfo, WonderDiscount};

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
                    *available =
                        Err("Great Engineer: You may only construct buildings.".to_string());
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

const ARCHITECT_DISCOUNT: WonderDiscount = WonderDiscount::new(true, 3);

pub(crate) fn great_architect() -> ActionCard {
    great_person_action_card::<_, String>(
        55,
        "Great Architect",
        &format!(
            "{GREAT_PERSON_DESCRIPTION} When constructing a wonder, you may ignore \
                the requirement advances (but not Engineering). \
                In addition, the cost of constructing the wonder is reduced by 3 culture tokens.",
        ),
        ActionType::free(),
        &[],
        |game, player| !playable_wonders(game, player).is_empty(),
    )
    .add_hand_card_request(
        |e| &mut e.on_play_action_card,
        0,
        |game, player, _| {
            Some(HandCardsRequest::new(
                playable_wonders(game, game.get_player(player))
                    .iter()
                    .map(|name| HandCard::Wonder(name.clone()))
                    .collect(),
                1..=1,
                "Great Architect: Select a wonder to build",
            ))
        },
        |game, s, _| {
            let HandCard::Wonder(name) = &s.choice[0] else {
                panic!("Invalid choice");
            };
            play_wonder_card(
                game,
                s.player_index,
                WonderCardInfo::new(name.clone(), ARCHITECT_DISCOUNT),
            );
        },
    )
    .build()
}

fn playable_wonders(game: &Game, player: &Player) -> Vec<String> {
    player
        .wonder_cards
        .iter()
        .filter(|name| !cities_for_wonder(name, game, player, &ARCHITECT_DISCOUNT).is_empty())
        .cloned()
        .collect()
}
