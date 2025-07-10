use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::combat::Combat;
use crate::combat_listeners::CombatRoundEnd;
use crate::consts::STACK_LIMIT;
use crate::content::ability::AbilityBuilder;
use crate::content::advances::AdvanceGroup;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::{PaymentRequest, UnitsRequest};
use crate::game::{Game, GameState};
use crate::leader::{Leader, LeaderInfo};
use crate::leader_ability::LeaderAbility;
use crate::map::Terrain;
use crate::movement::{MoveState, possible_move_destinations};
use crate::player::{Player, can_add_army_unit};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::tactics_card::CombatRole;
use crate::unit::UnitType;
use crate::wonder::Wonder;
use itertools::Itertools;

pub(crate) fn china() -> Civilization {
    Civilization::new(
        "China",
        vec![rice(), expansion(), fireworks(), imperial_army()],
        vec![sun_tzu(), qin(), wu()],
        None,
    )
}

fn rice() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::RiceCultivation,
        SpecialAdvanceRequirement::Advance(Advance::Irrigation),
        "Rice Cultivation",
        "Collect add additional +1 food from up to 2 Grassland spaces outside your city, \
        if occupied by one of your settlers.",
    )
    .add_transient_event_listener(
        |event| &mut event.collect_total,
        2,
        |i, game, collections, p| {
            let city = game.get_any_city(i.city);
            let food = collections
                .iter()
                .filter(|c| {
                    let pos = c.position;
                    pos.distance(city.position) > 0
                        && game.map.get(pos) == Some(&Terrain::Fertile)
                        && game
                            .player(i.info.player)
                            .units
                            .iter()
                            .any(|u| u.position == pos && u.is_settler())
                })
                .count()
                .min(2);
            i.total += ResourcePile::food(food as u8);
            i.info
                .add_log(p, &format!("Gain {}", ResourcePile::food(food as u8)));
        },
    )
    .build()
}

fn expansion() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Expansion,
        SpecialAdvanceRequirement::Advance(Advance::Husbandry),
        "Expansion",
        "When you recruited at least 1 settler: Every settler in your cities gains one move",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.recruit,
        10,
        |game, player, r| {
            if r.units.settlers == 0 {
                return;
            }

            let p = player.get(game);
            let settlers = movable_settlers(game, p);
            if settlers.is_empty() {
                return;
            }
            let moved_units = p
                .units
                .iter()
                .filter(|u| !u.is_settler())
                .map(|u| u.id)
                .collect_vec();

            game.state = GameState::Movement(MoveState {
                moved_units,
                movement_actions_left: settlers.len() as u32,
                ..MoveState::default()
            });

            player.log(
                game,
                &format!(
                    "Expansion allows to move the settlers at {}",
                    settlers.iter().map(ToString::to_string).join(", ")
                ),
            );
        },
    )
    .build()
}

fn movable_settlers(game: &Game, player: &Player) -> Vec<Position> {
    player
        .units
        .iter()
        .filter(|u| {
            u.is_settler()
                && player.try_get_city(u.position).is_some()
                && !possible_move_destinations(game, player.index, &[u.id], u.position)
                    .list
                    .is_empty()
        })
        .map(|u| u.position)
        .collect_vec()
}

fn fireworks() -> SpecialAdvanceInfo {
    ignore_hit_ability(
        SpecialAdvanceInfo::builder(
            SpecialAdvance::Fireworks,
            SpecialAdvanceRequirement::Advance(Advance::Metallurgy),
            "Fireworks",
            "In the first round of combat, you may pay 1 ore and 1 wood to ignore the first hit.",
        ),
        ResourcePile::wood(1) + ResourcePile::ore(1),
        91,
        |c, _r, _game| c.stats.round == 1,
    )
    .build()
}

fn ignore_hit_ability<B: AbilityInitializerSetup>(
    b: B,
    pile: ResourcePile,
    priority: i32,
    filter: impl Fn(&Combat, CombatRole, &Game) -> bool + Send + Sync + 'static + Clone,
) -> B {
    b.add_payment_request_listener(
        |e| &mut e.combat_round_end,
        priority,
        move |game, p, e| {
            let player_index = p.index;
            let player = p.get(game);
            let combat = &e.combat;
            if !filter(combat, combat.role(player_index), game) {
                return None;
            }

            let cost = p.payment_options().resources(player, pile.clone());

            if !apply_ignore_hit(e, player_index, false) {
                p.log(game, "Won't reduce the hits, no payment made.");
                return None;
            }

            if !player.can_afford(&cost) {
                p.log(game, "Not enough resources, no payment made.");
                return None;
            }

            Some(vec![PaymentRequest::optional(cost, "Ignore 1 hit")])
        },
        move |game, s, e| {
            if !s.choice[0].is_empty() {
                s.log(game, "Ignore the first hit");
                apply_ignore_hit(e, s.player_index, true);
            }
        },
    )
}

fn apply_ignore_hit(e: &mut CombatRoundEnd, p: usize, do_update: bool) -> bool {
    e.update_hits(e.combat.opponent_role(p), do_update, |h| {
        h.opponent_hit_cancels += 1;
    })
}

