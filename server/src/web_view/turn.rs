use crate::advance::Advance;
use crate::content::advances::trade_routes::next_turn_trade_route_reward;
use crate::events::{EventOrigin, EventPlayer};
use crate::game::Game;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use serde_json::{Value, json};

// This is a forecast from the visible board, not a simulated turn. In particular,
// do not trigger rewards, reveal secrets, or consume randomness while viewing it.
pub(super) fn trade_warning(game: &Game, seat: usize) -> Option<Value> {
    if game.round == 3 && game.age >= game.options.length.ages() {
        return None; // No next turn after the final age.
    }
    let player = game.player(seat);
    let (reward, routes) = next_turn_trade_route_reward(
        game,
        &EventPlayer::new(seat, EventOrigin::Advance(Advance::TradeRoutes)),
    )?;
    let cost = &reward.payment_options;
    let mut room = ResourcePile::empty();
    for resource in ResourceType::all() {
        let space = if resource.is_token() {
            u8::MAX
        } else {
            player
                .resource_limit
                .get(&resource)
                .saturating_sub(player.resources.get(&resource))
        };
        room.add_type(resource, i32::from(space));
    }
    if cost.first_valid_payment(&room).is_some() {
        return None; // Currency and civilization alternatives can avoid waste.
    }
    let waste = |gain: &ResourcePile| {
        let mut excess = ResourcePile::empty();
        for resource in gain.types().into_iter().filter(ResourceType::is_resource) {
            excess.add_type(
                resource,
                i32::from(gain.get(&resource).saturating_sub(room.get(&resource))),
            );
        }
        excess
    };
    let choices = super::decisions::payment_choices(cost, &room, true, false)
        .unwrap_or_else(|| vec![cost.default_payment()]);
    let lost = choices.iter().map(waste).min_by_key(ResourcePile::amount)?;
    (!lost.is_empty()).then(|| json!({"waste":lost,"routes":routes.len()}))
}
