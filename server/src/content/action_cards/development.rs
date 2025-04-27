use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::ActionCard;
use crate::content::action_cards::cultural_takeover::cultural_takeover;
use crate::content::action_cards::mercenaries::mercenaries;
use crate::content::builtin::Builtin;
use crate::content::custom_actions::CustomActionType;
use crate::content::effects::{CollectEffect, ConstructEffect, PermanentEffect};
use crate::content::incidents::great_builders::can_construct_anything;
use crate::content::incidents::great_explorer::{action_explore_request, explore_adjacent_block};
use crate::content::persistent_events::PositionRequest;
use crate::content::tactics_cards::{
    TacticsCardFactory, defensive_formation, encircled, for_the_people, heavy_resistance,
    improved_defenses, peltasts, tactical_retreat,
};
use crate::player::add_unit;
use crate::player_events::PlayingActionInfo;
use crate::playing_actions::{ActionCost, PlayingActionType};
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;

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
        "Construct a building without paying resources.",
        ActionCost::regular_with_cost(ResourcePile::culture_tokens(1)),
        |game, player, _| can_construct_anything(game, player),
    )
    .tactics_card(tactics_card)
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, _player, _name, _| {
            game.permanent_effects
                .push(PermanentEffect::Construct(ConstructEffect::CityDevelopment));
            game.actions_left += 1; // to offset the action spent for building
            game.add_info_log_item(
                "City Development: You may build a building in a city without \
                spending an action and without paying for it.",
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
        ActionCost::regular(),
        |_game, _player, _| true,
    )
    .tactics_card(tactics_card)
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, _player, _name, _| {
            game.permanent_effects
                .push(PermanentEffect::Collect(CollectEffect::ProductionFocus));
            game.actions_left += 1; // to offset the action spent for collecting
            game.add_info_log_item(
                "Production Focus: You may collect multiple times from the same tile.",
            );
        },
    )
    .build()
}

pub(crate) fn collect_only() -> Builtin {
    Builtin::builder("collect only", "-")
        .add_transient_event_listener(
            |event| &mut event.is_playing_action_available,
            4,
            |available, game, i| {
                if game
                    .permanent_effects
                    .iter()
                    .any(|e| matches!(e, &PermanentEffect::Collect(_)))
                    && !is_collect(i)
                {
                    *available = Err("You may only collect.".to_string());
                }
            },
        )
        .add_transient_event_listener(
            |event| &mut event.collect_options,
            2,
            |c, _context, game| {
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
                    .any(|e| matches!(e, &PermanentEffect::Collect(CollectEffect::Overproduction)))
                {
                    c.max_selection += 2;
                }
            },
        )
        .build()
}

fn is_collect(i: &PlayingActionInfo) -> bool {
    match &i.action_type {
        PlayingActionType::Collect => true,
        PlayingActionType::Custom(c) if *c == CustomActionType::FreeEconomyCollect => true,
        _ => false,
    }
}

fn explorer(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    let b = ActionCard::builder(
        id,
        "Explorer",
        "Explore a tile adjacent to a one of your cities - \
        AND/OR gain a free settler in one of your cities.",
        ActionCost::regular_with_cost(ResourcePile::culture_tokens(1)),
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
            |game, player_index, _| {
                let p = game.player(player_index);
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
                if s.choice.is_empty() {
                    game.add_info_log_item(&format!(
                        "{} decided not to gain a free settler",
                        s.player_name
                    ));
                } else {
                    let pos = s.choice[0];
                    game.add_info_log_item(&format!(
                        "{} decided to gain a free settler at {}",
                        s.player_name, pos
                    ));
                    add_unit(s.player_index, pos, UnitType::Settler, game);
                }
            },
        )
        .build()
}
