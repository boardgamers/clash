// Monumental Edition components: https://boardgamegeek.com/image/9490529
use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::city::MoodState;
use crate::civilization::Civilization;
use crate::content::ability::Ability;
use crate::content::advances::AdvanceGroup;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::PaymentRequest;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::leader::{Leader, LeaderInfo, leader_position};
use crate::leader_ability::LeaderAbility;
use crate::map::{Block, Terrain};
use crate::player::Player;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::tactics_card::CombatRole;

pub(crate) fn phoenicia() -> Civilization {
    Civilization::new("Phoenicia", vec![
        SpecialAdvanceInfo::builder(SpecialAdvance::CityIndependence, SpecialAdvanceRequirement::Advance(Advance::Fishing),
            "City Independence", "Ports in your color are immune to Cultural Influence and retain their owner when captured. Each is worth 1½ victory points. Your government advances score no points.").build(),
        SpecialAdvanceInfo::builder(SpecialAdvance::Biremes, SpecialAdvanceRequirement::Advance(Advance::WarShips),
            "Biremes", "Gain 2 combat value in naval combat each round, or only in the first round against an opponent with Warships.")
            .add_combat_strength_listener(112, |game, c, s, role| {
                if c.is_sea_battle(game) && (c.stats.round == 1 || !game.player(c.opponent(c.player(role))).can_use_advance(Advance::WarShips)) {
                    s.extra_combat_value += 2;
                    s.roll_log.push("Biremes adds +2 combat value".into());
                }
            }).build(),
        SpecialAdvanceInfo::builder(SpecialAdvance::Alphabet, SpecialAdvanceRequirement::Advance(Advance::Writing),
            "Alphabet", "Up to two Trade Routes may each yield 1 culture token or 2 ideas instead of their usual resource.").build(),
        SpecialAdvanceInfo::builder(SpecialAdvance::CedarsAndDyes, SpecialAdvanceRequirement::Advance(Advance::TradeRoutes),
            "Cedars & Dyes", "Ships and cities with Ports may establish Trade Routes to coastal cities up to 3 spaces away.").build(),
    ], vec![ithobaal(), hiram(), pygmalion()], Some(Block::new([Terrain::Fertile, Terrain::Mountain, Terrain::Forest, Terrain::Water])))
}

pub(crate) fn independent_port(game: &Game, owner: Option<usize>) -> bool {
    owner.is_some_and(|p| {
        game.player(p)
            .has_special_advance(SpecialAdvance::CityIndependence)
    })
}

pub(crate) fn leader_at(p: &Player, leader: Leader, position: Position) -> bool {
    p.active_leader() == Some(leader) && leader_position(p) == position
}

fn ithobaal() -> LeaderInfo {
    LeaderInfo::new(
        Leader::Ithobaal,
        "Ithobaal",
        LeaderAbility::builder(
            "Deliverance",
            "A city already containing your influence becomes Happy after Ithobaal captures it.",
        )
        .add_simple_persistent_event_listener(
            |e| &mut e.combat_start,
            12,
            |game, p, c| {
                if c.role(p.index) == CombatRole::Attacker
                    && c.has_leader(CombatRole::Attacker, game)
                    && c.defender_city(game)
                        .is_some_and(|city| !city.pieces.buildings(Some(p.index)).is_empty())
                {
                    p.get_mut(game)
                        .event_info
                        .insert("Deliverance".into(), c.defender_position().to_string());
                }
            },
        )
        .build(),
        LeaderAbility::builder(
            "Priest of Astarte",
            "Ithobaal adds 2 combat value within 2 spaces of one of your cities with a Temple.",
        )
        .add_combat_strength_listener(114, |game, c, s, role| {
            let p = game.player(c.player(role));
            if c.has_leader(role, game)
                && p.cities.iter().any(|city| {
                    city.pieces.temple.is_some()
                        && city.position.distance(c.defender_position()) <= 2
                })
            {
                s.extra_combat_value += 2;
                s.roll_log
                    .push("Priest of Astarte adds +2 combat value".into());
            }
        })
        .build(),
    )
}

fn hiram() -> LeaderInfo {
    LeaderInfo::new(Leader::Hiram, "Hiram",
        LeaderAbility::builder("Trade Master", "One Trade Route from a settler or ship sharing Hiram's space does not count toward the four-route limit.").build(),
        LeaderAbility::advance_gain_custom_action("Constructor", CustomActionType::Constructor, AdvanceGroup::Construction))
}

fn pygmalion() -> LeaderInfo {
    LeaderInfo::new(Leader::Pygmalion, "Pygmalion",
        LeaderAbility::builder("Mediterranean Trader", "One Trade Route from a settler or ship sharing Pygmalion's space yields an extra gold or culture token.").build(),
        LeaderAbility::builder("Protector", "Once per battle when defending a city, pay 1 culture token to cancel 1 hit. Recover the token if you win.")
            .add_payment_request_listener(|e| &mut e.combat_round_end, 92, |game, p, end| {
                let c = &end.combat;
                if c.role(p.index) != CombatRole::Defender || !c.has_leader(CombatRole::Defender, game)
                    || c.defender_city(game).is_none() || p.get(game).event_info.contains_key("Protector") { return None; }
                let cost = p.payment_options().resources(p.get(game), ResourcePile::culture_tokens(1));
                (p.get(game).can_afford(&cost) && end.update_hits(CombatRole::Attacker, false, |h| h.opponent_hit_cancels += 1))
                    .then(|| vec![PaymentRequest::optional(cost, "Protector · Cancel 1 hit")])
            }, |game, s, end| {
                if !s.choice[0].is_empty() {
                    end.update_hits(CombatRole::Attacker, true, |h| h.opponent_hit_cancels += 1);
                    s.player().get_mut(game).event_info.insert("Protector".into(), "paid".into());
                    s.log(game, "Protector canceled 1 hit");
                }
            }).build())
}

// Paid effects are settled even if the leader was lost during the battle.
pub(crate) fn resolve_leader_battle_effects() -> Ability {
    Ability::builder("Phoenician battle effects", "")
        .add_simple_persistent_event_listener(
            |e| &mut e.combat_end,
            -20,
            |game, p, stats| {
                if p.get_mut(game).event_info.remove("Protector").is_some()
                    && stats.result.as_ref().and_then(|r| r.winner()) == Some(stats.role(p.index))
                {
                    p.with_origin(EventOrigin::LeaderAbility("Protector".into()))
                        .gain_resources(game, ResourcePile::culture_tokens(1));
                }
                if let Some(position) = p.get_mut(game).event_info.remove("Deliverance") {
                    let position = Position::from_offset(&position);
                    if stats.captured_city(p.index).is_some()
                        && stats.player(p.index).survived_leader()
                        && p.get(game).try_get_city(position).is_some()
                    {
                        crate::city::set_city_mood(
                            game,
                            position,
                            &EventOrigin::LeaderAbility("Deliverance".into()),
                            MoodState::Happy,
                        );
                    }
                }
            },
        )
        .build()
}
