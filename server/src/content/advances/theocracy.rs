use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::city_pieces::Building::Temple;
use crate::construct::ConstructAdvanceBonus;
use crate::content::advances::{AdvanceGroup, AdvanceGroupInfo, advance_group_builder};
use crate::content::persistent_events::{AdvanceRequest, PositionRequest};
use crate::player::{Player, can_add_army_unit, gain_unit};
use crate::position::Position;
use crate::resource::apply_resource_limit;
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;

pub(crate) fn theocracy() -> AdvanceGroupInfo {
    advance_group_builder(
        AdvanceGroup::Theocracy,
        "Theocracy",
        vec![dogma(), devotion(), conversion(), fanaticism()],
    )
}

fn dogma() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Dogma,
        "Dogma",
        "Whenever you Construct a new Temple, \
        you may immediately get a Theocracy Advance for free. \
        You are now limited to a maximum of 2 ideas (discard if necessary). \
        Note: Dogma Advance does not apply when you conquer a city with a Temple.",
    )
    .add_once_initializer(move |game, player| {
        player.log(game, "Ideas limit reduced to 2");
        let p = player.get_mut(game);
        p.resource_limit.ideas = 2;
        apply_resource_limit(p);
    })
    .add_once_deinitializer(|game, player| {
        player.get_mut(game).resource_limit.ideas = 7;
    })
    .add_advance_request(
        |event| &mut event.construct,
        0,
        |game, p, building| {
            if matches!(building.building, Temple) {
                let player = p.get(game);
                let choices: Vec<Advance> = game
                    .cache
                    .get_advance_group(AdvanceGroup::Theocracy)
                    .advances
                    .iter()
                    .filter(|a| player.can_advance_free(a.advance, game))
                    .map(|a| a.advance)
                    .collect();
                if choices.is_empty() {
                    return None;
                }
                return Some(AdvanceRequest::new(choices));
            }
            None
        },
        |_game, s, i| {
            // the advance may trigger the Anarchy incident, which will remove Dogma
            // this needs to happen after the Dogma listener is processed
            i.gained_advance = Some(ConstructAdvanceBonus::new(s.choice, s.origin.clone()));
        },
    )
}

fn devotion() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Devotion,
        "Devotion",
        "Attempts to influence your cities with a Temple may not be boosted by culture tokens",
    )
    .add_transient_event_listener(
        |event| &mut event.on_influence_culture_attempt,
        4,
        |r, city, _, _| {
            if let Ok(info) = r {
                if info.is_defender && city.pieces.temple.is_some() {
                    info.set_no_boost();
                }
            }
        },
    )
}

fn conversion() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Conversion,
        "Conversion",
        "You add +1 to your Influence Culture roll \
        and gain 1 culture token when you make a successful Influence Culture attempt.",
    )
    .add_transient_event_listener(
        |event| &mut event.on_influence_culture_attempt,
        3,
        |r, _, _, _| {
            if let Ok(info) = r {
                if !info.is_defender {
                    info.roll_boost += 1;
                    info.info.log.push(
                        "Player gets +1 to Influence Culture roll for Conversion Advance"
                            .to_string(),
                    );
                }
            }
        },
    )
    .add_transient_event_listener(
        |event| &mut event.on_influence_culture_resolve,
        0,
        |game, outcome, (), p| {
            if outcome.success {
                p.gain_resources(game, ResourcePile::culture_tokens(1));
            }
        },
    )
}

fn fanaticism() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Fanaticism,
        "Fanaticism",
        "During a battle in a city with a Temple, \
        whether you are the attacker or defender, you add +2 combat value to your first combat roll. \
        If you lose the battle, you get 1 free Infantry Unit after the battle and \
        place it in one of your cities.",
    )
        .add_combat_strength_listener(1, |game, c, s, _role| {
            if c.first_round() && c.defender_temple(game) {
                s.extra_combat_value += 2;
                s.roll_log
                    .push("Player gets +2 combat value for Fanaticism Advance".to_string());
            }
        })
        .add_position_request(
            |event| &mut event.combat_end,
            104,
            |game, p, i| {
                let player_index = p.index;
                if i.is_loser(player_index)
                    && !game.player(player_index).cities.is_empty()
                    && game.player(player_index).available_units().infantry > 0
                {
                    let p = game.player(player_index);
                    let choices = cities_that_can_add_units(p);
                    let needed = 1..=1;
                    Some(PositionRequest::new(
                        choices,
                        needed,
                        "Select a city to place the free Infantry Unit",
                    ))
                } else {
                    None
                }
            },
            |game, s, _| {
                gain_unit(game, s.player_index, s.choice[0], UnitType::Infantry, &s.origin);
            },
        )
}

pub(crate) fn cities_that_can_add_units(p: &Player) -> Vec<Position> {
    let choices: Vec<Position> = p
        .cities
        .iter()
        .filter(|c| can_add_army_unit(p, c.position))
        .map(|c| c.position)
        .collect();
    choices
}
