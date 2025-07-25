use crate::content::persistent_events::PaymentRequest;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::log::{ActionLogBalance, ActionLogEntry, add_action_log_item};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{fmt, mem};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy, Hash, Ord, PartialOrd)]
pub enum ResourceType {
    Food,
    Wood,
    Ore,
    Ideas,
    Gold,
    MoodTokens,    // is not a resource, but a token, with no limit
    CultureTokens, // is not a resource, but a token, with no limit
}

impl ResourceType {
    #[must_use]
    pub fn is_token(&self) -> bool {
        matches!(self, ResourceType::MoodTokens | ResourceType::CultureTokens)
    }

    #[must_use]
    pub fn is_resource(&self) -> bool {
        !self.is_token()
    }

    #[must_use]
    pub fn is_gold(&self) -> bool {
        matches!(self, ResourceType::Gold)
    }

    #[must_use]
    pub fn all() -> Vec<ResourceType> {
        vec![
            ResourceType::Food,
            ResourceType::Wood,
            ResourceType::Ore,
            ResourceType::Ideas,
            ResourceType::Gold,
            ResourceType::MoodTokens,
            ResourceType::CultureTokens,
        ]
    }

    #[must_use]
    pub fn resources() -> Vec<ResourceType> {
        vec![
            ResourceType::Food,
            ResourceType::Wood,
            ResourceType::Ore,
            ResourceType::Ideas,
            ResourceType::Gold,
        ]
    }
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ResourceType::Food => write!(f, "Food"),
            ResourceType::Wood => write!(f, "Wood"),
            ResourceType::Ore => write!(f, "Ore"),
            ResourceType::Ideas => write!(f, "Ideas"),
            ResourceType::Gold => write!(f, "Gold"),
            ResourceType::MoodTokens => write!(f, "Mood Tokens"),
            ResourceType::CultureTokens => write!(f, "Culture Tokens"),
        }
    }
}

pub(crate) fn gain_resources(
    game: &mut Game,
    player: usize,
    resources: ResourcePile,
    origin: EventOrigin,
) {
    gain_resources_with_modifiers(game, player, resources, origin, vec![]);
}

pub(crate) fn gain_resources_with_modifiers(
    game: &mut Game,
    player: usize,
    resources: ResourcePile,
    origin: EventOrigin,
    modifiers: Vec<EventOrigin>,
) {
    if modifiers.is_empty() {
        game.log(player, &origin, &format!("Gain {resources}"));
    } else {
        let modifier_names = modifiers
            .iter()
            .map(|m| m.name(game))
            .collect_vec()
            .join(", ");
        game.log(
            player,
            &origin,
            &format!("Gain {resources} with {modifier_names}"),
        );
    }
    let p = game.player_mut(player);

    p.resources += resources.clone();
    apply_resource_limit(p);
    add_action_log_item(
        game,
        player,
        ActionLogEntry::resources(resources, ActionLogBalance::Gain),
        origin,
        modifiers,
    );
}

pub(crate) fn apply_resource_limit(p: &mut Player) {
    let waste = p.resources.apply_resource_limit(&p.resource_limit);
    p.wasted_resources += waste;
}

pub(crate) fn check_for_waste(game: &mut Game) {
    let map: Vec<usize> = game.players.iter().map(|p| p.index).collect();
    for p in map {
        let wasted_resources =
            mem::replace(&mut game.players[p].wasted_resources, ResourcePile::empty());
        if !wasted_resources.is_empty() {
            game.log(
                p,
                &EventOrigin::Ability("Waste".to_string()),
                &format!("Could not store {wasted_resources}",),
            );
        }
    }
}

pub(crate) fn lose_resources(
    game: &mut Game,
    player: usize,
    resources: ResourcePile,
    origin: EventOrigin,
    modifiers: Vec<EventOrigin>,
) {
    let p = game.player_mut(player);
    assert!(
        p.resources.has_at_least(&resources),
        "player should be able to pay {resources} - got {}",
        p.resources
    );
    p.resources -= resources.clone();
    add_action_log_item(
        game,
        player,
        ActionLogEntry::resources(resources, ActionLogBalance::Loss),
        origin,
        modifiers,
    );
}

pub(crate) fn pay_cost(
    game: &mut Game,
    player: usize,
    request: &PaymentRequest,
    payment: &ResourcePile,
) {
    if request.optional && payment.is_empty() {
        log_payment(game, player, &ResourcePile::empty(), &request.cost);
    } else {
        let cost = &request.cost;
        assert!(
            cost.can_afford(payment),
            "invalid payment for {cost:?} - got {payment}"
        );
        assert!(
            cost.is_valid_payment(payment),
            "Invalid payment - got {payment} for default cost {cost:?}",
        );

        log_payment(game, player, payment, cost);
    }
}

fn log_payment(game: &mut Game, player: usize, payment: &ResourcePile, cost: &PaymentOptions) {
    lose_resources(
        game,
        player,
        payment.clone(),
        cost.origin.clone(),
        cost.modifiers.clone(),
    );
    if cost.modifiers.is_empty() {
        game.log(player, &cost.origin, &format!("Pay {payment}"));
    } else {
        let modifiers = cost
            .modifiers
            .iter()
            .map(|m| m.name(game))
            .collect_vec()
            .join(", ");
        game.log(
            player,
            &cost.origin,
            &format!("Pay {payment} with {modifiers}"),
        );
    }
}
