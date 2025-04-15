use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder};
use crate::city_pieces::Building::Market;
use crate::content::advances::trade_routes::{TradeRoute, trade_route_log, trade_route_reward};
use crate::content::advances::{AdvanceGroup, CURRENCY, advance_group_builder};
use crate::content::custom_actions::CustomActionType::Taxes;
use crate::content::persistent_events::ResourceRewardRequest;
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
    Advance::builder(
        CURRENCY,
        "You may collect gold instead of food for Trade Routes and Taxes",
    )
    .with_advance_bonus(CultureToken)
}

fn bartering() -> AdvanceBuilder {
    Advance::builder("Bartering", "todo")
        .with_advance_bonus(MoodToken)
        .with_unlocked_building(Market)
}

fn taxes() -> AdvanceBuilder {
    Advance::builder(
        "Taxes",
        "Once per turn, as an action, you may spend 1 mood token to gain food, wood, or ore equal to the number of cities you control. If you have the Currency advance, you may gain gold instead of food, wood, or ore.")
        .add_custom_action(Taxes)
}

#[must_use]
pub fn tax_options(player: &Player) -> PaymentOptions {
    let mut c = vec![ResourceType::Food, ResourceType::Wood, ResourceType::Ore];
    if player.has_advance(CURRENCY) {
        c.insert(0, ResourceType::Gold);
    }
    PaymentOptions::sum(player.cities.len() as u32, &c)
}

pub(crate) fn collect_taxes(game: &mut Game, player_index: usize, gain: ResourcePile) {
    assert!(
        tax_options(game.player(player_index)).is_valid_payment(&gain),
        "Invalid gain for Taxes"
    );
    game.players[player_index].gain_resources(gain);
}

fn trade_routes() -> AdvanceBuilder {
    add_trade_routes(
        Advance::builder(
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
