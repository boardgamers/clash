use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder};
use crate::city_pieces::Building::Port;
use crate::collect::{CollectContext, CollectOptionsInfo};
use crate::content::advances::{advance_group_builder, AdvanceGroup, NAVIGATION};
use crate::game::Game;
use crate::resource_pile::ResourcePile;
use std::collections::HashSet;

pub(crate) fn seafaring() -> AdvanceGroup {
    advance_group_builder(
        "Seafaring",
        vec![
            Advance::builder("Fishing", "Your cities may Collect food from one Sea space")
                .add_player_event_listener(|event| &mut event.collect_options, fishing_collect, 0)
                .with_advance_bonus(MoodToken)
                .with_unlocked_building(Port),
            Advance::builder(
                NAVIGATION,
                "Ships may leave the map and return at the next sea space",
            )
            .with_advance_bonus(CultureToken),
            war_ships(),
            cartography(),
        ],
    )
}

fn war_ships() -> AdvanceBuilder {
    Advance::builder(
        "War Ships",
        "Ignore the first hit it the first round of combat when attacking with Ships or disembarking from Ships")
        .add_player_event_listener(
            |event| &mut event.on_combat_round,
            |s, c, g| {
                let attacker = s.attacker && g.map.is_water(c.attacker_position);
                let defender = !s.attacker && g.map.is_water(c.defender_position);
                if c.round == 1 && (attacker || defender) {
                    s.hit_cancels += 1;
                    s.roll_log.push("War Ships ignore the first hit in the first round of combat".to_string());
                }
            },
            0,
        )
}

fn cartography() -> AdvanceBuilder {
    Advance::builder(
        "Cartography",
        "Gain 1 idea after a move action where you moved a Ship. If you used navigation, gain an additional 1 culture token.", )
        .with_advance_bonus(CultureToken)
        .add_player_event_listener(
            |event| &mut event.before_move,
            |player, g, i| {
                // info is the action that we last used this ability for
                let key = g.actions_left.to_string();
                if player.info.get("Cartography").is_some_and(|info| info == &key) {
                    return;
                }
                let mut ship = false;
                let mut navigation = false;
                for id in &i.units {
                    let unit = g.get_player(i.player).get_unit(*id).expect("unit should exist");
                    if unit.unit_type.is_ship() {
                        ship = true;
                        if !unit.position.is_neighbor(i.to) {
                            navigation = true;
                        }
                    }
                }
                if ship {
                    player.info.insert("Cartography".to_string(), key);
                    player.gain_resources(ResourcePile::ideas(1));
                    player.add_info_log_item("Cartography gained 1 idea");
                    if navigation {
                        player.gain_resources(ResourcePile::culture_tokens(1));
                        player.add_info_log_item(" and 1 culture token (for using navigation)");
                    }
                }
            },
            0,
        )
}

fn fishing_collect(i: &mut CollectOptionsInfo, c: &CollectContext, game: &Game) {
    let city = game
        .get_any_city(c.city_position)
        .expect("city should exist");
    let port = city.port_position;
    if let Some(position) =
        port.filter(|p| game.enemy_player(c.player_index, *p).is_none())
            .or_else(|| {
                c.city_position.neighbors().into_iter().find(|p| {
                    game.map.is_water(*p) && game.enemy_player(c.player_index, *p).is_none()
                })
            })
    {
        i.choices.insert(
            position,
            if Some(position) == port {
                HashSet::from([
                    ResourcePile::food(1),
                    ResourcePile::gold(1),
                    ResourcePile::mood_tokens(1),
                ])
            } else {
                HashSet::from([ResourcePile::food(1)])
            },
        );
    }
}
