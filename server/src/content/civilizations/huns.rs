// Monumental Edition components: https://boardgamegeek.com/image/9490522
use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::city::{MoodState, activate_city};
use crate::civilization::Civilization;
use crate::combat::update_combat_strength;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::PaymentRequest;
use crate::events::{EventOrigin, EventPlayer};
use crate::game::{Game, GameState};
use crate::leader::{Leader, LeaderInfo, leader_position};
use crate::leader_ability::LeaderAbility;
use crate::map::Terrain;
use crate::movement::{CurrentMove, MoveState, MoveUnits};
use crate::player::Player;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::unit::{Unit, UnitType};

pub(crate) fn huns() -> Civilization {
    Civilization::new("Huns",vec![
        SpecialAdvanceInfo::builder(SpecialAdvance::Nomads,SpecialAdvanceRequirement::Advance(Advance::Storage),"Nomads","Activate a non-Angry size 1 city to move it one land space, alone or with units, during a Move action. The destination must have no city or enemies. Gain 1 resource matching the destination; Barren and Exhausted land give none.").build(),
        SpecialAdvanceInfo::builder(SpecialAdvance::MountedArchers,SpecialAdvanceRequirement::Advance(Advance::Husbandry),"Mounted Archers","Size 1 cities may recruit Cavalry without a Market. After recruiting Cavalry, you may move any of the newly recruited Cavalry as a free action.")
            .add_simple_persistent_event_listener(|e|&mut e.recruit,11,|game,p,r|{
                if r.units.cavalry==0{return;}
                let player=p.get(game);let first=player.next_unit_id-r.units.amount() as u32;
                let moved_units=player.units.iter().filter(|u|u.id<first||u.unit_type!=UnitType::Cavalry).map(|u|u.id).collect();
                game.state=GameState::Movement(MoveState{unit_only:true,moved_units,movement_actions_left:r.units.cavalry as u32,..MoveState::default()});
                p.log(game,"Mounted Archers · Move the newly recruited Cavalry for free");
            }).build(),
        SpecialAdvanceInfo::builder(SpecialAdvance::Raiders,SpecialAdvanceRequirement::Advance(Advance::Tactics),"Raiders","Enemy Settlers within 2 spaces of your armies cannot found cities or form trade routes unless they share a space with their own army or city.").build(),
        SpecialAdvanceInfo::builder(SpecialAdvance::HunnicTribes,SpecialAdvanceRequirement::AnyGovernment,"Hunnic Tribes","Pay 1 gold or culture token to choose the direction of all Barbarian armies moving because of an event you draw.")
            .add_payment_request_listener(|e|&mut e.incident,crate::incident::BASE_EFFECT_PRIORITY+150,|game,p,i|{
                (p.index==i.active_player&&i.barbarians.as_ref().is_some_and(|s|s.move_units)).then(||vec![PaymentRequest::optional(p.payment_options().sum(p.get(game),1,&[ResourceType::Gold,ResourceType::CultureTokens]),"Hunnic Tribes · Control Barbarian movement")])
            },|_,s,i|{if !s.choice[0].is_empty(){i.get_barbarian_state().hunnic_tribes=true;}}).build(),
    ],vec![attila(),bleda(),rugila()],None)
}
pub(crate) fn raided_settler(game: &Game, u: &Unit) -> bool {
    if !u.is_settler() {
        return false;
    }
    let p = game.player(u.player_index);
    if p.try_get_city(u.position).is_some()
        || p.get_units(u.position)
            .iter()
            .any(|other| other.is_army_unit())
    {
        return false;
    }
    game.players.iter().any(|enemy| {
        enemy.index != p.index
            && enemy.has_special_advance(SpecialAdvance::Raiders)
            && enemy
                .units
                .iter()
                .any(|army| army.is_army_unit() && army.position.distance(u.position) <= 2)
    })
}
pub(crate) fn city_destinations(
    game: &Game,
    p: &Player,
    from: Position,
    units: &[u32],
) -> Vec<Position> {
    let Some(city) = p.try_get_city(from) else {
        return vec![];
    };
    if !p.has_special_advance(SpecialAdvance::Nomads)
        || city.size() != 1
        || city.mood_state == MoodState::Angry
        || !city.can_activate()
        || city.nomad_mountain
        || !game.events.is_empty()
    {
        return vec![];
    }
    match &game.state {
        GameState::Movement(s)
            if !s.unit_only && s.movement_actions_left > 0 && !s.moved_cities.contains(&from) =>
        {
            ()
        }
        GameState::Playing if game.actions_left > 0 => (),
        _ => return vec![],
    }
    // Bonus movement belongs to the units recruited by the triggering ability.
    if matches!(game.state, GameState::Movement(_))
        && game.events.iter().any(|e| {
            matches!(
                e.event_type,
                crate::content::persistent_events::PersistentEventType::Recruit(_)
            )
        })
    {
        return vec![];
    }
    if units.iter().any(|id| {
        p.units
            .iter()
            .find(|u| u.id == *id)
            .is_none_or(|u| u.position != from)
    }) {
        return vec![];
    }
    let routes = if units.is_empty() {
        None
    } else {
        Some(crate::movement::possible_move_routes(p, game, units, from, None).unwrap_or_default())
    };
    from.neighbors()
        .into_iter()
        .filter(|to| {
            (game.map.is_land(*to) || game.map.get(*to) == Some(&Terrain::Unexplored))
                && game.try_get_any_city(*to).is_none()
                && game.enemy_player(p.index, *to).is_none()
                && routes.as_ref().is_none_or(|r| {
                    r.iter().any(|r| {
                        r.destination == *to
                            && !r.ignore_terrain_movement_restrictions
                            && r.cost.is_free()
                    })
                })
        })
        .collect()
}
pub(crate) fn execute_city_move(
    game: &mut Game,
    p: &EventPlayer,
    m: &MoveUnits,
) -> Result<(), String> {
    let from = m.city.ok_or("Choose a city")?;
    if !m.payment.is_empty()
        || m.embark_carrier_id.is_some()
        || !city_destinations(game, p.get(game), from, &m.units).contains(&m.destination)
    {
        return Err("Illegal Nomads move".into());
    }
    let GameState::Movement(state) = &mut game.state else {
        return Err("Start a Move action".into());
    };
    state.movement_actions_left -= 1;
    state.moved_cities.push(m.destination);
    state.moved_units.extend(&m.units);
    state.current_move = CurrentMove::None;
    p.get_mut(game)
        .event_info
        .insert("Nomads moving".into(), from.to_string());
    if game.map.get(m.destination) == Some(&Terrain::Unexplored) {
        crate::explore::move_to_unexplored_tile(game, p, &m.units, from, m.destination);
    } else {
        crate::movement::move_units(game, p.index, &m.units, m.destination, None);
    }
    Ok(())
}
pub(crate) fn finish_city_move(game: &mut Game, player: usize, from: Position, to: Position) {
    game.player_mut(player).event_info.remove("Nomads moving");
    let origin = EventOrigin::SpecialAdvance(SpecialAdvance::Nomads);
    activate_city(from, game, &origin);
    let mountain = game.map.get(to) == Some(&Terrain::Mountain);
    let city = game.player_mut(player).get_city_mut(from);
    city.position = to;
    city.nomad_mountain = mountain;
    let pile = match game.map.get(to) {
        Some(Terrain::Fertile) => ResourcePile::food(1),
        Some(Terrain::Forest) => ResourcePile::wood(1),
        Some(Terrain::Mountain) => ResourcePile::ore(1),
        _ => ResourcePile::empty(),
    };
    let p = EventPlayer::from_player(player, game, origin);
    p.log(game, &format!("Moved city {from} to {to}"));
    if !pile.is_empty() {
        p.gain_resources(game, pile);
    }
}
fn attila() -> LeaderInfo {
    LeaderInfo::new(Leader::Attila,"Attila",
        LeaderAbility::builder("Ruler","Happy or Neutral size 1 cities containing Attila act one size larger when collecting or recruiting.").build(),
        LeaderAbility::builder("Horse Master","In the first combat round, one Cavalry in Attila's army may use the Elephant die ability when rolling a Cavalry symbol.")
            .add_bool_request(|e|&mut e.combat_round_end,99,|game,p,r|{
                let role=r.combat.role(p.index);let hits=if role.is_attacker(){&r.attacker}else{&r.defender};
                (r.combat.stats.round==1&&r.combat.has_leader(role,game)).then_some(hits.horse_master_value).flatten().map(|value|format!("Horse Master · Cancel 1 incoming hit instead of {value} combat value?"))
            },|game,s,r|{if s.choice{let role=r.combat.role(s.player_index);let value=if role.is_attacker(){r.attacker.horse_master_value.unwrap()}else{r.defender.horse_master_value.unwrap()};r.update_hits(role,true,|h|h.combat_value=h.combat_value.saturating_sub(value));r.update_hits(r.combat.opponent_role(s.player_index),true,|h|h.opponent_hit_cancels+=1);s.log(game,&format!("Horse Master · Sacrificed {value} combat value to cancel 1 hit"));}}).build())
}
fn bleda() -> LeaderInfo {
    LeaderInfo::new(Leader::Bleda,"Bleda",
        LeaderAbility::builder("Enforcer","Bleda's city may act Happy for its first activation, regardless of mood. It cannot activate again that turn.")
            .add_custom_action(CustomActionType::Enforcer,|c|c.any_times().free_action().no_resources(),|b|b.add_simple_persistent_event_listener(|e|&mut e.custom_action,0,|game,p,_|{let pos=leader_position(p.get(game));p.get_mut(game).get_city_mut(pos).enforcer=true;p.log(game,"The leader's city will act Happy for its only activation this turn");}),|_,p|p.try_get_city(leader_position(p)).is_some_and(|c|c.activations==0&&!c.enforcer)).build(),
        LeaderAbility::builder("Battering Rams","In the first round when Bleda attacks a city, the defender cannot play a tactics card.")
            .add_simple_persistent_event_listener(|e|&mut e.combat_round_start_allow_tactics,120,|game,p,r|{if r.combat.stats.round==1&&r.combat.attacker()==p.index&&r.combat.stats.battleground.is_city()&&r.combat.has_leader(r.combat.role(p.index),game){update_combat_strength(game,r.combat.defender(),r,|_,_,s,_|{s.deny_tactics_card=true;s.roll_log.push("Battering Rams prevents tactics".into());});}}).build())
}
fn rugila() -> LeaderInfo {
    LeaderInfo::new(Leader::Rugila,"Rugila",
        LeaderAbility::builder("Tribute","After Rugila captures a city, gain 2 gold if another enemy city, including a Barbarian city, is within 2 spaces.")
            .add_simple_persistent_event_listener(|e|&mut e.combat_end,25,|game,p,c|{if c.player(p.index).survived_leader()&&c.captured_city(p.index).is_some()&&game.players.iter().filter(|enemy|enemy.index!=p.index).flat_map(|enemy|&enemy.cities).any(|city|city.position.distance(c.defender.position)<=2){p.gain_resources(game,ResourcePile::gold(2));}}).build(),
        LeaderAbility::builder("Unifier","A Barbarian city captured by Rugila becomes Happy after resolving the capture.")
            .add_simple_persistent_event_listener(|e|&mut e.combat_end,-35,|game,p,c|{if c.player(p.index).survived_leader()&&c.captured_city(p.index).is_some()&&c.opponent_player(p.index,game).civilization.is_barbarian(){crate::city::set_city_mood(game,c.defender.position,&p.origin,MoodState::Happy);}}).build())
}
