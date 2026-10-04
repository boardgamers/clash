// Monumental Edition components: https://boardgamegeek.com/image/8436929 and /8436928
use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::{
    EventResponse, MultiRequest, PaymentRequest, PersistentEventRequest, PositionRequest,
    UnitsRequest,
};
use crate::events::EventPlayer;
use crate::game::Game;
use crate::leader::{Leader, LeaderInfo, leader_position};
use crate::leader_ability::LeaderAbility;
use crate::map::Terrain;
use crate::payment::{PaymentConversion, PaymentConversionType};
use crate::player::Player;
use crate::position::Position;
use crate::resource::{ResourceType, lose_resources};
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::unit::{UnitType, set_unit_position};
use crate::victory_points::{SpecialVictoryPoints, VictoryPointAttribution};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Captive {
    pub owner: usize,
    pub unit_type: UnitType,
    pub id: u32,
}

pub(crate) fn release_captives(game: &mut Game, player: usize, amount: u8) {
    assert!(
        game.player(player).captives.len() >= amount as usize,
        "Not enough held captives"
    );
    let removed: Vec<_> = game
        .player_mut(player)
        .captives
        .drain(..amount as usize)
        .collect();
    for c in removed {
        game.player_mut(c.owner).held_units -= &c.unit_type;
    }
}
fn hold(game: &mut Game, p: &EventPlayer, mut c: Captive) {
    c.id = p
        .get(game)
        .captives
        .iter()
        .map(|c| c.id)
        .max()
        .map_or(0, |id| id + 1);
    assert!(p.get(game).captives.len() < 4);
    game.player_mut(c.owner).held_units += &c.unit_type;
    p.get_mut(game).captives.push(c);
    p.gain_resources(game, ResourcePile::captives(1));
}
fn captive_request(
    r: &PersistentEventRequest,
    a: EventResponse,
) -> (Vec<Captive>, Vec<Captive>, std::ops::RangeInclusive<u8>) {
    if let (PersistentEventRequest::SelectCaptives(r), EventResponse::SelectCaptives(a)) = (r, a) {
        let mut unique = a.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), a.len(), "Choose each captive only once");
        (r.choices.clone(), a, r.needed.clone())
    } else {
        panic!("Expected captive selection")
    }
}
pub(crate) fn aztecs() -> Civilization {
    Civilization::new(
        "Aztecs",
        vec![
            captives(),
            human_sacrifice(),
            aztec_gold(),
            tribute_empire(),
        ],
        vec![marqzen(), ahuitzotl(), acamapichtli()],
        None,
    )
}
fn captives() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(SpecialAdvance::Captives,SpecialAdvanceRequirement::Advance(Advance::Tactics),"Captives","After winning a land battle, keep defeated enemy army pieces as captives, excluding leaders. Hold up to 4; each is worth ½ objective point and stays out of its owner's supply.")
        .add_multi_choice_reward_request_listener(|e|&mut e.combat_end,22,|r|r,PersistentEventRequest::SelectCaptives,captive_request,|game,p,s|{
            if !s.is_winner(p.index)||!s.battleground.is_land(){return None;}
            let enemy=s.opponent(p.index);let choices:Vec<_>=enemy.losses.clone().into_iter().flat_map(|(u,n)|std::iter::repeat_n(u,n as usize)).filter(|u|u.is_army_unit()&&!u.is_leader()).enumerate().map(|(i,u)|Captive{owner:enemy.player,unit_type:u,id:i as u32}).collect();
            let max=(4-p.get(game).captives.len()).min(choices.len()) as u8;
            Some(MultiRequest::new(choices,0..=max,"Captives · Keep defeated army pieces"))
        },|game,s,_|for c in &s.choice{hold(game,&s.player(),c.clone());})
        .add_transient_event_listener(|e|&mut e.dynamic_victory_points,7,|points,game,(),p|points.push(SpecialVictoryPoints::new(p.get(game).captives.len() as f32*0.5,p.origin.clone(),VictoryPointAttribution::Objectives))).build()
}
fn human_sacrifice() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(SpecialAdvance::HumanSacrifice,SpecialAdvanceRequirement::Advance(Advance::Rituals),"Human Sacrifice","Spend a captive in place of a mood or culture token. After drawing an event, you may sacrifice a captive to cancel the entire event, including its icon, and draw a replacement; you may repeat this.")
        .add_custom_action(CustomActionType::SacrificeCaptives,|c|c.any_times().free_action().no_resources(),|b|b
            .add_multi_choice_reward_request_listener(|e|&mut e.custom_action,2,|r|r,PersistentEventRequest::SelectCaptives,captive_request,
                |game,p,_|Some(MultiRequest::new(p.get(game).captives.clone(),1..=p.get(game).captives.len() as u8,"Human Sacrifice · Choose captives to return to their owners")),
                |game,s,a|{
                    let p=s.player();
                    // Put the chosen pieces first so the shared payment path returns exactly these pieces.
                    let held=&mut p.get_mut(game).captives;held.sort_by_key(|c|!s.choice.contains(c));
                    let amount=s.choice.len() as u8;lose_resources(game,p.index,ResourcePile::captives(amount),s.origin.clone(),vec![],crate::log::ActionLogBalance::Pay);
                    a.payment=ResourcePile::captives(amount);
                })
            .add_resource_request(|e|&mut e.custom_action,1,|_,p,a|Some(crate::content::persistent_events::ResourceRewardRequest::new(p.reward_options().tokens(a.payment.captives),"Human Sacrifice · Gain mood or culture tokens".into()))),
            |_,p|!p.captives.is_empty())
        .add_transient_event_listener(|e|&mut e.general_payment_conversions,15,|conversions,(),(),_|conversions.push(PaymentConversion::resource_options(vec![ResourcePile::mood_tokens(1),ResourcePile::culture_tokens(1)],ResourcePile::captives(1),PaymentConversionType::Unlimited)))
        .add_bool_request(|e|&mut e.incident,10000,|game,p,i|(i.active_player==p.index&&!i.consumed&&i.passed.is_none()&&p.get(game).resources.captives>0).then(||format!("Human Sacrifice · Sacrifice 1 captive to cancel {} and draw another event?",game.cache.get_incident(i.incident_id).name)),|game,s,i|{
            if s.choice {lose_resources(game,s.player_index,ResourcePile::captives(1),s.origin.clone(),vec![],crate::log::ActionLogBalance::Pay);i.consumed=true;i.replaced_by_sacrifice=true;s.log(game,"Sacrificed a captive: the entire event and its icon are cancelled");}
        }).build()
}
fn aztec_gold() -> SpecialAdvanceInfo {
    let mut b=SpecialAdvanceInfo::builder(SpecialAdvance::AztecGold,SpecialAdvanceRequirement::Advance(Advance::Arts),"Aztec Gold","Found a city: gain 1 gold. For each adjacent Mountain, you may exchange 1 resource, mood token or culture token for 1 more gold.")
        .add_simple_persistent_event_listener(|e|&mut e.found_city,60,|game,p,_|p.gain_resources(game,ResourcePile::gold(1)));
    for i in 0..6 {
        b = b.add_payment_request_listener(
            |e| &mut e.found_city,
            59 - i,
            move |game, p, pos| {
                let count = pos
                    .neighbors()
                    .into_iter()
                    .filter(|pos| game.map.get(*pos) == Some(&Terrain::Mountain))
                    .count();
                (i < (count as i32)).then(|| {
                    vec![PaymentRequest::optional(
                        p.payment_options()
                            .sum(p.get(game), 1, &ResourceType::all()),
                        "Aztec Gold · Exchange 1 resource or token for 1 gold",
                    )]
                })
            },
            |game, s, _| {
                if !s.choice[0].is_empty() {
                    s.player().gain_resources(game, ResourcePile::gold(1));
                }
            },
        );
    }
    b.build()
}
pub(crate) fn tribute_city(game: &Game, p: &Player, from: Position, to: Position) -> bool {
    p.has_special_advance(SpecialAdvance::TributeEmpire)
        && from.distance(to) <= 2
        && game
            .try_get_any_city(to)
            .is_some_and(|city| city.player_index != p.index)
}
fn tribute_empire() -> SpecialAdvanceInfo {
    let mut b=SpecialAdvanceInfo::builder(SpecialAdvance::TributeEmpire,SpecialAdvanceRequirement::AnyGovernment,"Tribute Empire","Collect from enemy cities within 2 spaces, including Barbarian cities, regardless of their mood. With Captives, take a Barbarian army piece from a city instead of its resource.")
        .add_transient_event_listener(|e|&mut e.collect_options,20,|info,ctx,game,p|{
            for enemy in game.players.iter().filter(|enemy|enemy.index!=p.index){for city in &enemy.cities{
                if !tribute_city(game,p.get(game),ctx.city_position,city.position){continue;}
                if let Some(terrain)=game.map.get(city.position) {if let Some(options)=ctx.terrain_options.get(terrain){info.choices.insert(city.position,options.clone());}}
                if enemy.civilization.is_barbarian()&&p.get(game).has_special_advance(SpecialAdvance::Captives)&&p.get(game).captives.len()<4&&enemy.get_units(city.position).iter().any(|u|u.is_army_unit()&&!u.is_leader()) {info.choices.entry(city.position).or_default().insert(ResourcePile::captives(1));}
            }}
        });
    for i in 0..4 {
        b = b.add_units_request(
            |e| &mut e.collect,
            100 - i,
            |game, _, info| {
                let pos = *info.captive_cities.first()?;
                let enemy = game.get_any_city(pos).player_index;
                Some(UnitsRequest::new(
                    enemy,
                    game.player(enemy)
                        .get_units(pos)
                        .iter()
                        .filter(|u| u.is_army_unit() && !u.is_leader())
                        .map(|u| u.id)
                        .collect(),
                    1..=1,
                    "Tribute Empire · Choose a Barbarian captive",
                ))
            },
            |game, s, info| {
                let pos = info.captive_cities.remove(0);
                let owner = game.get_any_city(pos).player_index;
                let id = s.choice[0];
                let unit_type = game.player(owner).get_unit(id).unit_type;
                crate::player::remove_unit(owner, id, game);
                hold(
                    game,
                    &s.player(),
                    Captive {
                        owner,
                        unit_type,
                        id,
                    },
                );
            },
        );
    }
    b.build()
}
pub(crate) fn tributes_city(p: &Player, pos: Position) -> bool {
    p.active_leader() == Some(Leader::Acamapichtli)
        && leader_position(p) == pos
        && p.try_get_city(pos).is_some_and(|c| c.size() <= 3)
}
fn growth(i: &mut crate::player_events::CostInfo, game: &Game, p: &EventPlayer) {
    if i.city_position != Some(leader_position(p.get(game))) {
        return;
    }
    let resources = ResourceType::resources();
    let mut pairs = Vec::new();
    for a in &resources {
        for b in &resources {
            pairs.push(ResourcePile::of(*a, 1) + ResourcePile::of(*b, 1));
        }
    }
    i.cost.conversions.push(PaymentConversion::resource_options(
        pairs,
        ResourcePile::captives(1),
        PaymentConversionType::MayOverpay(4),
    ));
    i.info
        .add_log(p, "Growth: each captive replaces 2 construction resources");
}
fn acamapichtli() -> LeaderInfo {
    LeaderInfo::new(Leader::Acamapichtli,"Acamapichtli",
        LeaderAbility::builder("Growth","When building a building or wonder in Acamapichtli's city, each captive may replace 2 resources.")
            .add_transient_event_listener(|e|&mut e.building_cost,14,|i,_,game,p|growth(i,game,p))
            .add_transient_event_listener(|e|&mut e.wonder_cost,14,|i,_,game,p|growth(i,game,p)).build(),
        LeaderAbility::builder("Tributes","Once per turn, pay 2 mood tokens to Collect from Acamapichtli's city as a free action, if its size is 3 or less.")
            .add_action_modifier(crate::content::custom_actions::PlayingActionModifier::Tributes,|c|c.once_per_turn().free_action().resources(ResourcePile::mood_tokens(2)),crate::playing_actions::PlayingActionType::Collect).build())
}
fn ahuitzotl() -> LeaderInfo {
    LeaderInfo::new(Leader::Ahuitzotl,"Ahuitzotl",
        LeaderAbility::builder("Expansionist","In a battle with Ahuitzotl, cancel 1 hit each round if no Aztec city is within 2 spaces of him.")
            .add_simple_persistent_event_listener(|e|&mut e.combat_round_end,35,|game,p,r|{
                let role=r.combat.role(p.index);
                if r.combat.has_leader(role,game)&&p.get(game).cities.iter().all(|c|c.position.distance(leader_position(p.get(game)))>2){r.update_hits(r.combat.opponent_role(p.index),true,|hits|hits.opponent_hit_cancels+=1);p.log(game,"Expansionist cancels 1 hit");}
            }).build(),
        LeaderAbility::builder("Mass Sacrifice","Once per turn, sacrifice 2 captives to construct a building or wonder in Ahuitzotl's city as a free action, paying its normal cost.")
            .add_custom_action(CustomActionType::MassSacrifice,|c|c.once_per_turn().free_action().resources(ResourcePile::captives(2)),super::construction::purchase,|game,p|!super::construction::choices(game,p,&ResourcePile::captives(2)).is_empty()).build())
}
#[derive(Serialize, Deserialize)]
struct StormSnapshot {
    position: Position,
    mood: crate::city::MoodState,
    units: Vec<(u32, UnitType)>,
}
fn storm_lost(_game: &Game, p: &Player) -> Option<StormSnapshot> {
    let snapshot: StormSnapshot =
        serde_json::from_str(p.event_info.get("Storm Master snapshot")?).ok()?;
    let city = p.try_get_city(snapshot.position)?;
    (city.mood_state < snapshot.mood
        || snapshot
            .units
            .iter()
            .any(|(id, _)| !p.units.iter().any(|u| u.id == *id)))
    .then_some(snapshot)
}
fn fear_destinations(game: &Game, owner: usize, id: u32) -> Vec<Position> {
    let unit = game.player(owner).get_unit(id);
    unit.position
        .neighbors()
        .into_iter()
        .filter(|to| {
            game.map.is_land(*to)
                && game.enemy_player(owner, *to).is_none()
                && (!unit.is_army_unit()
                    || crate::player::can_add_army_unit(game.player(owner), *to))
        })
        .collect()
}
fn fear_targets(game: &Game, p: &Player) -> Vec<Position> {
    let from = leader_position(p);
    game.players
        .iter()
        .filter(|e| e.index != p.index)
        .flat_map(|e| {
            e.units
                .iter()
                .filter(|u| {
                    !u.is_ship()
                        && !u.is_transported()
                        && from.distance(u.position) == 1
                        && !fear_destinations(game, e.index, u.id).is_empty()
                })
                .map(|u| u.position)
        })
        .collect()
}
fn marqzen() -> LeaderInfo {
    LeaderInfo::new(Leader::Marqzen,"Marqzen",
        LeaderAbility::builder("Storm Master","When an event's text lowers mood or removes non-leader units in Marqzen's city, you may restore that mood and those units. Event icons still apply.")
            .add_simple_persistent_event_listener(|e|&mut e.incident,99,|game,p,i|{
                if i.consumed{return;}
                let position=leader_position(p.get(game));if let Some(city)=p.get(game).try_get_city(position){
                    let snapshot=StormSnapshot{position,mood:city.mood_state.clone(),units:p.get(game).get_units(position).iter().filter(|u|!u.is_leader()).map(|u|(u.id,u.unit_type)).collect()};
                    p.get_mut(game).event_info.insert("Storm Master snapshot".into(),serde_json::to_string(&snapshot).unwrap());
                }
            })
            .add_bool_request(|e|&mut e.incident,-1000,|game,p,i|(!i.consumed&&storm_lost(game,p.get(game)).is_some()).then(||"Storm Master · Restore the mood and units lost to this event?".into()),|game,s,_|{
                if s.choice {if let Some(snapshot)=storm_lost(game,s.player().get(game)){
                    let city=s.player().get_mut(game).get_city_mut(snapshot.position);if city.mood_state<snapshot.mood{city.mood_state=snapshot.mood;}
                    for (id,kind) in snapshot.units {if !s.player().get(game).units.iter().any(|u|u.id==id){crate::player::gain_unit(game,&s.player(),snapshot.position,kind);}}
                    s.log(game,&format!("Storm Master restored event losses at {}",snapshot.position));
                }}s.player().get_mut(game).event_info.remove("Storm Master snapshot");
            }).build(),
        LeaderAbility::builder("Spread of Fear","Once per turn, as an action, move an enemy land unit adjacent to Marqzen one land space without causing a battle.")
            .add_custom_action(CustomActionType::SpreadOfFear,|c|c.once_per_turn().action().no_resources(),|b|b
                .add_position_request(|e|&mut e.custom_action,3,|game,p,_|Some(PositionRequest::new(fear_targets(game,p.get(game)),1..=1,"Spread of Fear · Select an adjacent enemy space")),|_,s,a|a.action.city=Some(s.choice[0]))
                .add_units_request(|e|&mut e.custom_action,2,|game,p,a|{
                    let pos=a.action.city.unwrap();let enemy=game.players.iter().find(|e|e.index!=p.index&&e.get_units(pos).iter().any(|u|!u.is_ship()&&!u.is_transported())).unwrap();
                    Some(UnitsRequest::new(enemy.index,enemy.get_units(pos).iter().filter(|u|!u.is_ship()&&!u.is_transported()&&!fear_destinations(game,enemy.index,u.id).is_empty()).map(|u|u.id).collect(),1..=1,"Select the unit to move"))
                },|game,s,a|{let pos=a.action.city.unwrap();let enemy=game.players.iter().find(|e|e.index!=s.player_index&&e.get_units(pos).iter().any(|u|u.id==s.choice[0])).unwrap();a.selected_unit=Some((enemy.index,s.choice[0]));})
                .add_position_request(|e|&mut e.custom_action,1,|game,_,a|{let(owner,id)=a.selected_unit.unwrap();Some(PositionRequest::new(fear_destinations(game,owner,id),1..=1,"Choose the enemy unit's destination"))},|game,s,a|{let(owner,id)=a.selected_unit.unwrap();set_unit_position(owner,id,s.choice[0],game);s.log(game,&format!("Moved enemy unit to {}",s.choice[0]));}),
                |game,p|!fear_targets(game,p).is_empty()).build())
}
