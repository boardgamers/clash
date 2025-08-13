use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::city_pieces::Building::Port;
use crate::content::ability::Ability;
use crate::content::advances::{AdvanceGroup, AdvanceGroupInfo, advance_group_builder};
use crate::game::{Game, GameOptions};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use std::collections::HashSet;

pub(crate) fn seafaring(options: &GameOptions) -> AdvanceGroupInfo {
    advance_group_builder(
        AdvanceGroup::Seafaring,
        "Seafaring",
        options,
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
    .add_combat_strength_listener(5, |game, c, s, role| {
        if c.first_round() && (c.is_disembarking_attacker(role, game) || c.is_sea_battle(game)) {
            s.hit_cancels += 1;
            s.roll_log
                .push("WarShips ignore the first hit in the first round of combat".to_string());
        }
    })
}

fn cartography() -> AdvanceBuilder {
    let mut b = AdvanceInfo::builder(
        Advance::Cartography,
        "Cartography",
        "Gain 1 idea after a move action where you moved a Ship. \
        If you used navigation, gain an additional 1 culture token.",
    )
    .with_advance_bonus(CultureToken);
    b = add_cartography_bonus(b, 0, "Cartography", |_| true, ResourcePile::ideas(1));
    b = add_cartography_bonus(
        b,
        1,
        "Cartography-navigation",
        |navigation| navigation,
        ResourcePile::culture_tokens(1),
    );
    b
}

fn add_cartography_bonus(
    b: AdvanceBuilder,
    priority: i32,
    key: &'static str,
    pred: impl Fn(bool) -> bool + Sync + Clone + Send + 'static,
    bonus: ResourcePile,
) -> AdvanceBuilder {
    b.add_transient_event_listener(
        |event| &mut event.before_move,
        priority,
        move |game, i, (), p| {
            if game.map.is_land(i.from) {
                // ship construction (or no ship at all)
                return;
            }

            // info is the action that we last used this ability for
            let val = game.actions_left.to_string();
            if p.get(game)
                .event_info
                .get(key)
                .is_some_and(|info| info == &val)
            {
                return;
            }
            let mut ship = false;
            let mut navigation = false;
            for id in &i.units {
                let unit = p.get(game).get_unit(*id);
                if unit.is_ship() {
                    ship = true;
                    if !unit.position.is_neighbor(i.to) {
                        navigation = true;
                    }
                }
            }

            if ship && pred(navigation) {
                p.get_mut(game).event_info.insert(key.to_string(), val);
                p.gain_resources(game, bonus.clone());
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
            |i, c, game, _| {
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
