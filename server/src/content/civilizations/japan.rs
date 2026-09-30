// Monumental Edition components: https://boardgamegeek.com/image/9490524
use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::pay_action;
use crate::advance::{Advance, gain_advance_without_payment};
use crate::card::{HandCard, HandCardLocation};
use crate::city_pieces::{BUILDINGS, Building};
use crate::civilization::Civilization;
use crate::combat::{Combat, CombatModifier};
use crate::content::ability::{Ability, AbilityBuilder};
use crate::content::custom_actions::{
    CustomAction, CustomActionActivation, CustomActionType, on_custom_action,
};
use crate::content::persistent_events::{
    AdvanceRequest, HandCardsRequest, PaymentRequest, PersistentEventType, SelectedStructure,
    StructuresRequest,
};
use crate::events::{EventOrigin, EventPlayer};
use crate::game::Game;
use crate::leader::{Leader, LeaderInfo, leader_position};
use crate::leader_ability::LeaderAbility;
use crate::player::{CostTrigger, Player};
use crate::playing_actions::PlayingAction;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::structure::Structure;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CardAnnouncement {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<usize>,
    pub player: usize,
    pub card: u8,
    pub cancelled: bool,
    pub shogunate: bool,
}
impl CardAnnouncement {
    pub(crate) fn new(player: usize, card: u8) -> Self {
        Self {
            player,
            card,
            cancelled: false,
            shogunate: false,
            target: None,
        }
    }
}
pub(crate) fn shogunate_available(p: &Player) -> bool {
    p.has_special_advance(SpecialAdvance::Shogunate) && !p.event_info.contains_key("Shogunate card")
}
pub(crate) fn on_declare_card(game: &mut Game, a: CardAnnouncement) -> Result<(), String> {
    let players = game.human_players_sorted(a.player);
    let Some(a) = game.trigger_persistent_event(
        &players,
        |e| &mut e.declare_action_card,
        a,
        PersistentEventType::DeclareActionCard,
    ) else {
        return Ok(());
    };
    if a.cancelled {
        crate::action_card::discard_action_card(
            game,
            a.player,
            a.card,
            &EventOrigin::SpecialAdvance(SpecialAdvance::Subterfuge),
            HandCardLocation::DiscardPile,
        );
        return Ok(());
    }
    if let Some(target) = a.target {
        game.player_mut(a.player).event_info.insert(
            "Announced card target".into(),
            serde_json::to_string(&(a.card, target)).unwrap(),
        );
    }
    if a.shogunate {
        game.player_mut(a.player)
            .event_info
            .insert("Shogunate card".into(), "used".into());
    }
    if !a.shogunate && !game.cache.get_civil_card(a.card).action_type.free {
        pay_action(
            game,
            &EventPlayer::from_player(a.player, game, EventOrigin::CivilCard(a.card)),
        );
    }
    PlayingAction::ActionCard(a.card).execute_without_action_cost(game, a.player)
}
fn near_opponent(game: &Game, p: &Player, opponent: usize) -> bool {
    p.cities
        .iter()
        .map(|c| c.position)
        .chain(
            p.units
                .iter()
                .filter(|u| u.is_army_unit())
                .map(|u| u.position),
        )
        .any(|pos| {
            let enemy = game.player(opponent);
            enemy
                .cities
                .iter()
                .map(|c| c.position)
                .chain(enemy.units.iter().map(|u| u.position))
                .any(|other| pos.distance(other) <= 2)
        })
}
pub(crate) fn japan() -> Civilization {
    Civilization::new("Japan",vec![
        SpecialAdvanceInfo::builder(SpecialAdvance::Pottery,SpecialAdvanceRequirement::Advance(Advance::Storage),"Pottery","Once per turn, gain 1 culture token when you collect at least 3 food.")
            .add_simple_persistent_event_listener(|e|&mut e.collect,15,|game,p,c|{
                if c.total.food>=3&&!p.get(game).event_info.contains_key("Pottery") {
                    p.get_mut(game).event_info.insert("Pottery".into(),"used".into());p.gain_resources(game,ResourcePile::culture_tokens(1));
                }
            }).build(),
        SpecialAdvanceInfo::builder(SpecialAdvance::Horsemanship,SpecialAdvanceRequirement::Advance(Advance::Husbandry),"Horsemanship","Recruit Cavalry in Fortress cities. In the first round defending a Fortress, one Cavalry die ability adds +3 combat value instead of +2.").build(),
        SpecialAdvanceInfo::builder(SpecialAdvance::Subterfuge,SpecialAdvanceRequirement::Advance(Advance::Tactics),"Subterfuge","Discard an action card to cancel another player's non-combat action card when their units or cities are within 2 spaces of your cities or armies. The cancelled card is discarded; its costs are not paid.")
            .add_hand_card_request(|e|&mut e.declare_action_card,10,|game,p,a|{
                if a.cancelled||a.player==p.index||!near_opponent(game,p.get(game),a.player)||p.get(game).action_cards.is_empty(){return None;}
                Some(HandCardsRequest::new(p.get(game).action_cards.iter().map(|id|HandCard::ActionCard(*id)).collect(),0..=1,&format!("Subterfuge · Discard a card to cancel {}{}",game.cache.get_civil_card(a.card).name,a.target.map(|target|format!(" targeting {}",game.player(target).civilization.name)).unwrap_or_default())))
            },|game,s,a|{if let Some(HandCard::ActionCard(id))=s.choice.first(){
                crate::action_card::discard_action_card(game,s.player_index,*id,&s.origin,HandCardLocation::DiscardPile);a.cancelled=true;s.log(game,"Cancelled the action card. Its action and resource costs are not paid.");
            }}).build(),
        SpecialAdvanceInfo::builder(SpecialAdvance::Shogunate,SpecialAdvanceRequirement::AnyGovernment,"Shogunate","Once per turn, play an action-cost Action or Event card as a free action. Once per turn, Draft may buy an action card instead of an Infantry.")
            .add_bool_request(|e|&mut e.declare_action_card,20,|game,p,a|{
                if a.player==p.index&&shogunate_available(p.get(game))&&!game.cache.get_civil_card(a.card).action_type.free&&game.actions_left==0 { a.shogunate=true; return None; }
                (a.player==p.index&&shogunate_available(p.get(game))&&!game.cache.get_civil_card(a.card).action_type.free).then(||"Shogunate · Play this card without spending an action?".into())
            },|game,s,a|{a.shogunate=s.choice;if !s.choice {assert!(game.actions_left>0,"No actions left; use Shogunate");}}).build(),
    ],vec![go_toba(),jimmu(),suiko()],None)
}
fn scholar_choices(game: &Game, p: &Player) -> Vec<Advance> {
    game.cache
        .get_advances()
        .values()
        .filter(|a| {
            p.can_advance_free(a.advance, game)
                && p.can_afford(
                    &p.advance_cost(a.advance, game, CostTrigger::NoModifiers)
                        .cost,
                )
        })
        .map(|a| a.advance)
        .collect()
}
fn scholar(b: AbilityBuilder) -> AbilityBuilder {
    b.add_advance_request(
        |e| &mut e.custom_action,
        2,
        |game, p, _| Some(AdvanceRequest::new(scholar_choices(game, p.get(game)))),
        |game, s, a| {
            a.advance_purchase = Some((
                s.choice,
                s.player()
                    .get(game)
                    .advance_cost(s.choice, game, game.execute_cost_trigger()),
            ));
        },
    )
    .add_payment_request_listener(
        |e| &mut e.custom_action,
        1,
        |_, _, a| {
            Some(vec![PaymentRequest::mandatory(
                a.advance_purchase.as_ref().unwrap().1.cost.clone(),
                "Scholar · Research cost",
            )])
        },
        |game, s, a| {
            let (advance, cost) = a.advance_purchase.take().unwrap();
            cost.info.execute(game);
            gain_advance_without_payment(game, advance, &s.player(), s.choice[0].clone(), true);
        },
    )
}
fn sword_modifier(c: &Combat, p: usize) -> CombatModifier {
    if c.attacker() == p {
        CombatModifier::WayOfTheSwordAttacker
    } else {
        CombatModifier::WayOfTheSwordDefender
    }
}
fn go_toba() -> LeaderInfo {
    LeaderInfo::new(Leader::GoToba,"Go-Toba",
        LeaderAbility::builder("Way of the Sword","Before battle, pay 1 ore for +2 combat value in every round, even if Go-Toba dies.")
            .add_payment_request_listener(|e|&mut e.combat_start,4,|game,p,c| c.has_leader(c.role(p.index),game).then(||vec![PaymentRequest::optional(p.payment_options().resources(p.get(game),ResourcePile::ore(1)),"Way of the Sword · +2 combat value each round")]),|_,s,c|{if !s.choice[0].is_empty(){c.modifiers.push(sword_modifier(c,s.player_index));}}).build(),
        LeaderAbility::builder("Scholar","After constructing an Academy or Observatory in Go-Toba's city, you may buy an advance at its normal resource cost without spending an action.")
            .add_custom_action(CustomActionType::Scholar,|c|c.any_times().free_action().no_resources(),scholar,|_,_|false)
            .add_bool_request(|e|&mut e.construct,-30,|game,p,c|{
                (c.city_position==Some(leader_position(p.get(game)))&&matches!(c.building,Building::Academy|Building::Observatory)&&!scholar_choices(game,p.get(game)).is_empty()).then(||"Scholar · Buy an advance?".into())
            },|game,s,_|{if s.choice{on_custom_action(game,s.player_index,CustomActionActivation::new(CustomAction::new(CustomActionType::Scholar,None),ResourcePile::empty()));}}).build())
}
fn jimmu() -> LeaderInfo {
    LeaderInfo::new(Leader::Jimmu,"Jimmu",
        LeaderAbility::builder("Fortified","Your Fortresses within 2 spaces of Jimmu add +2 combat value in every round, including against Siegecraft.")
            .add_combat_strength_listener(110,|game,c,s,r|{if !r.is_attacker()&&c.defender_fortress(game)&&leader_position(game.player(c.player(r))).distance(c.defender_position())<=2{s.extra_combat_value+=2;s.roll_log.push("Fortified adds +2 combat value".into());}}).build(),
        LeaderAbility::builder("Mythical","After Jimmu survives a battle, you may pay 1 culture token to recover one tactics card you used in that battle.")
            .add_hand_card_request(|e|&mut e.combat_end,30,|game,p,c|{
                let stats=c.player(p.index);if !stats.survived_leader()||p.get(game).resources.culture_tokens==0{return None;}
                let cards=stats.tactics_cards.iter().filter(|id|game.action_cards_discarded.contains(id)).map(|id|HandCard::ActionCard(*id)).collect::<Vec<_>>();
                (!cards.is_empty()).then(||HandCardsRequest::new(cards,0..=1,"Mythical · Pay 1 culture to recover a tactics card"))
            },|game,s,_|{if let Some(HandCard::ActionCard(id))=s.choice.first(){
                crate::resource::pay_cost(game,s.player_index,&PaymentRequest::mandatory(s.player().payment_options().resources(s.player().get(game),ResourcePile::culture_tokens(1)),"Mythical"),&ResourcePile::culture_tokens(1));
                game.action_cards_discarded.retain(|c|c!=id);crate::action_card::gain_action_card(game,&s.player(),*id,HandCardLocation::DiscardPile);
            }}).build())
}
fn reclaimable(game: &Game, p: &Player) -> Vec<SelectedStructure> {
    let pos = leader_position(p);
    let Some(city) = p.try_get_city(pos) else {
        return vec![];
    };
    BUILDINGS
        .into_iter()
        .filter(|b| {
            *b != Building::Obelisk
                && city
                    .pieces
                    .building_owner(*b)
                    .is_some_and(|owner| owner != p.index)
                && p.is_building_available(*b, game)
        })
        .map(|b| SelectedStructure::new(pos, Structure::Building(b)))
        .collect()
}
fn suiko() -> LeaderInfo {
    LeaderInfo::new(Leader::Suiko,"Suiko",
        LeaderAbility::builder("Japanese Buddhism","As an action, pay 1 culture token to reclaim an influenced building other than an Obelisk in Suiko's city. Free action with State Religion.")
            .add_custom_action(CustomActionType::JapaneseBuddhism,|c|c.any_times().action().culture_tokens(1),|b|b.add_structures_request(|e|&mut e.custom_action,0,|game,p,_|Some(StructuresRequest::new(reclaimable(game,p.get(game)),1..=1,"Japanese Buddhism · Reclaim a building")),|game,s,_|{if let Structure::Building(b)=s.choice[0].structure { s.player().get_mut(game).get_city_mut(s.choice[0].position).pieces.set_building(b,s.player_index);s.log(game,&format!("Reclaimed {b} at {}",s.choice[0].position)); }}),|game,p|!reclaimable(game,p).is_empty()).build(),
        LeaderAbility::builder("Defender","Suiko adds +2 combat value when defending.").add_combat_strength_listener(111,|game,c,s,r|{if !r.is_attacker()&&c.has_leader(r,game){s.extra_combat_value+=2;s.roll_log.push("Defender adds +2 combat value".into());}}).build())
}
pub(crate) fn use_way_of_the_sword() -> Ability {
    Ability::builder("Way of the Sword", "")
        .add_combat_strength_listener(121, |_, c, s, r| {
            if c.modifiers.contains(&sword_modifier(c, c.player(r))) {
                s.extra_combat_value += 2;
                s.roll_log
                    .push("Way of the Sword adds +2 combat value".into());
            }
        })
        .build()
}

pub(crate) fn announce_card_target() -> Ability {
    Ability::builder("Card target", "")
        .add_player_request(
            |e| &mut e.declare_action_card,
            30,
            |game, p, a| {
                if p.index != a.player
                    || !game.players.iter().any(|other| {
                        other.index != p.index
                            && other.has_special_advance(SpecialAdvance::Subterfuge)
                            && !other.action_cards.is_empty()
                            && near_opponent(game, other, p.index)
                    })
                {
                    return None;
                }
                game.cache
                    .get_civil_card(a.card)
                    .target_choices
                    .clone()
                    .and_then(|choices| choices(game, p.index))
            },
            |_, s, a| a.target = Some(s.choice),
        )
        .build()
}
