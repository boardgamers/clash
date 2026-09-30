// Monumental Edition components: https://boardgamegeek.com/image/9490528
use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::combat::{get_combat_strength, update_combat_strength};
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::{PaymentRequest, ResourceRewardRequest};
use crate::game::Game;
use crate::leader::{Leader, LeaderInfo, leader_position};
use crate::leader_ability::{LeaderAbility, activate_leader_city, can_activate_leader_city};
use crate::payment::{PaymentConversion, PaymentConversionType, base_resources};
use crate::player::Player;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::unit::UnitType;

pub(crate) fn persia() -> Civilization {
    Civilization::new("Persia", vec![
        SpecialAdvanceInfo::builder(SpecialAdvance::PersianElephants, SpecialAdvanceRequirement::Advance(Advance::Husbandry), "Elephants",
            "You may recruit Elephants without a Market in your cities adjacent to Barren land. In the first combat round, an Elephant in your army limits the opponent’s combat bonus to +2.").build(),
        SpecialAdvanceInfo::builder(SpecialAdvance::Immortals, SpecialAdvanceRequirement::Advance(Advance::Draft), "Immortals",
            "Before each combat roll, pay up to 4 culture tokens for that much extra combat value.")
            .add_payment_request_listener(|e| &mut e.combat_round_start, 40, |game, p, _| {
                Some(vec![PaymentRequest::optional(p.payment_options().single_type(p.get(game), ResourceType::CultureTokens, 0..=4), "Immortals · Pay up to 4 culture for combat value")])
            }, |game, s, r| {
                let amount = s.choice[0].amount();
                update_combat_strength(game, s.player_index, r, |_, _, strength, _| { strength.extra_combat_value += amount as i8; if amount > 0 { strength.roll_log.push(format!("Immortals adds +{amount} combat value")); } });
            }).build(),
        SpecialAdvanceInfo::builder(SpecialAdvance::Zoroastrianism, SpecialAdvanceRequirement::Advance(Advance::Priesthood), "Zoroastrianism",
            "Cultural Influence may target an enemy non-leader army unit on land without a city, including Barbarians. Replace it with a matching unit from your supply. If enemies remain, the converted unit attacks them immediately.").build(),
        SpecialAdvanceInfo::builder(SpecialAdvance::Banking, SpecialAdvanceRequirement::Advance(Advance::Currency), "Banking",
            "At the start of your turn, before Trade Routes, gain 1 gold or culture token if you have any gold.")
            .add_resource_request(|e| &mut e.turn_start, 5, |game, p, _| (p.get(game).resources.gold > 0).then(|| ResourceRewardRequest::new(p.reward_options().sum(1, &[ResourceType::Gold, ResourceType::CultureTokens]), "Banking · Gain 1 gold or culture".into()))).build(),
    ], vec![cyrus(), darius(), xerxes()], None)
}
fn cyrus() -> LeaderInfo {
    LeaderInfo::new(Leader::Cyrus, "Cyrus the Great",
        LeaderAbility::builder("King of Persia", "As an action, activate Cyrus' city. Gain 1 mood or culture token per region containing your cities, up to the size of Cyrus' city.")
            .add_custom_action(CustomActionType::KingOfPersia, |c| c.any_times().action().no_resources(), |b| b
                .add_resource_request(|e| &mut e.custom_action, 0, |game, p, _| {
                    let pos = leader_position(p.get(game));
                    let regions = p.get(game).cities.iter().map(|c| crate::map::block_for_position(game, c.position).0).collect::<std::collections::HashSet<_>>().len();
                    let amount = regions.min(p.get(game).get_city(pos).size()) as u8;
                    activate_leader_city(game,p);
                    Some(ResourceRewardRequest::new(p.reward_options().tokens(amount), "King of Persia · Regional tribute".into()))
                }), can_activate_leader_city).build(),
        LeaderAbility::builder("Strategist", "When revealing a tactics card, Cyrus may use +2 combat value instead of the card's effects, even if the opponent cancels the card.")
            .add_bool_request(|e| &mut e.combat_round_start_reveal_tactics, 100, |game, p, r| {
                (r.combat.has_leader(r.combat.role(p.index),game) && get_combat_strength(p.index,r).tactics_card.is_some()).then(|| "Strategist · Replace your tactics card's effects with +2 combat value?".into())
            }, |game,s,r| { if s.choice { r.combat.stats.ignored_tactics_cards.push(get_combat_strength(s.player_index,r).tactics_card.unwrap()); update_combat_strength(game,s.player_index,r,|_,_,strength,_| { strength.extra_combat_value+=2; strength.roll_log.push("Strategist adds +2 combat value".into()); }); } }).build())
}
fn darius() -> LeaderInfo {
    LeaderInfo::new(Leader::Darius, "Darius",
        LeaderAbility::builder("Cultural Unity", "Gain 1 mood token whenever Cultural Influence from Darius' city succeeds.").build(),
        LeaderAbility::builder("Architect", "Once per turn, as a free action, pay 2 culture tokens to construct a building or wonder in Darius' city at its normal resource cost.")
            .add_custom_action(CustomActionType::Architect, |c| c.once_per_turn().free_action().culture_tokens(2),
                |b| crate::content::civilizations::construction::purchase(b),
                |game,p| !crate::content::civilizations::construction::choices(game,p,&ResourcePile::culture_tokens(2)).is_empty()).build())
}
fn xerxes() -> LeaderInfo {
    LeaderInfo::new(Leader::Xerxes, "Xerxes",
        LeaderAbility::builder("Mighty Army", "Xerxes' space can hold 5 army units. While his army has 5 units, it cannot play tactics cards or use die combat abilities.")
            .add_simple_persistent_event_listener(|e| &mut e.combat_round_start_allow_tactics, 100, |game,p,r| {
                if r.combat.has_leader(r.combat.role(p.index),game) && r.combat.fighting_units(game,p.index).len() == 5 {
                    update_combat_strength(game,p.index,r,|_,_,s,_| { s.deny_tactics_card=true; s.deny_combat_abilities=true; s.roll_log.push("Mighty Army: five units, no tactics or die abilities".into()); });
                }
            }).build(),
        LeaderAbility::builder("Great Builder", "Wonders in Xerxes' city cost 2 fewer resources or culture tokens, in any mix.")
            .add_transient_event_listener(|e| &mut e.wonder_cost, 15, |cost,w,game,p| {
                if w.city_position == leader_position(p.get(game)) {
                    let mut from=base_resources();from.push(ResourcePile::culture_tokens(1));from.push(ResourcePile::gold(1));
                    cost.cost.conversions.push(PaymentConversion::resource_options(from,ResourcePile::empty(),PaymentConversionType::MayNotOverpay(2)));
                    cost.info.add_log(p,"Great Builder reduces the wonder cost by 2 resources or culture");
                }
            }).build())
}
pub(crate) fn stack_limit(p: &Player, position: Position) -> usize {
    if p.units
        .iter()
        .any(|u| u.position == position && u.unit_type == UnitType::Leader(Leader::Xerxes))
    {
        5
    } else {
        4
    }
}
pub(crate) fn caps_opponent_bonus(
    game: &Game,
    combat: &crate::combat::Combat,
    player: usize,
) -> bool {
    if combat.stats.round != 1 {
        return false;
    }
    let opponent = game.player(combat.opponent(player));
    (opponent.active_leader() == Some(Leader::Hannibal)
        && combat.has_leader(combat.role(opponent.index), game))
        || opponent.has_special_advance(SpecialAdvance::PersianElephants)
            && combat
                .fighting_units(game, opponent.index)
                .iter()
                .any(|id| opponent.get_unit(*id).unit_type == UnitType::Elephant)
}
