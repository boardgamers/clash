// Monumental Edition references:
// https://boardgamegeek.com/image/7552363/mpappy (civilization board)
// https://boardgamegeek.com/image/7552364/mpappy (leaders)
// Rulebook p. 32 clarifies Canals and Great Gardens.
use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::gain_action_card_from_pile;
use crate::advance::Advance;
use crate::city_pieces::Building;
use crate::civilization::Civilization;
use crate::content::advances::theocracy::cities_that_can_add_units;
use crate::content::persistent_events::{PositionRequest, ResourceRewardRequest};
use crate::game::Game;
use crate::leader::{Leader, LeaderInfo, leader_position};
use crate::leader_ability::LeaderAbility;
use crate::payment::PaymentConversion;
use crate::player::{Player, can_add_army_unit, gain_unit};
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::tactics_card::CombatRole;
use crate::unit::{UnitType, carried_units, ship_capacity};
use crate::wonder::Wonder;

pub(crate) fn babylonia() -> Civilization {
    Civilization::new(
        "Babylonia",
        vec![canals(), code_of_laws(), star_catalogues(), ziggurats()],
        vec![hammurabi(), nebuchadnezzar(), nabopolassar()],
        None,
    )
}

fn canals() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Canals,
        SpecialAdvanceRequirement::Advance(Advance::Engineering),
        "Canals",
        "When collecting exactly 1 food, gain 1 extra food. You may also collect other resources.",
    )
    .add_transient_event_listener(
        |events| &mut events.collect_total,
        -10,
        |info, _, _, player| {
            if info.total.food == 1 {
                info.total.food += 1;
                info.info.add_log(player, "Added 1 food");
            }
        },
    )
    .build()
}

fn code_of_laws() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::CodeOfLaws,
        SpecialAdvanceRequirement::Advance(Advance::Writing),
        "Code of Laws",
        "After a successful Cultural Influence attempt, draw 1 action card if you have 4 or fewer.",
    )
    .add_transient_event_listener(
        |events| &mut events.on_influence_culture_resolve,
        10,
        |game, outcome, (), player| {
            if outcome.success && player.get(game).action_cards.len() <= 4 {
                gain_action_card_from_pile(game, player);
            }
        },
    )
    .build()
}

fn star_catalogues() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::StarCatalogues,
        SpecialAdvanceRequirement::Advance(Advance::Astronomy),
        "Star Catalogues",
        "Before drawing an event, gain 1 idea or 1 culture token.",
    )
    .add_resource_request(
        |events| &mut events.choose_incident,
        10,
        |_game, player, _| {
            Some(ResourceRewardRequest::new(
                player
                    .reward_options()
                    .sum(1, &[ResourceType::Ideas, ResourceType::CultureTokens]),
                "Star Catalogues · Before the event".to_string(),
            ))
        },
    )
    .build()
}

fn ziggurats() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Ziggurats,
        SpecialAdvanceRequirement::Advance(Advance::Dogma),
        "Ziggurats",
        "Keep the normal idea limit with Dogma. When building a Temple, ideas may replace any resources.",
    )
    .add_transient_event_listener(
        |events| &mut events.building_cost,
        10,
        |info, building, _, player| {
            if *building == Building::Temple {
                for resource in [ResourceType::Food, ResourceType::Wood, ResourceType::Ore] {
                    info.cost.conversions.push(PaymentConversion::unlimited(
                        ResourcePile::of(resource, 1), ResourcePile::ideas(1),
                    ));
                }
                info.info.add_log(player, "Ideas may replace Temple resources");
            }
        },
    )
    .build()
}

fn hammurabi() -> LeaderInfo {
    LeaderInfo::new(
        Leader::Hammurabi,
        "Hammurabi",
        // The city-specific payment is part of the normal Increase Happiness action.
        LeaderAbility::builder(
            "Lawgiver",
            "When increasing happiness, pay 1 culture token to make Hammurabi's city happy instead of its normal mood cost.",
        ).build(),
        LeaderAbility::builder(
            "Guardian",
            "Building a Fortress in Hammurabi's city costs no food or wood.",
        )
        .add_transient_event_listener(
            |events| &mut events.building_cost,
            11,
            |info, building, game, player| {
                if *building == Building::Fortress
                    && info.city_position == Some(leader_position(player.get(game)))
                {
                    info.cost.default.food = 0;
                    info.cost.default.wood = 0;
                    info.info.add_log(player, "Fortress costs no food or wood in Hammurabi's city");
                }
            },
        )
        .build(),
    )
}

fn nebuchadnezzar() -> LeaderInfo {
    LeaderInfo::new(
        Leader::Nebuchadnezzar,
        "Nebuchadnezzar II",
        LeaderAbility::wonder_expert(Wonder::GreatGardens),
        LeaderAbility::builder(
            "Iconoclast",
            "When attacking a city with a Temple, Nebuchadnezzar adds 2 combat value.",
        )
        .add_combat_strength_listener(106, |game, combat, strength, role| {
            if role == CombatRole::Attacker
                && combat.has_leader(role, game)
                && combat.defender_temple(game)
            {
                strength.extra_combat_value += 2;
                strength
                    .roll_log
                    .push("Iconoclast adds +2 combat value".to_string());
            }
        })
        .build(),
    )
}

fn nabopolassar() -> LeaderInfo {
    LeaderInfo::new(
        Leader::Nabopolassar,
        "Nabopolassar",
        LeaderAbility::builder(
            "Liberator",
            "Nabopolassar adds 2 combat value against a player with at least as many cities as you.",
        )
        .add_combat_strength_listener(107, |game, combat, strength, role| {
            let player = game.player(combat.player(role));
            let opponent = game.player(combat.opponent(player.index));
            if combat.has_leader(role, game) && opponent.is_human() && opponent.cities.len() >= player.cities.len() {
                strength.extra_combat_value += 2;
                strength.roll_log.push("Liberator adds +2 combat value".to_string());
            }
        })
        .build(),
        LeaderAbility::builder(
            "Revolt",
            "When one of your cities is captured, place a free infantry with Nabopolassar. If there is no room, place it in one of your cities with space.",
        )
        .add_position_request(
            |events| &mut events.combat_end,
            106,
            |game, player, combat| {
                let p = player.get(game);
                if !combat.is_defender(p.index) || !combat.is_loser(p.index)
                    || !combat.battleground.is_city() || p.available_units().infantry == 0
                { return None; }
                let position = leader_position(p);
                let carrier = reinforcement_carrier(game, p);
                if can_add_army_unit(p, position) && (!game.map.is_sea(position) || carrier.is_some()) {
                    let unit_id = p.next_unit_id;
                    gain_unit(game, player, position, UnitType::Infantry);
                    player.get_mut(game).get_unit_mut(unit_id).carrier_id = carrier;
                    return None;
                }
                let choices = cities_that_can_add_units(p);
                if choices.is_empty() { return None; }
                Some(PositionRequest::new(choices, 1..=1, "Place the Revolt infantry"))
            },
            |game, selection, _| gain_unit(game, &selection.player(), selection.choice[0], UnitType::Infantry),
        )
        .build(),
    )
}

fn reinforcement_carrier(game: &Game, player: &Player) -> Option<u32> {
    let position = leader_position(player);
    if !game.map.is_sea(position) {
        return None;
    }
    player
        .units
        .iter()
        .find(|unit| {
            unit.position == position
                && unit.is_ship()
                && carried_units(unit.id, player).len() < ship_capacity(player) as usize
        })
        .map(|unit| unit.id)
}
