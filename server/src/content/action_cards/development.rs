use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::ActionCard;
use crate::content::action_cards::cultural_takeover::cultural_takeover;
use crate::content::action_cards::mercenaries::mercenaries;
use crate::content::tactics_cards::{for_the_people, heavy_resistance, improved_defenses, tactical_retreat, TacticsCardFactory};
use crate::incident::ConstructEffect;
use crate::incident::PermanentIncidentEffect::Construct;
use crate::playing_actions::ActionType;
use crate::resource_pile::ResourcePile;

pub(crate) fn development_action_cards() -> Vec<ActionCard> {
    vec![
        mercenaries(13, for_the_people),
        mercenaries(14, heavy_resistance),
        cultural_takeover(15, heavy_resistance),
        cultural_takeover(16, improved_defenses),
        city_development(17, tactical_retreat)
    ]
}

fn city_development(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "City Development",
        "Construct a building without paying resources.",
        ActionType::regular_with_cost(ResourcePile::culture_tokens(1)),
        |_game, _player| true ,
    )
    .tactics_card(tactics_card)
    .add_simple_persistent_event_listener(
        |e| &mut e.on_play_action_card,
        0,
        |game, _player, _name, _| {
            game.permanent_incident_effects.push(Construct(ConstructEffect::CityDevelopment));
            game.actions_left += 1; // to offset the action spent for building
            game.add_info_log_item("City Development: You may build a building in a city without \
            spending an action and without paying for it.");
        },
    )
    .build()
}

