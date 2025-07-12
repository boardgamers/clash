use crate::advance::base_advance_cost;
use crate::content::custom_actions::CustomActionType;
use crate::events::{EventOrigin, check_event_origin};
use crate::game::{Game, GameContext};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::resource_pile::ResourcePile;

#[derive(Clone, Debug, PartialEq)]
pub enum ActionResourceCost {
    Free,
    Resources(ResourcePile),
    Tokens(u8),
    AdvanceCostWithoutDiscount,
}

impl ActionResourceCost {
    #[must_use]
    pub fn free() -> Self {
        ActionResourceCost::Free
    }

    #[must_use]
    pub fn resources(cost: ResourcePile) -> Self {
        if cost.is_empty() {
            return ActionResourceCost::Free;
        }
        ActionResourceCost::Resources(cost)
    }

    #[must_use]
    pub fn tokens(tokens: u8) -> Self {
        if tokens == 0 {
            return ActionResourceCost::Free;
        }
        ActionResourceCost::Tokens(tokens)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionCost {
    pub free: bool,
    pub cost: ActionResourceCost,
}

impl ActionCost {
    pub(crate) fn is_available(&self, game: &Game, player_index: usize) -> Result<(), String> {
        if game.context == GameContext::Replay {
            return Ok(());
        }

        let p = game.player(player_index);
        if !p.can_afford(&self.payment_options(p, check_event_origin())) {
            return Err("Not enough resources for action type".to_string());
        }

        if !(self.free || game.actions_left > 0) {
            return Err("No actions left".to_string());
        }
        Ok(())
    }

    #[must_use]
    pub fn payment_options(&self, player: &Player, origin: EventOrigin) -> PaymentOptions {
        match &self.cost {
            ActionResourceCost::Free => PaymentOptions::free(),
            ActionResourceCost::Resources(c) => {
                PaymentOptions::resources(player, origin, c.clone())
            }
            ActionResourceCost::Tokens(tokens) => PaymentOptions::tokens(player, origin, *tokens),
            ActionResourceCost::AdvanceCostWithoutDiscount => base_advance_cost(player),
        }
    }
}

impl ActionCost {
    #[must_use]
    pub fn new(free: bool, cost: ActionResourceCost) -> Self {
        Self { free, cost }
    }
}

pub(crate) struct ActionCostOncePerTurnBuilder {
    action: CustomActionType,
}

impl ActionCostOncePerTurnBuilder {
    #[must_use]
    pub fn new(action: CustomActionType) -> Self {
        Self { action }
    }

    #[must_use]
    #[allow(clippy::unused_self)]
    pub fn any_times(self) -> ActionCostBuilder {
        ActionCostBuilder::new(None)
    }

    #[must_use]
    pub fn once_per_turn(self) -> ActionCostBuilder {
        ActionCostBuilder::new(Some(self.action))
    }

    #[allow(clippy::unused_self)]
    #[must_use]
    pub fn once_per_turn_mutually_exclusive(
        self,
        mutually_exclusive: CustomActionType,
    ) -> ActionCostBuilder {
        ActionCostBuilder::new(Some(mutually_exclusive))
    }
}

pub(crate) struct ActionCostBuilder {
    once_per_turn: Option<CustomActionType>,
}

impl ActionCostBuilder {
    #[must_use]
    pub(crate) fn new(once_per_turn: Option<CustomActionType>) -> Self {
        Self { once_per_turn }
    }

    #[must_use]
    pub fn action(self) -> ActionResourceCostBuilder {
        ActionResourceCostBuilder::new(self.once_per_turn, false)
    }

    #[must_use]
    pub fn free_action(self) -> ActionResourceCostBuilder {
        ActionResourceCostBuilder::new(self.once_per_turn, true)
    }
}

pub(crate) struct ActionResourceCostBuilder {
    once_per_turn: Option<CustomActionType>,
    free: bool,
}

impl ActionResourceCostBuilder {
    #[must_use]
    fn new(once_per_turn: Option<CustomActionType>, free: bool) -> ActionResourceCostBuilder {
        ActionResourceCostBuilder {
            once_per_turn,
            free,
        }
    }

    #[must_use]
    pub fn no_resources(self) -> ActionCostOncePerTurn {
        ActionCostOncePerTurn::new(self.free, self.once_per_turn, ActionResourceCost::free())
    }

    #[must_use]
    pub fn resources(self, cost: ResourcePile) -> ActionCostOncePerTurn {
        ActionCostOncePerTurn::new(
            self.free,
            self.once_per_turn,
            ActionResourceCost::resources(cost),
        )
    }

    #[must_use]
    pub fn culture_tokens(self, cost: u8) -> ActionCostOncePerTurn {
        self.resources(ResourcePile::culture_tokens(cost))
    }

    #[must_use]
    pub fn tokens(self, cost: u8) -> ActionCostOncePerTurn {
        ActionCostOncePerTurn::new(
            self.free,
            self.once_per_turn,
            ActionResourceCost::tokens(cost),
        )
    }

    #[must_use]
    pub fn advance_cost_without_discounts(self) -> ActionCostOncePerTurn {
        ActionCostOncePerTurn::new(
            self.free,
            self.once_per_turn,
            ActionResourceCost::AdvanceCostWithoutDiscount,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionCostOncePerTurn {
    pub cost: ActionCost,
    pub once_per_turn: Option<CustomActionType>,
}

impl ActionCostOncePerTurn {
    #[must_use]
    pub(crate) fn new(
        free: bool,
        once_per_turn: Option<CustomActionType>,
        cost: ActionResourceCost,
    ) -> ActionCostOncePerTurn {
        ActionCostOncePerTurn {
            cost: ActionCost::new(free, cost),
            once_per_turn,
        }
    }
}
