use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::discard_action_card;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::card::{HandCard, all_action_hand_cards, HandCardLocation};
use crate::city_pieces::Building::Market;
use crate::content::ability::{AbilityBuilder, building_event_origin};
use crate::content::advances::trade_routes::{TradeRoute, trade_route_log, trade_route_reward};
use crate::content::advances::{AdvanceGroup, AdvanceGroupInfo, advance_group_builder};
use crate::content::civilizations::vikings::add_raid_bonus;
use crate::content::custom_actions::CustomActionType;
use crate::content::custom_actions::CustomActionType::Taxes;
use crate::content::persistent_events::{HandCardsRequest, ResourceRewardRequest};
use crate::game::Game;
use crate::player_events::{PersistentEvent, PersistentEvents};
use crate::resource::{ResourceType, gain_resources};
use crate::resource_pile::ResourcePile;
use crate::special_advance::SpecialAdvance;
use itertools::Itertools;

pub(crate) fn economy() -> AdvanceGroupInfo {
    advance_group_builder(
        AdvanceGroup::Economy,
        "Economy",
        vec![bartering(), trade_routes(), taxes(), currency()],
    )
}

fn currency() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Currency,
        "Currency",
        "You may collect gold instead of food for Trade Routes and Taxes",
    )
    .with_advance_bonus(CultureToken)
}

fn bartering() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Bartering,
        "Bartering",
        "Once per turn, as a free action, \
        you may spend discard an action card for 1 gold or 1 culture token.",
    )
    .with_advance_bonus(MoodToken)
    .add_custom_action(
        CustomActionType::Bartering,
        |c| c.once_per_turn().free_action().no_resources(),
        use_bartering,
        |_game, p| !p.action_cards.is_empty(),
    )
    .with_unlocked_building(Market)
}

fn use_bartering(b: AbilityBuilder) -> AbilityBuilder {
    b.add_hand_card_request(
        |event| &mut event.custom_action,
        1,
        |game, p, _| {
            Some(HandCardsRequest::new(
                all_action_hand_cards(p.get(game)),
                1..=1,
                "Select an action card to discard",
            ))
        },
        |game, s, _e| {
            let HandCard::ActionCard(card) = s.choice[0] else {
                panic!("Invalid type");
            };
            discard_action_card(game, s.player_index, card, &s.origin, HandCardLocation::DiscardPile);
        },
    )
    .add_resource_request(
        |event| &mut event.custom_action,
        0,
        |_game, p, _| {
            Some(ResourceRewardRequest::new(
                p.reward_options()
                    .sum(1, &[ResourceType::Gold, ResourceType::CultureTokens]),
                "Select a resource to gain".to_string(),
            ))
        },
    )
}

fn taxes() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Taxes,
        "Taxes",
        "Once per turn, as an action, you may spend 1 mood token to gain \
            food, wood, or ore equal to the number of cities you control. \
            If you have the Currency advance, you may gain gold instead of food, wood, or ore.",
    )
    .add_custom_action(
        Taxes,
        |c| {
            c.once_per_turn()
                .action()
                .resources(ResourcePile::mood_tokens(1))
        },
        use_taxes,
        |_, _| true,
    )
}

pub(crate) fn use_taxes(b: AbilityBuilder) -> AbilityBuilder {
    b.add_resource_request(
        |event| &mut event.custom_action,
        0,
        |game, p, _| {
            let player = p.get(game);
            let mut c = vec![ResourceType::Food, ResourceType::Wood, ResourceType::Ore];
            if player.can_use_advance(Advance::Currency) {
                c.insert(0, ResourceType::Gold);
            }
            Some(ResourceRewardRequest::new(
                p.reward_options().sum(player.cities.len() as u8, &c),
                "Select a resource to gain".to_string(),
            ))
        },
    )
}

fn trade_routes() -> AdvanceBuilder {
    add_trade_routes(
        AdvanceInfo::builder(
            Advance::TradeRoutes,
            "Trade Routes",
            "At the beginning of your turn, you gain 1 food for every trade route \
        you can make, to a maximum of 4. A trade route is made between one of your \
        Settlers or Ships and a non-Angry enemy player city within 2 spaces \
        (without counting through unrevealed Regions). Each Settler or Ship can only be paired \
        with one enemy player city. Likewise, each enemy player city must be paired with \
        a different Settler or Ship. In other words, to gain X food you must have at least \
        X Units (Settlers or Ships), each paired with X different enemy cities.",
        ),
        |event| &mut event.turn_start,
    )
}

pub(crate) fn add_trade_routes<E, S, V>(b: S, event: E) -> S
where
    E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
    S: AbilityInitializerSetup,
    V: Clone + PartialEq,
{
    b.add_resource_request_with_response(
        event,
        0,
        |game, p, _| {
            if !p.get(game).can_use_advance(Advance::TradeRoutes) {
                return None;
            }

            trade_route_reward(game, p).map(|(reward, routes)| {
                gain_market_bonus(game, &routes);
                ResourceRewardRequest::new(reward, "Collect trade routes reward".to_string())
            })
        },
        |game, s, _| {
            let (_, routes) = trade_route_reward(game, &s.player()).expect("No trade route reward");
            let log = trade_route_log(game, s.player_index, &routes, s.actively_selected);
            for l in &log {
                s.log(game, l);
            }
            let p = game.player(s.player_index);
            if p.has_special_advance(SpecialAdvance::Raiding) {
                add_raid_bonus(game, p.index, &routes);
            }
        },
    )
}

fn gain_market_bonus(game: &mut Game, routes: &[TradeRoute]) {
    let players = routes
        .iter()
        .filter_map(|r| {
            game.try_get_any_city(r.to).and_then(|c| {
                if c.pieces.market.is_some() {
                    Some(c.player_index)
                } else {
                    None
                }
            })
        })
        .unique()
        .collect_vec();
    for p in players {
        gain_resources(
            game,
            p,
            ResourcePile::gold(1),
            building_event_origin(Market),
        );
    }
}
