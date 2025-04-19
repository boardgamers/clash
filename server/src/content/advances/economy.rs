use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::discard_action_card;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{AdvanceInfo, AdvanceBuilder, Advance};
use crate::card::HandCard;
use crate::city_pieces::Building::Market;
use crate::content::advances::trade_routes::{TradeRoute, trade_route_log, trade_route_reward};
use crate::content::advances::{AdvanceGroup, CURRENCY, advance_group_builder};
use crate::content::builtin::Builtin;
use crate::content::custom_actions::CustomActionType;
use crate::content::custom_actions::CustomActionType::Taxes;
use crate::content::persistent_events::{HandCardsRequest, ResourceRewardRequest};
use crate::game::Game;
use crate::payment::PaymentOptions;
use crate::player::Player;
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
        CURRENCY,
        "You may collect gold instead of food for Trade Routes and Taxes",
    )
    .with_advance_bonus(CultureToken)
}

const BARTER_DESC: &str = "Once per turn, as a free action, \
        you may spend discard an action card for 1 gold or 1 culture token.";

fn bartering() -> AdvanceBuilder {
    AdvanceInfo::builder(
                Advance::Bartering,
        "Bartering", BARTER_DESC)
        .with_advance_bonus(MoodToken)
        .add_custom_action(CustomActionType::Bartering)
        .with_unlocked_building(Market)
}

pub(crate) fn use_bartering() -> Builtin {
    Builtin::builder(
        "Bartering", BARTER_DESC)
        .add_hand_card_request(
            |event| &mut event.custom_action,
            1,
            |game, player_index, _| {
                let cards = game
                    .player(player_index)
                    .action_cards
                    .iter()
                    .map(|a| HandCard::ActionCard(*a))
                    .collect_vec();

                Some(HandCardsRequest::new(
                    cards,
                    1..=1,
                    "Select an action card to discard",
                ))
            },
            |game, s, _e| {
                let HandCard::ActionCard(card) = s.choice[0] else {
                    panic!("Invalid type");
                };
                game.add_info_log_item(&format!(
                    "{} discarded an action card for 1 gold or 1 culture token",
                    s.player_name
                ));
                discard_action_card(game, s.player_index, card);
            },
        )
        .add_resource_request(
            |event| &mut event.custom_action,
            0,
            |_game, _player_index, _| {
                Some(ResourceRewardRequest::new(
                    PaymentOptions::sum(1, &[ResourceType::Gold, ResourceType::CultureTokens]),
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
        .build()
}

const TAXES_DESCRIPTION: &str = "Once per turn, as an action, you may spend 1 mood token to gain \
        food, wood, or ore equal to the number of cities you control. \
        If you have the Currency advance, you may gain gold instead of food, wood, or ore.";

fn taxes() -> AdvanceBuilder {
    AdvanceInfo::builder(
                Advance::Taxes,
        "Taxes", TAXES_DESCRIPTION).add_custom_action(Taxes)
}

pub(crate) fn use_taxes() -> Builtin {
    Builtin::builder("Taxes", TAXES_DESCRIPTION)
        .add_resource_request(
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
        .build()
}

#[must_use]
pub fn tax_options(player: &Player) -> PaymentOptions {
    let mut c = vec![ResourceType::Food, ResourceType::Wood, ResourceType::Ore];
    if player.has_advance(CURRENCY) {
        c.insert(0, ResourceType::Gold);
    }
    PaymentOptions::sum(player.cities.len() as u32, &c)
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
            if !game.player(player_index).has_advance("Trade Routes") {
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
        let name = game.player_name(p);
        game.add_info_log_item(&format!(
            "{name} gains 1 gold for using a Market in a trade route",
        ));
        game.player_mut(p).gain_resources(ResourcePile::gold(1));
    }
}
