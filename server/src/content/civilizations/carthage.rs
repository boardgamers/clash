// Monumental Edition components: https://boardgamegeek.com/image/7553246 and /7553242
// Pirate Allies and Hegemony clarifications: publisher rulebook, p. 32.
use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::content::ability::AbilityBuilder;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::{PositionRequest, UnitsRequest};
use crate::events::EventOrigin;
use crate::game::{Game, GameState};
use crate::leader::{Leader, LeaderInfo, leader_position};
use crate::leader_ability::LeaderAbility;
use crate::map::{Block, Terrain};
use crate::movement::{MoveState, MoveUnits, MovementRestriction};
use crate::payment::{PaymentConversion, PaymentConversionType};
use crate::player::{Player, gain_unit, remove_unit};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::unit::{UnitType, carried_units};
use std::collections::HashSet;

pub(crate) fn pirate_player(game: &Game) -> usize {
    game.players
        .iter()
        .find(|p| p.civilization.is_pirates())
        .expect("Pirates player")
        .index
}
fn origin() -> EventOrigin {
    EventOrigin::SpecialAdvance(SpecialAdvance::PirateAllies)
}
pub(crate) fn claim_pirate(game: &mut Game, player: usize, id: u32) -> u32 {
    let pirates = pirate_player(game);
    let mut unit = remove_unit(pirates, id, game);
    assert!(unit.is_ship());
    game.player_mut(pirates).held_units += &UnitType::Ship;
    let p = game.player_mut(player);
    unit.player_index = player;
    unit.id = p.next_unit_id;
    p.next_unit_id += 1;
    unit.pirate = true;
    let id = unit.id;
    p.units.push(unit);
    id
}
fn return_pirate(game: &mut Game, player: usize, id: u32) -> u32 {
    let mut unit = remove_unit(player, id, game);
    let pirates = pirate_player(game);
    let p = game.player_mut(pirates);
    unit.player_index = pirates;
    unit.pirate = false;
    unit.id = p.next_unit_id;
    p.next_unit_id += 1;
    let id = unit.id;
    p.units.push(unit);
    id
}
pub(crate) fn sync_pirates(game: &mut Game, player: usize) {
    if !game
        .player(player)
        .has_special_advance(SpecialAdvance::PirateAllies)
        || !game.events.is_empty()
    {
        return;
    }
    game.player_mut(player).event_info.remove("Attack Pirates");
    game.player_mut(player)
        .event_info
        .remove("Pirate attack reinforcements");
    let p = game.player(player);
    let friendly: HashSet<_> = p
        .units
        .iter()
        .filter(|u| !u.pirate)
        .map(|u| u.position)
        .collect();
    let release: Vec<_> = p
        .units
        .iter()
        .filter(|u| u.pirate && !friendly.contains(&u.position))
        .map(|u| u.id)
        .collect();
    for id in release {
        return_pirate(game, player, id);
    }
    let pirates = pirate_player(game);
    let claim: Vec<_> = game
        .player(pirates)
        .units
        .iter()
        .filter(|u| friendly.contains(&u.position))
        .map(|u| u.id)
        .collect();
    if !claim.is_empty() {
        game.log(
            player,
            &origin(),
            &format!("Control {} allied pirate ship(s)", claim.len()),
        );
    }
    for id in claim {
        claim_pirate(game, player, id);
    }
}
pub(crate) fn guard_moves(p: &Player, ids: &[u32]) -> bool {
    !ids.iter().any(|id| p.get_unit(*id).pirate)
        || ids.iter().any(|id| {
            let u = p.get_unit(*id);
            !u.pirate || !carried_units(*id, p).is_empty()
        })
}
pub(crate) fn ignores_mountains(p: &Player, ids: &[u32]) -> bool {
    ids.iter()
        .any(|id| p.get_unit(*id).unit_type == UnitType::Leader(Leader::Hannibal))
        && ids
            .iter()
            .any(|id| p.get_unit(*id).unit_type == UnitType::Elephant)
}
fn moved_this_turn(p: &Player) -> Vec<u32> {
    p.event_info
        .get("Carthage moved units")
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default()
}
pub(crate) fn record_move(game: &mut Game, p: usize, ids: &[u32]) {
    if game.player(p).civilization.name != "Carthage" {
        return;
    }
    let mut moved = moved_this_turn(game.player(p));
    for id in ids {
        moved.push(*id);
        moved.extend(carried_units(*id, game.player(p)));
    }
    moved.sort();
    moved.dedup();
    game.player_mut(p).event_info.insert(
        "Carthage moved units".into(),
        serde_json::to_string(&moved).unwrap(),
    );
}
fn complete_navigator(game: &mut Game, p: usize) {
    if game.state != GameState::Playing || !game.events.is_empty() {
        return;
    }
    game.player_mut(p).event_info.remove("Navigator active");
    let Some(s) = game.player_mut(p).event_info.remove("Navigator fleet") else {
        return;
    };
    let ids: Vec<u32> = serde_json::from_str(&s).unwrap();
    for id in ids {
        if let Some(u) = game.player_mut(p).units.iter_mut().find(|u| u.id == id) {
            u.movement_restrictions.push(MovementRestriction::Navigator);
        }
    }
}
pub(crate) fn carthage() -> Civilization {
    Civilization::new(
        "Carthage",
        vec![warbeasts(), hegemony(), pirate_allies(), mercenaries()],
        vec![dido(), hanno(), hannibal()],
        Some(Block::new([
            Terrain::Fertile,
            Terrain::Mountain,
            Terrain::Forest,
            Terrain::Water,
        ])),
    )
}
fn warbeasts() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(SpecialAdvance::Warbeasts,SpecialAdvanceRequirement::Advance(Advance::Husbandry),"Warbeasts","Recruit Elephants without a Market by replacing at least 1 normal resource per Elephant with gold or culture.")
        .add_transient_event_listener(|e|&mut e.recruit_cost,-10,|cost,units,game,p|{
            if units.elephants==0{return;}
            let city=p.get(game).get_city(cost.city_position.unwrap());
            cost.cost.conversions.push(PaymentConversion::resource_options(vec![ResourcePile::food(1)],ResourcePile::culture_tokens(1),PaymentConversionType::MayNotOverpay(units.elephants * UnitType::Elephant.cost().amount())));
            if city.pieces.market.is_none(){
                let base=cost.cost.default.clone();let elephant=UnitType::Elephant.cost();let mut alternatives=vec![base];
                for _ in 0..units.elephants{
                    alternatives=alternatives.into_iter().flat_map(|base|elephant.types().into_iter().filter_map(move |kind|{
                        (base.get(&kind)>0).then(||{let mut c=base.clone();c.add_type(kind,-1);c.gold+=1;c})
                    })).collect();
                }
                alternatives.sort_by_key(|r|r.to_string());alternatives.dedup();cost.cost.default=alternatives.remove(0);cost.cost.alternatives=alternatives;
                cost.cost.conversions.push(PaymentConversion::limited(ResourcePile::gold(1),ResourcePile::culture_tokens(1),units.elephants));
                cost.info.add_log(p,"Warbeasts: pay at least 1 gold or culture per Elephant instead of a normal resource");
            }
        }).build()
}
fn mercenaries() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(SpecialAdvance::Mercenaries,SpecialAdvanceRequirement::Advance(Advance::Draft),"Mercenaries","Use gold in place of mood tokens for Draft. A city with a Port may Draft a Ship instead of an Infantry.")
        .add_transient_event_listener(|e|&mut e.recruit_cost,-1,|cost,units,game,p|{
            let draft=ResourcePile::mood_tokens(crate::content::advances::warfare::draft_cost(p.get(game)));
            if units.ships>0 {
                if let Some(c)=cost.cost.conversions.iter_mut().find(|c|c.from.contains(&UnitType::Infantry.cost())){c.from.push(UnitType::Ship.cost());}
                else {cost.cost.conversions.insert(0,PaymentConversion::limited(UnitType::Ship.cost(),draft.clone(),1));}
            }
            if units.ships>0||units.infantry>0 {cost.cost.conversions.push(PaymentConversion::limited(ResourcePile::mood_tokens(1),ResourcePile::gold(1),draft.amount()));cost.info.add_log(p,"Mercenaries: Draft 1 Infantry or Ship with mood or gold");}
        }).build()
}
fn pirate_allies() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(SpecialAdvance::PirateAllies,SpecialAdvanceRequirement::Advance(Advance::Navigation),"Pirate Allies","Enter pirate spaces peacefully or attack. While one of your colored units shares their space, pirates count as your Ships: move them with an escort, embark, trade or use Hegemony. At most 4 Ships share a space; allied pirates are taken as casualties first.")
        .build()
}
fn hegemony_destinations(game: &Game, p: &Player, ship: u32) -> Vec<Position> {
    if !p.is_city_available() {
        return vec![];
    }
    p.get_unit(ship)
        .position
        .neighbors()
        .into_iter()
        .filter(|pos| {
            game.map
                .get(*pos)
                .is_some_and(crate::city::is_valid_city_terrain)
                && game.try_get_any_city(*pos).is_none()
                && game.enemy_player(p.index, *pos).is_none()
        })
        .collect()
}
fn founding_ships(game: &Game, p: &Player, free: bool) -> Vec<u32> {
    p.units
        .iter()
        .filter(|u| {
            u.is_ship()
                && (!free
                    || p.active_leader() == Some(Leader::QueenDido)
                        && leader_position(p) == u.position)
                && !hegemony_destinations(game, p, u.id).is_empty()
        })
        .map(|u| u.id)
        .collect()
}
fn hegemony_action(b: AbilityBuilder, free: bool) -> AbilityBuilder {
    b.add_units_request(
        |e| &mut e.custom_action,
        4,
        move |game, p, _| {
            Some(UnitsRequest::new(
                p.index,
                founding_ships(game, p.get(game), free),
                1..=1,
                "Hegemony · Choose the ship to found a city",
            ))
        },
        |_, s, a| a.selected_unit = Some((s.player_index, s.choice[0])),
    )
    .add_position_request(
        |e| &mut e.custom_action,
        3,
        |game, p, a| {
            Some(PositionRequest::new(
                hegemony_destinations(game, p.get(game), a.selected_unit.unwrap().1),
                1..=1,
                "Hegemony · Select the new city's land space",
            ))
        },
        |_, s, a| a.action.city = Some(s.choice[0]),
    )
    .add_units_request(
        |e| &mut e.custom_action,
        2,
        |game, p, a| {
            let pos = p.get(game).get_unit(a.selected_unit.unwrap().1).position;
            let units: Vec<_> = p
                .get(game)
                .get_units(pos)
                .iter()
                .filter(|u| !u.is_ship())
                .map(|u| u.id)
                .collect();
            Some(UnitsRequest::new(
                p.index,
                units.clone(),
                0..=units.len() as u8,
                "Hegemony · Land passengers in the new city; they cannot move again this turn",
            ))
        },
        |game, s, a| {
            let pos = a.action.city.unwrap();
            validate_landing(game, s.player().get(game), pos, &s.choice)
                .expect("Hegemony stack limit");
            for id in &s.choice {
                crate::unit::set_unit_position(s.player_index, *id, pos, game);
                let u = s.player().get_mut(game).get_unit_mut(*id);
                u.carrier_id = None;
                u.movement_restrictions.push(MovementRestriction::Navigator);
            }
        },
    )
    .add_simple_persistent_event_listener(
        |e| &mut e.custom_action,
        1,
        |game, p, a| {
            let id = a.selected_unit.unwrap().1;
            crate::unit::kill_units(game, &[id], p.index, None, &p.origin);
        },
    )
    .add_simple_persistent_event_listener(
        |e| &mut e.custom_action,
        0,
        |game, p, a| crate::city::found_city(game, p, a.action.city.unwrap()),
    )
}
fn hegemony() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(SpecialAdvance::Hegemony,SpecialAdvanceRequirement::Advance(Advance::Cartography),"Hegemony","As an action, remove a Ship to found a city on adjacent eligible land. Passengers in its sea space may land in the new city; they cannot move again this turn. Remaining passengers need space on other Ships.")
        .add_custom_action(CustomActionType::Hegemony,|c|c.any_times().action().no_resources(),|b|hegemony_action(b,false),|game,p|!founding_ships(game,p,false).is_empty()).build()
}
fn founder_settlers(game: &Game, p: &Player) -> Vec<u32> {
    p.units
        .iter()
        .filter(|u| u.position == leader_position(p) && u.can_found_city(game))
        .map(|u| u.id)
        .collect()
}
fn dido() -> LeaderInfo {
    LeaderInfo::new(Leader::QueenDido,"Queen Dido",
        LeaderAbility::builder("Founder","A unit sharing Dido's space may found a city as a free action.")
            .add_custom_action(CustomActionType::Founder,|c|c.any_times().free_action().no_resources(),|b|b.add_units_request(|e|&mut e.custom_action,0,|game,p,_|Some(UnitsRequest::new(p.index,founder_settlers(game,p.get(game)),1..=1,"Founder · Choose the Settler")),|game,s,_|crate::city::execute_found_city_action(game,s.player_index,s.choice[0]).unwrap()),|game,p|!founder_settlers(game,p).is_empty())
            .add_custom_action(CustomActionType::HegemonyFounder,|c|c.any_times().free_action().no_resources(),|b|hegemony_action(b,true),|game,p|p.has_special_advance(SpecialAdvance::Hegemony)&&!founding_ships(game,p,true).is_empty()).build(),
        LeaderAbility::builder("Sacrifice","When Dido's city is attacked, add +2 combat value. After surviving a round, you may discard Dido, replace her with Infantry, and force the attacker to retreat. No leader point is awarded.")
            .add_combat_strength_listener(119,|game,c,s,r|{if !r.is_attacker()&&c.defender_city(game).is_some()&&c.has_leader(r,game){s.extra_combat_value+=2;s.roll_log.push("Sacrifice adds +2 combat value".into());}})
            .add_bool_request(|e|&mut e.combat_round_end,-50,|game,p,r|{
                let c=&r.combat;(!c.role(p.index).is_attacker()&&c.defender_city(game).is_some()&&c.has_leader(c.role(p.index),game)&&r.final_result.is_none()&&c.retreat!=crate::combat::CombatRetreatState::EndAfterCurrentRound&&p.get(game).available_units().infantry>0).then(||"Sacrifice · Replace Dido with Infantry and force the attacker to retreat?".into())
            },|game,s,r|{if s.choice{
                let p=s.player();let id=p.get(game).units.iter().find(|u|u.unit_type==UnitType::Leader(Leader::QueenDido)).unwrap().id;let pos=p.get(game).get_unit(id).position;
                crate::unit::kill_units(game,&[id],p.index,None,&s.origin);gain_unit(game,&p,pos,UnitType::Infantry);
                r.combat.retreat=crate::combat::CombatRetreatState::EndAfterCurrentRound;s.log(game,"Dido sacrificed herself; the attacker retreats and no leader point is awarded");
            }}).build())
}
fn navigator_ships(p: &Player) -> Vec<u32> {
    let pos = leader_position(p);
    let moved = moved_this_turn(p);
    if p.units
        .iter()
        .any(|u| u.unit_type == UnitType::Leader(Leader::Hanno) && moved.contains(&u.id))
    {
        return vec![];
    }
    p.get_units(pos)
        .iter()
        .filter(|u| {
            u.is_ship()
                && !moved.contains(&u.id)
                && !u
                    .movement_restrictions
                    .contains(&MovementRestriction::Battle)
                && !u
                    .movement_restrictions
                    .contains(&MovementRestriction::Navigator)
        })
        .map(|u| u.id)
        .collect()
}
fn hanno() -> LeaderInfo {
    LeaderInfo::new(Leader::Hanno,"Hanno",
        LeaderAbility::builder("Navigator","Once per turn, move Hanno with a group of Ships as a free action. Those Ships must not have moved this turn and cannot move again afterwards.")
            .add_custom_action(CustomActionType::Navigator,|c|c.once_per_turn().free_action().no_resources(),|b|b.add_simple_persistent_event_listener(|e|&mut e.custom_action,-1,|game,p,_|{
                let allowed=navigator_ships(p.get(game));let mut state=MoveState::new();state.unit_only=true;state.movement_actions_left=1;state.moved_units=p.get(game).units.iter().filter(|u|!allowed.contains(&u.id)).map(|u|u.id).collect();game.state=GameState::Movement(state);p.get_mut(game).event_info.insert("Navigator active".into(),"yes".into());
            }),|_,p|p.units.iter().find(|u|u.unit_type==UnitType::Leader(Leader::Hanno)).is_some_and(|u|u.carrier_id.is_some_and(|id|navigator_ships(p).contains(&id))))
            .add_transient_event_listener(|e|&mut e.after_action,29,|game,(),(),p|complete_navigator(game,p.index)).build(),
        LeaderAbility::builder("Fleet Commander","While Hanno is aboard a Ship, add +2 combat value in naval battles in his space.")
            .add_combat_strength_listener(120,|game,c,s,r|{let p=game.player(c.player(r));if c.is_sea_battle(game)&&p.units.iter().find(|u|u.unit_type==UnitType::Leader(Leader::Hanno)).is_some_and(|u|u.is_transported()&&u.position==c.stats.player(p.index).position){s.extra_combat_value+=2;s.roll_log.push("Fleet Commander adds +2 combat value".into());}}).build())
}
fn hannibal() -> LeaderInfo {
    LeaderInfo::new(Leader::Hannibal,"Hannibal",
        LeaderAbility::builder("Father of Strategy","In Hannibal's first combat round, the opponent's total combat bonus is limited to +2.").build(),
        LeaderAbility::builder("Hannibal ad Portas","A moving group containing Hannibal and Elephants ignores Mountain movement restrictions.").build())
}

