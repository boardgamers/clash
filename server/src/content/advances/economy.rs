use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::discard_action_card;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::card::{HandCard, all_action_hand_cards};
use crate::city_pieces::Building::Market;
use crate::content::ability::AbilityBuilder;
use crate::content::advances::trade_routes::{TradeRoute, trade_route_log, trade_route_reward};
use crate::content::advances::{AdvanceGroup, advance_group_builder};
use crate::content::custom_actions::CustomActionType::Taxes;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::{HandCardsRequest, ResourceRewardRequest};
use crate::game::Game;
use crate::payment::ResourceReward;
use crate::player::{Player, gain_resources};
use crate::player_events::{PersistentEvent, PersistentEvents};
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;

pub(crate) fn economy() -> AdvanceGroup {
    advance_group_builder(
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
        |game, player_index, _| {
            Some(HandCardsRequest::new(
                all_action_hand_cards(game.player(player_index)),
                1..=1,
                "Select an action card to discard",
            ))
        },
        |game, s, _e| {
            let HandCard::ActionCard(card) = s.choice[0] else {
                panic!("Invalid type");
            };
            game.add_info_log_item(&format!(
                "{} discarded {} for 1 gold or 1 culture token",
                s.player_name,
                game.cache.get_action_card(card).name()
            ));
            discard_action_card(game, s.player_index, card);
        },
    )
    .add_resource_request(
        |event| &mut event.custom_action,
        0,
        |_game, _player_index, _| {
            Some(ResourceRewardRequest::new(
                ResourceReward::sum(1, &[ResourceType::Gold, ResourceType::CultureTokens]),
                "Select a resource to gain".to_string(),
            ))
        },
        |_game, s, _| {
            vec![format!(
                "{} gained {} for discarding an action card",
                s.player_name, s.choice
            )]
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

fn use_taxes(b: AbilityBuilder) -> AbilityBuilder {
    b.add_resource_request(
        |event| &mut event.custom_action,
        0,
        |game, player_index, _| {
            let options = tax_options(game.player(player_index));
            Some(ResourceRewardRequest::new(
                options,
                "Select a resource to gain".to_string(),
            ))
        },
        |_game, s, _| {
            vec![format!(
                "{} gained {} for using Taxes",
                s.player_name, s.choice
            )]
        },
    )
}

#[must_use]
pub fn tax_options(player: &Player) -> ResourceReward {
    let mut c = vec![ResourceType::Food, ResourceType::Wood, ResourceType::Ore];
    if player.can_use_advance(Advance::Currency) {
        c.insert(0, ResourceType::Gold);
    }
    ResourceReward::sum(player.cities.len() as u8, &c)
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
    b.add_resource_request(
        event,
        0,
        |game, player_index, _| {
            if !game
                .player(player_index)
                .can_use_advance(Advance::TradeRoutes)
            {
                return None;
            }

            trade_route_reward(game).map(|(reward, routes)| {
                gain_market_bonus(game, &routes);
                ResourceRewardRequest::new(reward, "Collect trade routes reward".to_string())
            })
        },
        |game, p, _| {
            let (_, routes) = trade_route_reward(game).expect("No trade route reward");
            trade_route_log(
                game,
                p.player_index,
                &routes,
                &p.choice,
                p.actively_selected,
            )
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
        gain_resources(game, p, ResourcePile::gold(1), |name, pile| {
            format!("{name} gained {pile} for using a Market in a trade route")
        });
    }
}
