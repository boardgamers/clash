use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::city_pieces::Building::Port;
use crate::content::ability::Ability;
use crate::content::advances::{AdvanceGroup, advance_group_builder};
use crate::game::Game;
use crate::player::gain_resources;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use std::collections::HashSet;

pub(crate) fn seafaring() -> AdvanceGroup {
    advance_group_builder(
        "Seafaring",
        vec![fishing(), navigation(), war_ships(), cartography()],
    )
}

fn fishing() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Fishing,
        "Fishing",
        "Your cities may Collect food from one Sea space",
    )
    .with_advance_bonus(MoodToken)
    .with_unlocked_building(Port)
}

fn navigation() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Navigation,
        "Navigation",
        "Ships may leave the map and return at the next sea space",
    )
    .with_advance_bonus(CultureToken)
}

fn war_ships() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::WarShips,
        "WarShips",
        "Ignore the first hit it the first round of combat \
        when attacking with Ships or disembarking from Ships",
    )
    .add_combat_strength_listener(5, |g, c, s, role| {
        let st = &c.stats;
        let disembark = role.is_attacker() && g.map.is_sea(st.attacker.position);
        let sea_battle = g.map.is_sea(st.defender.position);
        if c.first_round() && (disembark || sea_battle) {
            s.hit_cancels += 1;
            s.roll_log
                .push("WarShips ignore the first hit in the first round of combat".to_string());
        }
    })
}

fn cartography() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Cartography,"Cartography",
        "Gain 1 idea after a move action where you moved a Ship. If you used navigation, gain an additional 1 culture token.", )
        .with_advance_bonus(CultureToken)
        .add_transient_event_listener(
            |event| &mut event.before_move,
            0,
            |game,  i, ()| {
                // info is the action that we last used this ability for
                let key = game.actions_left.to_string();
                if game.player(i.player).event_info.get("Cartography").is_some_and(|info| info == &key) {
                    return;
                }
                let mut ship = false;
                let mut navigation = false;
                for id in &i.units {
                    let unit = game.player(i.player).get_unit(*id);
                    if unit.is_ship() {
                        ship = true;
                        if !unit.position.is_neighbor(i.to) {
                            navigation = true;
                        }
                    }
                }
                if ship {
                    game.player_mut(i.player).event_info.insert("Cartography".to_string(), key);
                    gain_resources(
                        game,
                        i.player,
                        ResourcePile::ideas(1),
                        |name, pile| format!("{name} gained {pile} from Cartography"),
                    );
                    if navigation {
                        gain_resources(
                            game,
                            i.player,
                            ResourcePile::culture_tokens(1),
                            |name, pile| format!("{name} gained {pile} from Cartography"),
                        );
                    }
                }
            },
        )
}

#[must_use]
fn is_enemy_player_or_pirate_zone(game: &Game, player_index: usize, position: Position) -> bool {
    game.enemy_player(player_index, position).is_some() || game.is_pirate_zone(position)
}

pub(crate) fn fishing_collect() -> Ability {
    Ability::builder("Fishing", "")
        .add_transient_event_listener(
            |event| &mut event.collect_options,
            1,
            |i, c, game| {
                let city = game.get_any_city(c.city_position);
                let port = city.port_position;
                let fishing = game.player(c.player_index).has_advance(Advance::Fishing);
                if !fishing && port.is_none() {
                    // short circuit
                    return;
                }

                if let Some(position) = port
                    .filter(|p| !is_enemy_player_or_pirate_zone(game, c.player_index, *p))
                    .or_else(|| {
                        c.city_position.neighbors().into_iter().find(|p| {
                            game.map.is_sea(*p)
                                && !is_enemy_player_or_pirate_zone(game, c.player_index, *p)
                        })
                    })
                {
                    i.choices.insert(
                        position,
                        if Some(position) == port {
                            if fishing {
                                HashSet::from([
                                    ResourcePile::food(1),
                                    ResourcePile::gold(1),
                                    ResourcePile::mood_tokens(1),
                                ])
                            } else {
                                HashSet::from([ResourcePile::gold(1), ResourcePile::mood_tokens(1)])
                            }
                        } else if fishing {
                            HashSet::from([ResourcePile::food(1)])
                        } else {
                            HashSet::new()
                        },
                    );
                }
            },
        )
        .build()
}
