use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::gain_action;
use crate::action_card::ActionCard;
use crate::collect::available_collect_actions_for_city;
use crate::construct::ConstructDiscount;
use crate::content::ability::Ability;
use crate::content::action_cards::cultural_takeover::cultural_takeover;
use crate::content::action_cards::mercenaries::mercenaries;
use crate::content::custom_actions::is_base_or_modifier;
use crate::content::effects::{CollectEffect, ConstructEffect, PermanentEffect};
use crate::content::incidents::great_builders::can_construct_any_building;
use crate::content::incidents::great_explorer::{action_explore_request, explore_adjacent_block};
use crate::content::persistent_events::PositionRequest;
use crate::content::tactics_cards::{
    TacticsCardFactory, defensive_formation, encircled, for_the_people, heavy_resistance,
    improved_defenses, peltasts, tactical_retreat,
};
use crate::game::Game;
use crate::player::{Player, gain_unit};
use crate::playing_actions::PlayingActionType;
use crate::unit::UnitType;
use crate::utils::remove_element_by;

pub(crate) fn development_action_cards() -> Vec<ActionCard> {
    vec![
        mercenaries(13, for_the_people),
        mercenaries(14, heavy_resistance),
        cultural_takeover(15, heavy_resistance),
        cultural_takeover(16, improved_defenses),
        city_development(17, tactical_retreat),
        city_development(18, peltasts),
        production_focus(19, tactical_retreat),
        production_focus(20, peltasts),
        explorer(21, encircled),
        explorer(22, defensive_formation),
    ]
}

fn city_development(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "City Development",
        "Construct a building without paying resources and without an action.",
        |c| c.action().culture_tokens(1),
        |game, p, _| can_construct_any_building(game, p, &[ConstructDiscount::NoResourceCost]),
    )
    .tactics_card(tactics_card)
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, p, _| {
            game.permanent_effects
                .push(PermanentEffect::Construct(ConstructEffect::CityDevelopment));
            gain_action(game, p); // to offset the action spent for building
            p.log(
                game,
                "You may build a building in a city without \
                spending an action and without paying resources.",
            );
        },
    )
    .build()
}

fn production_focus(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Production Focus",
        "For the next collect action, you may collect multiple times from the same tile. \
        The total amount of resources does not change.",
        |c| c.free_action().no_resources(),
        |game, player, _| collect_special_action(game, player),
    )
    .tactics_card(tactics_card)
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, p, _| {
            game.permanent_effects
                .push(PermanentEffect::Collect(CollectEffect::ProductionFocus));
            p.log(
                game,
                "Production Focus: You may collect multiple times from the same tile.",
            );
        },
    )
    .build()
}

pub(crate) fn collect_special_action(game: &Game, player: &Player) -> bool {
    !game
        .permanent_effects
        .iter()
        .any(|e| matches!(e, PermanentEffect::Collect(_)))
        && player
            .cities
            .iter()
            .any(|c| !available_collect_actions_for_city(game, player.index, c.position).is_empty())
}

pub(crate) fn collect_only() -> Ability {
    Ability::builder("collect only", "-")
        .add_transient_event_listener(
            |event| &mut event.is_playing_action_available,
            4,
            |available, game, t, p| {
                if game
                    .permanent_effects
                    .iter()
                    .any(|e| matches!(e, &PermanentEffect::Collect(_)))
                    && !is_base_or_modifier(t, p.get(game), &PlayingActionType::Collect)
                {
                    *available = Err("You may only collect.".to_string());
                }
            },
        )
        .add_transient_event_listener(
            |event| &mut event.collect_options,
            2,
            |c, _context, game, _| {
                if game
                    .permanent_effects
                    .iter()
                    .any(|e| matches!(e, &PermanentEffect::Collect(CollectEffect::ProductionFocus)))
                {
                    c.max_per_tile = c.max_selection;
                }
                if game
                    .permanent_effects
                    .iter()
                    .any(|e| matches!(e, &PermanentEffect::Collect(CollectEffect::MassProduction)))
                {
                    c.max_selection += 2;
                }
            },
        )
        .add_simple_persistent_event_listener(
            |event| &mut event.collect,
            2,
            |game, _, _| {
                remove_element_by(&mut game.permanent_effects, |e| {
                    matches!(e, &PermanentEffect::Collect(_))
                });
            },
        )
        .build()
}

fn explorer(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    let b = ActionCard::builder(
        id,
        "Explorer",
        "Explore a tile adjacent to a one of your cities - \
        AND/OR gain a free settler in one of your cities.",
        |c| c.action().culture_tokens(1),
        |game, player, _| {
            !action_explore_request(game, player.index)
                .choices
                .is_empty()
        },
    )
    .tactics_card(tactics_card);

    explore_adjacent_block(b)
        .add_position_request(
            |e| &mut e.play_action_card,
            0,
            |game, p, _| {
                let p = p.get(game);
                if p.available_units().settlers == 0 {
                    return None;
                }
                Some(PositionRequest::new(
                    p.cities.iter().map(|c| c.position).collect(),
                    0..=1,
                    "Gain a free settler",
                ))
            },
            |game, s, _a| {
                if !s.choice.is_empty() {
                    gain_unit(game, &s.player(), s.choice[0], UnitType::Settler);
                }
            },
        )
        .build()
}
