// Monumental Edition: https://boardgamegeek.com/image/9490519
// Beloved and Man God: publisher rulebook, p. 33.
use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::city_pieces::Building;
use crate::civilization::Civilization;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::ResourceRewardRequest;
use crate::events::{EventOrigin, EventPlayer};
use crate::game::Game;
use crate::leader::{Leader, LeaderInfo, leader_position};
use crate::leader_ability::LeaderAbility;
use crate::map::{Block, Terrain};
use crate::payment::{PaymentConversion, PaymentConversionType, base_resources};
use crate::player::{Data, Player};
use crate::player_events::CostInfo;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::tactics_card::CombatRole;
use std::collections::HashSet;

pub(crate) fn egypt() -> Civilization {
    Civilization::new(
        "Egypt",
        vec![flood_plains(), architecture(), embalming(), man_god()],
        vec![cleopatra(), imhotep(), ramses()],
        Some(Block::new([
            Terrain::Fertile,
            Terrain::Mountain,
            Terrain::Barren,
            Terrain::Barren,
        ])),
    )
}

fn flood_plains() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::FloodPlains,
        SpecialAdvanceRequirement::Advance(Advance::Irrigation),
        "Flood Plains",
        "Your cities may collect food or wood from Barren spaces. You may found cities on Barren spaces.",
    )
    .add_transient_event_listener(
        |e| &mut e.terrain_collect_options,
        -12,
        |options, _, _, _| {
            options.insert(
                Terrain::Barren,
                HashSet::from([ResourcePile::food(1), ResourcePile::wood(1)]),
            );
        },
    )
    .build()
}

fn architecture() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(SpecialAdvance::Architecture,
        SpecialAdvanceRequirement::Advance(Advance::Engineering), "Architecture",
        "Mood tokens may replace resources or culture tokens when constructing a building or wonder.")
    .add_transient_event_listener(|e| &mut e.building_cost, 12, |i, _, _, p| architecture_cost(i, p))
    .add_transient_event_listener(|e| &mut e.wonder_cost, 12, |i, _, _, p| architecture_cost(i, p))
    .build()
}

fn architecture_cost(i: &mut CostInfo, p: &EventPlayer) {
    let mut from = base_resources();
    from.push(ResourcePile::culture_tokens(1));
    from.push(ResourcePile::gold(1));
    i.cost.conversions.push(PaymentConversion::resource_options(
        from,
        ResourcePile::mood_tokens(1),
        PaymentConversionType::Unlimited,
    ));
    i.info.add_log(
        p,
        "Mood tokens may replace construction resources and culture",
    );
}

fn embalming() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(SpecialAdvance::Embalming,
        SpecialAdvanceRequirement::Advance(Advance::Rituals), "Embalming",
        "After a battle, gain 1 culture token if you have 4 or fewer. Gain 1 culture token when your leader dies or is discarded.")
    .add_simple_persistent_event_listener(|e| &mut e.combat_end, 23, |game, p, s| {
        if s.is_battle() && p.get(game).resources.culture_tokens <= 4 {
            p.gain_resources(game, ResourcePile::culture_tokens(1));
        }
    }).build()
}

pub(crate) const MAN_GOD: [(Advance, Advance); 4] = [
    (Advance::Dogma, Advance::Nationalism),
    (Advance::Devotion, Advance::Totalitarianism),
    (Advance::Conversion, Advance::AbsolutePower),
    (Advance::Fanaticism, Advance::ForcedLabor),
];

pub(crate) fn grants_advance(p: &Player, advance: Advance) -> bool {
    p.has_special_advance(SpecialAdvance::ManGod)
        && MAN_GOD
            .iter()
            .any(|(owned, granted)| *granted == advance && p.has_advance(*owned))
}

fn man_god() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(SpecialAdvance::ManGod,
        SpecialAdvanceRequirement::Advance(Advance::Priesthood), "Man God",
        "Each Theocracy advance also grants the corresponding Autocracy ability, without extra advances or victory points.")
    .add_initializer(|game, p, _| {
        for (owned, granted) in MAN_GOD {
            if p.get(game).has_advance(owned) && !p.get(game).has_advance(granted) && p.get(game).great_library_advance != Some(granted) {
                granted.info(game).listeners.clone().init(game, p.index);
            }
        }
    })
    .add_deinitializer(|game, p| {
        for (owned, granted) in MAN_GOD {
            if p.get(game).has_advance(owned) && !p.get(game).has_advance(granted) && p.get(game).great_library_advance != Some(granted) {
                granted.info(game).listeners.clone().deinit(game, p.index);
            }
        }
    }).build()
}

