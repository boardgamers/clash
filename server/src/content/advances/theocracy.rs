use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo, gain_advance_without_payment};
use crate::city_pieces::Building::Temple;
use crate::consts::STACK_LIMIT;
use crate::content::advances::{AdvanceGroup, advance_group_builder, get_group};
use crate::content::persistent_events::{AdvanceRequest, PositionRequest};
use crate::player::{Player, add_unit};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;

pub(crate) fn theocracy() -> AdvanceGroup {
    advance_group_builder(
        "Theocracy",
        vec![dogma(), devotion(), conversion(), fanaticism()],
    )
}

fn dogma() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Dogma,
        "Dogma",
        "Whenever you Construct a new Temple, \
        you may immediately get a Theocracy Advance for free, \
        You are now limited to a maximum of 2 ideas (discard if necessary) \
        Note: Dogma Advance does not apply when you conquer a city with a Temple.",
    )
    .add_one_time_ability_initializer(|game, player_index| {
        let p = &mut game.players[player_index];
        p.resource_limit.ideas = 2;
        p.gain_resources(ResourcePile::ideas(0)); // to trigger the limit
        let name = p.get_name();
        game.add_info_log_item(&format!(
            "{name} is now limited to a maximum of 2 ideas for Dogma Advance",
        ));
    })
    .add_ability_undo_deinitializer(|game, player_index| {
        game.players[player_index].resource_limit.ideas = 7;
    })
    .add_advance_request(
        |event| &mut event.construct,
        0,
        |game, player_index, building| {
            if matches!(building, Temple) {
                let player = game.player(player_index);
                let choices: Vec<Advance> = get_group("Theocracy")
                    .advances
                    .iter()
                    .filter(|a| player.can_advance_free(a.advance))
                    .map(|a| a.advance)
                    .collect();
                if choices.is_empty() {
                    return None;
                }
                return Some(AdvanceRequest::new(choices));
            }
            None
        },
        |game, c, _| {
            let verb = if c.actively_selected {
                "selected"
            } else {
                "got"
            };
            game.add_info_log_item(&format!(
                "{} {verb} {} as a reward for constructing a Temple",
                c.player_name, c.choice
            ));
            gain_advance_without_payment(
                game,
                c.choice,
                c.player_index,
                ResourcePile::empty(),
                true,
            );
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
        |r, city, _| {
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
        |r, _, _| {
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
        |game, outcome, ()| {
            if outcome.success {
                game.player_mut(outcome.player)
                    .gain_resources(ResourcePile::culture_tokens(1));
                game.add_info_log_item(
                    "Player gained 1 culture token for a successful \
                    Influence Culture attempt for Conversion Advance",
                );
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
    .add_combat_round_start_listener(1, |game, c, s, _role| {
        if c.round == 1 && c.defender_temple(game) {
            s.extra_combat_value += 2;
            s.roll_log
                .push("Player gets +2 combat value for Fanaticism Advance".to_string());
        }
    })
    .add_position_request(
        |event| &mut event.combat_end,
        104,
        |game, player_index, i| {
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
            let pos = s.choice[0];
            game.add_info_log_item(&format!(
                "{} gained 1 free Infantry Unit at {} for Fanaticism Advance",
                s.player_name, pos
            ));
            add_unit(s.player_index, pos, UnitType::Infantry, game);
        },
    )
}

pub(crate) fn cities_that_can_add_units(p: &Player) -> Vec<Position> {
    let choices: Vec<Position> = p
        .cities
        .iter()
        .filter(|c| {
            p.get_units(c.position)
                .iter()
                .filter(|u| u.unit_type.is_army_unit())
                .count()
                < STACK_LIMIT
        })
        .map(|c| c.position)
        .collect();
    choices
}
