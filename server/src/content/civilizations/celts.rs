// Monumental Edition components: https://boardgamegeek.com/image/7553244 and /7553248
use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::barbarians::{
    get_barbarian_reinforcement_choices, get_barbarians_event_player, get_barbarians_player,
    possible_barbarians_reinforcements, possible_barbarians_spawns,
};
use crate::card::{HandCard, HandCardLocation};
use crate::city::{City, gain_city};
use crate::civilization::Civilization;
use crate::combat::{Combat, get_combat_strength, update_combat_strength};
use crate::content::ability::AbilityBuilder;
use crate::content::advances::AdvanceGroup;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::{
    HandCardsRequest, PaymentRequest, PositionRequest, UnitTypeRequest, UnitsRequest,
};
use crate::game::Game;
use crate::leader::{Leader, LeaderInfo, leader_position};
use crate::leader_ability::{LeaderAbility, LeaderAbilityBuilder};
use crate::player::gain_unit;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::unit::{UnitType, set_unit_position};
use crate::victory_points::{SpecialVictoryPoints, VictoryPointAttribution};

pub(crate) fn celts() -> Civilization {
    Civilization::new("Celts",vec![
        SpecialAdvanceInfo::builder(SpecialAdvance::TribalWarfare,SpecialAdvanceRequirement::Advance(Advance::Tactics),"Tribal Warfare","Against another player's army, add +1 combat value per Barbarian city within 2 spaces of the battle, up to +4.")
            .add_combat_strength_listener(115,|game,c,s,r|{if game.player(c.opponent(c.player(r))).is_human()&&c.is_land_battle(game){let bonus=get_barbarians_player(game).cities.iter().filter(|city|city.position.distance(c.defender_position())<=2).count().min(4) as i8;s.extra_combat_value+=bonus;if bonus>0{s.roll_log.push(format!("Tribal Warfare adds +{bonus} combat value"));}}}).build(),
        SpecialAdvanceInfo::builder(SpecialAdvance::TribalAllies,SpecialAdvanceRequirement::Advance(Advance::Draft),"Tribal Allies","Once per turn, as an action, spawn Barbarians as for an event's Barbarian spawn icon. When you trigger Barbarian movement, pay 1 food to stop any number of those armies.")
            .add_custom_action(CustomActionType::TribalAllies,|c|c.once_per_turn().action().no_resources(),spawn,|game,p|!possible_barbarians_spawns(game,p).is_empty()||!possible_barbarians_reinforcements(game).is_empty())
            .add_payment_request_listener(|e|&mut e.stop_barbarian_movement,5,|game,p,m|{
                let own=game.events.iter().any(|e|matches!(&e.event_type,crate::content::persistent_events::PersistentEventType::Incident(i) if i.active_player==p.index));
                (own&&!m.is_empty()).then(||vec![PaymentRequest::optional(p.payment_options().resources(p.get(game),ResourcePile::food(1)),"Tribal Allies · Stop Barbarian armies")])
            },|game,s,_|{if !s.choice[0].is_empty(){s.player().get_mut(game).event_info.insert("Tribal Allies stop".into(),"paid".into());}})
            .add_position_request(|e|&mut e.stop_barbarian_movement,4,|game,p,m|p.get_mut(game).event_info.remove("Tribal Allies stop").map(|_|PositionRequest::new(m.clone(),0..=m.len() as u8,"Tribal Allies · Select armies that will not move")),|game,s,m|{m.retain(|pos|!s.choice.contains(pos));s.log(game,&format!("Stopped {} Barbarian armies",s.choice.len()));}).build(),
        SpecialAdvanceInfo::builder(SpecialAdvance::DruidicInfluence,SpecialAdvanceRequirement::Advance(Advance::Priesthood),"Druidic Influence","Influence a Barbarian settlement to place your culture marker there, worth 1 objective point. It remains Barbarian. Each settlement holds at most one marker; capturing or converting it removes the marker.")
            .add_transient_event_listener(|e|&mut e.dynamic_victory_points,6,|points,game,(),p|{let count=get_barbarians_player(game).cities.iter().filter(|c|c.influence_marker==Some(p.index)).count();points.push(SpecialVictoryPoints::new(count as f32,p.origin.clone(),VictoryPointAttribution::Objectives));}).build(),
        SpecialAdvanceInfo::builder(SpecialAdvance::TribalTrade,SpecialAdvanceRequirement::Advance(Advance::TradeRoutes),"Tribal Trade","Your units may trade with non-Angry Barbarian cities. Each such route gives food, including when you have Currency.").build(),
    ],vec![viriatus(),vercingetorix(),boudica()],None)
}
pub(crate) fn can_mark_city(game: &Game, p: usize, pos: Position) -> bool {
    game.player(p)
        .has_special_advance(SpecialAdvance::DruidicInfluence)
        && game.try_get_any_city(pos).is_some_and(|c| {
            c.influence_marker.is_none() && game.player(c.player_index).civilization.is_barbarian()
        })
}
fn spawn(b: AbilityBuilder) -> AbilityBuilder {
    b.add_position_request(
        |e| &mut e.custom_action,
        3,
        |game, p, _| {
            Some(PositionRequest::new(
                possible_barbarians_spawns(game, p.get(game)),
                1..=1,
                "Tribal Allies · Place a Barbarian city and Infantry",
            ))
        },
        |game, s, _| {
            let p = get_barbarians_event_player(game, &s.origin);
            gain_city(game, &p, City::new(p.index, s.choice[0]));
            gain_unit(game, &p, s.choice[0], UnitType::Infantry);
        },
    )
    .add_position_request(
        |e| &mut e.custom_action,
        2,
        |game, _, _| {
            Some(PositionRequest::new(
                possible_barbarians_reinforcements(game),
                1..=1,
                "Tribal Allies · Reinforce a Barbarian city",
            ))
        },
        |_, s, a| a.action.city = Some(s.choice[0]),
    )
    .add_unit_type_request(
        |e| &mut e.custom_action,
        1,
        |game, _, a| {
            a.action.city.map(|pos| {
                UnitTypeRequest::new(
                    get_barbarian_reinforcement_choices(game, pos),
                    get_barbarians_player(game).index,
                    "Choose the Barbarian reinforcement",
                )
            })
        },
        |game, s, a| {
            let p = get_barbarians_event_player(game, &s.origin);
            gain_unit(game, &p, a.action.city.unwrap(), s.choice);
        },
    )
}
pub(crate) fn loyalty_controller(game: &Game, c: &Combat) -> Option<usize> {
    let barb = c
        .players()
        .into_iter()
        .find(|p| game.player(*p).civilization.is_barbarian())?;
    game.players
        .iter()
        .find(|p| {
            p.index != c.opponent(barb)
                && p.active_leader() == Some(Leader::Viriatus)
                && leader_position(p).distance(c.defender_position()) <= 2
        })
        .map(|p| p.index)
}
fn empty_neighbors(game: &Game, pos: Position) -> Vec<Position> {
    pos.neighbors()
        .into_iter()
        .filter(|to| {
            game.map.is_land(*to)
                && game.try_get_any_city(*to).is_none()
                && game.players.iter().all(|p| p.get_units(*to).is_empty())
        })
        .collect()
}
fn viriatus() -> LeaderInfo {
    LeaderInfo::new(Leader::Viriatus,"Viriatus",
        LeaderAbility::builder("Terror","Before each combat round after the first, pay 2 mood tokens to force one opposing army unit out of battle into an adjacent empty land space.")
            .add_units_request(|e|&mut e.combat_round_start_allow_tactics,130,|game,p,r|{
                if r.combat.stats.round==1||!r.combat.has_leader(r.combat.role(p.index),game)||p.get(game).resources.mood_tokens<2||empty_neighbors(game,r.combat.defender_position()).is_empty(){return None;}
                let enemy=r.combat.opponent(p.index);Some(UnitsRequest::new(enemy,r.combat.fighting_units(game,enemy),0..=1,"Terror · Pay 2 mood to force an enemy army unit away"))
            },|_,s,r|r.displaced_unit=s.choice.first().copied())
            .add_position_request(|e|&mut e.combat_round_start_allow_tactics,129,|game,_,r|r.displaced_unit.map(|_|PositionRequest::new(empty_neighbors(game,r.combat.defender_position()),1..=1,"Terror · Choose the unit's destination")),|game,s,r|{
                let id=r.displaced_unit.take().unwrap();let enemy=r.combat.opponent(s.player_index);
                crate::resource::pay_cost(game,s.player_index,&PaymentRequest::mandatory(s.player().payment_options().resources(s.player().get(game),ResourcePile::mood_tokens(2)),"Terror"),&ResourcePile::mood_tokens(2));
                set_unit_position(enemy,id,s.choice[0],game);if enemy==r.combat.attacker(){r.combat.attackers.retain(|u|*u!=id);}
                s.log(game,&format!("Terror moved an enemy unit to {}",s.choice[0]));
                if r.combat.fighting_units(game,enemy).is_empty(){r.final_result=Some(if enemy==r.combat.attacker(){crate::combat_listeners::CombatResult::DefenderWins}else{crate::combat_listeners::CombatResult::AttackerWins});}
            }).build(),
        LeaderAbility::builder("Loyalty","Play tactics cards from your hand for Barbarians battling within 2 spaces of Viriatus.")
            .add_hand_card_request(|e|&mut e.loyalty,0,|game,p,r|{
                let barb=r.combat.players().into_iter().find(|id|game.player(*id).civilization.is_barbarian())?;
                if get_combat_strength(barb,r).deny_tactics_card{return None;}
                let cards=p.get(game).action_cards.iter().filter(|id|crate::tactics_card::can_play_tactics_card(game,barb,game.cache.get_action_card(**id),&r.combat)).map(|id|HandCard::ActionCard(*id)).collect();
                Some(HandCardsRequest::new(cards,0..=1,"Loyalty · Play a tactics card for the Barbarians"))
            },|game,s,r|{if let Some(HandCard::ActionCard(id))=s.choice.first(){let barb=r.combat.players().into_iter().find(|id|game.player(*id).civilization.is_barbarian()).unwrap();game.player_mut(barb).custom_data.insert("Loyalty controller".into(),crate::player::Data::Number(s.player_index as u32));update_combat_strength(game,barb,r,|_,_,strength,_|strength.tactics_card=Some(*id));crate::action_card::discard_action_card(game,s.player_index,*id,&s.origin,HandCardLocation::PlayToDiscardFaceDown);}}).build())
}
fn rapid_recruitment(mut b: LeaderAbilityBuilder) -> LeaderAbilityBuilder {
    for i in 0..8u32 {
        b = b.add_position_request(
            |e| &mut e.recruit,
            40 - i as i32,
            move |game, p, r| {
                if r.city_position != leader_position(p.get(game)) {
                    return None;
                }
                let first = p.get(game).next_unit_id - r.units.amount() as u32;
                let unit = p
                    .get(game)
                    .units
                    .iter()
                    .filter(|u| u.id >= first && !u.is_ship())
                    .nth(i as usize)?;
                let choices = r
                    .city_position
                    .neighbors()
                    .into_iter()
                    .filter(|pos| {
                        game.map.is_land(*pos)
                            && game.enemy_player(p.index, *pos).is_none()
                            && (!unit.is_army_unit()
                                || crate::player::can_add_army_unit(p.get(game), *pos))
                    })
                    .collect();
                Some(PositionRequest::new(
                    choices,
                    0..=1,
                    &format!(
                        "Rapid Recruitment · Deploy {} #{} beside the city",
                        unit.unit_type.non_leader_name(),
                        unit.id + 1
                    ),
                ))
            },
            move |game, s, r| {
                if let Some(pos) = s.choice.first() {
                    let p = s.player().get(game);
                    let first = p.next_unit_id - r.units.amount() as u32;
                    let id = p
                        .units
                        .iter()
                        .filter(|u| u.id >= first && !u.is_ship())
                        .nth(i as usize)
                        .unwrap()
                        .id;
                    set_unit_position(s.player_index, id, *pos, game);
                    s.log(game, &format!("Deployed unit #{} at {pos}", id + 1));
                }
            },
        );
    }
    b
}
fn vercingetorix() -> LeaderInfo {
    LeaderInfo::new(Leader::Vercingetorix,"Vercingetorix",
        LeaderAbility::builder("Warrior People","Vercingetorix adds +1 combat value per friendly Settler within 2 spaces.").add_combat_strength_listener(116,|game,c,s,r|{if c.has_leader(r,game){let p=game.player(c.player(r));let n=p.units.iter().filter(|u|u.is_settler()&&u.position.distance(leader_position(p))<=2).count() as i8;s.extra_combat_value+=n;if n>0{s.roll_log.push(format!("Warrior People adds +{n} combat value"));}}}).build(),
        rapid_recruitment(LeaderAbility::builder("Rapid Recruitment","When recruiting in Vercingetorix's city, each new land unit may be placed directly on adjacent revealed land without enemy units or cities.")).build())
}
fn boudica() -> LeaderInfo {
    LeaderInfo::new(
        Leader::Boudica,
        "Queen Boudica",
        LeaderAbility::builder(
            "Fearless",
            "Boudica adds +2 combat value against a player with more Warfare advances.",
        )
        .add_combat_strength_listener(117, |game, c, s, r| {
            let own = game.player(c.player(r));
            let enemy = game.player(c.opponent(own.index));
            let warfare = &game.cache.get_advance_group(AdvanceGroup::Warfare).advances;
            if c.has_leader(r, game)
                && enemy.is_human()
                && warfare
                    .iter()
                    .filter(|a| enemy.has_advance(a.advance))
                    .count()
                    > warfare
                        .iter()
                        .filter(|a| own.has_advance(a.advance))
                        .count()
            {
                s.extra_combat_value += 2;
                s.roll_log.push("Fearless adds +2 combat value".into());
            }
        })
        .build(),
        LeaderAbility::builder(
            "Great Numbers",
            "When a Settler founds a city in Boudica's space, pay 1 food to keep the Settler.",
        )
        .add_payment_request_listener(
            |e| &mut e.found_city,
            15,
            |game, p, pos| {
                (leader_position(p.get(game)) == *pos).then(|| {
                    vec![PaymentRequest::optional(
                        p.payment_options()
                            .resources(p.get(game), ResourcePile::food(1)),
                        "Great Numbers · Keep the Settler",
                    )]
                })
            },
            |game, s, pos| {
                if !s.choice[0].is_empty() {
                    gain_unit(game, &s.player(), *pos, UnitType::Settler);
                }
            },
        )
        .build(),
    )
}

/// Human decisions for tactics played on behalf of Barbarians retain Barbarian ownership.
pub(crate) fn delegated_player(game: &Game, actor: usize) -> usize {
    if game.player(actor).is_human() {
        return actor;
    }
    let in_combat = game.events.iter().any(|e| {
        matches!(
            e.event_type,
            crate::content::persistent_events::PersistentEventType::CombatRoundStart(_)
                | crate::content::persistent_events::PersistentEventType::CombatRoundEnd(_)
                | crate::content::persistent_events::PersistentEventType::CombatEnd(_)
        )
    });
    if in_combat {
        game.player(actor)
            .custom_data
            .get("Loyalty controller")
            .map_or(actor, |d| d.number() as usize)
    } else {
        actor
    }
}
pub(crate) fn clear_loyalty() -> crate::content::ability::Ability {
    crate::content::ability::Ability::builder("Loyalty control", "")
        .add_transient_event_listener(
            |e| &mut e.after_action,
            -1000,
            |game, (), (), _| {
                if game.events.is_empty() {
                    for p in &mut game.players {
                        p.custom_data.remove("Loyalty controller");
                    }
                }
            },
        )
        .build()
}
