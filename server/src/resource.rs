use crate::events::EventOrigin;
use crate::game::Game;
use crate::log::{ActionLogItem, add_action_log_item};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::resource_pile::ResourcePile;
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
    game.add_info_log_item(&format!(
        "{} gained {} for {}",
        game.player_name(player),
        resources,
        origin.name(game)
    ));
    let p = game.player_mut(player);

    p.resources += resources.clone();
    apply_resource_limit(p);
    add_action_log_item(game, ActionLogItem::GainResources { resources, origin });
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
            game.add_info_log_item(&format!(
                "{} could not store {wasted_resources}",
                game.player_name(p)
            ));
        }
    }
}

pub(crate) fn lose_resources(
    game: &mut Game,
    player: usize,
    resources: ResourcePile,
    origin: EventOrigin,
) {
    let p = game.player_mut(player);
    assert!(
        p.resources.has_at_least(&resources),
        "player should be able to pay {resources} - got {}",
        p.resources
    );
    p.resources -= resources.clone();
    add_action_log_item(game, ActionLogItem::LoseResources { resources, origin });
}

pub(crate) fn pay_cost(
    game: &mut Game,
    player: usize,
    cost: &PaymentOptions,
    payment: &ResourcePile,
) {
    game.add_info_log_item(&format!(
        "{} paid {} for {}",
        game.player_name(player),
        payment,
        cost.origin.name(game)
    ));

    assert!(cost.can_afford(payment), "invalid payment - got {payment}");
    assert!(
        cost.is_valid_payment(payment),
        "Invalid payment - got {payment} for default cost {}",
        cost.default
    );
    lose_resources(game, player, payment.clone(), cost.origin.clone());
}
