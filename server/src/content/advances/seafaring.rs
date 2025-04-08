use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder};
use crate::city_pieces::Building::Port;
use crate::collect::{CollectContext, CollectInfo};
use crate::content::advances::{AdvanceGroup, NAVIGATION, advance_group_builder};
use crate::game::Game;
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
    Advance::builder("Fishing", "Your cities may Collect food from one Sea space")
        .add_transient_event_listener(|event| &mut event.collect_options, 1, fishing_collect)
        .with_advance_bonus(MoodToken)
        .with_unlocked_building(Port)
}

fn navigation() -> AdvanceBuilder {
    Advance::builder(
        NAVIGATION,
        "Ships may leave the map and return at the next sea space",
    )
    .with_advance_bonus(CultureToken)
}

fn war_ships() -> AdvanceBuilder {
    Advance::builder(
        "War Ships",
        "Ignore the first hit it the first round of combat \
        when attacking with Ships or disembarking from Ships",
    )
    .add_combat_round_start_listener(5, |g, c, s, role| {
        let at = role.is_attacker();
        let attacker = at && g.map.is_sea(c.attacker_position);
        let defender = !at && g.map.is_sea(c.defender_position);
        if c.round == 1 && (attacker || defender) {
            s.hit_cancels += 1;
            s.roll_log
                .push("War Ships ignore the first hit in the first round of combat".to_string());
        }
    })
}

fn cartography() -> AdvanceBuilder {
    Advance::builder(
        "Cartography",
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
                    if unit.unit_type.is_ship() {
                        ship = true;
                        if !unit.position.is_neighbor(i.to) {
                            navigation = true;
                        }
                    }
                }
                if ship {
                    let player = game.player_mut(i.player);
                    player.event_info.insert("Cartography".to_string(), key);
                    player.gain_resources(ResourcePile::ideas(1));
                    game.add_info_log_item("Cartography gained 1 idea");
                    if navigation {
                        game.player_mut(i.player).gain_resources(ResourcePile::culture_tokens(1));
                        game.add_info_log_item(" and 1 culture token (for using navigation)");
                    }
                }
            },
        )
}

#[must_use]
fn is_enemy_player_or_pirate_zone(game: &Game, player_index: usize, position: Position) -> bool {
    game.enemy_player(player_index, position).is_some() || game.is_pirate_zone(position)
}

fn fishing_collect(i: &mut CollectInfo, c: &CollectContext, game: &Game) {
    let city = game.get_any_city(c.city_position);
    let port = city.port_position;
    if let Some(position) = port
        .filter(|p| !is_enemy_player_or_pirate_zone(game, c.player_index, *p))
        .or_else(|| {
            c.city_position.neighbors().into_iter().find(|p| {
                game.map.is_sea(*p) && !is_enemy_player_or_pirate_zone(game, c.player_index, *p)
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