fn imperial_army() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::ImperialArmy,
        SpecialAdvanceRequirement::AnyGovernment,
        "Imperial Army",
        "Once per turn, as an action, \
        you may convert any number of settlers into infantry units, and vice versa.",
    )
    .add_custom_action(
        CustomActionType::ImperialArmy,
        |c| c.once_per_turn().action().no_resources(),
        use_imperial_army,
        |_game, p| {
            !p.units
                .iter()
                .filter(|u| {
                    // infantry can always be converted - if the settler limit is reached,
                    // the player has to convert a settler to infantry
                    // we're not checking if that settler can be converted here, but later
                    (u.is_settler() && can_add_army_unit(p, u.position)) || u.is_infantry()
                })
                .map(|u| u.id)
                .collect_vec()
                .is_empty()
        },
    )
    .build()
}

fn use_imperial_army(b: AbilityBuilder) -> AbilityBuilder {
    b.add_units_request(
        |event| &mut event.custom_action,
        0,
        |game, p, _| {
            let player_index = p.index;
            let choices = convertible_units(game.player(player_index));
            let max = choices.len() as u8;
            Some(UnitsRequest::new(
                player_index,
                choices,
                1..=max,
                "Select units to convert",
            ))
        },
        |game, s, _| {
            let p = game.player_mut(s.player_index);
            let mut names = vec![];
            for id in &s.choice {
                let unit = p.get_unit_mut(*id);
                names.push(format!(
                    "{} at {}",
                    unit.unit_type.non_leader_name(),
                    unit.position
                ));
                if unit.is_settler() {
                    unit.unit_type = UnitType::Infantry;
                } else {
                    unit.unit_type = UnitType::Settler;
                }
            }

            s.log(game, &format!("Convert {}", names.join(", ")));
        },
    )
}

fn convertible_units(p: &Player) -> Vec<u32> {
    p.units
        .iter()
        .filter(|u| u.is_settler() || u.is_infantry())
        .map(|u| u.id)
        .collect_vec()
}

pub(crate) fn validate_imperial_army(units: &[u32], p: &Player) -> Result<(), String> {
    let settler_to_infantry = units
        .iter()
        .filter(|&&u| p.get_unit(u).is_settler())
        .collect_vec();
    let infantry_to_settler = units
        .iter()
        .filter(|&&u| p.get_unit(u).is_infantry())
        .collect_vec();

    let net_to_settler = infantry_to_settler.len() as i8 - settler_to_infantry.len() as i8;
    if (p.available_units().settlers as i8) < net_to_settler {
        return Err(format!(
            "Cannot convert {} infantry to settlers, only {} available",
            net_to_settler,
            p.available_units().settlers
        ));
    }

    let net_to_infantry = settler_to_infantry.len() as i8 - infantry_to_settler.len() as i8;
    if (p.available_units().infantry as i8) < net_to_infantry {
        return Err(format!(
            "Cannot convert {} settlers to infantry, only {} available",
            net_to_infantry,
            p.available_units().infantry
        ));
    }

    for pos in settler_to_infantry.iter().map(|&u| p.get_unit(*u).position) {
        let army_size = p
            .get_units(pos)
            .iter()
            .filter(|u| {
                (u.is_army_unit() && !infantry_to_settler.contains(&&u.id))
                    || settler_to_infantry.contains(&&u.id)
            })
            .count();
        if army_size > STACK_LIMIT {
            return Err(format!(
                "Cannot convert settlers at {pos} to infantry, stack limit exceeded"
            ));
        }
    }
    Ok(())
}

fn sun_tzu() -> LeaderInfo {
    LeaderInfo::new(
        Leader::SunTzu,
        "Sun Tzu",
        LeaderAbility::advance_gain_custom_action(
            "The Art of War",
            CustomActionType::ArtOfWar,
            AdvanceGroup::Warfare,
        ),
        LeaderAbility::builder(
            "Fast War",
            "Sun Tzu survives a land battle against army units and wins after the first round: \
            Gain 1 mood token and 1 culture token.",
        )
        .add_simple_persistent_event_listener(
            |event| &mut event.combat_end,
            23,
            |game, p, s| {
                let player_index = p.index;
                let c = s.player(player_index);
                if s.is_winner(player_index)
                    && s.round == 1
                    && s.is_battle()
                    && s.battleground.is_land()
                    && c.survived_leader()
                {
                    p.gain_resources(
                        game,
                        ResourcePile::mood_tokens(1) + ResourcePile::culture_tokens(1),
                    );
                }
            },
        )
        .build(),
    )
}

fn qin() -> LeaderInfo {
    LeaderInfo::new(
        Leader::Qin,
        "Qin Shi Huang",
        LeaderAbility::wonder_expert(Wonder::GreatWall),
        ignore_hit_ability(
            LeaderAbility::builder(
                "Tactician",
                "Land battle with leader: You may pay 2 culture tokens to ignore a hit.",
            ),
            ResourcePile::culture_tokens(2),
            92,
            Combat::has_leader,
        )
        .build(),
    )
}

fn wu() -> LeaderInfo {
    LeaderInfo::new(
        Leader::Wu,
        "Wu Zetian",
        LeaderAbility::advance_gain_custom_action(
            "Agriculture Economist",
            CustomActionType::AgricultureEconomist,
            AdvanceGroup::Agriculture,
        ),
        LeaderAbility::builder(
            "Brilliant Conqueror",
            "Land battle with leader: \
            If you have at least as many units as the opponent: Gain +2 combat value.",
        )
        .add_combat_strength_listener(105, |game, c, s, r| {
            let p = c.player(r);
            if c.is_land_battle_with_leader(r, game)
                && c.fighting_units(game, p).len() >= c.fighting_units(game, c.opponent(p)).len()
            {
                s.extra_combat_value += 2;
                s.roll_log
                    .push("Wu Zetian adds +2 combat value".to_string());
            }
        })
        .build(),
    )
}