pub(crate) fn change_government_ability(
    game: &mut Game,
    player: usize,
    advance: Advance,
    gain: bool,
) {
    if !game
        .player(player)
        .has_special_advance(SpecialAdvance::ManGod)
    {
        return;
    }
    if let Some((_, granted)) = MAN_GOD.iter().find(|(a, _)| *a == advance) {
        if game.player(player).great_library_advance == Some(*granted) {
            return;
        }
        let listeners = granted.info(game).listeners.clone();
        if gain {
            listeners.init(game, player);
        } else {
            listeners.deinit(game, player);
        }
    }
}

const BELOVED: &str = "Beloved:";
pub(crate) fn protection(p: &Player, position: Position) -> u8 {
    p.custom_data
        .get(&format!("{BELOVED}{position}"))
        .map_or(0, |v| v.number() as u8)
}

pub(crate) fn remove_protection(game: &mut Game, position: Position) {
    for player in &mut game.players {
        player.custom_data.remove(&format!("{BELOVED}{position}"));
    }
}

fn cleopatra() -> LeaderInfo {
    LeaderInfo::new(Leader::Cleopatra, "Cleopatra",
        LeaderAbility::builder("Legendary", "Cleopatra's city has +1 Cultural Influence range.").build(),
        LeaderAbility::builder("Beloved", "Once per turn, as a free action, place 1 culture token in Cleopatra's city. An attacking player must pay that many culture tokens before moving into the city. Remove these tokens when she dies or is replaced, or the city is destroyed.")
        .add_custom_action(CustomActionType::Beloved,
            |c| c.once_per_turn().free_action().culture_tokens(1),
            |b| b.add_simple_persistent_event_listener(|e| &mut e.custom_action, 0, |game, p, _| {
                let position = leader_position(p.get(game));
                let count = protection(p.get(game), position) + 1;
                p.get_mut(game).custom_data.insert(format!("{BELOVED}{position}"), Data::Number(count as u32));
                p.log(game, &format!("City {position} protected by {count} culture token(s)"));
            }),
            |_, p| p.try_get_city(leader_position(p)).is_some())
        .add_deinitializer(|game, p| p.get_mut(game).custom_data.retain(|k, _| !k.starts_with(BELOVED)))
        .build())
}

fn imhotep() -> LeaderInfo {
    LeaderInfo::new(
        Leader::Imhotep,
        "Imhotep",
        LeaderAbility::builder(
            "Innovator",
            "Gain 1 idea after activating Imhotep's city for an action other than collection.",
        )
        .build(),
        LeaderAbility::builder(
            "Physician",
            "An Academy or Observatory in Imhotep's city costs no food or wood.",
        )
        .add_transient_event_listener(
            |e| &mut e.building_cost,
            13,
            |i, b, game, p| {
                if matches!(b, Building::Academy | Building::Observatory)
                    && i.city_position == Some(leader_position(p.get(game)))
                {
                    i.cost.default.food = 0;
                    i.cost.default.wood = 0;
                    i.info.add_log(
                        p,
                        "Academy or Observatory costs no food or wood in Imhotep's city",
                    );
                }
            },
        )
        .build(),
    )
}

pub(crate) fn innovator(game: &mut Game, position: Position) {
    let player = game.get_any_city(position).player_index;
    let p = game.player(player);
    if p.active_leader() == Some(Leader::Imhotep) && leader_position(p) == position {
        EventPlayer::from_player(
            player,
            game,
            EventOrigin::LeaderAbility("Innovator".to_string()),
        )
        .gain_resources(game, ResourcePile::ideas(1));
    }
}

fn ramses() -> LeaderInfo {
    LeaderInfo::new(Leader::Ramses, "Ramses II",
        LeaderAbility::builder("Monumental", "After constructing an Obelisk in Ramses' city, gain 1 culture token or 2 ideas.")
        .add_resource_request(|e| &mut e.construct, 23, |game, p, c| {
            if c.building != Building::Obelisk || c.city_position != Some(leader_position(p.get(game))) { return None; }
            let mut reward = p.reward_options().sum(1, &[ResourceType::CultureTokens]);
            reward.payment_options.conversions.push(PaymentConversion::limited(ResourcePile::culture_tokens(1), ResourcePile::ideas(2), 1));
            Some(ResourceRewardRequest::new(reward, "Monumental · Gain 1 culture or 2 ideas".to_string()))
        }).build(),
        LeaderAbility::builder("Coastal Power", "Ramses adds 2 combat value when attacking a coastal city, or while aboard a ship battling pirates.")
        .add_combat_strength_listener(113, |game, c, s, role| {
            if !c.has_leader(role, game) { return; }
            let coastal_city = role == CombatRole::Attacker && c.defender_city(game).is_some()
                && c.defender_position().neighbors().iter().any(|p| game.map.is_sea(*p));
            let pirates = c.is_sea_battle(game) && game.player(c.opponent(c.player(role))).civilization.is_pirates();
            if coastal_city || pirates {
                s.extra_combat_value += 2;
                s.roll_log.push("Coastal Power adds +2 combat value".to_string());
            }
        }).build())
}