pub(crate) fn prepare_move(game: &mut Game, p: usize, m: &mut MoveUnits) -> Result<(), String> {
    if let Some(id) = m.embark_pirate {
        if !game
            .player(p)
            .has_special_advance(SpecialAdvance::PirateAllies)
        {
            return Err("Requires Pirate Allies".into());
        }
        let pirates = pirate_player(game);
        if game
            .player(pirates)
            .try_get_unit(id)
            .is_none_or(|u| u.position != m.destination)
        {
            return Err("Pirate ship is unavailable".into());
        }
        m.embark_carrier_id = Some(claim_pirate(game, p, id));
        m.embark_pirate = None;
    }
    if m.attack_pirates {
        if !game
            .player(p)
            .has_special_advance(SpecialAdvance::PirateAllies)
            || m.units.iter().any(|id| {
                !game.player(p).get_unit(*id).is_ship() || game.player(p).get_unit(*id).pirate
            })
        {
            return Err("Choose your colored Ships to attack pirates".into());
        }
        let release: Vec<_> = game
            .player(p)
            .get_units(m.destination)
            .iter()
            .filter(|u| u.pirate)
            .map(|u| u.id)
            .collect();
        for id in release {
            if !carried_units(id, game.player(p)).is_empty() {
                return Err("Disembark passengers before attacking their pirate carrier".into());
            }
            return_pirate(game, p, id);
        }
        let pirates = pirate_player(game);
        if game.player(pirates).get_units(m.destination).is_empty() {
            return Err("No pirates to attack".into());
        }
        let join: Vec<_> = game
            .player(p)
            .get_units(m.destination)
            .iter()
            .filter(|u| u.is_ship() && !u.pirate && !m.units.contains(&u.id))
            .map(|u| u.id)
            .collect();
        // Ships already sharing the destination join when combat starts.
        game.player_mut(p).event_info.insert(
            "Pirate attack reinforcements".into(),
            serde_json::to_string(&join).unwrap(),
        );
        game.player_mut(p)
            .event_info
            .insert("Attack Pirates".into(), "yes".into());
    }
    Ok(())
}
pub(crate) fn pirate_port(game: &Game, p: &Player, city: Position) -> bool {
    p.has_special_advance(SpecialAdvance::PirateAllies)
        && p.get_city(city).port_position.is_some_and(|pos| {
            game.players
                .iter()
                .any(|e| e.civilization.is_pirates() && !e.get_units(pos).is_empty())
                || p.get_units(pos).iter().any(|u| u.pirate)
        })
}
pub(crate) fn validate_recruit(
    game: &Game,
    p: &Player,
    r: &crate::recruit::Recruit,
) -> Result<(), String> {
    if r.attack_pirates && (r.units.ships == 0 || !pirate_port(game, p, r.city_position)) {
        return Err("Choose Ships at a Port with pirates".into());
    }
    if p.civilization.name == "Carthage" && r.units.ships > 0 {
        if let Some(pos) = p.get_city(r.city_position).port_position {
            let mut count = p
                .get_units(pos)
                .iter()
                .filter(|u| u.is_ship() && !r.replaced_units.contains(&u.id))
                .count();
            if p.has_special_advance(SpecialAdvance::PirateAllies) && !r.attack_pirates {
                count += game.player(pirate_player(game)).get_units(pos).len();
            }
            if count + r.units.ships as usize > 4 {
                return Err("Carthage may have at most 4 Ships in one space".into());
            }
        }
    }
    Ok(())
}

/// Check the combined army stack before Hegemony passengers land.
pub(crate) fn validate_landing(
    game: &Game,
    p: &Player,
    position: Position,
    units: &[u32],
) -> Result<(), String> {
    let mut group = p
        .get_units(position)
        .iter()
        .map(|u| u.id)
        .collect::<Vec<_>>();
    group.extend_from_slice(units);
    let armies = group
        .iter()
        .filter(|id| p.get_unit(**id).is_army_unit())
        .count();
    let limit = if group
        .iter()
        .any(|id| p.get_unit(*id).unit_type == UnitType::Leader(Leader::Xerxes))
    {
        5
    } else {
        4
    };
    if armies > limit as usize {
        return Err(format!(
            "At most {limit} army units may share the new city's space"
        ));
    }
    let _ = game;
    Ok(())
}
