use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::ActionCard;
use crate::content::action_cards::mercenaries::mercenaries;
use crate::content::builtin::Builtin;
use crate::content::tactics_cards::{for_the_people, heavy_resistance, TacticsCardFactory};
use crate::incident::PermanentIncidentEffect;
use crate::playing_actions::{ActionType, PlayingActionType};
use crate::tactics_card::TacticsCard;
use crate::utils::remove_element;

pub(crate) fn development_action_cards() -> Vec<ActionCard> {
    vec![
        mercenaries(13, for_the_people),
        mercenaries(14, heavy_resistance),
        cultural_takeover(15, heavy_resistance),
    ]
}

fn cultural_takeover(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Cultural Takeover",
        "You may influence Barbarian cities of size 1. \
        If successful, replace the Barbarian city with a city of your color. \
        Replace one of the Barbarian units with a Settler or Infantry of your color. \
        Remove the other Barbarian units.",
        ActionType::free(),
        |game, p| {
            true
        },
    )
        .add_simple_persistent_event_listener(
            |event| &mut event.on_play_action_card,
            0,
            |game, _player, _name, _a| {
                game.permanent_incident_effects.push(PermanentIncidentEffect::CulturalTakeover);
                game.add_info_log_item("Cultural Takeover: You may influence Barbarian cities of size 1.");
            },
        )
        .tactics_card(tactics_card)
        .build()
}

pub(crate) fn use_cultural_takeover() -> Builtin {
    Builtin::builder("cultural_takeover", "-")
        .add_transient_event_listener(
            |event| &mut event.is_playing_action_available,
            2,
            |available, game, i| {
                if game
                    .permanent_incident_effects
                    .contains(&PermanentIncidentEffect::CulturalTakeover)
                    && !matches!(i.action_type, PlayingActionType::InfluenceCultureAttempt)
                {
                    *available =
                        Err("Cultural Takeover: You may only influence culture.".to_string());
                }
            },
        )
        .add_transient_event_listener(
            |event| &mut event.on_influence_culture_attempt,
            1,
            |c, _, game| {
                //todo add resolution effect on_influence_culture_success
                if                 remove_element(
                                    &mut game.permanent_incident_effects,
                                    &PermanentIncidentEffect::CulturalTakeover,
                                )
                {
                    c.allow_barbarian = true;
                }
            },
        )
        .add_transient_event_listener(
            |event| &mut event.on_influence_culture_success,
            2,
            |game, _, _, _| {
                remove_element(
                    &mut game.permanent_incident_effects,
                    &PermanentIncidentEffect::CulturalTakeover,
                );
            },
        )
        .build()
}
