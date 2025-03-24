use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::{gain_advance, Advance, AdvanceBuilder};
use crate::city_pieces::Building::Temple;
use crate::consts::STACK_LIMIT;
use crate::content::advances::{advance_group_builder, get_group, AdvanceGroup};
use crate::content::custom_phase_actions::{AdvanceRequest, PositionRequest};
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
    Advance::builder("Dogma", "Whenever you Construct a new Temple, either through the Construct Action or through playing of cards, you may immediately get a Theocracy Advance for free, marking it with a cube from your Event tracker as normal.
    You are now limited to a maximum of 2 ideas. If you have more than 2 ideas when
    getting this Advance, you must immediately reduce down to 2.
    Note: Dogma Advance does not apply when you conquer a city with a Temple.")
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
            |event| &mut event.on_construct,
            0,
            |game, player_index, building| {
                if matches!(building, Temple) {
                    let player = game.get_player(player_index);
                    let choices: Vec<String> = get_group("Theocracy").advances
                        .iter()
                        .filter(|a| player.can_advance_free(a))
                        .map(|a| a.name.clone())
                        .collect();
                    if choices.is_empty() {
                        return None;
                    }
                    return Some(AdvanceRequest::new(choices));
                }
                None
            },
            |game, c,_| {
                let verb = if c.actively_selected {
                    "selected"
                } else {
                    "got"
                };
                game.add_info_log_item(&format!(
                    "{} {verb} {} as a reward for constructing a Temple", c.player_name, c.choice
                ));
                gain_advance(game, &c.choice, c.player_index, ResourcePile::empty(), true);
            },
        )
}

fn devotion() -> AdvanceBuilder {
    Advance::builder(
        "Devotion",
        "Attempts to influence your cities with a Temple may not be boosted by culture tokens",
    )
    .add_transient_event_listener(
        |event| &mut event.on_influence_culture_attempt,
        4,
        |info, city, _| {
            if info.is_defender && city.pieces.temple.is_some() {
                info.set_no_boost();
            }
        },
    )
}

fn conversion() -> AdvanceBuilder {
    Advance::builder("Conversion", "You add +1 to your Influence Culture roll and gain 1 culture token when you make a successful Influence Culture attempt.")
        .add_transient_event_listener(
            |event| &mut event.on_influence_culture_attempt,
            3,
            |info, _, _| {
                if !info.is_defender {
                    info.roll_boost += 1;
                    info.info.log.push("Player gets +1 to Influence Culture roll for Conversion Advance".to_string());
                }
            },
        )
        .add_transient_event_listener(
            |event| &mut event.on_influence_culture_success,
            0,
            |game, player, ()| {
                game.get_player_mut(*player).gain_resources(ResourcePile::culture_tokens(1));
                game.add_info_log_item("Player gained 1 culture token for a successful Influence Culture attempt for Conversion Advance");
            },
        )
}

fn fanaticism() -> AdvanceBuilder {
    Advance::builder("Fanaticism", "During a battle in a city with a Temple, whether you are the attacker or defender, you add +2 combat value to your first combat roll. If you lose the battle, you get 1 free Infantry Unit after the battle and place it in one of your cities.")
        .add_combat_round_start_listener(1, |game, c, s, _role| {
                if c.round == 1 && c.defender_temple(game) {
                    s.extra_combat_value += 2;
                    s.roll_log.push("Player gets +2 combat value for Fanaticism Advance".to_string());
                }
            },
        )
        .add_position_request(
            |event| &mut event.on_combat_end,
            4,
            |game, player_index, i| {
                if i.is_loser(player_index)
                    && !game.get_player(player_index).cities.is_empty()
                    && game.get_player(player_index).available_units().infantry > 0 {
                    let p = game.get_player(player_index);
                    let choices: Vec<Position> = p.cities.iter()
                        .filter(|c| p.get_units(c.position).iter().filter(|u| u.unit_type.is_army_unit()).count() < STACK_LIMIT)
                        .map(|c| c.position)
                        .collect();
                    let needed = 1..=1;
                    Some(PositionRequest::new(choices, needed, "Select a city to place the free Infantry Unit"))
                } else {
                    None
                }
            },
            |game, s,_| {
                let pos = s.choice[0];
                game.add_info_log_item(&format!(
                    "{} gained 1 free Infantry Unit at {} for Fanaticism Advance", s.player_name, pos
                ));
                game.get_player_mut(s.player_index).add_unit(pos, UnitType::Infantry);
            },
        )
}
